use crate::board::Board;
use crate::eval::score::Score;
use crate::mv::Move;
use crate::variation::Variation;

// pub use test_log::test;

pub trait Testing {
    fn cp(&self) -> Score;
}

// impl From<&str> for BareMove {
//     fn from(s: &str) -> Self {
//         s.mv()
//     }
// }

// impl From<&str> for BareMoveVariation {
//     fn from(s: &str) -> Self {
//         s.var()
//     }
// }

/// board + "g2g4"
/// board + "g2g4 h7h6"
/// board["g2"] = Some(Pawn)
/// b.var("g2g4 h7h5")
/// b.mv("g2g4 h7h5")

impl Board {
    pub fn var(&self, s: &str) -> Variation {
        self.parse_san_variation(s).unwrap()
    }

    pub fn mv(&self, s: &str) -> Move {
        self.parse_san_move(s).unwrap()
    }
}

impl Testing for &str {
    fn cp(&self) -> Score {
        Score::parse_pgn(self).unwrap()
    }
}

//     fn mv(&self) -> Move {
//         self.parse::<Move>().unwrap()
//     }

//     fn var(&self) -> Variation {
//         Variation::parse_uci(&self).unwrap()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_testing() {
        let b = Board::starting_pos();
        assert_eq!(b.mv("a2a4").to_san(&b), "a4");
        assert_eq!(b.var("a2a4 b7b5 a4a5").to_san(&b), "a4 b5 a5");
    }
}
