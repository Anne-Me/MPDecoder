use std::collections::HashMap;
use std::unreachable;
use std::vec;
use std::fmt;
use std::write;
use crate::tanner_graph::TannerGraph;
use crate::tanner_graph::Bit;

#[derive(Debug)]
pub enum DecodeError {
    WrongLength { expected: usize, got: usize },
    NotConverged { last_estimate: Vec<Bit> },
    MissingErrorRate,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::WrongLength { expected, got } =>
                write!(f, "wrong syndrome length: expected {expected}, got {got}"),
            DecodeError::NotConverged { last_estimate } =>{
                write!(f, "decoder did not converge, last estimate = ")?;
                    for b in last_estimate {
                        write!(f, "{b}")?;   // uses your new Display for Bit
                    }
                    Ok(())
            }
            DecodeError::MissingErrorRate => 
            write!(f, "Decoding Method LogRation or MinSum Needs the Channel Error Rate as Parameter."),
        }
    }
}

impl std::error::Error for DecodeError {}

pub struct Decoder {
    tanner: TannerGraph
}

#[derive(Clone, Copy, PartialEq)]
pub enum DecoderRule {
    Gallager,
    LogRatio,
    MinSum,
    NormalizedMinSum,
}

impl Decoder {

    pub fn new(tanner: TannerGraph) -> Decoder {
        Decoder{tanner:tanner}
    }

    pub fn decode(&self, syndrome: &Vec<Bit>, variant: DecoderRule, max_rounds: usize, error_rate: Option<f64>, max_weight: Option<usize>) -> Result<Vec<Bit>, DecodeError> {
        let mut correction = vec![Bit::ZERO; self.tanner.n];
        self.validate_length(syndrome)?;

        if error_rate.is_none() && (variant == DecoderRule::LogRatio || variant == DecoderRule::NormalizedMinSum || variant == DecoderRule::MinSum) {
            return Err(DecodeError::MissingErrorRate);
        }

        let p = error_rate.unwrap_or(1.0);
        let prior =((1 as f64 - p) / p).ln();
        let weight_error =max_weight.unwrap_or(self.tanner.n);

        // incoming messages are stored as tuple (from,value)
        let mut v_in_messages = vec![HashMap::new(); self.tanner.n];
        let mut c_in_messages = self.init_priors(prior);

        for _r in 0..max_rounds  {

            // check candidate correction term
            if self.check_parity_equations(&syndrome, &correction, weight_error) {
               return Ok(correction)
            }

            // create outgoing c messages
            for c in 0..self.tanner.m {
                for (jkey,_jval) in &c_in_messages[c] {
                    let mut j_message = 1.0;
                    match variant {
                        DecoderRule::LogRatio => {
                            // outgoing message according to logration rules
                            j_message = self.logratio_update(c, &c_in_messages, syndrome[c], *jkey);
                        },
                        DecoderRule::MinSum => {
                            // outgoing messages according to the minsum rule
                            j_message = self.minsum_update(c, &c_in_messages, syndrome[c], *jkey);
                        },
                        DecoderRule::NormalizedMinSum => {
                            const ALPHA: f64 = 0.8;
                            // outgoing messages according to the minsum rule
                            j_message = self.minsum_update(c, &c_in_messages, syndrome[c], *jkey);
                            j_message = j_message * ALPHA;
                        },
                        _ => unreachable!()
                    }
                    // find place where to put the message ingoing to v_j
                    v_in_messages[*jkey].insert(c,j_message);
                }
            }

            // create outgoing v messages
            for v in 0..self.tanner.n {
                let mut sum = 0.0;
                for (_ikey,ival) in &v_in_messages[v] {
                    sum = sum + *ival;
                }
                for (jkey, jval) in &v_in_messages[v] {
                    c_in_messages[*jkey].insert(v, prior + sum - *jval);
                }
                // prepare next candidate correction alogn the way
                correction[v] = if sum + prior >= 0.0 {Bit::ZERO} else {Bit::ONE};
            }
        }

         // last check
        if self.check_parity_equations(syndrome, &correction, weight_error) {
            return Ok(correction);
        }
        // RETURN ERROR
        return Err(DecodeError::NotConverged { last_estimate: correction })

    }

    fn init_priors(&self,p: f64) -> Vec<HashMap<usize,f64>> {
        let mut c_in_messages = vec![HashMap::new(); self.tanner.m];
        // initialize all messages coming from v with the a priori error probability
        for c in 0..self.tanner.m {
            let mut neighbourhood_messages = HashMap::new();
            for x in &self.tanner.c_nodes[c] {
                neighbourhood_messages.insert(*x,p);
            }
            c_in_messages[c] = neighbourhood_messages;
        }
        return c_in_messages
    }

    fn clip(mut val:f64) -> f64{
        const LMAX: f64 = 25.0;
        if val > LMAX {
            val = LMAX;
        } else if val < - LMAX {
            val = -LMAX;
        }
        return val
    }

