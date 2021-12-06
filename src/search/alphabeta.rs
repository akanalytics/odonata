use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::TtNode;
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::mv::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::{Ply, MAX_PLY};

use super::node::Event;

pub struct AlphaBeta;

// terminology
// ply is moves made. so ply 3 means w-> b-> w-> after which we score position
//
impl Algo {
    // #[inline]
    // pub fn is_leaf(&self, _ply: Ply, depth: Ply) -> bool {
    //     depth <= 0
    // }

    pub fn run_alphabeta(&mut self, board: &mut Board, node: &mut Node) -> (Score, Event) {
        let depth = node.depth;
        self.max_depth = depth;
        self.stats.depth = depth;
        self.pv_table = PvTable::new(MAX_PLY as usize);
        debug_assert!(self.current_variation.len() == 0);

        let (score, category) = self.alphabeta_recursive(board, node.ply, self.max_depth, node.alpha, node.beta, &Move::NULL_MOVE);
        self.stats.set_score(score, category);
        debug_assert!(self.current_variation.len() == 0);
        let (pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (self.pv_table.extract_pv(), Some(Score::default()))
        };
        self.results.set_pv(category, &pv);
        self.results.score = score;

        self.stats.record_iteration(self.max_depth, category, pv);
        (score, category)
    }

    pub fn alphabeta_recursive(
        &mut self,
        b: &mut Board,
        ply: Ply,
        depth: Ply,
        alpha: Score,
        beta: Score,
        last_move: &Move,
    ) -> (Score, Event) {
        self.clear_move(ply);
        self.report_progress();

        let mut n = Node { ply, depth, alpha, beta };

        let (cancelled, cat) = self.time_up_or_cancelled(ply, false);
        if cancelled {
            self.counts.inc(&n, cat);
            return (-Score::INFINITY, cat);
        }

        debug_assert!(
            n.alpha < n.beta || self.minmax,
            "alpha={}, beta={}, minmax={}",
            n.alpha,
            n.beta,
            self.minmax
        );

        self.clock.inc_nodes();
        self.counts.inc(&n, Event::Clock);
        // self.results.set_seldepth(&n);

        if n.is_zw() {
            self.counts.inc(&n, Event::NodeTypeZw);
            self.stats.inc_zw_nodes(ply);
        }
        if n.is_qs() {
            self.counts.inc(&n, Event::NodeTypeQuiesce);
        } else {
            self.counts.inc(&n, Event::NodeInterior);
        }

        let mut score = -Score::INFINITY;
        let mut category = Event::Unknown;
        let mut bm = Move::NULL_MOVE;
        let mut nt = NodeType::UpperAll;

        // we dont draw at root, as otherwise it wont play a move if insufficient-material draw [v32]
        if ply > 0 && b.draw_outcome().is_some() {
            // if board.draw_outcome().is_some() {
            self.stats.inc_leaf_nodes(&n);
            self.counts.inc(&n, Event::NodeLeafDraw);
            return (b.eval_draw(&mut self.eval, &n), Event::NodeLeafDraw); // will return a draw score
        }

        let mut tt_mv = Move::NULL_MOVE;
        match self.lookup(b, &mut n) {
            (Some(ab), None) => return (ab, Event::HashHit), // alpha, beta or a terminal node
            (None, Some(bm)) => tt_mv = bm,
            (Some(s), Some(mv)) => {
                tt_mv = mv;
                score = s;
                bm = mv;
                category = Event::HashHit;
            }
            _ => {}
        }
        self.stats.inc_interior_nodes(&n);

        // static eval
        let eval = b.eval_some(&self.eval, Switches::ALL_SCORING);

        if let Some(score) = self.standing_pat(b, &mut n, eval) {
            return (score, Event::PruneStandingPat);
        }

        if let Some(alphabeta) = self.razor(*last_move, b, eval, &n) {
            return (alphabeta, Event::PruneRazor);
        }

        if let Some(beta) = self.nmp(b, &n, eval) {
            return (beta, Event::PruneNullMovePrune);
        }

        let mut sorted_moves = self.move_orderer.get_sorted_moves(n, b, tt_mv);
        let mut count = 0;
        let mut quiets = 0;
        // if n.ply <= 4 {
        //     println!("ply {} lm {} a {} b {} eval {} pv {} board {} moves {}", n.ply, last_move, n.alpha, n.beta, eval, &self.pv_table.extract_pv_for(n.ply), b.to_fen(), b.legal_moves());
        // }
        while let Some((move_type, mv)) = sorted_moves.next_move(b, self) {
            if self.restrictions.skip_move(ply, &mv) {
                continue;
            }
            self.counts.inc(&n, Event::Moves);
            self.counts.inc_move(&n, move_type);
            count += 1;
            self.stats.inc_move(ply);
            let mut child_board = b.make_move(&mv);
            let ext = self.extend(b, &child_board, &mv, count, &n);

            if score > -Score::INFINITY {
                if let Some(est_score) = self.can_prune_move(&mv, count, move_type, b, &child_board, eval, &n, ext) {
                    self.explain_futility(&mv, move_type, est_score, &n);
                    self.stats.inc_fp_move(ply);
                    if score == -Score::INFINITY {
                        score = est_score;
                    }
                    if self.can_prune_remaining_moves(b, move_type, &n) {
                        break;
                    }
                    continue;
                }
            }
            self.repetition.push_move(&mv, &child_board);
            self.current_variation.push(mv);
            self.explainer.start(&n, &self.current_variation);
            child_board.set_repetition_count(self.repetition.count(&child_board));

            let lmr = if !self.minmax {
                self.lmr(b, &mv, count, quiets, move_type, &child_board, &n, nt, ext, tt_mv)
            } else {
                0
            };
            if lmr > 0 {
                quiets += 1;
            }

            let pvs = self.pvs_permitted(nt, b, &n, count);
            let (mut child_score, mut cat) = if pvs {
                debug_assert!(n.alpha.is_numeric());
                self.stats.inc_pvs_move(ply);
                // using [alpha, alpha + 1]
                self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - lmr - 1,
                    -n.alpha - Score::from_cp(1),
                    -n.alpha,
                    &mv,
                )
            } else {
                self.alphabeta_recursive(&mut child_board, ply + 1, depth + ext - lmr - 1, -n.beta, -n.alpha, &mv)
            };
            child_score = -child_score;

            if (lmr > 0 && self.lmr.re_search || pvs) && child_score > score && child_score < n.beta {
                // research with full window without reduction in depth
                self.stats.inc_pvs_research(ply);
                let res = self.alphabeta_recursive(&mut child_board, ply + 1, depth + ext - 1, -n.beta, -n.alpha, &mv);
                if lmr > 0 && self.lmr.re_search {
                    self.counts.inc(&n, Event::LmrReSearch);
                }
                if pvs {
                    self.counts.inc(&n, Event::PvsReSearch);
                }
                child_score = -res.0;
                cat = res.1;
            }
            b.undo_move(&mv);
            self.current_variation.pop();
            self.explainer.start(&n, &self.current_variation);
            self.repetition.pop();
            if ply > 1 && (cat == Event::UserCancelled || cat== Event::TimeUp) {
                self.explain_move(&mv, child_score, cat, &n);
                return (-Score::INFINITY, cat);
            }
            self.explain_move(&mv, child_score, cat, &n);

            // println!("move {} score {} alpha {} beta {}", mv, score, alpha, beta);
            if child_score > score {
                score = child_score;
                category = cat;
            }
            if child_score > n.alpha {
                n.alpha = child_score;
                bm = mv;
                nt = NodeType::ExactPv;
                debug_assert!(b.is_pseudo_legal_move(&bm), "bm {} on board {}", bm, b);
                self.history.raised_alpha(&n, b, &mv);
                self.record_move(ply, &mv);
            } else {
                self.history.duff(&n, b, &mv);
            }

            if n.alpha >= n.beta && !self.minmax {
                nt = NodeType::LowerCut;
                self.stats.inc_node_cut(ply, move_type, (count - 1) as i32);
                self.killers.store(ply, &mv);
                self.history.beta_cutoff(&n, b, &mv);
                self.report_refutation(n.ply);
                break;
            }
        }

