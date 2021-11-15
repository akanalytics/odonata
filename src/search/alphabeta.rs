use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::{TtNode};
use crate::eval::score::Score;
use crate::mv::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::{Ply, MAX_PLY};
use crate::eval::switches::Switches;

use super::node::Category;


pub struct AlphaBeta;

// terminology
// ply is moves made. so ply 3 means w-> b-> w-> after which we score position
//
impl Algo {
    // #[inline]
    // pub fn is_leaf(&self, _ply: Ply, depth: Ply) -> bool {
    //     depth <= 0
    // }

    pub fn run_alphabeta(&mut self, board: &mut Board, node: &mut Node) -> (Score, Category) {
        let depth = node.depth;
        self.max_depth = depth;
        self.stats.depth = depth;
        self.stats.new_iteration();
        self.pv_table = PvTable::new(MAX_PLY as usize);
        debug_assert!(self.current_variation.len() == 0);



        let (score, category) = self.alphabeta_recursive(
            board,
            node.ply,
            self.max_depth,
            node.alpha,
            node.beta,
            &Move::NULL_MOVE,
        );
        self.stats.set_score(score, category);
        debug_assert!(self.current_variation.len() == 0);
        let (pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (self.pv_table.extract_pv(), Some(Score::default()))
        };
        self.stats
            .record_iteration(self.max_depth, category, pv);
        (score, category)
    }

