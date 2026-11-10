use rand::rngs::StdRng;
use rand::RngExt;
use crate::tanner_graph::{Bit, TannerGraph};

pub struct Sample {
    pub error: Vec<Bit>,
    pub syndrome: Vec<Bit>,
}

pub struct QSample {
    pub error_x: Vec<Bit>,
    pub syndrome_x: Vec<Bit>,
    pub error_z: Vec<Bit>,
    pub syndrome_z: Vec<Bit>,
}


// flip every bit independently with probability p and compute the syndrome H*e
pub fn bitflip_sample(tanner: &TannerGraph, p: f64, rng: &mut StdRng) -> Sample {
    let error: Vec<Bit> = (0..tanner.n)
        .map(|_| if rng.random_bool(p) { Bit::ONE } else { Bit::ZERO })
        .collect();
    let syndrome = syndrome_of(tanner, &error);
    Sample { error, syndrome }
}

// given the errors as they would affect the codeword, compute the syndrome
fn syndrome_of(tanner: &TannerGraph, error: &[Bit]) -> Vec<Bit> {
    tanner
        .c_nodes
        .iter()
        .map(|check| check.iter().fold(Bit::ZERO, |acc, &v| acc ^ error[v]))
        .collect()
}

// depolariting noise X, Z or Y with prob p/3
// syndrome_x belongs to error_x and is measured by Hz, syndrome_z by Hx
pub fn depolarizing_sample(hx: &TannerGraph, hz: &TannerGraph, p: f64, rng: &mut StdRng) -> QSample {
    let mut error_x: Vec<Bit> = vec![Bit::ZERO; hx.n];
    let mut error_z: Vec<Bit> = vec![Bit::ZERO; hx.n];
    for i in 0..hx.n {
        // one draw per qubit decides between I, X, Z and Y
        let u: f64 = rng.random();
        if u < p / 3.0 {
            error_x[i] = Bit::ONE;
        } else if u < 2.0 * p / 3.0 {
            error_z[i] = Bit::ONE;
        } else if u < p {
            error_x[i] = Bit::ONE;
            error_z[i] = Bit::ONE;
        }
    }
    let syndrome_x = syndrome_of(hz, &error_x);
    let syndrome_z = syndrome_of(hx, &error_z);
    QSample { error_x, syndrome_x, error_z, syndrome_z }
}


// X errors with prob px and Z errors with prob pz, independently (Y = X and Z with prob px*pz)
// syndrome_x belongs to error_x and is measured by Hz, syndrome_z by Hx
pub fn independent_xz(hx: &TannerGraph, hz: &TannerGraph, px: f64, pz: f64, rng: &mut StdRng) -> QSample {
    let Sample { error: error_x, syndrome: syndrome_x } = bitflip_sample(hz, px, rng);
    let Sample { error: error_z, syndrome: syndrome_z } = bitflip_sample(hx, pz, rng);
    QSample { error_x, syndrome_x, error_z, syndrome_z }
}

// sample from dem file specified probabilities
pub fn dem_sample(tanner: &TannerGraph, priors: &[f64], rng: &mut StdRng) -> Sample {
    let error: Vec<Bit> = priors
        .iter()
        .map(|&p| if rng.random_bool(p) { Bit::ONE } else { Bit::ZERO })
        .collect();
    let syndrome = syndrome_of(tanner, &error);
    Sample {error, syndrome}
}