    fn logratio_update(&self, c: usize, c_in_messages: &Vec<HashMap<usize,f64>>, syndrome_bit: Bit, own_key: usize) -> f64{
        let mut j_message = 1.0;
        for (xkey,xval) in &c_in_messages[c] { // number of a node
             if *xkey != own_key {
                    j_message = j_message * (Self::clip(*xval)/2.0).tanh();
                }
            }
        // apply sign
        if syndrome_bit == Bit::ZERO {
            j_message = 2.0 * j_message.atanh();
        } else {
            j_message = -2.0 * j_message.atanh();
        }
        return j_message
    }

    fn minsum_update(&self, c: usize, c_in_messages: &Vec<HashMap<usize,f64>>, syndrome_bit: Bit, own_key: usize) -> f64{
        let mut j_message = 1.0;
        let mut sign = 1.0;
        let mut min = f64::MAX;
        for (xkey,xval) in &c_in_messages[c] { 
             if *xkey != own_key {
                    sign = sign * Self::sign(*xval);
                    min = min.min(*&xval.abs())
                }
            }
        // apply sign
        if syndrome_bit == Bit::ZERO {
            j_message = 1.0 * sign * min;
        } else {
            j_message = -1.0 * sign * min;
        }
        return j_message
    }

    fn sign(val: f64) -> f64 {
        if val > 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    pub fn gallager_decode(&self, syndrome: &Vec<Bit>, max_rounds: usize, max_weight: Option<usize>) -> Result<Vec<Bit>, DecodeError> {
        let mut correction = vec![Bit::ZERO; self.tanner.n];
        self.validate_length(syndrome)?;
        let weight_error = max_weight.unwrap_or(self.tanner.n);
        
        // incoming messages are stored as tuple (from,value)
        let mut v_in_messages = vec![HashMap::new(); self.tanner.n];
        let mut c_in_messages = vec![HashMap::new(); self.tanner.m];

        // initialize all messages coming from v to 0 meaning 'no error'
        for c in 0..self.tanner.m {
            let mut neighbourhood_messages = HashMap::new();
            for x in &self.tanner.c_nodes[c] {
                neighbourhood_messages.insert(*x,Bit::ZERO);
            }
            c_in_messages[c] = neighbourhood_messages;
        }

        for _r in 0..max_rounds  {

            // check candidate correction term
            if self.check_parity_equations(&syndrome, &correction, weight_error) {
               return Ok(correction)
            }

            // create outgoing c messages
            for c in 0..self.tanner.m {
                for (jkey,_jval) in &c_in_messages[c] {
                    // outgoing message is the xor of all messages from edges that are not j and xor the syndrome bit
                    let mut j_message = syndrome[c];
                    for (xkey,xval) in &c_in_messages[c] { // number of a node
                        if *xkey != *jkey {
                            j_message = j_message ^ *xval;
                        }
                    }
                    // find place where to put the message ingoing to v_j
                    v_in_messages[*jkey].insert(c,j_message);
                }
            }


            for v in 0..self.tanner.n {
                // correction
                let mut ones = 0;
                let mut zeros = 0;
                for (ikey,_ival) in &v_in_messages[v] {
                    // all incoming messages other than i must be ONE else ZERO
                    let mut i_message = Bit::ONE;
                    if v_in_messages[v].len() == 1 {
                        i_message = Bit::ZERO;
                    }
                    for (xkey,xval) in &v_in_messages[v] {
                        if *xkey != *ikey {
                            i_message = i_message & *xval;
                        }
                        if *xval == Bit::ZERO {
                            zeros = zeros +1;
                        } else {
                            ones = ones + 1;
                        }
                    }
                    c_in_messages[*ikey].insert(v, i_message);
                }
                // created majority along the way, update correction for next test
                if ones > zeros {
                    correction[v] = Bit::ONE;
                } else {
                    correction[v] = Bit::ZERO;
                }
            }
        }

        // last check
        if self.check_parity_equations(syndrome, &correction, weight_error) {
            return Ok(correction);
        }

        // RETURN ERROR
        return Err(DecodeError::NotConverged { last_estimate: correction })
    }


    fn check_parity_equations(&self, syndrome: &Vec<Bit>, correction: &Vec<Bit>, weight_error: usize) -> bool {
        let mut total_weight = 0;
        for i in 0..self.tanner.n {
            if correction[i] == Bit::ONE {
                total_weight = total_weight + 1;
            }
        }
        if total_weight > weight_error {
            return false;
        }

        for check in 0..self.tanner.m {
            let mut res = Bit::ZERO;
            for x in &self.tanner.c_nodes[check] {
                res = res ^ correction[*x];
            }
            if res != syndrome[check] {
                return false;
            }
        }
        return true
    }

    fn validate_length(&self, syndrome: &Vec<Bit>) -> Result<(), DecodeError> {
        if syndrome.len() == self.tanner.m {
            return Ok(());
        }
        return Err(DecodeError::WrongLength { expected: self.tanner.m, got: syndrome.len()})
    }

}



#[cfg(test)]
mod test {
    use std::{assert_eq};
    use super::*;

