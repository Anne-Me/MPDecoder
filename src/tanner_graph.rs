
pub struct TannerGraph {
    pub n: usize,
    pub m: usize,
    pub v_nodes: Vec<Vec<usize>>, // for each variable node v, the connections to the c side
    pub c_nodes: Vec<Vec<usize>>, // for each check node c, hte connections to the v side
}

impl TannerGraph {
    // parity check matrix H as SparseBinMat: row c holds the variable nodes of check c,
    // sparse_bin_mat needs the positions of each row sorted
    pub fn to_sparse_bin_mat(&self) -> SparseBinMat {
        let rows = self
            .c_nodes
            .iter()
            .map(|check| {
                let mut row = check.clone();
                row.sort_unstable();
                row
            })
            .collect();
        SparseBinMat::new(self.n, rows)
    }
}

use std::ops::BitXor;
use std::ops::BitAnd;
use std::fmt;
use sparse_bin_mat::SparseBinMat;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bit(bool);

impl Bit {
    pub const ZERO: Bit = Bit(false);
    pub const ONE: Bit = Bit(true);
}

impl BitXor for Bit {
    type Output = Bit;
    fn bitxor(self, rhs: Bit) -> Bit {
        Bit(self.0 ^ rhs.0)
    }
}

impl BitAnd for Bit {
    type Output = Bit;
    fn bitand(self, rhs: Bit) -> Bit {
        Bit(self.0 & rhs.0)
    }
}

impl fmt::Display for Bit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Bit::ONE =>
                write!(f, "1"),
            Bit::ZERO =>
                write!(f, "0"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_sparse_bin_mat_hamming_7_4() {
        // check positions deliberately unsorted, the conversion has to sort them
        let tanner = TannerGraph {
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
                vec![4, 0, 3, 1],
                vec![0, 2, 3, 5],
                vec![6, 3, 2, 1],
            ],
        };
        let mat = tanner.to_sparse_bin_mat();

        // H = 1 1 0 1 1 0 0
        //     1 0 1 1 0 1 0
        //     0 1 1 1 0 0 1
        let expected = SparseBinMat::new(7, vec![
            vec![0, 1, 3, 4],
            vec![0, 2, 3, 5],
            vec![1, 2, 3, 6],
        ]);
        assert_eq!(mat, expected);
        assert_eq!(mat.number_of_rows(), 3);
        assert_eq!(mat.number_of_columns(), 7);
        assert_eq!(mat.is_one_at(0, 4), Some(true));
        assert_eq!(mat.is_one_at(0, 2), Some(false));

        // the three checks of the hamming code are independent
        assert_eq!(mat.rank(), 3);
    }
}
