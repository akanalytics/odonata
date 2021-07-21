use crate::bitboard::bitboard::Bitboard;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::config::{Component, Config};
use crate::eval::score::Score;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::Ply;
use crate::{debug, logger::LogInit, trace};
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub struct QSearch {
    pub enabled: bool,
    pub only_captures: bool,
    pub promos: bool,
    pub ignore_see_fails: bool,
    pub see_cutoff: i64,
    max_ply: u16,
    pub coarse_delta_prune: Score,
}

impl Component for QSearch {
    fn settings(&self, c: &mut Config) {
        c.set("qsearch.enabled", "type check default true");
        c.set("qsearch.only_captures", "type check default true");
        c.set("qsearch.promos", "type check default true");
        c.set("qsearch.see.cutoff", "type spin default 0 min -5000 max 5000");
        c.set("qsearch.see.ignore_fails", "type check default true");
        c.set("qsearch.max_ply", "type spin default 10 min 0 max 100");
        c.set(
            "qsearch.coarse_delta_prune_cp",
            "type spin default 900 min 0 max 10000",
        );
    }
    fn configure(&mut self, c: &Config) {
        debug!("qsearch.configure");
        self.enabled = c.bool("qsearch.enabled").unwrap_or(self.enabled);
        self.only_captures = c.bool("qsearch.only_captures").unwrap_or(self.only_captures);
        self.promos = c.bool("qsearch.promos").unwrap_or(self.promos);
        self.see_cutoff = c.int("qsearch.see.cutoff").unwrap_or(self.see_cutoff);
        self.ignore_see_fails = c
            .bool("qsearch.see.ignore_fails")
            .unwrap_or(self.ignore_see_fails);
        self.max_ply = c.int("qsearch.max_ply").unwrap_or(self.max_ply as i64) as u16;
        if let Some(cp) = c.int("qsearch.coarse_delta_prune_cp") {
            self.coarse_delta_prune = Score::from_cp(cp as i32);
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
            ignore_see_fails: true,
            see_cutoff: 0,
            promos: false,
            max_ply: 10,
            coarse_delta_prune: Score::from_cp(1000),
        }
    }
}

