use crate::board::Board;
use crate::eval::score::Score;
use crate::infra::component::Component;
use crate::search::node::Node;
use crate::types::Ply;
use crate::Algo;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::fmt;

use super::node::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Aspiration {
    pub enabled: bool,
    pub min_depth: Ply,
    pub window: Score,
    pub max_iter: i32,
    pub multiplier1: f32,
    pub multiplier2: f32,
}

impl Component for Aspiration {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for Aspiration {
    fn default() -> Self {
        Aspiration {
            enabled: true,
            min_depth: 4,
            window: Score::from_cp(55),
            max_iter: 2,
            multiplier1: 3.2,
            multiplier2: 4.0,
        }
    }
}

impl Algo {
    pub fn aspiration(&mut self, b: &mut Board, n: &mut Node) -> (Score, Event) {
        let score = self.results.best_score;
        if n.depth <= self.aspiration.min_depth || !self.aspiration.enabled || !score.is_numeric() {
            self.counts.inc(n, Event::Aspiration0);
            self.run_alphabeta(b, n)
        } else {
            let mut aspiration_count = 0;
            let mut delta = ((4 + n.ply / 4) * self.aspiration.window.as_i16() as i32) as f32 / 6.0;
            let mut alpha1 = score - Score::from_f32(delta);
            let mut beta1 = score + Score::from_f32(delta);
            let ret = loop {
                aspiration_count += 1;
                if aspiration_count > self.aspiration.max_iter {
                    break self.run_alphabeta(b, n);
                }
                alpha1 = max(n.alpha, alpha1);
                beta1 = min(n.beta, beta1);
                info!("Search window {} {}", alpha1, beta1);
                let mut n1 = Node {
                    alpha: alpha1,
                    beta: beta1,
                    ..*n
                };
                let (new_score, event) = self.run_alphabeta(b, &mut n1);
                if new_score == -Score::INFINITY {
                    // no legal moves available
                    break (new_score, event);
                }
                if new_score.is_mate() {
                    break self.run_alphabeta(b, n);
                }
                delta = match aspiration_count {
                    1 => self.aspiration.multiplier1,
                    _ => self.aspiration.multiplier2,
                } * delta;


                if new_score <= alpha1 && alpha1 > n.alpha {
                    self.counts.inc(n, Event::AspirationFailLow);
                    alpha1 = new_score - Score::from_f32(delta);
                    beta1 = new_score; // beta1; // score;
                } else if new_score >= beta1 && beta1 < n.beta {
                    alpha1 = new_score; // alpha1; //score;
                    self.counts.inc(n, Event::AspirationFailHigh);
                    beta1 = new_score + Score::from_f32(delta);
                } else {
                    // info!("Found {:?} in search window {} {}", new_score, alpha1, beta1);
                    break (new_score, event);
                }
            };
            match aspiration_count {
                1 => self.counts.inc(n, Event::Aspiration1),
                2 => self.counts.inc(n, Event::Aspiration2),
                3 => self.counts.inc(n, Event::Aspiration3),
                _ => self.counts.inc(n, Event::AspirationN),
            }
            ret
        }
    }
}

impl fmt::Display for Aspiration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::search::engine::*;
    use crate::search::timecontrol::*;
    use crate::tags::*;
    use crate::utils::*;

    // use crate::search::timecontrol::*;

    #[test]
    #[ignore]
    fn test_aspiration() {
        let mut engine = Engine::new();

        let positions = &Catalog::example_game()[101..102];
        let mut node_count = 0;
        for pos in positions {
            engine.new_game();
            let suggested_depth = pos.acd().unwrap();
            engine.algo.set_timing_method(TimeControl::Depth(suggested_depth - 1));
            engine.set_position(pos.clone());

            engine.search();
            let mut results = engine.algo.results_as_position().clone();
            let nodes = results.acn().unwrap();
            node_count += nodes;

            // just leave acd
            results.tags_mut().remove(Tag::PV);
            results.tags_mut().remove(Tag::SM);
            results.tags_mut().remove(Tag::BM);
            results.tags_mut().remove(Tag::CE);
            results.tags_mut().remove(Tag::ACN);
            println!("{:>12} {:>12} {}", Formatting::u128(nodes), Formatting::u128(node_count), results);
        }
    }
}
