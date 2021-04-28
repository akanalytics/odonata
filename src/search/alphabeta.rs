use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::eval::eval::Scorable;
use crate::eval::score::Score;
use crate::movelist::Move;
use crate::search::algo::Algo;
use crate::search::searchprogress::SearchProgress;
use crate::tt::{Entry, NodeType};
use crate::types::{Color, Ply};
use crate::outcome::GameEnd;

pub struct AlphaBeta;

impl Algo {
    pub fn sigma(b: &Board) -> i32 {
        if b.color_us() == Color::White {
            1
        } else {
            -1
        }
    }

    pub fn alphabeta_recursive2(
        &mut self,
        board: &mut Board,
        ply: Ply,
        mut alpha: Score,
        beta: Score,
        last_move: &Move,
    ) -> Score {
        debug_assert!(self.max_depth > 0);
        if self.search_stats.total().nodes() % 1000000 == 0 && self.search_stats.total().nodes() != 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            self.task_control.invoke_callback(&sp);
        }


        if self.time_up_or_cancelled(ply, false) {
            return Score::MinusInf;
        }

        if board.draw_outcome().is_some() {
            self.search_stats.inc_leaf_nodes(ply);
            return Self::sigma(board) * board.eval(&self.eval); // will return a draw score
        }

        let mut score = Score::MinusInf;
        let mut bm = Move::NULL_MOVE;
        let mut node_type = NodeType::All;

        // FIXME tt probe for leaves?
        if self.is_leaf(ply) {
            let score = self.qsearch2(last_move, ply, board, alpha, beta);
            assert!(self.task_control.is_cancelled() || score > Score::MinusInf);
            return score;
        }

        if self.tt.enabled() {
            // FIXME avoid the cloned!
            if let Some(entry) = self.tt.probe_by_board(board).cloned() {
                let depth = self.max_depth - ply;
                if entry.depth >= depth {
                    self.search_stats.inc_tt_nodes(ply);
                    //println!("Entry:{:?}", entry);
                    // for bounded scores, we know iterating through the nodes might raise alpha, lower beta
                    // doing this now allows us potentuially to cut off without looking at the child nodes
                    match entry.node_type {
                        NodeType::Pv => {
                            // previously this position raised alpha, but didnt trigger a cut
                            // no point going through moves as we know what the max score is
                            if entry.score > alpha {
                                self.record_new_pv(ply, &entry.bm, true);
                            }
                            return entry.score
                        }
                        NodeType::Cut => {
                            // previously this position raised alpha (sufficiently to cause a cut).
                            // not all child nodes were scored, so score is a lower bound
                            if entry.score > alpha {
                                self.record_new_pv(ply, &entry.bm, true);
                                node_type = NodeType::Pv;
                                alpha = entry.score; 
                                if alpha >= beta {
                                    // self.record_new_pv(ply, &bm);
                                    return entry.score;
                                }
                                score = entry.score;
                                bm = entry.bm; // might help with move ordering
                            }
                        }
                        NodeType::All => {
                            // previously this position didnt raise alpha, the score is an upper bound
                            // if the score is still below alpha, this too is an ALL node
                            if alpha >= entry.score {
                                // self.record_new_pv(ply, &bm);
                                return entry.score;
                            }
                            // beta = cmp::min(beta, entry.score);
                        }
                        NodeType::Unused => panic!("Node type Unused returned on tt probe"),
                    }
                }
            }
        }

        self.search_stats.inc_interior_nodes(ply);

        // // FIXME!!!!
        // if self.max_depth > self.search_stats.selective_depth() {
        //     let sp = SearchProgress::from_search_stats(&self.search_stats());
        //     self.task_control.invoke_callback(&sp);
        // }
        // bailing here means the score is +/- inf and wont be used
        // FIXME!

        let mut moves = board.legal_moves();
        if moves.is_empty() {
            self.search_stats.inc_leaf_nodes(ply);
            return Self::sigma(board) * board.eval(&self.eval);
        }

        self.order_moves(ply, &mut moves);
        let original_alpha = alpha;
        for (_i, mv) in moves.iter().enumerate() {
            let mut child_board = board.make_move(mv);
            self.repetition.push(&mv, &child_board);
            child_board.set_repetition_count(self.repetition.count(&child_board));
            debug_assert!(alpha < beta || self.minmax);
            self.current_variation.set_last_move(ply + 1, mv);

            let child_score = -self.alphabeta_recursive2(&mut child_board, ply + 1, -beta, -alpha, &mv);
            if self.task_control.is_cancelled() {
                return Score::MinusInf;
            }
            assert!(child_score > Score::MinusInf);
            board.undo_move(mv);
            self.repetition.pop();

            if child_score > score {
                score = child_score;
            }
            if child_score > alpha {
                alpha = child_score;
                bm = *mv;
                node_type = NodeType::Pv;
                self.record_new_pv(ply, &bm, false);
            }

            if alpha >= beta && !self.minmax {
                node_type = NodeType::Cut;
                break;
            }
        }

        if self.tt.enabled() {
            if score <= original_alpha {
                // node_type = NodeType::All
            } else if score >= beta {
                // node_type = NodeType::Cut;
                debug_assert!(!bm.is_null())
            } else {
                // node_type = NodeType::Pv;
                debug_assert!(!bm.is_null())
            }
            let entry = Entry {
                hash: board.hash(),
                score,
                depth: self.max_depth - ply,
                node_type,
                bm, // not set for NodeType::All
            };
            self.tt.insert(entry);
        }
        self.current_variation.set_last_move(ply, &Move::NULL_MOVE);
        score
    }

    fn record_new_pv(&mut self, ply: Ply, mv: &Move, terminal_move: bool) {
        self.pv_table.set(ply + 1, mv, terminal_move);
        self.pv_table.propagate_from(ply + 1);
        self.search_stats.inc_improvements(ply);
        if ply == 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            self.task_control.invoke_callback(&sp);
        }
    }
}
