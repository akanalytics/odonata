use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::mv::MoveDetail;
use crate::piece::{Color, Piece, Ply};
use crate::Bitboard;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::{Node, Counter};

#[derive(Clone, Copy, Debug, Default)]
struct MoveTally {
    mv1: MoveDetail,
    count1: i64,
    mv2: MoveDetail,
    count2: i64,
    mv3: MoveDetail,
    count3: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CounterMove {
    enabled: bool,
    clear_every_move: bool,
    age_factor: i32,
    min_depth: Ply,
    max_ply: Ply,

    #[serde(skip)]
    counter_moves: Box<[[[MoveTally; 64]; Piece::len()]; 2]>,
}

impl Component for CounterMove {
    fn new_game(&mut self) {
        self.counter_moves = Box::new([[[MoveTally::default(); 64]; Piece::len()]; 2]);
    }

    fn new_position(&mut self) {
        if self.clear_every_move {
            self.counter_moves = Box::new([[[MoveTally::default(); 64]; Piece::len()]; 2]);
        }
        self.adjust_by_factor(self.age_factor);
    }
}

impl Default for CounterMove {
    fn default() -> Self {
        CounterMove {
            enabled: true,
            clear_every_move: false,
            age_factor: 4,
            min_depth: 4,
            max_ply: 5,
            counter_moves: Box::new([[[MoveTally::default(); 64]; Piece::len()]; 2]),
        }
    }
}

impl fmt::Display for CounterMove {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl CounterMove {
    // #[inline]
    // pub fn counter_move_for(&self, b: &Board, parent: Move) -> Option<Move> {
    //     if let Some(m) = self.counter_move_unchecked(b.color_us(), parent) {
    //         if !m.is_null() && b.is_pseudo_legal_move(&m) && b.is_legal_move(&m) {
    //             return Some(m);
    //         }
    //     }
    //     None
    // }

    pub fn adjust_by_factor(&mut self, age_factor: i32) {
        for c in Color::ALL {
            for p in Piece::ALL_BAR_NONE {
                for to in Bitboard::all().squares() {
                    self.counter_moves[c][p][to].count1 *= age_factor as i64 / 128;
                    self.counter_moves[c][p][to].count1 *= age_factor as i64 / 128;
                    self.counter_moves[c][p][to].count1 *= age_factor as i64 / 128;
                }
            }
        }
    }

    /// returns a percent 0-100
    #[inline]
    pub fn counter_move_unchecked(&self, c: Color, parent: MoveDetail, child: MoveDetail, n: &Node) -> i32 {
        if !self.enabled || n.depth < self.min_depth || n.ply > self.max_ply {
            return 0;
        }
        let mt = &self.counter_moves[c][parent.mover_piece()][parent.to()];
        let total = mt.count1 + mt.count2 + mt.count3 + 1;
        if mt.mv1 == child {
            Metrics::incr(Counter::MatchedCounterMove);
            return (100 * mt.count1 / total) as i32;
        }
        if mt.mv2 == child {
            Metrics::incr(Counter::MatchedCounterMove);
            return (100 * mt.count2 / total) as i32;
        }
        if mt.mv3 == child {
            Metrics::incr(Counter::MatchedCounterMove);
            return (100 * mt.count3 / total) as i32;
        }
        return 0;
    }

    /// The color is the player of the move being stored
    #[inline]
    pub fn store(&mut self, c: Color, parent: MoveDetail, mv: MoveDetail, n: &Node) {
        if !self.enabled || mv.is_capture() || mv.is_castle() || n.depth < self.min_depth || n.ply > self.max_ply {
            return;
        }

        let mut mt = &mut self.counter_moves[c][parent.mover_piece()][parent.to()];
        if mt.mv1 == mv {
            mt.count1 += (n.depth * n.depth) as i64;
            return;
        }
        if mt.mv2 == mv {
            mt.count2 += (n.depth * n.depth) as i64;
            return;
        }
        if mt.mv3 == mv {
            mt.count3 += (n.depth * n.depth) as i64;
            return;
        }
        let count = std::cmp::min(std::cmp::min(mt.count1, mt.count2), mt.count3);
        if mt.count1 == count {
            mt.mv1 = mv;
            mt.count1 = (n.depth * n.depth) as i64;
            return;
        }
        if mt.count2 == count {
            mt.mv2 = mv;
            mt.count2 = (n.depth * n.depth) as i64;
            return;
        }
        if mt.count3 == count {
            mt.mv3 = mv;
            mt.count3 = (n.depth * n.depth) as i64;
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;

    #[test]
    fn hh_serde_test() {
        let hh = CounterMove::default();
        let text = toml::to_string(&hh).unwrap();
        info!("toml\n{}", text);
        let hh2: CounterMove = toml::from_str(&text).unwrap();
        info!("from toml\n{}", hh2);
    }
}
