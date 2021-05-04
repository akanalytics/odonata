use crate::bitboard::Bitboard;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::eval::Scorable;
use crate::eval::score::Score;
use crate::log_debug;
use crate::movelist::Move;
use crate::movelist::MoveList;
use crate::search::algo::Algo;
use crate::types::Ply;
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub struct QSearch {
    pub enabled: bool,
    pub only_captures: bool,
    see: bool,
    max_ply: u16,
    coarse_delta_prune: Score,
}

impl Configurable for QSearch {
    fn settings(&self, c: &mut Config) {
        c.set("qsearch.enabled", "type check default true");
        c.set("qsearch.only_captures", "type check default true");
        c.set("qsearch.see", "type check default true");
        c.set("qsearch.max_ply", "type spin default 10 min 0 max 100");
        c.set("qsearch.coarse_delta_prune_cp", "type spin default 900 min 0 max 10000");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("qsearch.configure with {}", c);
        self.enabled = c.bool("qsearch.enabled").unwrap_or(self.enabled);
        self.only_captures = c.bool("qsearch.only_captures").unwrap_or(self.only_captures);
        self.see = c.bool("qsearch.see").unwrap_or(self.see);
        self.max_ply = c.int("qsearch.max_ply").unwrap_or(self.max_ply as i64) as u16;
        if let Some(cp) = c.int("qsearch.coarse_delta_prune_cp") {
            self.coarse_delta_prune = Score::cp(cp as i32);
        }
    }
}

impl Default for QSearch {
    fn default() -> Self {
        QSearch {
            enabled: true,
            only_captures: false,
            see: true,
            max_ply: 10,
            coarse_delta_prune: Score::cp(1000),
        }
    }
}

impl fmt::Display for QSearch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "only captures    : {}", self.only_captures)?;
        writeln!(f, "see enabled      : {}", self.see)?;
        writeln!(f, "max_ply          : {}", self.max_ply)?;
        writeln!(f, "coarse_del_prune : {}", self.coarse_delta_prune)?;
        Ok(())
    }
}

impl Algo {


    pub fn qsearch2(&mut self, mv: &Move, ply: Ply, board: &mut Board, alpha: Score, beta: Score) -> Score {
        if !self.qsearch.enabled || ply <= 1 || (!mv.is_capture() && self.qsearch.only_captures) {
            self.search_stats.inc_leaf_nodes(ply);
            return board.eval(&self.eval);
        }
        let score = self.qsearch(mv.to(), ply, board, alpha, beta);
        assert!(self.task_control.is_cancelled() || score > Score::MinusInf);
        score
    }



    pub fn qsearch_ext(
        &mut self,
        mut sq: Bitboard,
        ply: Ply,
        board: &mut Board,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        self.report_progress();
    

        let in_check = board.is_in_check(board.color_us());
        let standing_pat;
        if in_check {
            standing_pat = alpha;
        } else {
            standing_pat = board.eval(&self.eval);
        }
        if standing_pat > alpha {
            if standing_pat >= beta {
                self.search_stats.inc_q_leaf_nodes(ply);
                return beta;
            }
            alpha = standing_pat;
        }

   
        let gain_needed = alpha - standing_pat;

        // coarse delta pruning - if were > queen down already, no way to make it up (regardless of in check)
        if gain_needed > self.qsearch.coarse_delta_prune {
            self.search_stats.inc_q_leaf_nodes(ply);
            return alpha;
        }
   

        // need to add checking moves on ply 0
        let moves = board.pseudo_legal_moves();
        let mut moves: MoveList = moves
            .iter()
            .filter(|mv| mv.to() == sq && mv.is_capture() && board.is_legal_move(mv))
            .cloned()
            .collect();

        // check evasions + e/p (wrong see) + mandat recaptures always applied
        // In order to
        // correct SEE errors due to pinned and overloaded pieces,
        // at least one mandatory recapture is always tried at the
        // destination squares of previous moves.) For all other
        // moves, a static exchange evaluation is performed to
        // decide if the move should be tried.
        self.order_moves(ply, &mut moves);
        for mv in moves.iter() {
            if !in_check && !mv.is_ep_capture() && mv.to().disjoint(sq) {
                let score = board.eval_move_see(&self.eval, &mv);
                let losing = false;
                if score < Score::Cp(0) || score == Score::Cp(0) && (losing || ply > 2 + self.max_depth) {
                    continue;
                } 
            }
            
            let mut child = board.make_move(&mv);
            // delta prune on the move
            if !child.is_in_check(child.color_us()) && board.eval_move_material(&self.eval, &mv) < gain_needed {
                board.undo_move(mv);
                continue;
            }
            let score = -self.qsearch_ext(sq ^ mv.to(), ply + 1, &mut child, -beta, -alpha);
            board.undo_move(mv);
            if score > beta {
                return score;
            }
            if score > alpha {
                alpha = score;
            }
            // don't see_evaluate the hot square again
            sq -= mv.to();
        }
    alpha
}




    pub fn qsearch(
        &mut self,
        sq: Bitboard,
        ply: Ply,
        board: &mut Board,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        self.report_progress();

        if self.time_up_or_cancelled(ply, false) {
            return alpha;
        }

        // this will handle mates too at quiescent node
        let standing_pat;
        if ply == self.max_depth {
            standing_pat = board.eval(&self.eval);
        } else {
            // in qsearch a mate score might mean a queen sacrifice. But in reality
            // opponent would just play some other move
            standing_pat = board.eval_qsearch(&self.eval);
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
        if standing_pat < alpha - self.qsearch.coarse_delta_prune {
            self.search_stats.inc_q_leaf_nodes(ply);
            return alpha;
        }

        let moves = board.pseudo_legal_moves();
        let mut moves: MoveList = moves
            .iter()
            .filter(|mv| mv.to() == sq && mv.is_capture() && board.is_legal_move(mv))
            .cloned()
            .collect();

        if moves.len() == 0 {
            self.search_stats.inc_q_leaf_nodes(ply);
        } else {
            self.search_stats.inc_q_interior_nodes(ply);
        }

        self.order_moves(ply, &mut moves);

        for mv in moves.iter() {
            let mut child = board.make_move(mv);
            let score = -self.qsearch(sq, ply + 1, &mut child, -beta, -alpha);
            board.undo_move(mv);
            if score > beta {
                return score;
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
            let mut search = Algo::new()
                .set_timing_method(TimeControl::NodeCount(1_000_000))
                .set_callback(Uci::uci_info)
                .clone();
            search.qsearch.enabled = qs;
            search.search(pos.board());
            println!("{}", search);
            assert_eq!(search.pv().to_string(), pos.pv().unwrap().to_string(), "{}", pos.id().unwrap());
            assert_eq!(search.score(), Score::WhiteWin { minus_ply: -3 });
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

// Hello from freddie
