use std::fmt;

use odonata_base::domain::node::{Event, Node};
use odonata_base::domain::staticeval::StaticEval;
use odonata_base::domain::BoundType;
use odonata_base::eg::{EndGame, LikelyOutcome};
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::Metrics;
use odonata_base::prelude::*;
use serde::{Deserialize, Serialize};

use crate::search::algo::Search;
use crate::search::trail::Trail;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recognizer {
    enabled:        bool,
    min_depth:      Ply,
    terminal_depth: Ply,
}

impl Configurable for Recognizer {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.min_depth.set(p.get("min_depth"))?;
        self.terminal_depth.set(p.get("terminal_depth"))?;
        Ok(p.is_modified())
    }
}

impl Default for Recognizer {
    fn default() -> Self {
        Self {
            enabled:        true,
            min_depth:      -100,
            terminal_depth: 2,
        }
    }
}

impl Component for Recognizer {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl fmt::Display for Recognizer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl Search {
    #[inline]
    pub fn lookup(&mut self, trail: &mut Trail, b: &Board, n: &mut Node) -> (Option<Score>, Option<Move>) {
        if n.ply == 0 {
            return (None, None);
        }

        // let (score, mv) = self.wdl_detection(b, n);
        // if score.is_some() {
        //     return (score,mv);
        // }

        if let Some(entry) = self.tt.probe_by_board(b, n.ply, n.depth) {
            let score = entry.score.as_score(n.ply);
            debug_assert!(score.is_finite());
            Metrics::incr_node(n, Event::TtNodeHit);

            // FIXME! v33
            if entry.depth >= n.depth {
                // if entry.draft >= draft  && (ply >= 1 || self.tt.allow_tt_at_root) && !(b.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats)

                // if n.ply == 0 && self.restrictions.is_none() {
                //     return (Some(entry.score), Some(entry.bm))
                // }

                // if entry.draft >= draft && !(b.repetition_count().total > 1 && self.repetition.avoid_tt_on_repeats)
                match entry.bt {
                    BoundType::ExactPv => {
                        if score >= n.beta {
                            // self.stats.inc_node_cut(n.ply, MoveType::Hash, -1);
                            Metrics::incr_node(n, Event::TtPv);
                            Metrics::incr_node(n, Event::NodeInteriorCut);
                            // self.stats.inc_node_cut(n.ply, MoveType::Hash, -1);
                            // self.stats.inc_leaf_tt_nodes(n.ply);
                            self.report_refutation(n.ply);
                            trail.prune_node(n, score, Event::TtPv);
                            return (Some(score), None);
                        }
                        if score <= n.alpha {
                            Metrics::incr_node(n, Event::TtPv);
                            Metrics::incr_node(n, Event::NodeInteriorAll);
                            // self.stats.inc_node_all(n.ply);
                            // self.stats.inc_leaf_tt_nodes(n.ply);
                            trail.prune_node(n, score, Event::TtPv);
                            return (Some(score), None);
                        }

                        if self.tt.allow_truncated_pv && !self.controller.analyse_mode
                            // && self.mte.time_sensitive()
                            && score > n.alpha
                        {
                            // let _mv = entry.validate_move(b);
                            // trail.push_move(n, mv);
                            trail.terminal(n, score, Event::TtPv);
                            // trail.pop_move(n, mv);
                            // self.stats.inc_leaf_tt_nodes(n.ply);
                            Metrics::incr_node(n, Event::TtPv);
                            return (Some(score), None);
                        }
                        return (None, Some(entry.validate_move(b))); // else we just use the hash move for move ordering
                    }
                    BoundType::LowerCut => {
                        // previously this position raised alpha (sufficiently to cause a cut).
                        // not all child nodes were scored, so score is a lower bound
                        // FIXME: probably dont set alpha just the hinted mv and re-search the node
                        if score >= n.beta {
                            // self.stats.inc_node_cut(n.ply, MoveType::Hash, -1);
                            // self.record_truncated_move(ply, &entry.bm);
                            // self.stats.inc_leaf_tt_nodes(n.ply);
                            self.report_refutation(n.ply);
                            Metrics::incr_node(n, Event::TtCut);
                            Metrics::incr_node(n, Event::NodeInteriorCut);
                            trail.prune_node(n, score, Event::TtCut);
                            return (Some(score), None);
                        }
                        // if self.tt.allow_truncated_pv && entry.score > n.alpha {
                        //     // nt = NodeType::Pv;
                        //     // n.alpha = entry.score;
                        //     self.record_truncated_move(n.ply, &entry.bm);
                        //     // score = entry.score;
                        //     // bm = entry.bm; // need to set bm as alpha raising mv might be skipped
                        //     //             // tt_mv = Some(entry.bm); // might help with move ordering
                        //     return (Some(entry.score), Some(entry.bm)); // else we just use the hash move for move ordering
                        // }
                        return (None, Some(entry.validate_move(b))); // else we just use the hash move for move ordering
                    }
                    BoundType::UpperAll => {
                        // previously this position didnt raise alpha, the score is an upper bound
                        // if the score is still below alpha, this too is an ALL node
                        if score <= n.alpha {
                            // self.record_truncated_move(ply, &entry.bm);
                            // self.stats.inc_leaf_tt_nodes(n.ply);
                            Metrics::incr_node(n, Event::TtAll);
                            Metrics::incr_node(n, Event::NodeInteriorAll);
                            trail.prune_node(n, score, Event::TtAll);
                            return (Some(score), None);
                        }
                    }
                    BoundType::Unused => unreachable!(),
                }
            }
            // not enough draft - just use for move guidance
            return (None, Some(entry.validate_move(b)));
        } else {
            Metrics::incr_node(n, Event::TtNodeMiss);
        }

        // not found
        let (score, mv) = self.wdl_detection(b, n);
        if let Some(s) = score {
            debug_assert!(s.is_finite());
            return (score, mv);
        }

        (None, None)
    }

