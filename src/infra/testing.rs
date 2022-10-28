use crate::eval::score::Score;
use crate::infra::utils::Uci;
use crate::{domain::info::BareMoveVariation, mv::BareMove};

// pub use test_log::test;


pub trait Testing {
    fn cp(&self) -> Score;
    fn mv(&self) -> BareMove;
    fn var(&self) -> BareMoveVariation;
}


impl From<&str> for BareMove {
    fn from(s: &str) -> Self {
        s.mv()
    }
}

impl From<&str> for BareMoveVariation {
    fn from(s: &str) -> Self {
        s.var()
    }
}


impl Testing for &str {
    fn cp(&self) -> Score {
        Score::parse_pgn(self).unwrap()
    }

    fn mv(&self) -> BareMove {
        self.parse::<BareMove>().unwrap()
    }

    fn var(&self) -> BareMoveVariation {
        BareMoveVariation::parse_uci(&self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;
    use crate::bits::Square;
    use super::*;

    #[test]
    fn test_testing() {
        assert_eq!(
            "a2a4".mv(),
            BareMove {
                from: Square::A2,
                to: Square::A4,
                promo: None
            }
        );
    }
}
