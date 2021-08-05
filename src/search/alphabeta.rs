use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::cache::tt2::{NodeType, TtNode};
use crate::eval::score::Score;
use crate::mv::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::{MoveType, Ply, MAX_PLY};

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
        self.search_stats.reset_keeping_pv();
        self.pv_table = PvTable::new(MAX_PLY as usize);
        self.search_stats.score = self.alphabeta_recursive(
            board,
            node.ply,
            self.max_depth,
            node.alpha,
            node.beta,
            &Move::NULL_MOVE,
        );

        let (pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (self.pv_table.extract_pv(), Some(Score::default()))
        };
        self.search_stats
            .record_iteration(self.max_depth, !self.task_control.is_cancelled(), pv);
    }

    pub fn alphabeta_recursive(
        &mut self,
        board: &mut Board,
        ply: Ply,
        depth: Ply,
        mut alpha: Score,
        beta: Score,
        last_move: &Move,
    ) -> Score {
        // debug_assert!(depth > 0);
        self.clear_move(ply);
        self.report_progress();

        if self.time_up_or_cancelled(ply, false) {
            return -Score::INFINITY;
        }

        if alpha + Score::from_cp(1) == beta {
            self.search_stats.inc_zw_nodes(ply);
        }

        if board.draw_outcome().is_some() {
            self.search_stats.inc_leaf_nodes(ply);
            return board.eval_draw(&mut self.eval); // will return a draw score
        }

        let mut score = -Score::INFINITY;
        let mut bm = Move::NULL_MOVE;
        let mut nt = NodeType::All;

        if !self.tt.probe_leaf_nodes && self.is_leaf(ply, depth) {
            return self.qsearch(last_move, ply, depth, board, alpha, beta);
        }

        let draft = depth;
        let mut tt_mv = Move::NULL_MOVE;
        if let Some(entry) = self.tt.probe_by_board(board, ply, draft) {
            // we use thr tt_mv for ordering regardless of draft
            tt_mv = entry.bm;
            if entry.draft >= draft && !(board.repetition_count() > 0 && self.repetition.avoid_tt_on_repeats)
            {
                match entry.node_type {
                    NodeType::Pv => {
                        // previously this position raised alpha, but didnt trigger a cut
                        // no point going through moves as we know what the max score is
                        if entry.score >= beta {
                            self.search_stats.inc_node_cut(ply, MoveType::Hash);
                            self.search_stats.inc_tt_nodes(ply);
                            return entry.score;
                        }
                        if entry.score <= alpha {
                            self.search_stats.inc_node_all(ply);
                            self.search_stats.inc_tt_nodes(ply);
                            return entry.score;
                        }

                        if self.tt.allow_truncated_pv && entry.score > alpha {
                            self.record_truncated_move(ply, &entry.bm);
                            self.search_stats.inc_tt_nodes(ply);
                            return entry.score;
                        }
                        // else we just use the hash move for move ordering
                    }
                    NodeType::Cut => {
                        // previously this position raised alpha (sufficiently to cause a cut).
                        // not all child nodes were scored, so score is a lower bound
                        // FIXME: probably dont set alpha just the hinted mv and re-search the node
                        if entry.score >= beta {
                            self.search_stats.inc_node_cut(ply, MoveType::Hash);
                            self.tt.store(board.hash(), entry);
                            // self.record_truncated_move(ply, &entry.bm);
                            self.search_stats.inc_tt_nodes(ply);
                            return entry.score;
                        }
                        if self.tt.allow_truncated_pv && entry.score > alpha {
                            nt = NodeType::Pv;
                            alpha = entry.score;
                            self.record_truncated_move(ply, &entry.bm);
                            score = entry.score;
                            bm = entry.bm; // need to set bm as alpha raising mv might be skipped
                                           // tt_mv = Some(entry.bm); // might help with move ordering
                        }
                    }
                    NodeType::All => {
                        // previously this position didnt raise alpha, the score is an upper bound
                        // if the score is still below alpha, this too is an ALL node
                        if entry.score <= alpha {
                            // self.record_truncated_move(ply, &entry.bm);
                            self.search_stats.inc_tt_nodes(ply);
                            return entry.score;
                        }
                    }
                    NodeType::Unused | NodeType::Terminal => unreachable!(),
                }
            }
        }

        if self.tt.probe_leaf_nodes && self.is_leaf(ply, depth) {
            return self.qsearch(last_move, ply, depth, board, alpha, beta);
        }

        self.search_stats.inc_interior_nodes(ply);

        let futility = self.futility.can_prune_at_node(
            board,
            &Node {
                ply,
                depth,
                alpha,
                beta,
            },
            &self.eval,
        );

        // null move
        if !self.minmax
            && beta.is_numeric()
            && self.nmp.allow(
                &board,
                &Node {
                    ply,
                    depth,
                    alpha,
                    beta,
                },
                &self.pv_table,
            )
        {
            let r = self.nmp.depth_reduction(board, ply, depth);
            let mv = Move::NULL_MOVE;
            let mut child_board = board.make_move(&mv);
            self.current_variation.set_last_move(ply + 1, &mv);
            self.search_stats.inc_nmp(ply);
            let child_score = -self.alphabeta_recursive(
                &mut child_board,
                ply + 1,
                depth - r - 1,
                -beta,
                -beta + Score::from_cp(1),
                &mv,
            );
            board.undo_move(&mv);
            if child_score >= beta {
                self.search_stats.inc_node_cut(ply, MoveType::Null);
                return child_score;
            }
        }

        let mut sorted_moves = self.move_orderer.get_sorted_moves(ply, tt_mv);
        let mut count = 0;
        while let Some((move_type, mv)) = sorted_moves.next_move(board, self) {
            count += 1;
            self.search_stats.inc_move(ply);
            if futility && score > -Score::INFINITY && self.futility.can_prune_move(&mv, board) {
                self.search_stats.inc_fp_move(ply);
                continue;
            }
            let mut child_board = board.make_move(&mv);
            self.repetition.push_move(&mv, &child_board);
            child_board.set_repetition_count(self.repetition.count(&child_board));
            debug_assert!(
                alpha < beta || self.minmax,
                "alpha={}, beta={}, minmax={}",
                alpha,
                beta,
                self.minmax
            );
            self.current_variation.set_last_move(ply + 1, &mv);

            let ext = self.extensions.extend(
                board,
                &mv,
                &child_board,
                &Node {
                    ply,
                    depth,
                    alpha,
                    beta,
                },
                &mut self.search_stats,
            );
            let lmr = self.reductions.lmr(
                board,
                &mv,
                count,
                move_type,
                &child_board,
                &Node {
                    ply,
                    depth,
                    alpha,
                    beta,
                },
                nt, 
                &mut self.search_stats,
            );

            let pvs = !self.minmax && self.pvs.permitted(
                nt,
                board,
                &Node {
                    ply,
                    depth,
                    alpha,
                    beta,
                },
            );
            
            let mut child_score;
            if pvs {
                debug_assert!(alpha.is_numeric());
                self.search_stats.inc_pvs_move(ply);
                // using [alpha, alpha + 1]
                child_score = -self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - lmr - 1,
                    -alpha - Score::from_cp(1),
                    -alpha,
                    &mv,
                );
            } else {
                child_score =
                    -self.alphabeta_recursive(&mut child_board, ply + 1, depth + ext - lmr - 1, -beta, -alpha, &mv);
            } 

            if (lmr > 0 && self.reductions.lmr_re_search || pvs) && child_score > score && child_score < beta {
                // research with full window without reduction in depth
                self.search_stats.inc_pvs_research(ply);
                child_score = -self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - 1,
                    -beta,
                    -alpha,
                    &mv,
                );
            }
            board.undo_move(&mv);
            self.repetition.pop();
            if ply > 1 && self.task_control.is_cancelled() {
                return -Score::INFINITY;
            }

            // println!("move {} score {} alpha {} beta {}", mv, score, alpha, beta);
            if child_score > score {
                score = child_score;
            }
            if child_score > alpha {
                alpha = child_score;
                bm = mv;
                nt = NodeType::Pv;
                debug_assert!(board.is_pseudo_legal_move(&bm));
                self.record_move(ply, &mv);
            }

            if alpha >= beta && !self.minmax {
                nt = NodeType::Cut;
                self.search_stats.inc_node_cut(ply, move_type);
                self.killers.store(ply, &mv);
                break;
            }
        }

        if count == 0 {
            // nt = NodeType::Terminal;
            self.search_stats.inc_leaf_nodes(ply);
            return board.eval(
                &mut self.eval,
                &Node {
                    ply,
                    depth,
                    alpha,
                    beta,
                },
            );
        } else if nt == NodeType::All {
            self.search_stats.inc_node_all(ply);
            // nothing
        } else if nt == NodeType::Cut {
            debug_assert!(!bm.is_null())
        } else if nt == NodeType::Pv {
            self.search_stats.inc_node_pv(ply);
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
        self.current_variation.set_last_move(ply, &Move::NULL_MOVE);
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::Uci;
    use crate::search::timecontrol::*;

    #[test]
    fn test_2_mates() -> Result<(), String> {
        let positions = Catalog::mate_in_2();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(3))
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
    fn test_mate_in_3() -> Result<(), String> {
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
    fn test_mate_in_4() -> Result<(), String> {
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
