use std::collections::HashMap;

use crate::binary_decoding::{DecodeError, Decoder, DecoderRule};
use crate::tanner_graph::{Bit, TannerGraph};
use sparse_bin_mat::{BinNum, SparseBinMat};

pub struct QuantumDecoder {
    DecoderX: Decoder,
    DecoderZ: Decoder,
    hx_echelon: Echelon,
    hz_echelon: Echelon,
}


impl QuantumDecoder {
    pub fn new(tannerx: TannerGraph, tannerz: TannerGraph) -> QuantumDecoder {

        let hx_echelon = tannerx.to_sparse_bin_mat();
        let hz_echelon = tannerz.to_sparse_bin_mat();
        // The Hx matrix in tannerx detects Z errrors and Hz in tannerz detcts X errors
        QuantumDecoder{
            DecoderZ: Decoder::new(tannerx), 
            DecoderX: Decoder::new(tannerz),
            hx_echelon: Echelon::new(hx_echelon),
            hz_echelon: Echelon::new(hz_echelon),
        }
    }

    
    pub fn decode(&self, syndrome_x: &Vec<Bit>, syndrome_z: &Vec<Bit>, variant: DecoderRule, max_rounds: usize, error_rate: Option<f64>, max_weight: Option<usize>) -> Result<(Vec<Bit>,Vec<Bit>), DecodeError> {
        
        let correction_x = self.DecoderX.decode(syndrome_x, variant, max_rounds, error_rate, max_weight)?;
        let correction_z = self.DecoderZ.decode(syndrome_z, variant, max_rounds, error_rate, max_weight)?;

        return Ok((correction_x, correction_z))
    }

    pub fn degenerate_correction(&self, correction_x: &Vec<Bit>, error_x: &Vec<Bit>, correction_z: &Vec<Bit>, error_z: &Vec<Bit>) -> bool {
        self.hx_echelon.in_row_space(error_x, correction_x)
            && self.hz_echelon.in_row_space(error_z, correction_z)
    }

}

pub struct Echelon {
    matrix: SparseBinMat,
    pivots: Vec<usize>,
}

impl Echelon {
    pub fn new(echelon: SparseBinMat) -> Echelon {
        let pivots = echelon.rows().map(|v| v.as_slice()[0]).collect();

        Echelon {
            matrix: echelon,
            pivots,
        }
    }

    fn in_row_space(&self, correction: &Vec<Bit>, error: &Vec<Bit>) -> bool {
        let mut r: Vec<Bit> = error.iter().zip(correction).map(|(&e,&c)| e^c).collect();

        let mut k = 0; // pivot index
        for i in 0..r.len() {
            if r[i] == Bit::ONE {
                while k < self.pivots.len() && self.pivots[k] < i {
                    k += 1;
                }
                if k == self.pivots.len() || self.pivots[k] != i {
                    return false; 
                }
                for &col in self.matrix.row(k).unwrap().as_slice() {
                    r[col] = r[col] ^ Bit::ONE;
                }
            }
        }

        // check if r is zero
        r.iter().all(|&b| b == Bit::ZERO)
    }
}