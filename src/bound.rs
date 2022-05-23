use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum NodeType {
    Unused = 0,
    UpperAll = 1, // All node, score = upperbound ()
    LowerCut = 2, // Cut node, score = lowerbound (we've not looked at all possible scores)
    ExactPv = 3,  // PV node. score is exact
}

impl NodeType {
    pub fn unpack_2bits(bits: u64) -> NodeType {
        match bits {
            0 => Self::Unused,
            1 => Self::UpperAll,
            2 => Self::LowerCut,
            3 => Self::ExactPv,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NodeType::Unused => "UN",
                NodeType::UpperAll => "ALL",
                NodeType::LowerCut => "CUT",
                NodeType::ExactPv => "PV",
            }
        )
    }
}

impl Default for NodeType {
    #[inline]
    fn default() -> Self {
        Self::Unused
    }
}
