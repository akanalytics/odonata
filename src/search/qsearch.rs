use crate::bitboard::bitboard::Bitboard;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::config::{Component, Config};
use crate::eval::score::Score;
use crate::{debug, trace, logger::LogInit};
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::Ply;
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub struct QSearch {
    pub enabled: bool,
    pub only_captures: bool,
    pub promos: bool,
    pub ignore_see_fails: bool,
    pub see: bool,
    pub see_cutoff: i64,
    max_ply: u16,
    pub coarse_delta_prune: Score,
}

impl Component for QSearch {
    fn settings(&self, c: &mut Config) {
        c.set("qsearch.enabled", "type check default true");
        c.set("qsearch.only_captures", "type check default true");
        c.set("qsearch.promos", "type check default true");
        c.set("qsearch.see", "type check default true");
        c.set("qsearch.see.cutoff", "type spin default 0 min -5000 max 5000");
        c.set("qsearch.see.ignore_fails", "type check default true");
        c.set("qsearch.max_ply", "type spin default 10 min 0 max 100");
        c.set(
            "qsearch.coarse_delta_prune_cp",
            "type spin default 900 min 0 max 10000",
        );
    }
    fn configure(&mut self, c: &Config) {
        debug!("qsearch.configure with {}", c);
        self.enabled = c.bool("qsearch.enabled").unwrap_or(self.enabled);
        self.only_captures = c.bool("qsearch.only_captures").unwrap_or(self.only_captures);
        self.promos = c.bool("qsearch.promos").unwrap_or(self.promos);
        self.see = c.bool("qsearch.see").unwrap_or(self.see);
        self.see_cutoff = c.int("qsearch.see.cutoff").unwrap_or(self.see_cutoff);
        self.ignore_see_fails = c.bool("qsearch.see.ignore_fails").unwrap_or(self.ignore_see_fails);
        self.max_ply = c.int("qsearch.max_ply").unwrap_or(self.max_ply as i64) as u16;
        if let Some(cp) = c.int("qsearch.coarse_delta_prune_cp") {
            self.coarse_delta_prune = Score::from_cp(cp as i16);
        }
    }
    fn new_game(&mut self) {}

    fn new_search(&mut self) {}
}

impl Default for QSearch {
    fn default() -> Self {
        QSearch {
            enabled: true,
            only_captures: false,
            see: true,
            ignore_see_fails: true,
            see_cutoff: 0,
            promos: true,
            max_ply: 10,
            coarse_delta_prune: Score::from_cp(1000),
        }
    }
}

impl fmt::Display for QSearch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "only captures    : {}", self.only_captures)?;
        writeln!(f, "see enabled      : {}", self.see)?;
        writeln!(f, "ignore see fails : {}", self.ignore_see_fails)?;
        writeln!(f, "promos           : {}", self.promos)?;
        writeln!(f, "see cutoff       : {}", self.see_cutoff)?;
        writeln!(f, "max_ply          : {}", self.max_ply)?;
        writeln!(f, "coarse_del_prune : {}", self.coarse_delta_prune)?;
        Ok(())
    }
}

impl Algo {
    // if the move resulted in checkmate, we should return a mate score
    // if the move results in a position which after quiese, is potentially a mate,
    // we should not return a mate score, as only captures have been considered,
    // and a mate score might cut a genuine mate score elsewhere
    pub fn qsearch(&mut self, mv: &Move, ply: Ply, board: &mut Board, alpha: Score, beta: Score) -> Score {
        if !self.qsearch.enabled || ply <= 1 || (!mv.is_capture() && self.qsearch.only_captures) {
            self.search_stats.inc_leaf_nodes(ply);
            return board.eval(&mut self.eval, &Node { ply, alpha, beta });
        }
        let score = if self.qsearch.see {
            self.qsearch_see(Bitboard::EMPTY, ply, board, alpha, beta)
        } else {
            self.qsearch_sq(mv.to(), ply, board, alpha, beta)
        };
        assert!(self.task_control.is_cancelled() || score > Score::MinusInf);
        score
    }

