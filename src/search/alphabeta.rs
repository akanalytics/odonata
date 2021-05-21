use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::eval::score::Score;
use crate::movelist::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::searchprogress::SearchProgress;
use crate::tt::{Entry, NodeType};
use crate::types::{Ply, MAX_PLY};

pub struct AlphaBeta;

// terminology
// ply is moves made. so ply 3 means w-> b-> w-> after which we score position
// 
impl Algo {
    #[inline]
    pub fn is_leaf(&self, ply: Ply) -> bool {
        ply == self.max_depth
    }

    pub fn alphabeta(&mut self, node: &mut Node) {
        self.search_stats.reset_keeping_pv();
        self.pv_table = PvTable::new(MAX_PLY as usize);
        self.search_stats.score =
            self.alphabeta_recursive(node.board, node.ply, node.alpha, node.beta, &Move::NULL_MOVE);

        let pv = if self.tt.use_tt_for_pv {
            self.tt.extract_pv(node.board)
        } else {
            self.pv_table.extract_pv()            
        };
        self.search_stats.record_time_actual_and_completion_status(
            self.max_depth,
            !self.task_control.is_cancelled(),
            pv,
        );
    }

    pub fn alphabeta_recursive(
        &mut self,
        board: &mut Board,
        ply: Ply,
        mut alpha: Score,
        beta: Score,
        last_move: &Move,
    ) -> Score {
       
        debug_assert!(self.max_depth > 0);
        self.report_progress();

        if self.time_up_or_cancelled(ply, false) {
            return Score::MinusInf;
        }

        if board.draw_outcome().is_some() {
            self.search_stats.inc_leaf_nodes(ply);
            return board.eval(&mut self.eval); // will return a draw score
        }

        let mut score = Score::MinusInf;
        let mut bm = Move::NULL_MOVE;
        let mut tt_mv: Option<Move> = None;
        let mut node_type = NodeType::All;

        // FIXME tt probe for leaves?
        if self.is_leaf(ply) {
            let score = self.qsearch(last_move, ply, board, alpha, beta);
            debug_assert!(self.task_control.is_cancelled() || score > Score::MinusInf);
            return score;
        }

        // FIXME avoid the cloned!
        if let Some(entry) = self.tt.probe_by_board(board).cloned() {
            let draft = self.max_depth - ply;
            if entry.draft >= draft {
                self.search_stats.inc_tt_nodes(ply);
                //println!("Entry:{:?}", entry);
                // for bounded scores, we know iterating through the nodes might raise alpha, lower beta
                // doing this now allows us potentuially to cut off without looking at the child nodes
                match entry.node_type {
                    NodeType::Pv => {
                        // previously this position raised alpha, but didnt trigger a cut
                        // no point going through moves as we know what the max score is
                        if entry.score > alpha {
                            self.record_new_pv(ply, &entry.bm, true);
                        }
                        debug_assert!(!entry.bm.is_null());
                        return entry.score;
                    }
                    NodeType::Cut => {
                        // previously this position raised alpha (sufficiently to cause a cut).
                        // not all child nodes were scored, so score is a lower bound
                        // FIXME: probably dont set alpha just the hinted mv and re-search the node
                        if entry.score > alpha {
                            self.record_new_pv(ply, &entry.bm, true);
                            node_type = NodeType::Pv;
                            alpha = entry.score;
                            if alpha >= beta {
                                self.search_stats.inc_cuts(ply);
                                return entry.score;
                            }
                            score = entry.score;
                            bm = entry.bm;
                            tt_mv = Some(entry.bm); // might help with move ordering
                            debug_assert!(!entry.bm.is_null());
                        }
                    }
                    NodeType::All => {
                        // previously this position didnt raise alpha, the score is an upper bound
                        // if the score is still below alpha, this too is an ALL node
                        if entry.score <= alpha {
                            return entry.score;
                        }
                    }
                    NodeType::Unused => panic!(),
                }
            }
        }

        self.search_stats.inc_interior_nodes(ply);


        let mut moves = board.legal_moves();
        if moves.is_empty() {
            self.search_stats.inc_leaf_nodes(ply);
            return board.eval(&mut self.eval);
        }

        self.order_moves(ply, &mut moves, &tt_mv);
        for (_i, mv) in moves.iter().enumerate() {
            let mut child_board = board.make_move(mv);
            self.repetition.push(&mv, &child_board);
            child_board.set_repetition_count(self.repetition.count(&child_board));
            debug_assert!(alpha < beta || self.minmax);
            self.current_variation.set_last_move(ply + 1, mv);

            let child_score = -self.alphabeta_recursive(&mut child_board, ply + 1, -beta, -alpha, &mv);
            board.undo_move(mv);
            self.repetition.pop();
            if self.task_control.is_cancelled() {
                return Score::MinusInf;
            }
            debug_assert!(child_score > Score::MinusInf);

            // println!("move {} score {} alpha {} beta {}", mv, score, alpha, beta);
            if child_score > score {
                score = child_score;
            }
            if child_score > alpha {
                alpha = child_score;
                bm = *mv;
                node_type = NodeType::Pv;
                self.record_new_pv(ply, &bm, false);
                debug_assert!(!bm.is_null(), "bm is null at {} mv {}", board, mv );

            }

            if alpha >= beta && !self.minmax {
                node_type = NodeType::Cut;
                break;
            }
        }

        if node_type == NodeType::All {
            // nothing
        } else if node_type == NodeType::Cut {
            self.search_stats.inc_cuts(ply);
            debug_assert!(!bm.is_null())
        } else if node_type == NodeType::Pv {
            // self.record_new_pv(ply, &bm, false);
            debug_assert!(!bm.is_null())
        } else {
            panic!("Node type {:?} ", node_type);
        }
        if self.tt.enabled() {
            let entry = Entry {
                score,
                draft: self.max_depth - ply,
                node_type,
                bm, // not set for NodeType::All
            };
            self.tt.store(board.hash(), entry);
        }
        self.current_variation.set_last_move(ply, &Move::NULL_MOVE);
        score
    }

    fn record_new_pv(&mut self, ply: Ply, mv: &Move, terminal_move: bool) {
        self.pv_table.set(ply + 1, mv, terminal_move);
        self.pv_table.propagate_from(ply + 1);
        self.search_stats.inc_improvements(ply);
        if ply == 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            self.task_control.invoke_callback(&sp);
        }
        
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
            search.qsearch.see = true;
            // search.tt.enabled = false;
            search.search(pos.board());
            println!("{}", search);
            assert_eq!(search.pv().to_string(), pos.pv()?.to_string(), "#{} {}", i, pos);
            assert_eq!(
                search.score(),
                Score::WhiteWin { minus_ply: -3 - pos.board().total_halfmoves() },
                "#{} {}",
                i,
                pos
            );
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_mate_in_3_sync() -> Result<(), String> {
        let position = Catalog::mate_in_3()[0].clone();
        let expected_pv = position.pv()?;
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(5)).build();
        search.tt.enabled = false;
        search.qsearch.see = true;
        search.search(position.board());
        println!("{}", search);
        assert_eq!(search.pv_table.extract_pv(), expected_pv);
        assert_eq!(search.score(), Score::WhiteWin { minus_ply: -4 });
        Ok(())
    }
}
