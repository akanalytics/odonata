use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::TtNode;
use crate::eval::score::{Score, ToScore};
use crate::mv::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::MAX_PLY;

use super::node::Event;

pub struct AlphaBeta;

impl Algo {
    pub fn run_alphabeta(&mut self, board: &mut Board, n: &mut Node) -> (Score, Event) {
        self.max_depth = n.depth;
        self.stats.depth = n.depth;
        self.pv_table = PvTable::new(MAX_PLY as usize);
        debug_assert!(self.current_variation.len() == 0);

        let (score, category) = match self.alphabeta(board, *n, Move::NULL_MOVE) {
            Ok((score, category)) => (score, category),
            Err(category) => (-Score::INFINITY, category),
        };
        self.stats.set_score(score, category);
        debug_assert!(
            self.current_variation.len() == 0
                || category == Event::UserCancelled
                || category == Event::TimeUp
        );
        let (pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (self.pv_table.extract_pv(), Some(Score::default()))
        };
        self.results.set_pv(category, &pv);
        self.results.score = score;
        if n.alpha == -Score::INFINITY
            && n.beta == Score::INFINITY
            && category != Event::TimeUp
            && category != Event::UserCancelled
        {
            debug_assert!(
                score.is_numeric_or_mate(),
                "Score was inf: node {:?} cat {} \n{}",
                n,
                category,
                self
            );
        }

        self.stats.record_iteration(self.max_depth, category, pv);
        (score, category)
    }