    pub fn qsearch_see(
        &mut self,
        mut recaptures: Bitboard,
        ply: Ply,
        board: &mut Board,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        self.report_progress();

        let in_check = board.is_in_check(board.color_us());
        let standing_pat;
        if ply == self.max_depth {
            standing_pat = board.eval(&mut self.eval, &Node { ply, alpha, beta });
            trace!("{}", board.debug() + "Standing pat (eval)" + standing_pat);
            if standing_pat.is_mate() {
                return standing_pat;
            }
        } else if in_check {
            standing_pat = alpha;
        } else {
            standing_pat = board.eval_qsearch(&mut self.eval, &Node { ply, alpha, beta });
            trace!("{}", board.debug() + "Standing pat (eval_qsearch)" + standing_pat);
        }
        if standing_pat > alpha {
            if standing_pat >= beta {
                self.search_stats.inc_q_leaf_nodes(ply);
                trace!("{}", board.debug() + ply + "fail high - standing pat" + standing_pat + "cmp" + beta);
                return beta;
            }
            alpha = standing_pat;
        }

        // let gain_needed = alpha - standing_pat;
        //if standing_pat < Score::Cp(-10000)
        //     || alpha < Score::Cp(-10000)
        //     || alpha > Score::Cp(10000)
        //     || standing_pat > Score::Cp(10000)
        // {
        //     Score::MinusInf
        // } else {
        //     alpha - standing_pat
        // };

        // coarse delta pruning - if were > queen down already, no way to make it up (regardless of in check)
        // if gain_needed > self.qsearch.coarse_delta_prune {
        //     self.search_stats.inc_q_leaf_nodes(ply);
        //     return alpha;
        // }

        // need to add checking moves on ply 0
        let moves = board.legal_moves();
        let mut moves: MoveList = moves
            .iter()
            .filter(|mv| mv.is_capture() || (mv.is_promo() & self.qsearch.promos) || in_check)
            .cloned()
            .collect();

        if moves.is_empty() {
            self.search_stats.inc_q_leaf_nodes(ply);
        } else {
            self.search_stats.inc_q_interior_nodes(ply);
        }
        self.order_moves(ply, &mut moves, &None);
        for mv in moves.iter() {
            // FIXME! skip illegal moves
            trace!("{}", board.debug() + "examining move" + mv + "using" + Node { ply, alpha, beta} );
            if !in_check && !mv.is_ep_capture() && mv.to().as_bb().disjoint(recaptures) {
                let score = board.eval_move_see(&self.eval, &mv);
                let winning = false;
                // allow 8 matched attackers
                let bar = self.qsearch.see_cutoff as i32;
                if score < Score::Cp(bar) || score == Score::Cp(bar) && (winning || ply <= self.max_depth + 1) {
                    trace!("{}", board.debug() + "see score bad" + score + "cmp" + Score::Cp(bar) + "for" + mv  );
                    if self.qsearch.ignore_see_fails {
                        continue; 
                    } else {
                        continue;  // FIXME? Add to a bad move list
                    }
                } else {
                    trace!("{}", board.debug() + "see score good" + score + "cmp" + Score::Cp(bar) + "for" + mv  );
                }
            }
            let mut child = board.make_move(&mv);
            // delta prune on the move - FIXME - think about delta prune when checks
            if !in_check
                && board.eval_move_material(&self.eval, &mv) + standing_pat <= alpha
                && !child.is_in_check(child.color_us())  // = will_check_them
            {
                board.undo_move(mv);
                continue;
            }
            // mark the square so the recapture is considered
            trace!("{}", board.debug() + ply + "iterating on " + mv);
            let score = -self.qsearch_see(recaptures ^ mv.to().as_bb(), ply + 1, &mut child, -beta, -alpha);
            board.undo_move(mv);
            if score > beta {
                trace!("{}", board.debug() + ply + score + "fails high" + beta + mv);
                return score;
            }
            if score > alpha {
                trace!("{}", board.debug() + ply + score + "raises alpha" + alpha + mv);
                alpha = score;
            }
            // don't see_evaluate the hot square again
            recaptures -= mv.to().as_bb();
        }

        // we should not return a mate score, as only captures have been considered,
        // and a mate score might cut a genuine mate score elsewhere
        trace!("{}", board.debug() + ply + "returns with score of" + alpha);
        alpha.clamp(Score::Cp(-10000), Score::Cp(10000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::Uci;
    use crate::eval::eval::*;
    use crate::position::*;
    use crate::search::timecontrol::*;

    #[ignore]
    #[test]
    fn test_qsearch() -> Result<(), String> {
        for &see in [false, true].iter() {
            let pos = &Catalog::mate_in_2()[0];
            let mut search = Algo::new()
                .set_timing_method(TimeControl::NodeCount(1_000_000))
                .set_callback(Uci::uci_info)
                .clone();
            //search.qsearch.enabled = qs;
            search.qsearch.see = see;
            search.search(pos.board());
            println!("{}", search);
            assert_eq!(search.pv().to_string(), pos.pv()?.to_string(), "{}", pos.id()?);
            assert_eq!(search.score(), Score::white_win(3));
        }
        Ok(())
    }

    #[test]
    fn test_qsearch_examples() -> Result<(), String> {
        trace!("test_qsearch_examples");
        let pos = &Catalog::quiese()[1];
        let mut b = pos.board().color_flip();
        trace!("board {}", b.to_fen());
        let mut eval = SimpleScorer::new();
        eval.position = false;
        eval.mobility = false;
        eval.phasing = false;
        let node = Node::root();
        let static_eval = b.eval(&mut eval, &node);

        let mut algo = Algo::new().set_timing_method(TimeControl::Depth(0)).set_eval(eval).build();
        let quiese_eval = algo.qsearch_see(Bitboard::EMPTY, node.ply, &mut b, node.alpha, node.beta);

        println!("{}", algo);
        trace!("static: {}  quiese: {}", static_eval, quiese_eval);
        Ok(())
    }


    #[test]
    fn test_see_vs_sq() -> Result<(), String> {
        let mut eval = SimpleScorer::new();
        eval.mobility = false;
        eval.position = false;
        eval.material = true;

        // white gains a pawn after quiese
        let pos = Position::parse_epd("7k/8/8/8/8/p7/8/R6K w - - 0 1 sm Ra3; ce 100;")?; 
        let (alpha, beta) = (Score::MinusInf, Score::PlusInf);

        let static_eval = pos.board().eval(&mut eval, &Node { ply: 0, alpha, beta }).cp().unwrap_or(0);

        let mut search_sq = Algo::new()
            .set_timing_method(TimeControl::NodeCount(1_000_000))
            .set_eval(eval.clone())
            .build();
        search_sq.qsearch.see = false;
        search_sq.max_depth = 3;

        let mut search_see = Algo::new()
            .set_timing_method(TimeControl::NodeCount(1_000_000))
            .set_eval(eval.clone())
            .build();
        search_see.qsearch.see = true;
        search_see.max_depth = 3;


        let score = search_sq.qsearch(&pos.sm()?, 3, &mut pos.board().clone(), alpha, beta);
        if let Some(ce) = score.cp() {
            assert_eq!(ce - static_eval, 100, "sq");
        } else {
            panic!("sq score was {} not a cp score", score);
        }

        let score = search_see.qsearch(&pos.sm()?, 3, &mut pos.board().clone(), alpha, beta);
        if let Some(ce) = score.cp() {
            assert_eq!(ce - static_eval, pos.ce()? as i16, "see");
        } else {
            panic!("see score was {} not a cp score", score);
        }

        Ok(())
    }
}
