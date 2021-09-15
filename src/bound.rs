use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum NodeType {
    Unused = 0,
    All = 1,      // All node, score = upperbound ()
    Cut = 2,      // Cut node, score = lowerbound (we've not looked at all possible scores)
    Pv = 3,       // PV node. score is exact
    Terminal = 4, // no legal moves from this node
}

impl NodeType {
    pub fn unpack_2bits(bits: u64) -> NodeType {
        match bits {
            0 => Self::Unused,
            1 => Self::All,
            2 => Self::Cut,
            3 => Self::Pv,
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
                NodeType::Terminal => "TE",
                NodeType::All => "AU",
                NodeType::Cut => "CL",
                NodeType::Pv => "PV",
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
