use crate::bits::bitboard::Bitboard;
use crate::board::Board;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::piece::{Color, MoveType, Piece, Ply};
use crate::variation::Variation;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::Node;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum AccumulateMethod {
    Power,
    Squared,
    Zero,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum HistoryBoard {
    FromTo,
    PieceTo,
    PieceFromTo,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum ScoreMethod {
    GoodLessBad,
    GoodOverGoodAndBad,
    GoodLessBadOverGoodAndBad,
}

#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    good: i64,
    bad: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryHeuristic {
    enabled: bool,
    age_factor: i64,
    alpha: i64,
    beta: i64,
    malus_factor: i64,
    variation: bool,
    alpha_method: AccumulateMethod,
    beta_method: AccumulateMethod,
    duff_method: AccumulateMethod,
    score_method: ScoreMethod,
    board: HistoryBoard,
    min_depth: Ply,
    max_ply: Ply,

    #[serde(skip)]
    history: Box<[[[[Tally; 64]; 64]; Piece::len()]; 2]>,
    // clear_every_move: bool,
}

impl Component for HistoryHeuristic {
    fn new_game(&mut self) {
        self.adjust_by_factor(0);
    }

    fn new_position(&mut self) {
        self.adjust_by_factor(self.age_factor);
    }
}

impl Default for HistoryHeuristic {
    fn default() -> Self {
        HistoryHeuristic {
            enabled: true,
            // clear_every_move: true,
            min_depth: 4,
            max_ply: 5,
            age_factor: 4,
            malus_factor: 10,
            variation: false,
            alpha: 1,
            beta: 1,
            alpha_method: AccumulateMethod::Squared,
            beta_method: AccumulateMethod::Squared,
            duff_method: AccumulateMethod::Squared,
            score_method: ScoreMethod::GoodLessBadOverGoodAndBad,
            board: HistoryBoard::PieceTo,
            history: Box::new([[[[Tally::default(); 64]; 64]; Piece::len()]; 2]),
        }
    }
}

impl fmt::Display for HistoryHeuristic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "age.factor       : {}", self.age_factor)?;
        // writeln!(f, "clear.every.move : {}", self.clear_every_move)?;
        // for c in Color::ALL {
        //     for p in Piece::ALL {
        //         writeln!(f, "{} {}", c, p.name())?;
        //         for sq in Bitboard::all().flip_vertical().squares() {
        //             write!(f, "{:>7}", self.history[c][p][sq])?;
        //             if sq.file_index() == 7 {
        //                 writeln!(f)?;
        //             }
        //         }
        //     }
        // }
        Ok(())
    }
}

impl HistoryHeuristic {
    fn adjust_by_factor(&mut self, age_factor: i64) {
        for c in Color::ALL {
            for p in Piece::ALL {
                for fr in Bitboard::all().squares() {
                    for to in Bitboard::all().squares() {
                        self.history[c][p][fr][to].bad *= age_factor / 1024;
                        self.history[c][p][fr][to].good *= age_factor / 1024;
                    }
                }
            }
        }
    }

    #[inline]
    pub fn history_heuristic_bonus(&self, c: Color, mv: &Move, _n: &Node, bd: &Board) -> i32 {
        if !self.enabled {
            return 0;
        }
        use HistoryBoard::*;
        let tally = match self.board {
            PieceTo => self.history[c][mv.mover_piece(bd)][0][mv.to()],
            FromTo => self.history[c][0][mv.from()][mv.to()],
            PieceFromTo => self.history[c][mv.mover_piece(bd)][mv.from()][mv.to()],
        };
        use ScoreMethod::*;
        (match self.score_method {
            GoodLessBad => tally.good - tally.bad,
            GoodOverGoodAndBad => {
                (tally.good as f32 / (1 + tally.good + tally.bad) as f32 * 500.0) as i64
            }
            GoodLessBadOverGoodAndBad => {
                100 * (tally.good - tally.bad) / ((1 + tally.good + tally.bad) * 100)
            }
        }) as i32
    }

    #[inline]
    fn get_mut(&mut self, c: Color, mv: Move, bd: &Board) -> &mut Tally {
        if !self.enabled {
            return &mut self.history[c][0][0][0];
        }
        use HistoryBoard::*;
        match self.board {
            PieceTo => &mut self.history[c][mv.mover_piece(bd)][0][mv.to()],
            FromTo => &mut self.history[c][0][mv.from()][mv.to()],
            PieceFromTo => &mut self.history[c][mv.mover_piece(bd)][mv.from()][mv.to()],
        }
    }

