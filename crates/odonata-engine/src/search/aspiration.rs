use super::{algo::Search, trail::Trail};
use odonata_base::{
    boards::Position,
    domain::{
        node::{Event, Node},
        score::Score,
    },
    infra::{component::Component, metric::Metrics},
    piece::Ply,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    fmt,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Aspiration {
    enabled:            bool,
    min_depth:          Ply,
    window:             Score,
    max_iter:           i32,
    max_window:         Score,
    multiplier1:        f32,
    multiplier2:        f32,
    change_both_bounds: bool,
    fail_soft:          bool,
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
            enabled:            true,
            min_depth:          4,
            window:             Score::from_cp(100),
            max_iter:           4,
            max_window:         Score::from_cp(1200),
            multiplier1:        4.0,
            multiplier2:        4.0,
            change_both_bounds: false,
            fail_soft:          true,
        }
    }
}

impl Search {
    pub fn aspirated_search(
        &mut self,
        trail: &mut Trail,
        pos: &mut Position,
        n: &mut Node,
        score: Score,
    ) -> Result<(Score, Event), Event> {
        if n.depth <= self.aspiration.min_depth || !self.aspiration.enabled || !score.is_numeric() {
            Metrics::incr_node(n, Event::AspirationNone);
            self.alphabeta_root_search(trail, pos, n)
        } else {
            let mut aspiration_count = 0;
            // let mut delta = ((4 + n.ply / 4) * self.aspiration.window.as_i16() as i32) as f32 / 6.0;
            let mut delta = self.aspiration.window.as_i16() as f32;
            let mut alpha1 = score - Score::from_f32(delta);
            let mut beta1 = score + Score::from_f32(delta);
            let ret = loop {
                aspiration_count += 1;
                if aspiration_count > self.aspiration.max_iter
                    || delta > self.aspiration.max_window.as_i16() as f32
                {
                    break self.alphabeta_root_search(trail, pos, n);
                }
                alpha1 = max(n.alpha, alpha1);
                beta1 = min(n.beta, beta1);
                debug!("Search window {} {}", alpha1, beta1);
                let mut n1 = Node {
                    alpha: alpha1,
                    beta: beta1,
                    ..*n
                };
                let (new_score, event) = self.alphabeta_root_search(trail, pos, &mut n1)?;
                if new_score == -Score::INFINITY {
                    // no legal moves available
                    break Ok((new_score, event));
                }
                if new_score.is_mate() {
                    break self.alphabeta_root_search(trail, pos, n);
                }
                delta *= match aspiration_count {
                    1 => self.aspiration.multiplier1,
                    _ => self.aspiration.multiplier2,
                };

                if new_score <= alpha1 && alpha1 > n.alpha {
                    if self.aspiration.fail_soft {
                        alpha1 = new_score - Score::from_f32(delta);
                    } else {
                        alpha1 = alpha1 - Score::from_f32(delta);
                    }
                    if self.aspiration.change_both_bounds {
                        beta1 = new_score;
                    }
                } else if new_score >= beta1 && beta1 < n.beta {
                    if self.aspiration.change_both_bounds {
                        alpha1 = new_score;
                    }
                    if self.aspiration.fail_soft {
                        beta1 = new_score + Score::from_f32(delta);
                    } else {
                        beta1 = beta1 + Score::from_f32(delta);
                    }
                } else {
                    // info!("Found {:?} in search window {} {}", new_score, alpha1, beta1);
                    break Ok((new_score, event));
                }
            };
            match aspiration_count {
                1 => Metrics::incr_node(n, Event::Aspiration1),
                2 => Metrics::incr_node(n, Event::Aspiration2),
                3 => Metrics::incr_node(n, Event::Aspiration3),
                _ => Metrics::incr_node(n, Event::AspirationN),
            }
            ret
        }
    }
}

impl fmt::Display for Aspiration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    // use crate::search::timecontrol::*;

    // #[test]
    // #[ignore]
    // fn test_aspiration() {
    //     let mut engine = ThreadedSearch::new();

    //     let positions = &Catalog::example_game()[101..102];
    //     let mut node_count = 0;
    //     for pos in positions {
    //         engine.new_game();
    //         let suggested_depth = pos.int("acd").unwrap() as i32;
    //         engine
    //             .search
    //             .set_timing_method(TimeControl::Depth(suggested_depth - 1));
    //         engine.set_position(pos.clone());

    //         engine.search_sync();
    //         let mut results = engine.search.results_as_position().clone();
    //         let nodes = results.int("acn").unwrap();
    //         node_count += nodes;

    //         // just leave acd
    //         results.tags_mut().pv = None;
    //         results.tags_mut().supplied_move = None;
    //         results.tags_mut().best_moves = None;
    //         results.tags_mut().centipawn_evaluation = None;
    //         results.tags_mut().acn = None;
    //         println!("{nodes:>12} {node_count:>12} {results}");
    //     }
    // }
}
