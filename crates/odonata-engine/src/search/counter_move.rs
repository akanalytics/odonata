use std::fmt;
use std::fmt::Debug;

use odonata_base::domain::node::{Counter, Node};
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::Metrics;
use odonata_base::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default)]
struct MoveTally {
    mv1:    Move,
    count1: i64,
    mv2:    Move,
    count2: i64,
    mv3:    Move,
    count3: i64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterMove {
    enabled:          bool,
    clear_every_move: bool,
    age_factor:       i32,
    min_depth:        Ply,
    max_ply:          Ply,

    #[serde(skip, default = "counter_moves_default")]
    counter_moves: Box<[[[MoveTally; 64]; 64]; 2]>,
}

impl Default for CounterMove {
    fn default() -> Self {
        CounterMove {
            enabled:          true,
            clear_every_move: false,
            age_factor:       10,
            min_depth:        4,
            max_ply:          128,
            counter_moves:    counter_moves_default(),
        }
    }
}

impl Configurable for CounterMove {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.clear_every_move.set(p.get("clear_every_move"))?;
        self.age_factor.set(p.get("age_factor"))?;
        self.min_depth.set(p.get("min_depth"))?;
        self.max_ply.set(p.get("max_ply"))?;
        Ok(p.is_modified())
    }
}

impl Debug for CounterMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CounterMove")
            .field("enabled", &self.enabled)
            .field("clear_every_move", &self.clear_every_move)
            .field("age_factor", &self.age_factor)
            .field("min_depth", &self.min_depth)
            .field("max_ply", &self.max_ply)
            .field("counter_moves", &"<omitted>")
            .finish()
    }
}

impl Component for CounterMove {
    fn new_game(&mut self) {
        self.counter_moves = Box::new([[[MoveTally::default(); 64]; 64]; 2]);
    }

    fn new_position(&mut self) {
        if self.clear_every_move {
            self.counter_moves = Box::new([[[MoveTally::default(); 64]; 64]; 2]);
        }
        self.adjust_by_factor(self.age_factor);
    }
}

fn counter_moves_default() -> Box<[[[MoveTally; 64]; 64]; 2]> {
    Box::new([[[MoveTally::default(); 64]; 64]; 2])
}

impl fmt::Display for CounterMove {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl CounterMove {
    pub fn adjust_by_factor(&mut self, age_factor: i32) {
        for c in Color::ALL {
            for p in Piece::ALL {
                for to in Bitboard::all().squares() {
                    self.counter_moves[c][p.index()][to].count1 *= age_factor as i64 / 128;
                    self.counter_moves[c][p.index()][to].count1 *= age_factor as i64 / 128;
                    self.counter_moves[c][p.index()][to].count1 *= age_factor as i64 / 128;
                }
            }
        }
    }

    /// returns a percent 0-100
    #[inline]
    pub fn counter_move_unchecked(&self, c: Color, parent: Move, child: Move, n: &Node) -> i32 {
        if !self.enabled || n.depth < self.min_depth || n.ply > self.max_ply {
            return 0;
        }
        let mt = &self.counter_moves[c][parent.from()][parent.to()];
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
        0
    }

    /// The color is the player of the move being stored
    #[inline]
    pub fn store(&mut self, c: Color, parent: Move, mv: Move, n: &Node, b: &Board) {
        if !self.enabled || mv.is_capture() || mv.is_castle(b) || n.depth < self.min_depth || n.ply > self.max_ply {
            return;
        }

        let mt = &mut self.counter_moves[c][parent.from()][parent.to()];
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
            #[allow(clippy::needless_return)]
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
