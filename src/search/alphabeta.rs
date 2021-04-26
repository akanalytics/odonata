use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::eval::Scorable;
use crate::eval::score::Score;
use crate::movelist::Move;
use crate::search::algo::Algo;
use crate::search::searchprogress::SearchProgress;
use crate::tt::{Entry, NodeType};
use crate::types::{Color, Ply};
use std::cmp;

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
        mut beta: Score,
        last_move: &Move,
    ) -> Score {
        debug_assert!(self.max_depth > 0);
        if self.search_stats.total().nodes() % 1000000 == 0 && self.search_stats.total().nodes() != 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            self.task_control.invoke_callback(&sp);
        }

        let mut score = Score::MinusInf;

        if self.time_up_or_cancelled(ply, false) {
            return score;
        }

        if board.repetition_count() >= 2 {
            self.search_stats.inc_leaf_nodes(ply);
            return Self::sigma(board) * board.eval(&self.eval); // will return a draw score
        }

        let mut bm = Move::NULL_MOVE;

        if self.tt.enabled() {
            if let Some(entry) = self.tt.get(board.hash()) {
                let depth = self.max_depth - ply;
                if entry.depth >= depth {
                    //println!("Entry:{:?}", entry);
                    // for bounded scores, we know iterating through the nodes might raise alpha, lower beta
                    // doing this now allows us potentuially to cut off without looking at the child nodes
                    match entry.node_type {
                        NodeType::Exact => {
                            score = entry.score;
                            alpha = entry.score;
                            beta = entry.score;
                            bm = entry.bm;
                        }
                        NodeType::LowerBound => {
                            alpha = cmp::max(alpha, entry.score);
                            bm = entry.bm;
                        }
                        NodeType::UpperBound => {
                            beta = cmp::min(beta, entry.score);
                            debug_assert!(alpha < beta);
                        }
                        NodeType::Unused => panic!("Node type Unused returned on tt probe"),
                    }
                }
            }
            if alpha >= beta {
                self.current_variation.set_last_move(ply, &Move::NULL_MOVE);
                self.record_new_pv(ply, &bm);
                return score;
            }
        }

        if self.is_leaf(ply) {
            return self.qsearch2(last_move.to, ply, board, alpha, beta);
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

            board.undo_move(mv);
            self.repetition.pop();

            if child_score > score {
                score = child_score;
            }
            if child_score > alpha {
                alpha = child_score;
                bm = *mv;
                self.record_new_pv(ply, &bm);
            }

            if alpha >= beta && !self.minmax {
                break;
            }
        }

        if self.tt.enabled() {
            let node_type;
            if score <= original_alpha {
                node_type = NodeType::UpperBound
            } else if score >= beta {
                node_type = NodeType::LowerBound;
            } else {
                node_type = NodeType::Exact;
            }
            let entry = Entry {
                hash: board.hash(),
                score,
                depth: self.max_depth - ply,
                node_type: node_type,
                bm, // not set for NodeType::UpperBound
            };
            self.tt.insert(entry);
        }
        self.current_variation.set_last_move(ply, &Move::NULL_MOVE);
        score
    }

    fn record_new_pv(&mut self, ply: Ply, mv: &Move) {
        self.pv_table.set(ply + 1, mv);
        self.pv_table.propagate_from(ply + 1);
        self.search_stats.inc_improvements(ply);
        if ply == 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            self.task_control.invoke_callback(&sp);
        }
    }
}
