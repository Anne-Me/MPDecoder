use std::collections::HashMap;
use std::vec;

use crate::tanner_graph::TannerGraph;
use crate::tanner_graph::Bit;


pub struct Decoder {
    tanner: TannerGraph
}

impl Decoder {
    pub fn mp_decode(&self, syndrome: &Vec<Bit>, max_rounds: usize) -> Vec<Bit> {
        let mut correction = vec![Bit::ZERO;syndrome.len()];
        
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

        for r in 0..max_rounds  {

            // check candidate correction term
            if self.check_parity_equations(&syndrome, &correction) {
                return correction;
            }

            // create outgoing c messages
            for c in 0..self.tanner.m {
                for (jkey,jval) in &c_in_messages[c] {
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
                for (ikey,_ival) in &v_in_messages[v] {
                    // all incoming messages other than i must be ONE else ZERO
                    let mut i_message = Bit::ONE;
                    let mut ones = 0;
                    let mut zeros = 0;
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
                    // created majority along the way, update correction for next test
                    if ones > zeros {
                        correction[v] = Bit::ONE;
                    } else {
                        correction[v] = Bit::ZERO;
                    }
                }
            }

        }

        // RETURN ERROR
        return correction
    }

    fn check_parity_equations(&self, syndrome: &Vec<Bit>, correction: &Vec<Bit>) -> bool {
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

}



#[cfg(test)]
mod test {
    use std::assert_eq;
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
        assert!(dec.check_parity_equations(&syndrome, &correction));

        syndrome[0]= Bit::ONE;
        correction[2] = Bit::ONE;
        assert!(!dec.check_parity_equations(&syndrome, &correction));

        syndrome[0]= Bit::ZERO;
        syndrome[2] = Bit::ONE;
        correction[4] = Bit::ZERO;
        correction[0] = Bit::ONE;
        assert!(!dec.check_parity_equations(&syndrome, &correction));


        syndrome[0]= Bit::ONE;
        syndrome[1] = Bit::ONE;
        correction[0] = Bit::ONE;
        correction[1] = Bit::ZERO;
        correction[4] = Bit::ONE;
        correction[5] = Bit::ZERO;
        assert!(!dec.check_parity_equations(&syndrome, &correction));
    }

    #[test]
    fn test_check_parity_equations_false() {
        let dec = hamming_7_4();
        let mut syndrome = vec![Bit::ZERO;dec.tanner.m];
        let mut correction = vec![Bit::ZERO;dec.tanner.n];
        assert!(dec.check_parity_equations(&syndrome, &correction));

        syndrome[0]= Bit::ONE;
        correction[4] = Bit::ONE;
        assert!(dec.check_parity_equations(&syndrome, &correction));

        syndrome[0]= Bit::ZERO;
        syndrome[2] = Bit::ONE;
        correction[4] = Bit::ZERO;
        correction[6] = Bit::ONE;
        assert!(dec.check_parity_equations(&syndrome, &correction));


        syndrome[0]= Bit::ONE;
        syndrome[1] = Bit::ONE;
        correction[0] = Bit::ONE;
        correction[1] = Bit::ONE;
        correction[4] = Bit::ONE;
        correction[6] = Bit::ZERO;
        assert!(dec.check_parity_equations(&syndrome, &correction));
    }

}