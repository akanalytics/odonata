use std::fmt::Display;

use super::Board;
use crate::bits::bitboard::Bitboard;
use crate::bits::Square;
use crate::piece::Color;
use crate::{Piece, PreCalc};
use itertools::Itertools;
use static_init::dynamic;
use tabled::builder::Builder;

#[dynamic(lazy)]
static EMPTY: Board = Board::default();

#[derive(Debug)]
pub struct Analysis<'a> {
    board: &'a Board,
    attacks: [Bitboard; Square::len()],
}

// todo use occupied.popcount to index.

impl<'a> Default for Analysis<'a> {
    fn default() -> Self {
        Self {
            board: &EMPTY,
            attacks: [(); Square::len()].map(|_| Bitboard::default()),
        }
    }
}

impl Display for Analysis<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut builder = Builder::new();
        writeln!(f, "{}", self.board)?;
        for squares in &self.board.line_pieces().squares().enumerate().chunks(5) {
            let mut row = vec![];
            for (_i, sq) in squares {
                row.push(format!("{}\n{:#}", sq, self.attacks[sq]));
            }
            builder.add_record(row);
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
            attacks: [(); Square::len()].map(|_| Bitboard::default()),
        };
        let occ = b.occupied();

        let us = Bitboard::empty();
        for sq in b.line_pieces().squares() {
            let c = if sq.is_in(b.white()) {
                Color::White
            } else {
                Color::Black
            };
            let p = b.piece_unchecked(sq);
            me.attacks[sq] = bb.attacks(c, p, us, occ, sq);
        }
        me
    }
}

impl<'a> Analysis<'a> {
    #[inline]
    pub fn knight_attacks(&self, knights: Bitboard) -> Bitboard {
        (knights - self.board.pinned(Color::White) - self.board.pinned(Color::Black))
            .squares()
            .fold(Bitboard::empty(), |a, sq| {
                a | PreCalc::default().knight_attacks(sq)
            })
    }

    #[inline]
    pub fn king_attacks(&self, kings: Bitboard) -> Bitboard {
        kings.squares().fold(Bitboard::empty(), |a, sq| {
            a | PreCalc::default().king_attacks(sq)
        })
    }

    #[inline]
    pub fn attacks_and_defends_from(&self, sq: Square) -> Bitboard {
        match self.board.piece_unchecked(sq) {
            Piece::Bishop | Piece::Rook | Piece::Queen => self.attacks[sq],
            Piece::Knight => PreCalc::default().knight_attacks(sq),
            Piece::King => PreCalc::default().king_attacks(sq),
            Piece::Pawn => {
                PreCalc::default().pawn_attacks_from_sq(self.board.color_of_unchecked(sq), sq)
            }
        }
    }

    #[inline]
    pub fn all_attacks_from(&self, from_region: Bitboard) -> Bitboard {
        from_region
            .squares()
            .fold(Bitboard::EMPTY, |a, sq| a | self.attacks[sq])
            | self.knight_attacks(self.board.knights() & from_region)
            | self.king_attacks(self.board.kings() & from_region)
            | PreCalc::default().pawn_attacks_from(
                Color::White,
                self.board.pawns() & from_region & self.board.white(),
            )
            | PreCalc::default().pawn_attacks_from(
                Color::Black,
                self.board.pawns() & from_region & self.board.black(),
            )
    }

    // pub fn attackers_of(&self, to: Square) -> Bitboard {
    //     self.board
    //         .line_pieces()
    //         .squares()
    //         .fold(Bitboard::empty(), |attackers, from| {
    //             attackers | from.as_bb().iff(to.is_in(self.attacks[from]))
    //         })
    //         | PreCalc::default().knight_attacks(to) & self.board.knights()
    //         | PreCalc::default().king_attacks(to) & self.board.kings()
    //         | PreCalc::default().pawn_attackers(to.as_bb(), Color::White)
    //         | PreCalc::default().pawn_attackers(to.as_bb(), Color::Black)
    // }

    #[inline]
    pub fn slider_attacks(&self, c: Color, p: Piece) -> Bitboard {
        self.board.pieces(p)
            & self
                .board
                .color(c)
                .squares()
                .fold(Bitboard::EMPTY, |a, sq| a | self.attacks[sq])
                - self.board.color(c)
    }

    // #[inline]
    // fn slider_defends(&self, c: Color, p: Piece) -> Bitboard {
    //     self.board.pieces(p)
    //         & self
    //             .board
    //             .color(c)
    //             .squares()
    //             .fold(Bitboard::EMPTY, |a, sq| a | self.attacks[sq])
    //         & self.board.color(c)
    // }
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
    use crate::infra::profiler::PerfProfiler;
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
        let mut prof = PerfProfiler::new("king_attacks".into());
        for _ in 0..100 {
            prof.benchmark(|| analysis.king_attacks(b.kings() & b.white()));
            // assert_eq!(
            //     ka,
            //     bbd.within_chebyshev_distance_inclusive(Square::E1, 1) - Bitboard::E1
            // );
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
        let mut prof = PerfProfiler::new("knight_attacks".into());
        for _ in 0..100 {
            prof.benchmark(|| analysis.knight_attacks(b.knights() & b.white()));
        }
    }
}
