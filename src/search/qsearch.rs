use crate::bitboard::bitboard::Bitboard;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::infra::parsed_config::{Component};
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::Ply;
// // use crate::{debug, logger::LogInit, trace};
use std::fmt;
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct QSearch {
    pub enabled: bool,
    pub only_on_capture: bool,
    pub promos: bool,
    pub ignore_see_fails: bool,
    pub see_cutoff: Score,
    pub max_ply: u16,
    pub coarse_delta_prune: Score,
    pub switches: Switches,
}


impl Default for QSearch {
    fn default() -> Self {
        QSearch {
            enabled: true,
            only_on_capture: false,
            ignore_see_fails: true,
            see_cutoff: Score::from_cp(1),
            promos: false,
            max_ply: 10,
            coarse_delta_prune: Score::from_cp(1000),
            switches: Switches::ALL_SCORING | Switches::INSUFFICIENT_MATERIAL, // MBPOWSCTI
        }
    }
}


impl Component for QSearch {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}


impl fmt::Display for QSearch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "only on capture  : {}", self.only_on_capture)?;
        writeln!(f, "ignore see fails : {}", self.ignore_see_fails)?;
        writeln!(f, "promos           : {}", self.promos)?;
        writeln!(f, "see cutoff       : {}", self.see_cutoff)?;
        writeln!(f, "max_ply          : {}", self.max_ply)?;
        writeln!(f, "coarse_del_prune : {}", self.coarse_delta_prune)?;
        writeln!(f, "eval switches    : {}", self.switches)?;
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

        if !self.qsearch.enabled || (!mv.is_capture() && self.qsearch.only_on_capture) {
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
            trace!(
                "Standing pat (eval on depth 0 on {}) {}",
                board.to_fen(),
                standing_pat
            );
            // early return if a draw or mate
            if standing_pat.is_mate() || board.draw_outcome().is_some() {
                // self.record_new_pv(board, ply, &Move::NULL_MOVE, true);
                self.stats.inc_q_leaf_nodes(ply);
                return standing_pat;
            }
            if in_check {
                standing_pat = alpha;
            }
        } else if in_check {
            standing_pat = alpha;
        } else {
            standing_pat = board.eval_some(&mut self.eval, self.qsearch.switches);
            trace!(
                "[ply {}] Standing pat (eval_qsearch on {}) {}",
                ply,
                board.to_fen(),
                standing_pat
            );
        }
        if standing_pat > alpha {
            if standing_pat >= beta {
                self.stats.inc_q_leaf_nodes(ply);
                trace!(
                    "{}",
                    board.debug() + ply + "fail high - standing pat" + standing_pat + ">=" + beta
                );
                // self.record_new_pv(b, ply, &Move::NULL_MOVE, true);
                return standing_pat;
            }
            trace!(
                "{}",
                board.debug() + ply + "alpha raised " + standing_pat + ">" + alpha
            );
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
            self.stats.inc_q_leaf_nodes(ply);
        } else {
            self.stats.inc_q_interior_nodes(ply);
        }
        self.order_moves(ply, &mut moves, &None);
        for &mv in moves.iter() {


        // let mut sorted_moves = self.move_orderer.get_sorted_moves(ply, Move::NULL_MOVE);
        // sorted_moves.qsearch = true;
        // while let Some((_move_type, mv)) = sorted_moves.next_move(board, self) {
            trace!(
                "{}",
                board.debug()
                    + "examining move"
                    + &mv
                    + "using"
                    + Node {
                        ply,
                        depth,
                        alpha,
                        beta
                    }
            );
            if !in_check && !mv.is_ep_capture() && mv.to().as_bb().disjoint(recaptures) {
                // apply a see > 0 filter unless
                // e/p or in_check (see <0 may be only evasion) or a recapture
                let score = board.eval_move_see(&self.eval, &mv);
                let winning = false;
                // allow 8 matched attackers
                if score < self.qsearch.see_cutoff || score == self.qsearch.see_cutoff && (winning || depth >= -1) {
                    trace!(
                        "{}",
                        board.debug() + "see score bad" + score + "<" + self.qsearch.see_cutoff + "for" + &mv
                    );
                    if self.qsearch.ignore_see_fails {
                        continue;
                    } else {
                        continue; // FIXME? Add to a bad move list
                    }
                } else {
                    trace!(
                        "{}",
                        board.debug() + "see score good" + score + "cmp" + self.qsearch.see_cutoff + "for" + &mv
                    );
                }
            }
            let mut child = board.make_move(&mv);
            self.current_variation.push(mv);
            self.explainer.start(&self.current_variation);
            // delta prune on the move - FIXME - think about delta prune when checks
            if !in_check
                && board.eval_move_material(&self.eval, &mv) + standing_pat <= alpha
                && !child.is_in_check(child.color_us())
            // = will_check_them
            {
                board.undo_move(&mv);
                self.current_variation.pop();
                self.explainer.stop();
                continue;
            }
            // mark the square so the recapture is considered
            trace!("{}", board.debug() + ply + "iterating on " + &mv);
            let score = -self.qsearch_see(
                recaptures ^ mv.to().as_bb(),
                ply + 1,
                depth - 1,
                &mut child,
                -beta,
                -alpha,
            );
            board.undo_move(&mv);
            self.current_variation.pop();
            self.explainer.stop();
            if score > beta {
                trace!("{}", board.debug() + ply + score + "fails high" + beta + &mv);
                return score;
            }
            if score > alpha {
                trace!("{}", board.debug() + ply + score + "raises alpha" + alpha + &mv + " -> move added to pv");
                self.record_move(ply, &mv);
                alpha = score;
            }
            // don't see_evaluate the hot square again
            recaptures -= mv.to().as_bb();
        }

        // we should not return a mate score, as only captures have been considered,
        // and a mate score might cut a genuine mate score elsewhere
        trace!("ply {} returns with score of {} and pv {}", ply, alpha, self.pv_table.extract_pv_for(ply).to_string());
        alpha.clamp(Score::from_cp(-20000), Score::from_cp(20000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::eval::eval::*;
    use crate::position::*;
    use crate::search::engine::Engine;
    use crate::search::timecontrol::*;
    use crate::test_env_log::test;
    use anyhow::Result;

    #[test]
    fn qsearch_serde_test() {
        info!("toml\n{}", toml::to_string(&QSearch::default()).unwrap());
    }

    #[test]
    fn test_quiesce_catalog() -> Result<()> {
        let positions = Catalog::quiesce();
        // let pos = Position::find_by_id("pawn fork", &positions ).unwrap();
        for pos in positions {
            let mut engine = Engine::new();
            engine.algo.eval.set_switches(false);
            engine.algo.eval.material = true;
            engine.algo.set_timing_method(TimeControl::Depth(0));
            //                .set_callback(Uci::uci_info)
            // engine.algo.eval.mb.enabled = true;
            engine.set_position(pos.clone());
            engine.search();
            // debug!("{}", search);
            println!(
                "search:{}\nexpected:{}\nresults:{}",
                engine.algo.pv().to_string(),
                pos.pv()?.to_string(),
                engine.algo.results_as_position(),
            );
            assert_eq!(
                engine.algo.pv().to_string(),
                pos.pv()?.to_string(),
                "{} {}\n{}",
                pos.id()?,
                pos.board().to_san_variation(engine.algo.pv(), None),
                engine
            );
            // forward score is from POV of mover at end of PV line
            let qboard = engine.algo.pv().apply_to(pos.board());
            let mut static_eval = qboard.eval_some(&mut engine.algo.eval, engine.algo.qsearch.switches).cp().unwrap();
            if qboard.color_us() != pos.board().color_us() {
                static_eval = -static_eval;
            }
            assert_eq!(
                static_eval,
                engine.algo.results_as_position().ce().unwrap() as i16,
                "{}",
                engine.algo.results_as_position()
            );
        }
        Ok(())
    }

    #[test]
    fn test_qsearch_ex2() -> Result<()> {
        debug!("test_qsearch_examples");
        let positions = Catalog::quiesce();
        let pos = Position::find_by_id("QS.01", &positions).unwrap();
        // let pos = &Catalog::quiesc         e()[1];
        // let mut b = pos.board().clone(); //.color_flip();
        debug!("position {}", pos);
        let mut eval = SimpleScorer::new();
        eval.position = false;
        eval.mobility = false;
        eval.phasing = false;
        let node = Node::root(0);
        let static_eval = pos.board().eval(&mut eval, &node);

        let mut algo = Algo::new()
            .set_timing_method(TimeControl::Depth(0))
            .set_eval(eval)
            .build();
        let quiesce_eval = algo.qsearch_see(
            Bitboard::EMPTY,
            node.ply,
            algo.max_depth,
            &mut pos.board().clone(),
            node.alpha,
            node.beta,
        );

        println!("{}", algo);
        debug!("static: {}  quiesce: {}", static_eval, quiesce_eval);
        algo.set_position(pos.clone()).search();
        debug!("{}", algo);
        Ok(())
    }

    #[test]
    fn test_qsearch_see() -> Result<()> {
        let mut eval = SimpleScorer::new();
        eval.mb.enabled = false;
        eval.set_switches(false);
        eval.material = true;
        eval.mb.set_classical_piece_values();

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