    // setup default hamming decoder for test
    fn hamming_7_4() -> Decoder {
        Decoder{
        tanner: TannerGraph {
            n: 7,
            m: 3,
            v_nodes: vec![
                vec![0, 1],
                vec![0, 2],
                vec![1, 2],
                vec![0, 1, 2],
                vec![0],
                vec![1],
                vec![2],
            ],
            c_nodes: vec![
                vec![0, 1, 3, 4],
                vec![0, 2, 3, 5],
                vec![1, 2, 3, 6],
            ],
        }
        }
    }   

    #[test]
    fn test_check_parity_equations_true() {
        let dec = hamming_7_4();
        let mut syndrome = vec![Bit::ZERO;dec.tanner.m];
        let mut correction = vec![Bit::ZERO;dec.tanner.n];
        assert!(dec.check_parity_equations(&syndrome, &correction, 1));

        syndrome[0]= Bit::ONE;
        correction[2] = Bit::ONE;
        assert!(!dec.check_parity_equations(&syndrome, &correction, 1));

        syndrome[0]= Bit::ZERO;
        syndrome[2] = Bit::ONE;
        correction[4] = Bit::ZERO;
        correction[0] = Bit::ONE;
        assert!(!dec.check_parity_equations(&syndrome, &correction, 1));


        syndrome[0]= Bit::ONE;
        syndrome[1] = Bit::ONE;
        correction[0] = Bit::ONE;
        correction[1] = Bit::ZERO;
        correction[4] = Bit::ONE;
        correction[5] = Bit::ZERO;
        assert!(!dec.check_parity_equations(&syndrome, &correction, 1));
    }

    #[test]
    fn test_check_parity_equations_false() {
        let dec = hamming_7_4();
        let mut syndrome = vec![Bit::ZERO;dec.tanner.m];
        let mut correction = vec![Bit::ZERO;dec.tanner.n];
        assert!(dec.check_parity_equations(&syndrome, &correction, 7));

        syndrome[0]= Bit::ONE;
        correction[4] = Bit::ONE;
        assert!(dec.check_parity_equations(&syndrome, &correction, 7));

        syndrome[0]= Bit::ZERO;
        syndrome[2] = Bit::ONE;
        correction[4] = Bit::ZERO;
        correction[6] = Bit::ONE;
        assert!(dec.check_parity_equations(&syndrome, &correction, 7));


        syndrome[0]= Bit::ONE;
        syndrome[1] = Bit::ONE;
        correction[0] = Bit::ONE;
        correction[1] = Bit::ONE;
        correction[4] = Bit::ONE;
        correction[6] = Bit::ZERO;
        assert!(dec.check_parity_equations(&syndrome, &correction, 7));
    }

    #[test]
    fn decode_success_1() {
        let dec = hamming_7_4();

        // all 0 error and syndrome
        let mut syndrome = vec![Bit::ZERO;dec.tanner.m];
        let mut exp_correction = vec![Bit::ZERO;dec.tanner.n];
        let res = dec.gallager_decode(&syndrome, 5, None).unwrap();
        assert_eq!(res, exp_correction);

        // syndrome (0,1,0), expected correction (0,0,0,0,0,1,0)
        syndrome[1] = Bit::ONE;
        exp_correction[5] = Bit::ONE;
        let res1 = dec.gallager_decode(&syndrome, 5, None).unwrap();
        assert_eq!(res1, exp_correction);
    }


    #[test]
    fn decode_failure() {
        // syndrome (1,1,1) cannot be decoded
        let dec = hamming_7_4();
        let syndrome = vec![Bit::ONE; dec.tanner.m];
        let exp_correction = vec![Bit::ZERO, Bit::ZERO, Bit::ZERO, Bit::ZERO, Bit::ONE, Bit::ONE, Bit::ONE];

        // without a error threshhold it ouputs the first codeword it finds that produces the syndrome
        //let res = dec.gallager_decode(&syndrome, 10, None).unwrap();
        //assert_eq!(res, exp_correction);

        // with error threshhold it does not converge for this syndrome
        let res = dec.gallager_decode(&syndrome, 10,Some(1));
        assert!(matches!(res, Err(DecodeError::NotConverged {last_estimate: exp_correction}) ));
    }

    #[test]
    fn clip() {
        let var = 12.34;
        let res = Decoder::clip(var);
        assert!((var-res).abs() < 0.000001);

        let var = 25.34;
        let res = Decoder::clip(var);
        assert!((25.0-res).abs() < 0.000001);

        let var = -25.34;
        let res = Decoder::clip(var);
        assert!((-25.00-res).abs() < 0.000001);

        let var = -12.34;
        let res = Decoder::clip(var);
        assert!((var-res).abs() < 0.000001);

    }

    #[test]
    // Log Ration Decoder test
    fn decode_log_ratio_simple_edge_cases() {
        let dec = hamming_7_4();

        // no error
        let syndrome = vec![Bit::ZERO; dec.tanner.m];
        let exp_correction = vec![Bit::ZERO; dec.tanner.n];

        let res = dec.decode(&syndrome, DecoderRule::LogRatio, 10, Some(0.143), None).unwrap();
        assert_eq!(res, exp_correction);

    }
}