        if count == 0 {
            self.stats.inc_leaf_nodes(&n);
            if n.is_qs() {
                self.counts.inc(&n, Event::NodeLeafQuietEval);
                return (b.eval(&self.eval, &n), Event::NodeLeafQuietEval);
            } else {
                self.counts.inc(&n, Event::NodeLeafStalemate);
                // FIXME VER:0.4.14
                // (board.eval_draw(&mut self.eval, &n),
                return (b.eval(&self.eval, &n), Event::NodeLeafStalemate);
            }
        }
        if nt == NodeType::UpperAll {
            self.stats.inc_node_all(ply);
            self.counts.inc(&n, Event::NodeInteriorAll);
            // nothing
        } else if nt == NodeType::LowerCut {
            debug_assert!(!bm.is_null());
            self.counts.inc(&n, Event::NodeInteriorCut);
        } else if nt == NodeType::ExactPv {
            self.stats.inc_node_pv(ply);
            self.counts.inc(&n, Event::NodeInteriorPv);
            // self.record_new_pv(ply, &bm, false);
            debug_assert!(!bm.is_null())
        } else {
            panic!("Node type {:?} ", nt);
        }
        let entry = TtNode { score, depth, nt, bm };
        self.tt.store(b.hash(), entry);
        self.explain_node(&bm, nt, score, &n, &self.pv_table.extract_pv_for(ply));
        (score, category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::Uci;
    use crate::search::timecontrol::*;
    use anyhow::Result;

    #[test]
    fn test_2_mates() -> Result<()> {
        let positions = Catalog::mate_in_2();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            search.set_timing_method(TimeControl::Depth(5)).set_callback(Uci::uci_info);
            // search.tt.enabled = false;
            search.set_position(pos.clone()).search();
            // println!("{}", search);
            assert_eq!(search.pv().to_string(), pos.pv()?.to_string(), "#{} {}", i, pos);
            assert_eq!(search.score().is_mate(), true);
            println!("Mate in {}", search.score().mate_in().unwrap());
            assert_eq!(search.score().mate_in(), Some(2), "#{} {}", i, pos);
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_mate_in_3() -> Result<()> {
        let positions = Catalog::mate_in_3();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            search.set_timing_method(TimeControl::Depth(5));
            let expected_pv = pos.pv()?;
            search.set_position(pos.clone()).search();
            println!("{}", search);

            assert_eq!(search.score().mate_in(), Some(3), "#{} {}", i, pos);
            assert_eq!(search.pv_table.extract_pv(), expected_pv, "#{} {}", i, pos);
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_mate_in_4() -> Result<()> {
        let positions = Catalog::mate_in_4();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            search.set_timing_method(TimeControl::Depth(7));
            search.set_position(pos.clone()).search();
            // println!("{}", search);
            if pos.get("pv").is_ok() {
                let expected_pv = pos.pv()?;
                assert_eq!(search.pv_table.extract_pv().to_string(), expected_pv.to_string(), "#{} {}", i, pos);
            }
            println!("Mate in {}", search.score().mate_in().unwrap());
            assert_eq!(search.score().mate_in(), Some(4), "#{} {}", i, pos);
        }
        Ok(())
    }
}
