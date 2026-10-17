
pub struct TannerGraph {
    pub n: usize,
    pub m: usize,
    pub v_nodes: Vec<Vec<usize>>, // for each variable node v, the connections to the c side
    pub c_nodes: Vec<Vec<usize>>, // for each check node c, hte connections to the v side
}

use std::ops::BitXor;
use std::ops::BitAnd;

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
