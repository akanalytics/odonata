use crate::outcome::Outcome;
use std::fmt;



#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Score {
    MinusInf,
    WhiteLoss { ply: i32 }, // WhiteLoss(1) < WhiteLoss(5)
    Cp(i32),
    WhiteWin { minus_ply: i32 }, // // WhiteWin(-5) < WhiteWin(-1)
    PlusInf,
}

impl Default for Score {
    fn default() -> Self { Self::MinusInf }
}



impl Score {
    #[inline]
    pub fn cp(centipawn: i32) -> Score {
        Score::Cp(centipawn)
    }

    /// Outcome must be game ending else panic
    #[inline]
    pub fn from_outcome(o: Outcome, ply: i32) -> Score {
        if o.is_draw() {
            return Score::Cp(0);
        }
        if let Some(c) = o.winning_color() {
            return c.chooser_wb(Score::WhiteWin { minus_ply: -ply }, Score::WhiteLoss { ply });
        }
        panic!("Tried to final score a non-final board outcome:{}", o);
    }

    pub fn is_mate(&self) -> bool {
        matches!(self, Self::WhiteLoss { ply: _ } | Self::WhiteWin { minus_ply: _ })
    }

    #[inline]
    pub fn negate(self) -> Score {
        match self {
            Self::MinusInf => Self::PlusInf,
            Self::WhiteLoss { ply } => Self::WhiteWin { minus_ply: -ply },
            Self::Cp(s) => Self::Cp(-s),
            Self::WhiteWin { minus_ply } => Self::WhiteLoss { ply: -minus_ply },
            Self::PlusInf => Self::MinusInf,
        }
    }
}

impl std::ops::Add for Score {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        if let Score::Cp(s2) = other {
            if let Score::Cp(s1) = self {
                return Score::Cp(s1 + s2);
            } else {
                return self; // if self is an infinite or mate then adding cp/mp makes no difference
            }
        }
        panic!("Can only add centipawns not {} + {}", self, other);
    }
}

impl std::ops::Sub for Score {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        if let Score::Cp(s2) = other {
            if let Score::Cp(s1) = self {
                return Score::Cp(s1 - s2);
            } else {
                return self; // if self is an infinite or mate then subtracting cp/mp makes no difference
            }
        }
        panic!("Can only subtract centipawns not {} - {}", self, other);
    }
}

impl std::ops::Neg for Score {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.negate()
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::MinusInf => f.write_str("-inf"),
            Self::WhiteLoss { ply } => write!(f, "win({})", ply),
            Self::Cp(cp) => write!(f, "{} cp", cp),
            Self::WhiteWin { minus_ply } => write!(f, "loss({})", -minus_ply),
            Self::PlusInf => f.write_str("+inf"),
        }
    }
}

// score config needs to be by colour and by MG/EG
// option to have minimizing nodes use different config
// what can we cache
// pass in alpha beta so eval can short circuit (lazy evaluation)
// some human-like tweaks: aggresive/defensive, open/closed preference, test an opening, lay traps, complicate the position,
// consider odd / even parity and tempo
