use crate::types::MAX_PLY;
use crate::config::{Config, Configurable};
use crate::log_debug;
use std::fmt;
use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::eval::eval::{Scorable, Score, SimpleScorer};
use crate::movelist::Move;
use crate::pvtable::PvTable;
use crate::search::timecontrol::{TimeControl, MoveTimeEstimator};
use crate::search::stats::SearchStats;
use crate::search::quiescence::Quiescence;



// #[derive(Copy, Clone, Debug, Default)]
// pub struct AlphaBeta {
//     minimax: bool,
// } 


// // pub trait AlphaBeta {
// //     pub fn alphabeta(board: &mut Board, int depth, alpha: Score, beta: Score) -> Score;
// // }




// impl Configurable for AlphaBeta {
//     fn settings(&self, c: &mut Config) {
//         c.set("alphabeta.minimax", "type check default false");
//     }
    
//     fn configure(&mut self, c: &Config) {
//         log_debug!("alphabeta.configure with {}", c);
//         self.minimax = c.bool("alphabeta.minimax").unwrap_or(self.minimax);
//     }
// }
        




// impl fmt::Display for AlphaBeta {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         writeln!(f, "minimax          : {}", self.minimax)
//     }
// }



// impl Algo {

//     pub fn alphabeta(depth: u32, alpha: Score, beta: Score) -> Score {

//         if depth == 0 {}
//             return board.eval(&self.eval);
//         }
//         self.search_stats.inc_leaf_nodes(node.ply);
//         return;
//         return Evaluate();

//         GenerateLegalMoves();

//         while (MovesLeft()) {

//             MakeNextMove();

//             val = -AlphaBeta(depth - 1, -beta, -alpha);

//             UnmakeMove();

//             if (val >= beta)

//                 return beta;

//             if (val > alpha)

//                 alpha = val;

//         }

//         return alpha;
// }