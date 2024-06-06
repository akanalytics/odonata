use std::fmt;

pub use anyhow::{Context as _, Result};
pub use itertools::Itertools;

pub use crate::bits::{Bitboard, Square};
pub use crate::boards::board::Board;
pub use crate::domain::score::{Score, ToScore as _};
pub use crate::domain::timecontrol::TimeControl;
pub use crate::domain::SearchOptions;
pub use crate::infra::math::Math;
pub use crate::infra::param::{Configurable, Param};
pub use crate::infra::utils::{file_create, file_open};
pub use crate::mv::Move;
pub use crate::piece::{Color, FlipSide, FlipVertical, Hash, Piece, Ply};
pub use crate::variation::Variation;
pub use crate::MoveList;

pub struct Displayable<F>(pub F)
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result;

impl<F> fmt::Display for Displayable<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0(f)
    }
}

#[cfg(test)]
pub use test_log::test;

pub mod testing {
    pub use crate::bits::Bitboard;
    pub use crate::domain::score::Score;
    pub use crate::mv::Move;
    pub use crate::prelude::{Board, Displayable};
    pub use crate::variation::Variation;
    pub use crate::MoveList;

    pub trait Testing {
        fn squares(&self) -> Bitboard;
        fn cp(&self) -> Score;
        fn board(&self) -> Board;
        fn moves(&self, b: &Board) -> MoveList;
        fn var(&self, b: &Board) -> Variation;
        fn mv(&self, b: &Board) -> Move;
    }

    impl Testing for &str {
        fn squares(&self) -> Bitboard {
            Bitboard::parse_squares(self).unwrap()
        }

        fn cp(&self) -> Score {
            Score::parse_pgn_pawn_value(self).unwrap()
        }

        fn board(&self) -> Board {
            Board::parse_fen(self).unwrap()
        }

        fn moves(&self, b: &Board) -> MoveList {
            b.parse_san_movelist(self).unwrap()
        }

        fn var(&self, b: &Board) -> Variation {
            b.parse_san_variation(self).unwrap()
        }

        fn mv(&self, b: &Board) -> Move {
            b.parse_san_move(self).unwrap()
        }
    }
}

#[cfg(test)]
pub use testing::*;

#[cfg(test)]
mod tests {
    use test_log::test;

    use crate::prelude::*;

    #[test]
    fn test_testing() {
        let b = &Board::starting_pos();
        assert_eq!("a2a4".mv(b).to_san(b), "a4");
        assert_eq!("a2a4 b7b5 a4a5".var(b).to_san(b), "a4 b5 a5");
    }
}
