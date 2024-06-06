use std::fmt;

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum BoundType {
    #[default]
    Unused   = 0,
    UpperAll = 1, // All node, score = upperbound ()
    LowerCut = 2, // Cut node, score = lowerbound (we've not looked at all possible scores)
    ExactPv  = 3, // PV node. score is exact
}

impl BoundType {
    #[must_use]
    pub fn unpack_2bits(bits: u64) -> BoundType {
        match bits {
            0 => Self::Unused,
            1 => Self::UpperAll,
            2 => Self::LowerCut,
            3 => Self::ExactPv,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for BoundType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            BoundType::Unused => "UN",
            BoundType::UpperAll => "ALL",
            BoundType::LowerCut => "CUT",
            BoundType::ExactPv => "PV",
        })
    }
}
