use crate::bitboard::Bitboard;
use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::{Scorable, Score};
use crate::log_debug;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::searchprogress::SearchProgress;
use crate::types::Color;
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub struct Quiescence {
    pub enabled: bool,
    see: bool,
    max_ply: u16,
    coarse_delta_prune: Score,
}

impl Configurable for Quiescence {
    fn settings(&self, c: &mut Config) {
        c.set("quiescence.enabled", "type check default true");
        c.set("quiescence.see", "type check default true");
        c.set("quiescence.max_ply", "type spin default 10 min 0 max 100");
        c.set("quiescence.coarse_delta_prune_cp", "type spin default 900 min 0 max 10000");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("quiescence.configure with {}", c);
        self.enabled = c.bool("quiescence.enabled").unwrap_or(self.enabled);
        self.see = c.bool("quiescence.see").unwrap_or(self.see);
        self.max_ply = c.int("quiescence.max_ply").unwrap_or(self.max_ply as i64) as u16;
        if let Some(cp) = c.int("quiescence.coarse_delta_prune_cp") {
            self.coarse_delta_prune = Score::cp(cp as i32);
        }
    }
}

impl Default for Quiescence {
    fn default() -> Self {
        Quiescence { enabled: false, see: true, max_ply: 10, coarse_delta_prune: Score::cp(900) }
    }
}

impl fmt::Display for Quiescence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "see enabled      : {}", self.see)?;
        writeln!(f, "max_ply          : {}", self.max_ply)?;
        writeln!(f, "coarse_del_prune : {}", self.coarse_delta_prune)?;
        Ok(())
    }
}

impl Algo {
    #[inline]
    fn evaluate_leaf(&mut self, node: &mut Node) {
        node.score = node.board.eval(&self.eval);
        self.search_stats.inc_leaf_nodes(node.ply);
    }

    #[inline]
    pub fn quiescence_search(&mut self, node: &mut Node) {
        if !self.quiescence.enabled || node.ply == 1 {
            self.evaluate_leaf(node);
            return;
        }

        // swap to negamax
        if Node::is_maximizing(node.board) {
            node.score = self.qsearch(node.last_move.to, node.ply, node.board, node.alpha, node.beta);
        } else {
            node.score = -self.qsearch(node.last_move.to, node.ply, node.board, -node.beta, -node.alpha);
        }
    }

    // int Quiesce( int alpha, int beta ) {
    //     int stand_pat = Evaluate();
    //     if( stand_pat >= beta )
    //         return beta;
    //     if( alpha < stand_pat )
    //         alpha = stand_pat;
    //     until( every_capture_has_been_examined )  {
    //         MakeCapture();
    //         score = -Quiesce( -beta, -alpha );
    //         TakeBackMove();
    //         if( score >= beta )
    //             return beta;
    //         if( score > alpha )
    //            alpha = score;
    //     }
    //     return alpha;
    // }
    fn qsearch(&mut self, sq: Bitboard, ply: u32, board: &Board, mut alpha: Score, beta: Score) -> Score {
        if self.search_stats.total().nodes() % 1000000 == 0 && self.search_stats.total().nodes() != 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            // sp.depth = None;
            self.task_control.invoke_callback(&sp);
        }

        if self.time_up_or_cancelled(ply, false) {
            return alpha;
        }

        // this will handle mates too
        let mut standing_pat = board.eval(&self.eval);
        if board.color_us() == Color::Black {
            standing_pat = -standing_pat;
        }
        // if standing_pat.is_mate() {
        //     return standing_pat;
        // }
        if standing_pat > alpha {
            if standing_pat >= beta {
                self.search_stats.inc_q_leaf_nodes(ply);
                return beta;
            }
            alpha = standing_pat;
        }

        // coarse delta pruning
        if standing_pat < alpha - self.quiescence.coarse_delta_prune {
            self.search_stats.inc_q_leaf_nodes(ply);
            return alpha;
        }

        let mut moves = board.legal_capture_moves();
        moves.retain(|mv| mv.to() == sq);

        if moves.len() == 0 {
            self.search_stats.inc_q_leaf_nodes(ply);
        } else {
            self.search_stats.inc_q_interior_nodes(ply);
        }

        self.order_moves(ply, &mut moves);

        for (_i, mv) in moves.iter().enumerate() {
            let mut child_board = board.make_move(mv);
            let score = -self.qsearch(sq, ply + 1, &mut child_board, -beta, -alpha);
            if score > beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
        alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::Uci;
    use crate::search::timecontrol::*;

    #[ignore]
    #[test]
    fn test_qsearch() {
        for &qs in [false, true].iter() {
            let pos = &Catalog::mate_in_2()[0];
            let mut search =
                Algo::new().set_timing_method(TimeControl::NodeCount(1_000_000)).set_callback(Uci::uci_info);
            search.quiescence.enabled = qs;
            search.search(pos.board().clone());
            println!("{}", search);
            assert_eq!(search.pv().to_string(), pos.pv().unwrap().to_string(), "{}", pos.id().unwrap());
            assert_eq!(search.score, Score::WhiteWin { minus_ply: -3 });
        }
    }
}

//         let is_cut = self.process_child(&mv, node, &child);
//         if is_cut {
//             self.search_stats.inc_cuts(node.ply);
//             break;
//         }
//     }

//     Score::Cp(0)
// }

// {

//     val = Evaluate();
//     if (val >= beta)
//         return beta;

//     if (val > alpha)
//         alpha = val;

//     GenerateGoodCaptures();

//     while (CapturesLeft()) {
//         MakeNextCapture();
//         val = -Quies(-beta, -alpha);
//         UnmakeMove();

//         if (val >= beta)
//             return beta;

//         if (val > alpha)
//             alpha = val;

//     }

//     return alpha;

// }    }

// if moves.is_empty() {
//     self.evaluate_leaf(node);
//     return;
// }

// if self.time_up_or_cancelled(node.ply, self.search_stats.total().nodes(), false) {
//     return;
// }