    pub fn is_accepted(&self, n: &Node, mv: Move, _mt: MoveType) -> bool {
        if !self.enabled {
            return false;
        }

        if mv.is_null() {
            return false;
        }

        if  n.ply > self.max_ply || n.depth < self.min_depth {
            return false;
        }
        // if mt == MoveType::Hash {
        //     return true;
        // }
        if mv.is_capture() { 
            return false;
        }
        true
    }

    #[inline]
    pub fn raised_alpha(&mut self, n: &Node, b: &Board, mv: Move, mt: MoveType) {
        if !self.is_accepted(n, mv, mt) {
            return;
        }
        use AccumulateMethod::*;
        let add = self.alpha
            * (match self.alpha_method {
                Power => 2 << (n.depth / 4),
                Squared => n.depth * n.depth,
                Zero => 0,
            }) as i64 * if mt == MoveType::Killer { 2 } else { 1 };
        if i64::checked_add(self.get_mut(b.color_us(), mv, b).good, add).is_none() {
            self.adjust_by_factor(2);
        }
        self.get_mut(b.color_us(), mv, b).good += add
    }

    #[inline]
    pub fn beta_variation(&mut self, n: &Node, b: &Board, var: &Variation, mv: Move, mt: MoveType) {
        if !self.is_accepted(n, mv, mt) {
            return;
        }
        self.beta_cutoff(n, b, mv, mt);
        if self.variation {
            for m in var.moves().rev().skip(1).step_by(2).take(3) {
                self.beta_cutoff(n, b, m, mt);
            }
        }
    }

    #[inline]
    fn beta_cutoff(&mut self, n: &Node, b: &Board, mv: Move, mt: MoveType) {
        if !self.is_accepted(n, mv, mt) {
            return;
        }
        use AccumulateMethod::*;
        let add = self.beta
            * (match self.alpha_method {
                Power => 2 << (n.depth / 4),
                Squared => n.depth * n.depth,
                Zero => 0,
            }) as i64 * if mt == MoveType::Killer { 2 } else { 1 };
        if i64::checked_add(self.get_mut(b.color_us(), mv, b).good, 2 * add).is_none() {
            self.adjust_by_factor(2);
        }
        self.get_mut(b.color_us(), mv, b).good += add ;
    }

    #[inline]
    pub fn duff(&mut self, n: &Node, b: &Board, mv: Move, mt: MoveType) {
        if !self.is_accepted(n, mv, mt) {
            return;
        }
        use AccumulateMethod::*;
        let add = (match self.alpha_method {
            Power => (2 << (n.depth / 4)) / self.malus_factor as i32,
            Squared => n.depth * n.depth / self.malus_factor as i32,
            Zero => 0,
        }) as i64 * if mt == MoveType::Killer { 2 } else { 1 };
        if i64::checked_add(self.get_mut(b.color_us(), mv, b).bad, add).is_none() {
            self.adjust_by_factor(2);
        }
        self.get_mut(b.color_us(), mv, b).bad += add
    }
}

#[cfg(test)]
mod tests {
    use crate::bits::square::Square;
    use test_log::test;

    use super::*;

    #[test]
    fn hh_serde_test() {
        let hh = HistoryHeuristic::default();
        let text = toml::to_string(&hh).unwrap();
        info!("toml\n{}", text);
        let hh2: HistoryHeuristic = toml::from_str(&text).unwrap();
        info!("from toml\n{}", hh2);
    }

    #[test]
    fn hh_test() {
        let bd = Board::starting_pos();
        let mut hh = HistoryHeuristic::default();
        hh.get_mut(
            Color::White,
            Move::new_quiet(Piece::Pawn, Square::A2, Square::A3),
            &bd,
        );
        hh.get_mut(
            Color::White,
            Move::new_quiet(Piece::Pawn, Square::A2, Square::A3),
            &bd,
        )
        .good = 1;
        assert_eq!(
            hh.get_mut(
                Color::White,
                Move::new_quiet(Piece::Pawn, Square::A2, Square::A3),
                &bd
            )
            .good,
            1
        );
        hh.new_position();
        hh.new_game();
    }
}
