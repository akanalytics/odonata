use crate::bound::NodeType;
use crate::cache::tt2::TtNode;
use crate::infra::parsed_config::Component;
use crate::search::node::Node;
use crate::board::Board;
use crate::mv::Move;
use std::{fmt};
use serde::{Deserialize, Serialize};
use crate::search::algo::Algo;
use crate::types::MoveType;
use super::score::Score;
use crate::eval::switches::Switches;




#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Recognizer {
    enabled: bool, 
}


impl Default for Recognizer {
    fn default() -> Self {
        Self {
            enabled: false,
        }
    }
}

impl Component for Recognizer {
    fn new_game(&mut self) {
    }

    fn new_position(&mut self) {
    }
}

impl fmt::Display for Recognizer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}


impl Algo {
    #[inline]
    pub fn lookup(&mut self, b: &mut Board, n: &Node) -> (Option<Score>, Option<Move>) {
        if n.ply == 0 {
            return (None, None)
        }
        if b.draw_outcome().is_some() {
            self.stats.inc_leaf_nodes(n.ply);
            let draw = b.eval_draw(&mut self.eval, &n); // will return a draw score
            return (Some(draw), None)
        }

        if self.tt.probe_leaf_nodes || self.is_leaf(n.ply, n.depth) {

            if let Some(entry) = self.tt.probe_by_board(b, n.ply, n.depth) {

                // FIXME! v33
                if entry.draft >= n.depth && !(self.repetition.avoid_tt_on_repeats && b.repetition_count().total > 0) {
                //if entry.draft >= draft  && (ply >= 1 || self.tt.allow_tt_at_root) && !(b.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats)

                    if n.ply == 0 && self.restrictions.is_none() {
                        return (Some(entry.score), Some(entry.bm))
                    }

                    // if entry.draft >= draft && !(b.repetition_count().total > 1 && self.repetition.avoid_tt_on_repeats)
                    match entry.node_type {
                        NodeType::Pv => {
                            // previously this position raised alpha, but didnt trigger a cut
                            // no point going through moves as we know what the max score is
                            if entry.score >= n.beta {
                                self.stats.inc_node_cut(n.ply, MoveType::Hash, -1);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                self.report_refutation(n.ply);
                                return (Some(entry.score), None);
                            }
                            if entry.score <= n.alpha {
                                self.stats.inc_node_all(n.ply);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                return (Some(entry.score), None);
                            }

                            if self.tt.allow_truncated_pv && entry.score > n.alpha {
                                self.record_truncated_move(n.ply, &entry.bm);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                return (Some(entry.score), None);
                            }
                            return (None, Some(entry.bm)); // else we just use the hash move for move ordering
                        }
                        NodeType::Cut => {
                            // previously this position raised alpha (sufficiently to cause a cut).
                            // not all child nodes were scored, so score is a lower bound
                            // FIXME: probably dont set alpha just the hinted mv and re-search the node
                            if entry.score >= n.beta {
                                self.stats.inc_node_cut(n.ply, MoveType::Hash, -1);
                                // self.record_truncated_move(ply, &entry.bm);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                self.report_refutation(n.ply);
                                return (Some(entry.score), None);
                            }
                            if self.tt.allow_truncated_pv && entry.score > n.alpha {
                                // nt = NodeType::Pv;
                                // n.alpha = entry.score;
                                self.record_truncated_move(n.ply, &entry.bm);
                                // score = entry.score;
                                // bm = entry.bm; // need to set bm as alpha raising mv might be skipped
                                //             // tt_mv = Some(entry.bm); // might help with move ordering
                                return (Some(entry.score), Some(entry.bm)); // else we just use the hash move for move ordering
                            }
                            return (None, Some(entry.bm)); // else we just use the hash move for move ordering
                        }
                        NodeType::All => {
                            // previously this position didnt raise alpha, the score is an upper bound
                            // if the score is still below alpha, this too is an ALL node
                            if entry.score <= n.alpha {
                                // self.record_truncated_move(ply, &entry.bm);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                return (Some(entry.score), None);
                            }
                        }
                        NodeType::Unused | NodeType::Terminal => unreachable!(),
                    }
                }
                // not enough draft - just use for move guidance
                return (None, Some(entry.bm))
            }
            // not found
        }
        // was leaf and isnt probed
        let (score, mv) = self.wdl_detection(b, n);
        if score.is_some() {
            return (score,mv);
        }

        return (None, None);
    }

    #[inline]
    pub fn wdl_detection(&mut self, b: &mut Board, n: &Node) -> (Option<Score>, Option<Move>) {

        if !self.recognizer.enabled  || n.depth == 0 || n.ply == 0 {
            return (None, None)
        }
        let mut win_bonus = Score::zero();
        let pawns = b.pawns();
        if pawns.is_empty() {
            let minors_majors = b.line_pieces() | b.knights();
            let ours = minors_majors & b.us();
            let theirs = minors_majors & b.them();
            if ours.popcount() >= 2 && theirs.is_empty() {
                // our win as we know its not a draw
                win_bonus = Score::from_cp(250);
            } else if ours.is_empty() && theirs.popcount() >= 2 {
                win_bonus = Score::from_cp(-250);
            }

            if win_bonus != Score::zero() {
                let mut score = b.eval_some(&self.eval, Switches::ALL_SCORING);
                // if score.is_mate() {
                //     return (None, None)
                // }
                score = score.clamp_score() + win_bonus;
                
                // let entry = TtNode {
                //     score,
                //     draft: n.depth,
                //     node_type: NodeType::Pv,
                //     bm: Move::NULL_MOVE, 
                // };
                // self.tt.store(b.hash(), entry);
                return (Some(score), None);
            }
        }

        return (None, None)
    }

}

// 8/NN6/8/8/8/2K2nk1/4P3/8 w - - 0 1