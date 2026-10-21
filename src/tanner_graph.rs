
pub struct TannerGraph {
    pub n: usize,
    pub m: usize,
    pub v_nodes: Vec<Vec<usize>>, // for each variable node v, the connections to the c side
    pub c_nodes: Vec<Vec<usize>>, // for each check node c, hte connections to the v side
}

use std::ops::BitXor;
use std::ops::BitAnd;
use std::fmt;

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
