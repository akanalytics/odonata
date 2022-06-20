use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::TtNode;
use crate::eval::score::Score;
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::other::pvtable::PvTable;
use crate::piece::{Ply, MAX_PLY};
use crate::search::algo::Algo;
use crate::search::node::{Counter, Node, Timing};

use super::node::Event;

pub struct AlphaBeta;

impl Algo {
    pub fn alphabeta_root_search(&mut self, board: &mut Board, n: &mut Node) -> (Score, Event) {
        debug_assert!(n.alpha < n.beta);

        let t = Metrics::timing_start();
        let depth = n.depth;
        self.max_depth = depth;
        // self.stats.depth = depth;
        self.pv_table = PvTable::new(MAX_PLY as usize);
        debug_assert!(self.current_variation.len() == 0);

        let (score, category) = match self.alphabeta(
            board,
            n.ply,
            self.max_depth,
            n.alpha,
            n.beta,
            Move::NULL_MOVE,
        ) {
            Ok((score, category)) => (score, category),
            Err(category) => (-Score::INFINITY, category),
        };

        if self.controller.is_cancelled() {
            Metrics::incr(Counter::SearchTimeUp);
        } else {
            Metrics::incr(Counter::SearchComplete);
        }

        debug_assert!(
            self.current_variation.len() == 0
                || category == Event::UserCancelled
                || category == Event::SearchTimeUp
        );
        let (pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (self.pv_table.extract_pv(), Some(Score::default()))
        };
        self.progress.set_pv(category, &pv);
        self.progress.score = score;
        if n.alpha == -Score::INFINITY
            && n.beta == Score::INFINITY
            && category != Event::SearchTimeUp
            && category != Event::UserCancelled
        {
            debug_assert!(
                score.is_finite(),
                "Score was inf: node {:?} cat {} \n{}",
                n,
                category,
                self
            );
        }

        // self.stats.record_iteration(self.max_depth, category, pv);
        Metrics::profile(t, Timing::TimingSearchRoot);
        (score, category)
    }

    #[inline]
    fn static_eval(&self, b: &Board, n: &Node) -> Score {
        Metrics::incr_node(n, Event::EvalStatic);
        let mut eval = b.static_eval(&self.eval);

        if self.tt.use_tt_for_eval {
            if let Some(entry) = self.tt.probe_by_board(b, n.ply, 1) {
                if entry.depth >= self.tt.tt_for_eval_depth {
                    if entry.nt == NodeType::ExactPv {
                        Metrics::incr_node(n, Event::TtHitEvalNode);
                        eval = entry.score;
                    } else if entry.nt == NodeType::LowerCut && entry.score > eval {
                        Metrics::incr_node(n, Event::TtHitEvalNode);
                        eval = entry.score;
                    } else if entry.nt == NodeType::UpperAll && entry.score < eval {
                        Metrics::incr_node(n, Event::TtHitEvalNode);
                        eval = entry.score;
                    }
                    // }
                }
            }
        }
        eval
    }

