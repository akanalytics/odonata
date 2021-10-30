use crate::bound::NodeType;
use crate::infra::parsed_config::Component;
use crate::search::node::Node;
use crate::board::Board;
use crate::mv::Move;
use std::{fmt};
use serde::{Deserialize, Serialize};
use crate::search::algo::Algo;
use crate::types::MoveType;
use super::score::Score;




#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Recognizer {
}


impl Default for Recognizer {
    fn default() -> Self {
        Self {
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
    pub fn lookup(&mut self, board: &Board, n: &Node) -> (Option<Score>, Option<Move>) {

        if !self.tt.probe_leaf_nodes && self.is_leaf(n.ply, n.depth) {
            return (None, None)
        }

        if let Some(entry) = self.tt.probe_by_board(board, n.ply, n.depth) {
            if n.ply == 0 && self.restrictions.is_none() {
                return (None, Some(entry.bm))
            }

            // FIXME! v33
            if entry.draft >= n.depth && !(board.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats) {
            //if entry.draft >= draft  && (ply >= 1 || self.tt.allow_tt_at_root) && !(board.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats)

                // if entry.draft >= draft && !(board.repetition_count().total > 1 && self.repetition.avoid_tt_on_repeats)
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
            return (None, Some(entry.bm))
        }
        return (None, None)
    }
}