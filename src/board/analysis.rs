use super::Board;
use crate::bits::bitboard::Bitboard;
use crate::types::Color;
use crate::PreCalc;
use static_init::dynamic;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analysis<'a> {
    board: &'a Board,
    pinned: [Bitboard; 2],
    checkers: [Bitboard; 2],
    discoverers: [Bitboard; 2],
    moves: [[Bitboard; 64]; 2],
}

#[dynamic(lazy)]
static EMPTY: Board = Board::default();

impl<'a> Default for Analysis<'a> {
    fn default() -> Self {
        Self {
            board: &EMPTY,
            pinned: Default::default(),
            checkers: Default::default(),
            discoverers: Default::default(),
            moves: [[Bitboard::EMPTY; 64]; 2],
        }
    }
}

impl<'a> Analysis<'a> {
    pub fn of(board: &'a Board) -> Analysis {
        Self {
            board,
            ..Default::default()
        }
    }

    #[inline]
    pub fn king_attacks(&self, c: Color) -> Bitboard {
        PreCalc::default().king_attacks((self.board.kings() & self.board.color(c)).square())
    }

    #[inline]
    pub fn knight_attacks(&self, c: Color) -> Bitboard {
        let mut att = Bitboard::empty();
        let knights = self.board.knights() & self.board.color(c);
        match knights.popcount() {
            0 => att,
            1 => PreCalc::default().knight_attacks(knights.square()),
            2 => {
                PreCalc::default().knight_attacks(knights.first_square())
                    | PreCalc::default().knight_attacks(knights.last_square())
            }
            _ => {
                for sq in knights.squares() {
                    att |= PreCalc::default().knight_attacks(sq)
                }
                att
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::square::Square;
    use crate::infra::profiler::Profiler;
    use crate::{bits::precalc::PreCalc, BoardBuf};
    use test_log::test;

    #[test]
    fn benchmark_king_attacks() {
        let board =
            Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
                .unwrap();
        let analysis = Analysis::of(&board);
        let bbd = PreCalc::default();
        let mut prof = Profiler::new("king_attacks".into());
        for _ in 0..100 {
            prof.start();
            let ka = analysis.king_attacks(Color::White);
            prof.stop();
            assert_eq!(
                ka,
                bbd.within_chebyshev_distance_inclusive(Square::E1, 1) - Bitboard::E1
            );
        }
        assert_eq!(
            analysis.king_attacks(Color::Black),
            bbd.within_chebyshev_distance_inclusive(Square::E8, 1) - Bitboard::E8
        );
    }

    #[test]
    fn bench_knight_attacks() {
        let board =
            Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
                .unwrap();
        let analysis = Analysis::of(&board);
        // let bbd = PreCalc::default();
        let mut prof = Profiler::new("knight_attacks".into());
        for _ in 0..100 {
            prof.start();
            let _ka = analysis.knight_attacks(Color::White);
            prof.stop();
        }
    }
}
