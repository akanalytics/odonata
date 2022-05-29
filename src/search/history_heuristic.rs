use crate::bitboard::bitboard::Bitboard;
use crate::board::Board;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::types::{Color, Piece, Ply};
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
    good: i32,
    bad: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryHeuristic {
    enabled: bool,
    scale: i32,
    age_factor: i32,
    malus_factor: i32,
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
            scale: 1,
            min_depth: 4,
            max_ply: 5,
            age_factor: 4,
            malus_factor: 10,
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
        writeln!(f, "scale            : {}", self.scale)?;
        // writeln!(f, "clear.every.move : {}", self.clear_every_move)?;
        // for c in Color::ALL {
        //     for p in Piece::ALL_BAR_NONE {
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
    pub fn adjust_by_factor(&mut self, age_factor: i32) {
        for c in Color::ALL {
            for p in Piece::ALL_BAR_NONE {
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
    pub fn history_heuristic_bonus(&self, c: Color, mv: &Move, n: &Node) -> i32 {
        if !self.enabled || n.depth < self.min_depth || n.ply > self.max_ply {
            return 0;
        }
        use HistoryBoard::*;
        let tally = match self.board {
            PieceTo => self.history[c][mv.mover_piece()][0][mv.to()],
            FromTo => self.history[c][0][mv.from()][mv.to()],
            PieceFromTo => self.history[c][mv.mover_piece()][mv.from()][mv.to()],
        };
        use ScoreMethod::*;
        match self.score_method {
            GoodLessBad => tally.good - tally.bad,
            GoodOverGoodAndBad => {
                (tally.good as f32 / (1 + tally.good + tally.bad) as f32 * 500.0) as i32
            }
            GoodLessBadOverGoodAndBad => {
                100 * (tally.good - tally.bad) / ((1 + tally.good + tally.bad) * 100)
            }
        }
    }

    #[inline]
    fn get_mut(&mut self, c: Color, mv: &Move) -> &mut Tally {
        if !self.enabled {
            return &mut self.history[c][0][0][0];
        }
        use HistoryBoard::*;
        match self.board {
            PieceTo => &mut self.history[c][mv.mover_piece()][0][mv.to()],
            FromTo => &mut self.history[c][0][mv.from()][mv.to()],
            PieceFromTo => &mut self.history[c][mv.mover_piece()][mv.from()][mv.to()],
        }
    }

    #[inline]
    pub fn raised_alpha(&mut self, n: &Node, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() || n.depth < self.min_depth || n.ply > self.max_ply {
            return;
        }
        use AccumulateMethod::*;
        let add = match self.alpha_method {
            Power => 2 << (n.depth / 4),
            Squared => n.depth * n.depth,
            Zero => 0,
        };
        if i32::checked_add(self.get_mut(b.color_us(), mv).good, add).is_none() {
            self.adjust_by_factor(2);
        }
        self.get_mut(b.color_us(), mv).good += add
    }

    #[inline]
    pub fn beta_cutoff(&mut self, n: &Node, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() || n.depth < self.min_depth || n.ply > self.max_ply {
            return;
        }
        use AccumulateMethod::*;
        let add = match self.alpha_method {
            Power => 2 << (n.depth / 4),
            Squared => n.depth * n.depth,
            Zero => 0,
        };
        if i32::checked_add(self.get_mut(b.color_us(), mv).good, add).is_none() {
            self.adjust_by_factor(2);
        }
        self.get_mut(b.color_us(), mv).good += add
    }

    #[inline]
    pub fn duff(&mut self, n: &Node, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() || n.depth < self.min_depth || n.ply > self.max_ply {
            return;
        }
        use AccumulateMethod::*;
        let add = match self.alpha_method {
            Power => (2 << (n.depth / 4)) / self.malus_factor,
            Squared => n.depth * n.depth / self.malus_factor,
            Zero => 0,
        };
        if i32::checked_add(self.get_mut(b.color_us(), mv).bad, add).is_none() {
            self.adjust_by_factor(2);
        }
        self.get_mut(b.color_us(), mv).bad += add
    }
}

#[cfg(test)]
mod tests {
    use crate::bitboard::square::Square;
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
        let mut hh = HistoryHeuristic::default();
        hh.get_mut(
            Color::White,
            &Move::new_quiet(Piece::Pawn, Square::A1, Square::H8),
        );
        hh.get_mut(
            Color::White,
            &Move::new_quiet(Piece::Pawn, Square::A1, Square::H8),
        )
        .good = 1;
        assert_eq!(
            hh.get_mut(
                Color::White,
                &Move::new_quiet(Piece::Pawn, Square::A1, Square::H8)
            )
            .good,
            1
        );
        hh.new_position();
        hh.new_game();
    }
}