impl fmt::Display for QSearch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "only captures    : {}", self.only_captures)?;
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
    // if the move results in a position which after quiesce, is potentially a mate,
    // we should not return a mate score, as only captures have been considered,
    // and a mate score might cut a genuine mate score elsewhere
    pub fn qsearch(
        &mut self,
        mv: &Move,
        ply: Ply,
        depth: Ply,
        board: &mut Board,
        alpha: Score,
        beta: Score,
    ) -> Score {
        self.clear_move(ply);

        if !self.qsearch.enabled || (!mv.is_capture() && self.qsearch.only_captures) {
            self.search_stats.inc_leaf_nodes(ply);
            let node = Node {
                ply,
                depth,
                alpha,
                beta,
            };
            let score = board.eval(&mut self.eval, &node);
            return score;
        }
        let score = self.qsearch_see(Bitboard::EMPTY, ply, 0, board, alpha, beta);
        debug_assert!(self.task_control.is_cancelled() || score > -Score::INFINITY);
        score
    }

    pub fn qsearch_see(
        &mut self,
        mut recaptures: Bitboard,
        ply: Ply,
        depth: Ply,
        board: &mut Board,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        self.clear_move(ply);
        self.report_progress();

        let in_check = board.is_in_check(board.color_us());
        let mut standing_pat;
        if depth == 0 {
            let node = Node {
                ply,
                depth,
                alpha,
                beta,
            };
            standing_pat = board.eval(&mut self.eval, &node);
            trace!("{}", board.debug() + "Standing pat (eval)" + standing_pat);
            if standing_pat.is_mate() {
                // self.record_new_pv(board, ply, &Move::NULL_MOVE, true);
                return standing_pat;
            }
            if in_check {
                standing_pat = alpha;
            }
        } else if in_check {
            standing_pat = alpha;
        } else {
            standing_pat = board.eval_qsearch(
                &mut self.eval,
                &Node {
                    ply,
                    depth,
                    alpha,
                    beta,
                },
            );
            trace!("{}", board.debug() + "Standing pat (eval_qsearch)" + standing_pat);
        }
        if standing_pat > alpha {
            if standing_pat >= beta {
                self.search_stats.inc_q_leaf_nodes(ply);
                trace!(
                    "{}",
                    board.debug() + ply + "fail high - standing pat" + standing_pat + "cmp" + beta
                );
                // self.record_new_pv(b, ply, &Move::NULL_MOVE, true);
                return standing_pat;
            }
            alpha = standing_pat;
        }

        // let gain_needed = alpha - standing_pat;
        //if standing_pat < Score::from_cp(-10000)
        //     || alpha < Score::from_cp(-10000)
        //     || alpha > Score::from_cp(10000)
        //     || standing_pat > Score::from_cp(10000)
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
            trace!(
                "{}",
                board.debug()
                    + "examining move"
                    + mv
                    + "using"
                    + Node {
                        ply,
                        depth,
                        alpha,
                        beta
                    }
            );
            if !in_check && !mv.is_ep_capture() && mv.to().as_bb().disjoint(recaptures) {
                let score = board.eval_move_see(&self.eval, &mv);
                let winning = false;
                // allow 8 matched attackers
                let bar = self.qsearch.see_cutoff as i32;
                if score < Score::from_cp(bar) || score == Score::from_cp(bar) && (winning || depth >= -1) {
                    trace!(
                        "{}",
                        board.debug() + "see score bad" + score + "cmp" + Score::from_cp(bar) + "for" + mv
                    );
                    if self.qsearch.ignore_see_fails {
                        continue;
                    } else {
                        continue; // FIXME? Add to a bad move list
                    }
                } else {
                    trace!(
                        "{}",
                        board.debug() + "see score good" + score + "cmp" + Score::from_cp(bar) + "for" + mv
                    );
                }
            }
            let mut child = board.make_move(&mv);
            // delta prune on the move - FIXME - think about delta prune when checks
            if !in_check
                && board.eval_move_material(&self.eval, &mv) + standing_pat <= alpha
                && !child.is_in_check(child.color_us())
            // = will_check_them
            {
                board.undo_move(mv);
                continue;
            }
            // mark the square so the recapture is considered
            trace!("{}", board.debug() + ply + "iterating on " + mv);
            let score = -self.qsearch_see(
                recaptures ^ mv.to().as_bb(),
                ply + 1,
                depth - 1,
                &mut child,
                -beta,
                -alpha,
            );
            board.undo_move(mv);
            if score > beta {
                trace!("{}", board.debug() + ply + score + "fails high" + beta + mv);
                return score;
            }
            if score > alpha {
                trace!("{}", board.debug() + ply + score + "raises alpha" + alpha + mv);
                self.record_move(ply, mv);
                alpha = score;
            }
            // don't see_evaluate the hot square again
            recaptures -= mv.to().as_bb();
        }

        // we should not return a mate score, as only captures have been considered,
        // and a mate score might cut a genuine mate score elsewhere
        trace!("{}", board.debug() + ply + "returns with score of" + alpha);
        alpha.clamp(Score::from_cp(-10000), Score::from_cp(10000))
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

    #[test]
    fn test_quiesce_catalog() -> Result<(), String> {
        let positions = Catalog::quiesce();
        // let pos = Position::find_by_id("pawn fork", &positions ).unwrap();
        for pos in &positions {
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(0))
                .set_callback(Uci::uci_info)
                .clone();
            search.search(pos.board());
            debug!("{}", search);
            assert_eq!(
                search.pv().to_string(),
                pos.pv()?.to_string(),
                "{} {}\n{}",
                pos.id()?,
                pos.board().to_san_variation(search.pv(), None),
                search
            );
            // forward score is from POV of mover at end of PV line
            let qboard = search.pv().apply_to(pos.board());
            let mut static_eval = qboard.eval(&mut search.eval, &Node::root(0)).cp().unwrap();
            if qboard.color_us() != pos.board().color_us() {
                static_eval = -static_eval;
            }
            assert_eq!(
                static_eval,
                search.results().ce().unwrap() as i16,
                "{}", search.results()
            );
        }
        Ok(())
    }

    #[test]
    fn test_qsearch_ex2() -> Result<(), String> {
        debug!("test_qsearch_examples");
        let positions = Catalog::quiesce();
        let pos = Position::find_by_id("QS.01", &positions).unwrap();
        // let pos = &Catalog::quiesc         e()[1];
        let mut b = pos.board().clone(); //.color_flip();
        debug!("board {}", b.to_fen());
        let mut eval = SimpleScorer::new();
        eval.position = false;
        eval.mobility = false;
        eval.phasing = false;
        let node = Node::root(0);
        let static_eval = b.eval(&mut eval, &node);

        let mut algo = Algo::new()
            .set_timing_method(TimeControl::Depth(0))
            .set_eval(eval)
            .build();
        let quiesce_eval = algo.qsearch_see(
            Bitboard::EMPTY,
            node.ply,
            algo.max_depth,
            &mut b,
            node.alpha,
            node.beta,
        );

        println!("{}", algo);
        debug!("static: {}  quiesce: {}", static_eval, quiesce_eval);
        algo.search(&b);
        debug!("{}", algo);
        Ok(())
    }

    #[test]
    fn test_see() -> Result<(), String> {
        let mut eval = SimpleScorer::new();
        eval.mobility = false;
        eval.position = false;
        eval.material = true;
        eval.tempo = 0;

        // white gains a pawn after quiesce
        let pos = Position::parse_epd("7k/8/8/8/8/p7/8/R6K w - - 0 1 sm Ra3; ce 100;")?;
        let (alpha, beta) = (-Score::INFINITY, Score::INFINITY);

        let static_eval = pos
            .board()
            .eval(
                &mut eval,
                &Node {
                    ply: 0,
                    depth: 0,
                    alpha,
                    beta,
                },
            )
            .cp()
            .unwrap_or(0);

        let mut search_see = Algo::new()
            .set_timing_method(TimeControl::NodeCount(1_000_000))
            .set_eval(eval.clone())
            .build();
        search_see.max_depth = 3;

        let score = search_see.qsearch(
            &pos.sm()?,
            3,
            search_see.max_depth,
            &mut pos.board().clone(),
            alpha,
            beta,
        );
        if let Some(ce) = score.cp() {
            assert_eq!(ce - static_eval, pos.ce()? as i16, "see");
        } else {
            panic!("see score was {} not a cp score", score);
        }

        Ok(())
    }
}