    pub fn alphabeta(
        &mut self,
        b: &mut Board,
        ply: Ply,
        depth: Ply,
        alpha: Score,
        beta: Score,
        last_move: Move,
    ) -> Result<(Score, Event), Event> {
        debug_assert!(alpha < beta);
        debug_assert!(ply >= 0);

        self.clear_move(ply);
        self.report_progress();

        let mut n = Node {
            ply,
            depth,
            alpha,
            beta,
        };
        debug_assert!(n.alpha < n.beta, "alpha={}, beta={}", n.alpha, n.beta,);

        let (cancelled, mut cat) = self.time_up_or_cancelled(ply, false);
        if cancelled {
            return Err(cat);
        }

        self.clock.inc_nodes();
        // self.results.set_seldepth(&n);

        if n.is_qs() {
            Metrics::incr_node(&n, Event::NodeLeafQs);
            let t = Metrics::timing_start();
            // QS starts from ply=0
            let s = self.qs(Node { ply: 0, ..n }, b, Some(last_move));
            Metrics::profile(t, Timing::TimingQs);
            return Ok((s, Event::NodeLeafQs));
        }

        Metrics::incr_node(&n, Event::NodeTotal);
        if n.is_zw() {
            Metrics::incr_node(&n, Event::NodeZw);
        }

        let mut score = -Score::INFINITY;
        let mut category = Event::Unknown;
        let mut bm = None;
        let mut nt = NodeType::UpperAll;

        // we dont draw at root, as otherwise it wont play a move if insufficient-material draw [v32]
        if ply > 0 && b.draw_outcome().is_some() {
            Metrics::incr_node(&n, Event::NodeLeafDraw);
            return Ok((b.eval_draw(&mut self.eval, &n), Event::NodeLeafDraw)); // will return a draw score
        }

        let mut tt_mv = Move::NULL_MOVE;
        match self.lookup(b, &mut n) {
            (Some(ab), None) => {
                debug_assert!(ab.is_finite(), "lookup returned {}", ab);
                return Ok((ab, Event::HashHit));
            } // alpha, beta or a terminal node
            (None, Some(bm)) => tt_mv = bm,
            (Some(s), Some(mv)) => {
                tt_mv = mv;
                if !mv.is_null() {
                    bm = Some(mv);
                    score = s;
                }
                category = Event::HashHit;
            }
            _ => {}
        }

        // static eval
        let eval = self.static_eval(b, &n);

        // use crate::eval::score::ToScore;
        // if self.show_metrics_on_exit
        //     && n.depth < 7 && n.beta.is_numeric()
        //     && !b.is_in_check(b.color_us()) && n.is_zw()
        //     && eval - n.depth * 110_i32.cp() >= n.beta
        // {
        //     return Ok((eval - n.depth * 110_i32.cp(), Event::StandingPatSuccess));
        // }

        if let Some(s) = self.reverse_fut(b, eval, &n, 0) {
            return Ok((s, Event::RevFutPrune));
        }
        if let Some(alphabeta) = self.razor_node(last_move, b, eval, &n)? {
            return Ok((alphabeta, Event::PruneRazor));
        }
        if let Some(s) = self.nmp_node(b, &n, eval)? {
            return Ok((s, Event::NmpSuccess));
        }

        let mut sorted_moves = self
            .move_orderer
            .create_sorted_moves(n, b, tt_mv, last_move);
        let mut count = 0;
        let mut quiets = 0;
        while let Some((mt, mv)) = sorted_moves.next_move(b, self) {
            if self.restrictions.skip_move(ply, &mv) {
                continue;
            }
            Metrics::incr_node(&n, Event::MoveCount);
            let mut child_board = b.make_move(&mv);
            let ext = self.extend(b, &child_board, mv, mt, count, &n, last_move);
            let is_quiet = self.is_quiet(b, mv, mt, &child_board, &n, ext);
            if is_quiet {
                quiets += 1;
            }

            if bm.is_some() {
                if let Some(_est) =
                    self.can_futility_prune_move(mv, count, mt, b, &child_board, eval, &n, ext)
                {
                    if self.can_prune_remaining_moves(b, mt, &n) {
                        break;
                    }
                    continue;
                }
                if self.can_lmp_move(b, count, is_quiet, quiets, &n, mv) {
                    continue;
                }
            }

            self.repetition.push_move(&mv, &child_board);
            self.current_variation.push(mv);
            // self.explainer.start(&n, &self.current_variation);
            child_board.set_repetition_count(self.repetition.count(&child_board));

            let mut s; //= n.alpha + Score::from_cp(1);
                       // debug_assert!(s > n.alpha);
            let mut ev; //  = Event::Unknown;


            count += 1;
            let lmr = self.lmr(b, mv, count, quiets, mt, &child_board, &n, nt, ext, tt_mv);
            if n.is_fw() && !self.pvs_permitted(nt, b, &n, count) {
                Metrics::incr_node(&n, Event::SearchFwFd);
                (s, ev) = self.alphabeta(
                    &mut child_board,
                    ply + 1,
                    depth + ext - 1,
                    -n.beta,
                    -n.alpha,
                    mv,
                )?;
                s = -s;
                // // full depth
                // if s > n.alpha && !(lmr == 0) {
                // Metric::incr_node(&n, Event::SearchFwFd);
                //     (s, ev) = self.alphabeta(
                //         &mut child_board,
                //         ply + 1,
                //         depth + ext - 1,
                //         -n.beta,
                //         -n.alpha,
                //         mv,
                //     )?;
                //     s = -s;
                // }
            } else {
                if lmr > 0 {
                    ev = Event::FutilityD0;
                    if let Some(est) = self.can_futility_prune_move(
                        mv,
                        count,
                        mt,
                        b,
                        &child_board,
                        eval,
                        &Node {
                            ply: ply + 1,
                            depth: depth + ext - lmr - 1,
                            alpha: n.alpha,
                            beta: n.alpha + Score::from_cp(1),
                        },
                        ext,
                    ) {
                        s = est;
                    } else {
                        Metrics::incr_node(&n, Event::SearchZwRd);
                        (s, ev) = self.alphabeta(
                            &mut child_board,
                            ply + 1,
                            depth + ext - lmr - 1,
                            -n.alpha + Score::from_cp(-1),
                            -n.alpha,
                            mv,
                        )?;
                        s = -s;
                    }
                } else {
                    Metrics::incr_node(&n, Event::SearchZwFd);
                    (s, ev) = self.alphabeta(
                        &mut child_board,
                        ply + 1,
                        depth + ext - 1,
                        -n.alpha + Score::from_cp(-1),
                        -n.alpha,
                        mv,
                    )?;
                    s = -s;
                }
                // // adds nothing!
                // // full width, red dep
                // if s > n.alpha && !(lmr == 0) {
                //     (s, ev) = self.alphabeta(
                //         &mut child_board,
                //         ply + 1,
                //         depth + ext - lmr - 1,
                //         -n.beta,
                //         -n.alpha,
                //         mv,
                //     )?;
                //     s = -s;
                // }

                // full depth, zw
                if s > n.alpha && !(lmr == 0) {
                    Metrics::incr_node(&n, Event::ReSearchZwFd);
                    (s, ev) = self.alphabeta(
                        &mut child_board,
                        ply + 1,
                        depth + ext - 1,
                        -n.alpha + Score::from_cp(-1),
                        -n.alpha,
                        mv,
                    )?;
                    s = -s;
                }

                // full window
                if s > n.alpha && !(lmr == 0 && n.is_zw()) {
                    Metrics::incr_node(&n, Event::ReSearchFwFd);
                    (s, ev) = self.alphabeta(
                        &mut child_board,
                        ply + 1,
                        depth + ext - 1,
                        -n.beta,
                        -n.alpha,
                        mv,
                    )?;
                    s = -s;
                }
            }
            //     // using [alpha, alpha + 1]
            //     debug_assert!(n.alpha.is_numeric());
            //     self.stats.inc_pvs_move(ply);
            //     (s, ev) = self.alphabeta(
            //         &mut child_board,
            //         ply + 1,
            //         depth + ext - lmr - 1,
            //         -n.alpha - Score::from_cp(1),
            //         -n.alpha,
            //         mv,
            //     )?;
            //     s = -s;
            // }

            let cat = ev;

            // b.undo_move(&mv);
            self.current_variation.pop();
            self.repetition.pop();
            self.explain_move(&b, mv, s, cat, &n, count, ext, lmr);

            if s > score {
                score = s;
                category = cat;
                bm = Some(mv);
            }
            if s > n.alpha {
                n.alpha = s;
                nt = NodeType::ExactPv;
                debug_assert!(
                    b.is_pseudo_legal_move(&bm.unwrap()),
                    "bm {} on board {}",
                    bm.unwrap(),
                    b
                );
                self.history.raised_alpha(&n, b, &mv);
                self.record_move(ply, &mv);
            } else {
                self.history.duff(&n, b, &mv);
            }

            if n.alpha >= n.beta {
                nt = NodeType::LowerCut;
                // self.stats.inc_node_cut(ply, move_type, (count - 1) as i32);
                self.killers.store(ply, &mv);
                // self.history.beta_cutoff(&n, b, &mv);
                self.history.beta_variation(&n, b, &self.current_variation, mv);
                self.counter_move.store(b.color_us(), last_move, mv, &n);
                self.report_refutation(n.ply);
                break;
            }
        }

        if count == 0 {
            if n.is_qs() {
                Metrics::incr_node(&n, Event::NodeLeafQuietEval);
                return Ok((
                    b.eval_with_outcome(&self.eval, &n),
                    Event::NodeLeafQuietEval,
                ));
            } else {
                Metrics::incr_node(&n, Event::NodeLeafStalemate);
                return Ok((
                    b.eval_with_outcome(&self.eval, &n),
                    Event::NodeLeafStalemate,
                ));
            }
        }
        if nt == NodeType::UpperAll {
            cat = Event::NodeInteriorAll;
        } else if nt == NodeType::LowerCut {
            cat = Event::NodeInteriorCut;
            // debug_assert!(bm.is_null());
        } else if nt == NodeType::ExactPv {
            cat = Event::NodeInteriorPv;
            debug_assert!(bm.is_some())
        } else {
            panic!("Node type {:?} ", nt);
        }
        Metrics::incr_node(&n, cat);
        // aspiration search fails dont get stored
        if score > -Score::INFINITY && score < Score::INFINITY {
            let entry = TtNode {
                score,
                depth,
                nt,
                bm: bm.unwrap_or_default(),
            };
            Metrics::incr_node(&n, Event::TtStoreNode);
            self.tt.store(b.hash(), entry);
        }
        debug_assert!(
            n.alpha != -Score::INFINITY || n.beta != Score::INFINITY || score.is_finite(),
            "alpha/beta full width with score: {} node {}\ncat: {} count: {} quiets: {}\nbm: {}\nnt: {}\nboard: {:#}",
            score,
            n,
            category,
            count,
            quiets,
            bm.unwrap_or_default(),
            nt,
            b,
        );
        self.explain_node(
            &b,
            bm.unwrap_or_default(),
            nt,
            score,
            eval,
            &n,
            cat,
            &self.pv_table.extract_pv_for(ply),
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
            let mut algo = Algo::new();
            algo.set_callback(crate::comms::uci::Uci::uci_info);
            // search.tt.enabled = false;
            algo.set_position(pos.clone()).search();
            algo.set_timing_method(TimeControl::Depth(11));
            // println!("{}", search);
            assert_eq!(
                algo.pv().to_string(),
                pos.pv()?.to_string(),
                "#{} Expected: {}\nSearch: {}",
                i,
                pos,
                algo.pv().to_string()
            );
            assert_eq!(algo.score().is_mate(), true);
            println!("#{i} gives mate in {}", algo.score().mate_in().unwrap());
            assert_eq!(algo.score().mate_in(), Some(2), "#{} {}", i, pos);
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
            println!("#{i} gives mate in {}", search.score().mate_in().unwrap());
            assert_eq!(search.score().mate_in(), Some(4), "#{} {}", i, pos);
        }
        Ok(())
    }
}