    #[inline]
    pub fn wdl_detection(&mut self, b: &Board, n: &mut Node) -> (Option<Score>, Option<Move>) {
        if !self.recognizer.enabled || n.depth < self.recognizer.min_depth || n.ply == 0 {
            return (None, None);
        }
        let endgame = EndGame::from_board(b);
        Metrics::inc_endgame(endgame);

        // if b.draw_outcome().is_some()  {
        //     let draw = b.eval_draw(&mut self.eval, &n); // will return a draw score
        //     return (Some(draw), None)
        // }

        // its a helpmate or draw like KNkn, so search just a tiny depth then let eval do its job
        match endgame.likely_outcome(b) {
            LikelyOutcome::DrawImmediate => {
                let draw = self.eval.eval_draw(b, n.ply).pov_score(); // will return a draw score
                                                                      // self.stats.inc_leaf_nodes(n);
                Metrics::incr_node(n, Event::EndgameDraw);
                Metrics::incr_node(n, Event::EndgameHit);
                return (Some(draw), None);
            }

            LikelyOutcome::LikelyDraw | LikelyOutcome::WhiteWin | LikelyOutcome::WhiteLoss => {
                Metrics::incr_node(n, Event::EndgameHit);
                if n.depth > self.recognizer.terminal_depth && !n.is_qs() {
                    n.depth = self.recognizer.terminal_depth;
                }
            }
            lo @ (LikelyOutcome::WhiteWinOrDraw | LikelyOutcome::WhiteLossOrDraw) => {
                Metrics::incr_node(n, Event::EndgameWinOrDraw);
                Metrics::incr_node(n, Event::EndgameHit);
                let draw = self.eval.eval_draw(b, n.ply).pov_score(); // will return a draw score
                if b.color_us() == Color::White && lo == LikelyOutcome::WhiteLossOrDraw
                    || b.color_us() == Color::Black && lo == LikelyOutcome::WhiteWinOrDraw
                {
                    if draw <= n.alpha {
                        // if we are the side that cannot win, fail low, score at most a draw
                        return (Some(draw), None);
                    }
                    n.beta = draw;
                }
                if b.color_us() == Color::White && lo == LikelyOutcome::WhiteWinOrDraw
                    || b.color_us() == Color::Black && lo == LikelyOutcome::WhiteLossOrDraw
                {
                    // if they cannot win, score is at least a draw.
                    if draw >= n.beta {
                        return (Some(draw), None);
                    }
                    n.alpha = draw;
                }
            }
            LikelyOutcome::UnknownOutcome => Metrics::incr_node(n, Event::EndgameMiss),
        }

        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::domain::node::Node;
    use odonata_base::eg::EndGame;
    use odonata_base::Epd;
    use test_log::test;

    use crate::search::engine::ThreadedSearch;

    #[test]
    fn test_recog_simple() {
        let mut engine = ThreadedSearch::new();
        let pos = Epd::parse_epd("k7/1p6/3N4/8/8/8/1K6/B6N w - - 0 1").unwrap();
        let mut n = Node::root(4);
        n.ply = 1;
        let eg = EndGame::from_board(&pos.board());
        let res = engine.search.wdl_detection(&pos.board(), &mut n);
        println!("{:?}\nEndGame: {:?}", res, eg);
    }

    // #[test]
    // fn test_recog_pos() {
    //     // let pos = Position::parse_epd("8/NN6/8/8/8/2K2nk1/4P3/8 w - - 0 1; id 'RECOG.01'; am e2f3; bm Nd6;c0 'white shouldnt take knight as recapture of pawn makes it KNN v k'").unwrap();
    //     let pos =
    //         Position::parse_epd("k7/1p6/3N4/8/8/8/1K6/B6N w - - 0 1; id 'RECOG.02'; bm Nxb7; c0 'white should take pawn to leave KBN v k'")
    //             .unwrap();
    //     // let pos = Position::parse_epd("k7/8/K1p5/8/3N4/8/6N1/7B w - - 5 1; id 'RECOG.03'; am Nxc6; bm Kb6; c0 'white shouldnt take pawn c4 as triggers stalemate'").unwrap();
    //     // let pos = Position::parse_epd("k6K/8/2pN4/8/3N4/8/8/8 w - - 5 1;  id 'RECOG.04'; bm Nxc6; c0 'white should force stalemate'").unwrap();
    //     let mut engine = Engine::new();
    //     engine.set_position(pos.clone());
    //     engine.algo.set_timing_method(TimeControl::Depth(3));
    //     engine.algo.recognizer.enabled = true;
    //     engine.search();
    //     println!("{}", engine.algo.results_as_position());
    //     println!("{}", pos);
    //     assert_eq!(engine.algo.results_as_position().bm().unwrap(), pos.bm().unwrap());
    // }
}

// debug!("{}", sea

// 8/NN6/8/8/8/2K2nk1/4P3/8 w - - 0 1 - white shouldn't take knight
// k7/8/K1p5/8/3N4/8/6N1/7B w - - 5 1 - white shouldn't take pawn c4 as triggers stalemate
