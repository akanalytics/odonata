use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::{TtNode};
use crate::eval::score::Score;
use crate::mv::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::{MoveType, Ply, MAX_PLY};
use crate::eval::switches::Switches;


pub struct AlphaBeta;

// terminology
// ply is moves made. so ply 3 means w-> b-> w-> after which we score position
//
impl Algo {
    #[inline]
    pub fn is_leaf(&self, _ply: Ply, depth: Ply) -> bool {
        depth <= 0
    }

    pub fn run_alphabeta(&mut self, board: &mut Board, node: &mut Node) {
        let depth = node.depth;
        self.max_depth = depth;
        self.stats.depth = depth;
        self.stats.new_iteration();
        self.pv_table = PvTable::new(MAX_PLY as usize);
        debug_assert!(self.current_variation.len() == 0);
        self.stats.score = self.alphabeta_recursive(
            board,
            node.ply,
            self.max_depth,
            node.alpha,
            node.beta,
            &Move::NULL_MOVE,
        );
        debug_assert!(self.current_variation.len() == 0);
        let (pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (self.pv_table.extract_pv(), Some(Score::default()))
        };
        self.stats
            .record_iteration(self.max_depth, !self.task_control.is_cancelled(), pv);

    }

    pub fn alphabeta_recursive(
        &mut self,
        board: &mut Board,
        ply: Ply,
        depth: Ply,
        alpha: Score,
        beta: Score,
        last_move: &Move,
    ) -> Score {
        // debug_assert!(depth > 0);
        self.clear_move(ply);
        self.report_progress();

        if self.time_up_or_cancelled(ply, false) {
            return -Score::INFINITY;
        }

        let mut n = Node {
            ply,
            depth,
            alpha,
            beta,
        };

        if n.alpha + Score::from_cp(1) == n.beta {
            self.stats.inc_zw_nodes(ply);
        }


        // we dont draw at root, as otherwise it wont play a move if insufficient-material draw [v32]
        if ply > 0 && board.draw_outcome().is_some() {
        // if board.draw_outcome().is_some() {
            self.stats.inc_leaf_nodes(ply);
            return board.eval_draw(&mut self.eval, &n); // will return a draw score
        }

        let mut score = -Score::INFINITY;
        let mut bm = Move::NULL_MOVE;
        let mut nt = NodeType::All;

        if !self.tt.probe_leaf_nodes && self.is_leaf(ply, depth) {
            // entering qsearch is a leaf node
            self.stats.inc_leaf_qsearch_nodes(ply);
            return self.qsearch(last_move, ply, depth, board, n.alpha, n.beta);
        }

        let draft = depth;
        let mut tt_mv = Move::NULL_MOVE;
        if let Some(entry) = self.tt.probe_by_board(board, ply, draft) {
            // we use thr tt_mv for ordering regardless of draft
            tt_mv = entry.bm;
            // FIXME! v33
            if entry.draft >= draft && !(board.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats)
            //if entry.draft >= draft  && (ply >= 1 || self.tt.allow_tt_at_root) && !(board.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats)
 
            // if entry.draft >= draft && !(board.repetition_count().total > 1 && self.repetition.avoid_tt_on_repeats)
            {
                match entry.node_type {
                    NodeType::Pv => {
                        // previously this position raised alpha, but didnt trigger a cut
                        // no point going through moves as we know what the max score is
                        if entry.score >= n.beta {
                            self.stats.inc_node_cut(ply, MoveType::Hash, -1);
                            self.stats.inc_leaf_tt_nodes(ply);
                            self.report_refutation(n.ply);
                            return entry.score;
                        }
                        if entry.score <= n.alpha {
                            self.stats.inc_node_all(ply);
                            self.stats.inc_leaf_tt_nodes(ply);
                            return entry.score;
                        }

                        if self.tt.allow_truncated_pv && entry.score > n.alpha {
                            self.record_truncated_move(ply, &entry.bm);
                            self.stats.inc_leaf_tt_nodes(ply);
                            return entry.score;
                        }
                        // else we just use the hash move for move ordering
                    }
                    NodeType::Cut => {
                        // previously this position raised alpha (sufficiently to cause a cut).
                        // not all child nodes were scored, so score is a lower bound
                        // FIXME: probably dont set alpha just the hinted mv and re-search the node
                        if entry.score >= n.beta {
                            self.stats.inc_node_cut(ply, MoveType::Hash, -1);
                            self.tt.store(board.hash(), entry);
                            // self.record_truncated_move(ply, &entry.bm);
                            self.stats.inc_leaf_tt_nodes(ply);
                            self.report_refutation(n.ply);
                            return entry.score;
                        }
                        if self.tt.allow_truncated_pv && entry.score > n.alpha {
                            nt = NodeType::Pv;
                            n.alpha = entry.score;
                            self.record_truncated_move(ply, &entry.bm);
                            score = entry.score;
                            bm = entry.bm; // need to set bm as alpha raising mv might be skipped
                                           // tt_mv = Some(entry.bm); // might help with move ordering
                        }
                    }
                    NodeType::All => {
                        // previously this position didnt raise alpha, the score is an upper bound
                        // if the score is still below alpha, this too is an ALL node
                        if entry.score <= n.alpha {
                            // self.record_truncated_move(ply, &entry.bm);
                            self.stats.inc_leaf_tt_nodes(ply);
                            return entry.score;
                        }
                    }
                    NodeType::Unused | NodeType::Terminal => unreachable!(),
                }
            }
        }



        if self.tt.probe_leaf_nodes && self.is_leaf(ply, depth) {
            let e = self.extensions.extend_at_leaf(board);
            if e == 0 {
                self.stats.inc_leaf_qsearch_nodes(ply);
                return self.qsearch(last_move, ply, depth, board, n.alpha, n.beta);
            }
            // depth += e;
        }


        // we are now looking at moves (null, killer, generated etc) so this is an interior node
        self.stats.inc_interior_nodes(ply);

        // static eval
        let eval = board.eval_some(&self.eval, Switches::ALL_SCORING);

        // razoring
        if let Some(alphabeta) = self.razor(*last_move, board, eval, &n) {
            return alphabeta;
        }



        // let futility = self.futility.can_prune_at_node(
        //     board,
        //     &n,
        //     eval,
        // );
        // if let Some(futility) = futility {
        //     if let Some(fut_score) = self.futility.can_prune_all_moves(board, futility, &n, &self.eval) {
        //         return fut_score;
        //     }
        // }
        // null move
        if !self.minmax
            && self.nmp.allow(
                &board,
               &n,
                &self.pv_table,
            )
        {
            let r = self.nmp.depth_reduction(eval, &n);
            let mv = Move::NULL_MOVE;
            // try futility pruning on null move
            // if let Some(futility) = futility {
            //     if score > -Score::INFINITY {
            //         if let Some(score) = self.futility.can_prune_move(&mv, board, futility, &n, &self.eval) {
            //             if score >= n.beta {
            //                 self.search_stats.inc_node_cut(ply, MoveType::Null);
            //                 return score;
            //             }
            //         }
            //     }
            // }
            let mut child_board = board.make_move(&mv);
            self.current_variation.push(mv);
            self.explainer.start(&self.current_variation);
            self.stats.inc_nmp(ply);
            let child_score = -self.alphabeta_recursive(
                &mut child_board,
                ply + 1,
                depth - r - 1,
                -n.beta,
                -n.beta + Score::from_cp(1),
                &mv,
            );
            board.undo_move(&mv);
            self.current_variation.pop();
            self.explainer.start(&self.current_variation);
            if child_score >= n.beta {
                self.stats.inc_node_cut(ply, MoveType::Null, -1);
                self.report_refutation(n.ply);
                self.explain_nmp(child_score, n.beta);
                return child_score;
            }
        }

        let mut sorted_moves = self.move_orderer.get_sorted_moves(ply, tt_mv);
        let mut count = 0;
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
                // check we have a score (and hence a move) nefore we risk pruning everything
                // if score > -Score::INFINITY {
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
                    move_type,
                    &child_board,
                    &n,
                    nt,
                    allow_red,
                    &mut self.stats,
                )
            } else {
                0
            };

            let pvs = !self.minmax
                && self.pvs.permitted(
                    nt,
                    board,
                    &n,
                );
            let mut child_score;
            if pvs {
                debug_assert!(n.alpha.is_numeric());
                self.stats.inc_pvs_move(ply);
                // using [alpha, alpha + 1]
                child_score = -self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - lmr - 1,
                    -n.alpha - Score::from_cp(1),
                    -n.alpha,
                    &mv,
                );
            } else {
                child_score = -self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - lmr - 1,
                    -n.beta,
                    -n.alpha,
                    &mv,
                );
            }

            if (lmr > 0 && self.lmr.re_search || pvs) && child_score > score && child_score < n.beta
            {
                // research with full window without reduction in depth
                self.stats.inc_pvs_research(ply);
                child_score =
                    -self.alphabeta_recursive(&mut child_board, ply + 1, depth + ext - 1, -n.beta, -n.alpha, &mv);
            }
            board.undo_move(&mv);
            self.current_variation.pop();
            self.explainer.start(&self.current_variation);
            self.repetition.pop();
            if ply > 1 && self.task_control.is_cancelled() {
                return -Score::INFINITY;
            }

            // println!("move {} score {} alpha {} beta {}", mv, score, alpha, beta);
            if child_score > score {
                score = child_score;
            }
            if child_score > n.alpha { 
                self.explain_raised_alpha(&mv, child_score, n.alpha);
                n.alpha = child_score;
                bm = mv;
                nt = NodeType::Pv;
                debug_assert!(board.is_pseudo_legal_move(&bm));
                self.history.raised_alpha(ply, board, &mv);
                self.record_move(ply, &mv);
            }

            if n.alpha >= n.beta && !self.minmax {
                nt = NodeType::Cut;
                self.stats.inc_node_cut(ply, move_type, (count - 1) as i32 );
                self.killers.store(ply, &mv);
                self.history.beta_cutoff(ply, board, &mv);
                self.report_refutation(n.ply);
                break;
            }
        }

        if count == 0 {
            // nt = NodeType::Terminal;
            self.stats.inc_leaf_nodes(ply);
            return board.eval(
                &mut self.eval,
                &n,
            ); // Score::DRAW;
        } else if nt == NodeType::All {
            self.stats.inc_node_all(ply);
            // nothing
        } else if nt == NodeType::Cut {
            debug_assert!(!bm.is_null())
        } else if nt == NodeType::Pv {
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
        score
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