    pub fn alphabeta_recursive(
        &mut self,
        board: &mut Board,
        ply: Ply,
        depth: Ply,
        alpha: Score,
        beta: Score,
        last_move: &Move,
    ) -> (Score, Category) {
        // debug_assert!(depth > 0);
        self.clear_move(ply);
        self.report_progress();


        if self.time_up_or_cancelled(ply, false) {
            return (-Score::INFINITY, Category::Cancelled);
        }

        let mut n = Node {
            ply,
            depth,
            alpha,
            beta,
        };

        if n.depth == 0 {
            self.clear_move(ply);
        }   

        if n.alpha + Score::from_cp(1) == n.beta {
            self.stats.inc_zw_nodes(ply);
        }


        let mut score = -Score::INFINITY;
        let mut category = Category::Unknown;
        let mut bm = Move::NULL_MOVE;
        let mut nt = NodeType::UpperAll;

        // we dont draw at root, as otherwise it wont play a move if insufficient-material draw [v32]
        if ply > 0 && board.draw_outcome().is_some() {
        // if board.draw_outcome().is_some() {
            self.stats.inc_leaf_nodes(&n);
            return (board.eval_draw(&mut self.eval, &n), Category::Draw); // will return a draw score
        }

        let mut tt_mv = Move::NULL_MOVE;
        match self.lookup(board, &mut n) {
            (Some(ab), None) => return (ab, Category::Transposition),  // alpha, beta or a terminal node
            (None, Some(bm)) => tt_mv = bm,
            (Some(s), Some(mv)) => { tt_mv = mv; score = s; bm = mv},
            _ => {},
        }                


        let _e = self.extensions.extend_at_leaf(board);
            // if e == 0 {
            //     self.stats.inc_leaf_qsearch_nodes(ply);
            //     self.alphabeta_recursive(&mut board, ply + 1, depth - 1, -n.beta, -n.alpha, &mv)                
            //     return self.qsearch(last_move, ply, depth, board, n.alpha, n.beta);
            // }
            // depth += e;
            // we are now looking at moves (null, killer, generated etc) so this is an interior node
        self.stats.inc_interior_nodes(&n);



        // static eval
        let eval = board.eval_some(&self.eval, Switches::ALL_SCORING);

        // razoring
        if let Some(alphabeta) = self.razor(*last_move, board, eval, &n) {
            return (alphabeta, Category::Razor);
        }



        // let futility = self.futility.can_prune_at_node(
        //     board,
        //     &n,
        //     eval,
        // );
        if let Some(fut_score) = self.futility.standing_pat(board, &mut n, eval, &self.eval) {
            return (fut_score, Category::StandingPat);
        }

        if let Some(beta) = self.nmp(board, &mut n, eval) {
            return (beta, Category::NullMovePrune);
        }

        let mut sorted_moves = self.move_orderer.get_sorted_moves(n, board, tt_mv);
        let mut count = 0;
        let mut quiets = 0;
        while let Some((move_type, mv)) = sorted_moves.next_move(board, self) {
            if self.restrictions.skip_move(ply, &mv) {
                continue;
            }
            count += 1;
            self.stats.inc_move(ply);
            let (ext, allow_red) = self.extensions.extend(
                board,
                &mv,
                &n,
                &self.eval.phaser,
                &mut self.stats,
            );
            if allow_red {
                // check we have a score (and hence a move) before we risk pruning everything VER:0.4.14
                if score > -Score::INFINITY {
                    if let Some(est_score) = self.futility.can_prune_move(&mv, move_type, board, eval, &n, &self.eval) {
                        self.explain_futility(&mv, move_type, est_score, n.alpha);
                        self.stats.inc_fp_move(ply);
                        if score == -Score::INFINITY {
                            score = est_score;

                        }
                        if self.futility.can_prune_remaining_moves(board, move_type, &n) {
                            break;
                        } else {
                            continue;
                        }
                    }
                }
            }
            let mut child_board = board.make_move(&mv);
            self.repetition.push_move(&mv, &child_board);
            self.current_variation.push(mv);
            self.explainer.start(&self.current_variation);
            child_board.set_repetition_count(self.repetition.count(&child_board));
            debug_assert!(
                n.alpha < n.beta || self.minmax,
                "alpha={}, beta={}, minmax={}",
                n.alpha,
                n.beta,
                self.minmax
            );

            let lmr = if !self.minmax {
                self.lmr.lmr(
                    board,
                    &mv,
                    count,
                    quiets,
                    move_type,
                    &child_board,
                    &n,
                    nt,
                    allow_red,
                    tt_mv,
                    &mut self.stats,
                )
            } else {
                0
            };
            if lmr > 0 {
                quiets += 1;
            }

            let pvs = !self.minmax
                && self.pvs.permitted(
                    nt,
                    board,
                    &n,
                );
            let (mut child_score, mut cat) = 
                if pvs {
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
                    self.alphabeta_recursive(
                        &mut child_board,
                        ply + 1,
                        depth + ext - lmr - 1,
                        -n.beta,
                        -n.alpha,
                        &mv,
                    )
                };
            child_score = -child_score;

            if (lmr > 0 && self.lmr.re_search || pvs) && child_score > score && child_score < n.beta
            {
                // research with full window without reduction in depth
                self.stats.inc_pvs_research(ply);
                let res = self.alphabeta_recursive(&mut child_board, ply + 1, depth + ext - 1, -n.beta, -n.alpha, &mv);
                child_score = -res.0;
                cat = res.1;
            }
            board.undo_move(&mv);
            self.current_variation.pop();
            self.explainer.start(&self.current_variation);
            self.repetition.pop();
            if ply > 1 && self.task_control.is_cancelled() {
                return (-Score::INFINITY, Category::Cancelled);
            }

            // println!("move {} score {} alpha {} beta {}", mv, score, alpha, beta);
            if child_score > score {
                score = child_score;
                category = cat;
            }
            if child_score > n.alpha { 
                self.explain_raised_alpha(&mv, child_score, n.alpha);
                n.alpha = child_score;
                bm = mv;
                nt = NodeType::ExactPv;
                debug_assert!(board.is_pseudo_legal_move(&bm), "bm {} on board {}", bm, board);
                self.history.raised_alpha(&n, board, &mv);
                self.record_move(ply, &mv);
            } else {
                self.history.duff(&n, board, &mv);
            }

            if n.alpha >= n.beta && !self.minmax {
                nt = NodeType::LowerCut;
                self.stats.inc_node_cut(ply, move_type, (count - 1) as i32 );
                self.killers.store(ply, &mv);
                self.history.beta_cutoff(&n, board, &mv);
                self.report_refutation(n.ply);
                break;
            }

        }

        if count == 0 {
            self.stats.inc_leaf_nodes(&n);
            if n.is_qs() {
                return (board.eval(
                    &mut self.eval,
                    &n,
                ), Category::Quiesce);
            }
            else {
                return 
                // FIXME VER:0.4.14
                // (board.eval_draw(&mut self.eval, &n),
                (board.eval(
                &mut self.eval,
                &n,
                ), 
                Category::Stalemate);
            }
        } else if nt == NodeType::UpperAll {
            self.stats.inc_node_all(ply);
            // nothing
        } else if nt == NodeType::LowerCut {
            debug_assert!(!bm.is_null())
        } else if nt == NodeType::ExactPv {
            self.stats.inc_node_pv(ply);
            // self.record_new_pv(ply, &bm, false);
            debug_assert!(!bm.is_null())
        } else {
            panic!("Node type {:?} ", nt);
        }

        let entry = TtNode {
            score,
            draft: depth,
            node_type: nt,
            bm, // not set for NodeType::All
        };
        self.tt.store(board.hash(), entry);
        self.explain_node(&bm, nt, score, &self.pv_table.extract_pv_for(ply));
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
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(5))
                .set_callback(Uci::uci_info)
                .build();
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
            let mut search = Algo::new().set_timing_method(TimeControl::Depth(5)).build();
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
            let mut search = Algo::new().set_timing_method(TimeControl::Depth(7)).build();
            search.set_position(pos.clone()).search();
            // println!("{}", search);
            if pos.get("pv").is_ok() {
                let expected_pv = pos.pv()?;
                assert_eq!(
                    search.pv_table.extract_pv().to_string(),
                    expected_pv.to_string(),
                    "#{} {}",
                    i,
                    pos
                );
            }
            println!("Mate in {}", search.score().mate_in().unwrap());
            assert_eq!(search.score().mate_in(), Some(4), "#{} {}", i, pos);
        }
        Ok(())
    }
}