    // score >= beta fail high
    // score <= alpha fail low
    // for [alpha-1, a] a score of either bound indicates fail low or high
    pub fn alphabeta(
        &mut self,
        b: &mut Board,
        mut n: Node,
        last_move: Move,
    ) -> Result<(Score, Event), Event> {
        self.clear_move(n.ply);
        self.report_progress();

        let (cancelled, mut cat) = self.time_up_or_cancelled(n.ply, false);
        if cancelled {
            self.counts.inc(&n, cat);
            return Err(cat);
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
            self.stats.inc_zw_nodes(n.ply);
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
        if n.ply > 0 && b.draw_outcome().is_some() {
            // if board.draw_outcome().is_some() {
            self.stats.inc_leaf_nodes(&n);
            self.counts.inc(&n, Event::NodeLeafDraw);
            return Ok((b.eval_draw(&mut self.eval, &n), Event::NodeLeafDraw)); // will return a draw score
        }

        let ext = self.extensions(b, &n);
        n.depth += ext;

        let mut tt_mv = Move::NULL_MOVE;
        match self.lookup(b, &mut n) {
            (Some(ab), None) => {
                debug_assert!(ab.is_numeric_or_mate(), "lookup returned {}", ab);
                return Ok((ab, Event::HashHit));
            } // alpha, beta or a terminal node
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
        let mut eval = b.eval_some(&self.eval);

        if self.tt.use_tt_for_eval {
            if let Some(entry) = self.tt.probe_by_board(b, n.ply, n.depth) {
                // if entry.depth >= n.depth {
                if entry.nt == NodeType::ExactPv {
                    eval = entry.score;
                } else if entry.nt == NodeType::LowerCut && entry.score > eval {
                    eval = entry.score;
                } else if entry.nt == NodeType::UpperAll && entry.score < eval {
                    eval = entry.score;
                }
                // }
            }
        }
        if let Some(s) = self.standing_pat(b, &mut n, eval) {
            return Ok((s, Event::PruneStandingPat));
        }
        if let Some(alphabeta) = self.razor(last_move, b, eval, &n)? {
            return Ok((alphabeta, Event::PruneRazor));
        }
        if let Some(s) = self.nmp(b, &n, eval)? {
            return Ok((s, Event::PruneNullMovePrune));
        }

        let mut sorted_moves = self
            .move_orderer
            .create_sorted_moves(n, b, tt_mv, last_move);
        let mut count = 0;
        let mut quiets = 0;
        while let Some((move_type, mv)) = sorted_moves.next_move(b, self) {
            if self.restrictions.skip_move(n.ply, &mv) {
                continue;
            }
            self.counts.inc(&n, Event::Moves);
            self.counts.inc_move(&n, move_type);
            count += 1;
            self.stats.inc_move(n.ply);

            let is_quiet = self.is_quiet(b, mv, count, move_type, &n, ext);
            if is_quiet {
                quiets += 1;
            }
            if !n.is_pv() && self.can_lmp(is_quiet, quiets, &n) {
                continue;
            }

            if score > -Score::INFINITY {
                if let Some(est_score) =
                    self.can_futility_prune_move(mv, count, move_type, b, eval, &n, ext)
                {
                    self.stats.inc_fp_move(n.ply);
                    if score == -Score::INFINITY {
                        score = est_score;
                    }
                    if self.can_prune_remaining_moves(b, move_type, &n) {
                        break;
                    }
                    continue;
                }
            }

            let mut b2 = b.make_move(&mv);
            self.repetition.push_move(&mv, &b2);
            self.current_variation.push(mv);
            // self.explainer.start(&n, &self.current_variation);
            b2.set_repetition_count(self.repetition.count(&b2));

            let r = if !n.is_pv() && !self.minmax {
                self.lmr(b, mv, count, quiets, move_type, &b2, &n, nt, ext, tt_mv)
            } else {
                0
            };

            // if first move:
            //    a,b with ext.
            // else
            //    node logic around reductions. Node No ext. Not in check. No mate in eval/a/b
            //    node lofic around eval improving. is Pv (not zw)
            //    move: hash, killer, counter
            //

            // let pvs = self.pvs_permitted(nt, b, &n, count);

            // let pvs =
            let mut s = n.alpha + Score::from_cp(1);
            debug_assert!(s > n.alpha, "S > alpha failed for {} > {}", s, n.alpha);
            let mut ev; //  = Event::Unknown;

            if count == 1 || n.is_qs() {
                (s, ev) = self.alphabeta(
                    &mut b2,
                    Node {
                        ply: n.ply + 1,
                        depth: n.depth - 1,
                        alpha: -n.beta,
                        beta: -n.alpha,
                    },
                    mv,
                )?;
                s = -s;
            } else {
                (s, ev) = self.alphabeta(
                    &mut b2,
                    Node {
                        ply: n.ply + 1,
                        depth: n.depth - r - 1,
                        alpha: -n.alpha - 1.cp(),
                        beta: -n.alpha,
                    },
                    mv,
                )?;
                s = -s;
                // full window
                if s > n.alpha && !(r == 0 && n.is_zw()) {
                    (s, ev) = self.alphabeta(
                        &mut b2,
                        Node {
                            ply: n.ply + 1,
                            depth: n.depth - 1,
                            alpha: -n.beta,
                            beta: -n.alpha,
                        },
                        mv,
                    )?;
                    s = -s;
                }
            }
            let cat = ev;

            b.undo_move(&mv);
            self.current_variation.pop();
            self.repetition.pop();
            self.explain_move(&b, mv, s, cat, &n, count, ext, r);

            if s > score {
                score = s;
                category = cat;
            }
            if s > n.alpha {
                n.alpha = s;
                bm = mv;
                nt = NodeType::ExactPv;
                debug_assert!(b.is_pseudo_legal_move(&bm), "bm {} on board {}", bm, b);
                self.history.raised_alpha(&n, b, &mv);
                self.record_move(n.ply, &mv);
            } else {
                self.history.duff(&n, b, &mv);
            }

            if n.alpha >= n.beta && !self.minmax {
                nt = NodeType::LowerCut;
                self.stats
                    .inc_node_cut(n.ply, move_type, (count - 1) as i32);
                self.killers.store(n.ply, &mv);
                self.history.beta_cutoff(&n, b, &mv);
                self.counter_move.store(b.color_us(), last_move, mv);
                self.report_refutation(n.ply);
                break;
            }
        }

        if count == 0 {
            self.stats.inc_leaf_nodes(&n);
            if n.is_qs() {
                self.counts.inc(&n, Event::NodeLeafQuietEval);
                return Ok((
                    b.eval_with_outcome(&self.eval, &n),
                    Event::NodeLeafQuietEval,
                ));
            } else {
                self.counts.inc(&n, Event::NodeLeafStalemate);
                // FIXME VER:0.4.14
                // (board.eval_draw(&mut self.eval, &n),
                return Ok((
                    b.eval_with_outcome(&self.eval, &n),
                    Event::NodeLeafStalemate,
                ));
            }
        }
        if nt == NodeType::UpperAll {
            self.stats.inc_node_all(n.ply);
            cat = Event::NodeInteriorAll;
            self.counts.inc(&n, cat);
            // nothing
        } else if nt == NodeType::LowerCut {
            debug_assert!(!bm.is_null());
            cat = Event::NodeInteriorCut;
            self.counts.inc(&n, cat);
        } else if nt == NodeType::ExactPv {
            self.stats.inc_node_pv(n.ply);
            cat = Event::NodeInteriorPv;
            self.counts.inc(&n, cat);
            // self.record_new_pv(ply, &bm, false);
            debug_assert!(!bm.is_null())
        } else {
            panic!("Node type {:?} ", nt);
        }
        // aspiration search fails dont get stored
        if score > -Score::INFINITY && score < Score::INFINITY {
            let entry = TtNode {
                score,
                depth: n.depth,
                nt,
                bm,
            };
            self.tt.store(b.hash(), entry);
        }
        debug_assert!(
            n.alpha != -Score::INFINITY || n.beta != Score::INFINITY || score.is_numeric_or_mate(),
            "alpha/beta full width with score: {} node {}\ncat: {} count: {} quiets: {}\nbm: {}\nnt: {}\nboard: {:#}",
            score,
            n,
            category,
            count,
            quiets,
            bm,
            nt,
            b,
        );
        self.explain_node(
            &b,
            bm,
            nt,
            score,
            eval,
            &n,
            cat,
            &self.pv_table.extract_pv_for(n.ply),
        );
        Ok((score, category))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::search::timecontrol::*;
    use anyhow::Result;

    #[test]
    fn test_2_mates() -> Result<()> {
        let positions = Catalog::mate_in_2();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            search.set_timing_method(TimeControl::Depth(8)); // .set_callback(Uci::uci_info);
                                                             // search.tt.enabled = false;
            search.set_position(pos.clone()).search();
            // println!("{}", search);
            assert_eq!(
                search.pv().to_string(),
                pos.pv()?.to_string(),
                "#{} {}",
                i,
                pos
            );
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
