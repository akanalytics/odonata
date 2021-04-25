use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::eval::{Scorable, SimpleScorer};
use crate::eval::score::Score;
use crate::globals::counts;
use crate::log_debug;
use crate::movelist::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::move_time_estimator::MoveTimeEstimator;
use crate::search::quiescence::Quiescence;
use crate::search::searchprogress::SearchProgress;
use crate::search::searchstats::SearchStats;
use crate::search::timecontrol::TimeControl;
use crate::types::{Color, Ply};
use crate::types::MAX_PLY;
use std::fmt;

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
        alpha: Score,
        beta: Score,
        last_move: &Move,
    ) -> Score {
        debug_assert!(self.max_depth > 0);
        if self.search_stats.total().nodes() % 1000000 == 0 && self.search_stats.total().nodes() != 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            self.task_control.invoke_callback(&sp);
        }

        let mut score = Score::MinusInf;
        let mut alpha = alpha;

        if self.time_up_or_cancelled(ply, false) {
            return score;
        }

        if board.repetition_count() >= 2 {
            self.search_stats.inc_leaf_nodes(ply);
            return Self::sigma(board) * board.eval(&self.eval);
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
        let original_score = score;
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
                self.pv_table.set(ply + 1, mv);
                self.pv_table.propagate_from(ply + 1);
                self.search_stats.inc_improvements(ply);
                if ply == 0 {
                    let sp = SearchProgress::from_search_stats(&self.search_stats());
                    self.task_control.invoke_callback(&sp);
                }
            }

            if alpha >= beta && !self.minmax {
                break;
            }
        }

        self.current_variation.set_last_move(ply, &Move::NULL_MOVE);
        score
    }
}
