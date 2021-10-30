use crate::eval::material_balance::MaterialBalance;
use crate::infra::parsed_config::Component;
use crate::{Bitboard, Piece, PreCalc};
use crate::board::Board;
use crate::board::boardcalcs::BoardCalcs;
use crate::mv::Move;
use std::cmp;
use serde::{Deserialize, Serialize};




#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Recognizer {
}


impl Default for Recognizer {
    fn default() -> Self {
        Self {
            enabled: true,
        }
    }
}

impl Component for Recognizer {
    fn new_game(&mut self) {
    }

    fn new_position(&mut self) {
    }
}

impl Recognizer {
    pub fn lookup(b: &Board, n: &Node) -> (Option<Score>, Option<Move>) {

    if let Some(entry) = self.tt.probe_by_board(board, n.ply, n.depth) {
        // we use thr tt_mv for ordering regardless of draft
        tt_mv = entry.bm;
        // FIXME! v33
        if entry.draft >= draft && !(board.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats) {
        //if entry.draft >= draft  && (ply >= 1 || self.tt.allow_tt_at_root) && !(board.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats)

        // if entry.draft >= draft && !(board.repetition_count().total > 1 && self.repetition.avoid_tt_on_repeats)
            match entry.node_type {
                NodeType::Pv => {
                    // previously this position raised alpha, but didnt trigger a cut
                    // no point going through moves as we know what the max score is
                    if entry.score >= n.beta {
                        self.stats.inc_node_cut(ply, MoveType::Hash, -1);
                        self.stats.inc_leaf_tt_nodes(ply);
                        self.report_refutation(n.ply);
                        return entry.score;
                    }
                    if entry.score <= n.alpha {
                        self.stats.inc_node_all(ply);
                        self.stats.inc_leaf_tt_nodes(ply);
                        return entry.score;
                    }

                    if self.tt.allow_truncated_pv && entry.score > n.alpha {
                        self.record_truncated_move(ply, &entry.bm);
                        self.stats.inc_leaf_tt_nodes(ply);
                        return entry.score;
                    }
                    // else we just use the hash move for move ordering
                }
                NodeType::Cut => {
                    // previously this position raised alpha (sufficiently to cause a cut).
                    // not all child nodes were scored, so score is a lower bound
                    // FIXME: probably dont set alpha just the hinted mv and re-search the node
                    if entry.score >= n.beta {
                        self.stats.inc_node_cut(ply, MoveType::Hash, -1);
                        self.tt.store(board.hash(), entry);
                        // self.record_truncated_move(ply, &entry.bm);
                        self.stats.inc_leaf_tt_nodes(ply);
                        self.report_refutation(n.ply);
                        return entry.score;
                    }
                    if self.tt.allow_truncated_pv && entry.score > n.alpha {
                        nt = NodeType::Pv;
                        n.alpha = entry.score;
                        self.record_truncated_move(ply, &entry.bm);
                        score = entry.score;
                        bm = entry.bm; // need to set bm as alpha raising mv might be skipped
                                       // tt_mv = Some(entry.bm); // might help with move ordering
                    }
                }
                NodeType::All => {
                    // previously this position didnt raise alpha, the score is an upper bound
                    // if the score is still below alpha, this too is an ALL node
                    if entry.score <= n.alpha {
                        // self.record_truncated_move(ply, &entry.bm);
                        self.stats.inc_leaf_tt_nodes(ply);
                        return entry.score;
                    }
                }
                NodeType::Unused | NodeType::Terminal => unreachable!(),
            }
        }
    }
}