use std::fmt::Display;

use super::Board;
use crate::bits::bitboard::Bitboard;
use crate::bits::Square;
use crate::piece::Color;
use crate::PreCalc;
use itertools::Itertools;
use static_init::dynamic;
use tabled::builder::Builder;

#[dynamic(lazy)]
static EMPTY: Board = Board::default();

#[derive(Debug)]
pub struct Analysis<'a> {
    board: &'a Board,

    sliders: Bitboard,
    attacks: [Bitboard; Square::len()],
}

// todo use occupied.popcount to index.

impl<'a> Default for Analysis<'a> {
    fn default() -> Self {
        Self {
            board: &EMPTY,
            sliders: Bitboard::empty(),
            attacks: [(); Square::len()].map(|_| Bitboard::default()),
        }
    }
}

impl Display for Analysis<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut builder = Builder::new();
        writeln!(f, "{}", self.board);
        for squares in &self.sliders.squares().enumerate().chunks(5) {
            let mut row = vec![];
            for (_i, sq) in squares {
                row.push(format!("{}\n{:#}", sq, self.attacks[sq]));
            }
            builder = builder.add_record(row);
        }
        let tab = builder.build();
        tab.fmt(f)?;
        Ok(())
    }
}

impl<'a> Analysis<'a> {
    #[inline]
    pub fn of(b: &'a Board) -> Self {
        let bb = PreCalc::default();
        let mut me = Self {
            board: &b,
            sliders: b.line_pieces(),
            attacks: [(); Square::len()].map(|_| Bitboard::default()),
        };
        let occ = b.occupied();

        let us = Bitboard::empty();
        for sq in b.occupied().squares() {
            let c = if sq.is_in(b.white()) {
                Color::White
            } else {
                Color::Black
            };
            let p = b.piece_at(sq.as_bb());
            me.attacks[sq] = bb.attacks(c, p, us, occ, sq);
        }
        me
    }
}

impl<'a> Analysis<'a> {
    pub fn knight_attacks(&self, knights: Bitboard) -> Bitboard {
        knights.squares().fold(Bitboard::empty(), |a, sq| {
            a | PreCalc::default().knight_attacks(sq)
        })
    }

    pub fn king_attacks(&self, kings: Bitboard) -> Bitboard {
        PreCalc::default().king_attacks(kings.first_square())
    }

    pub fn slider_attacks(&self, sliders: Bitboard) -> Bitboard {
        sliders
            .squares()
            .fold(Bitboard::EMPTY, |a, sq| a | self.attacks[sq])
    }
}

// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct Analysis<'a> {
//     pinned: [Bitboard; 2],
//     checkers: [Bitboard; 2],
//     discoverers: [Bitboard; 2],
//     moves: [[Bitboard; 64]; 2],
// }

// impl<'a> Default for Analysis<'a> {
//     fn default() -> Self {
//         Self {
//             board: &EMPTY,
//             pinned: Default::default(),
//             checkers: Default::default(),
//             discoverers: Default::default(),
//             moves: [[Bitboard::EMPTY; 64]; 2],
//         }
//     }
// }

// impl<'a> Analysis<'a> {
//     pub fn of(board: &'a Board) -> Analysis {
//         Self {
//             board,
//             ..Default::default()
//         }
//     }

//     #[inline]
//     pub fn king_attacks(&self, c: Color) -> Bitboard {
//         PreCalc::default().king_attacks((self.board.kings() & self.board.color(c)).square())
//     }

//     #[inline]
//     pub fn knight_attacks(&self, c: Color) -> Bitboard {
//         let mut att = Bitboard::empty();
//         let knights = self.board.knights() & self.board.color(c);
//         match knights.popcount() {
//             0 => att,
//             1 => PreCalc::default().knight_attacks(knights.square()),
//             2 => {
//                 PreCalc::default().knight_attacks(knights.first_square())
//                     | PreCalc::default().knight_attacks(knights.last_square())
//             }
//             _ => {
//                 for sq in knights.squares() {
//                     att |= PreCalc::default().knight_attacks(sq)
//                 }
//                 att
//             }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::precalc::PreCalc;
    use crate::bits::square::Square;
    use crate::catalog::Catalog;
    use crate::infra::profiler::Profiler;
    use test_log::test;

    #[test]
    fn test_analysis() {
        // let bb = PreCalc::default();
        let b = Catalog::starting_board();
        let analysis = Analysis::of(&b);
        println!("{analysis}");
    }

    #[test]
    fn benchmark_king_attacks() {
        let b = Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
            .unwrap();
        let analysis = Analysis::of(&b);
        let bbd = PreCalc::default();
        let mut prof = Profiler::new("king_attacks".into());
        for _ in 0..100 {
            prof.start();
            let ka = analysis.king_attacks(b.kings() & b.white());
            prof.stop();
            assert_eq!(
                ka,
                bbd.within_chebyshev_distance_inclusive(Square::E1, 1) - Bitboard::E1
            );
        }
        assert_eq!(
            analysis.king_attacks(b.kings() & b.black()),
            bbd.within_chebyshev_distance_inclusive(Square::E8, 1) - Bitboard::E8
        );
    }

    #[test]
    fn bench_knight_attacks() {
        let b = Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
            .unwrap();
        let analysis = Analysis::of(&b);
        // let bbd = PreCalc::default();
        let mut prof = Profiler::new("knight_attacks".into());
        for _ in 0..100 {
            prof.start();
            let _ka = analysis.slider_attacks(b.knights() & b.white());
            prof.stop();
        }
    }
}
