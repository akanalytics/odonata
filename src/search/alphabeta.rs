use tracing::{event, instrument, Level};

use crate::{
    boards::Board,
    cache::tt2::{TtNode, TtScore},
    domain::{NodeType, Trail},
    eval::score::Score,
    infra::metric::Metrics,
    mv::Move,
    search::{
        algo::Algo,
        node::{Counter, Node, Timing},
        qs::RunQs,
    },
    variation::Variation,
};

use super::node::Event;

pub struct AlphaBeta;

impl Algo {
    pub fn alphabeta_root_search(
        &mut self,
        trail: &mut Trail,
        board: &mut Board,
        n: &mut Node,
    ) -> (Score, Event) {
        self.clock_checks = 0;
        *trail = Trail::new(board.clone());
        self.current_variation = Variation::new();
        self.max_depth = 0;
        trail.clear();
        trail.set_tree_crit(self.explainer.tree_crit.clone());
        debug_assert!(n.alpha < n.beta);

        let t = Metrics::timing_start();
        let depth = n.depth;
        self.max_depth = depth;
        // self.stats.depth = depth;
        debug_assert!(self.current_variation.len() == 0);

        let (score, category) = match self.alphabeta("begin", trail, board, *n, Move::new_null()) {
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
        let (_pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (trail.root_pv().clone(), Some(Score::default()))
        };
        // assert!(trail.pv().starts_with(&pv), "Board {board:L>} {pv} (table) != {tpv} (trail) \n\n{trail:#}", tpv= trail.pv());
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
    fn static_eval(&mut self, b: &Board, n: &Node) -> Score {
        Metrics::incr_node(n, Event::InteriorEvalStatic);
        // let mut score = b.static_eval(&self.eval);

        if let Some(entry) = self.tt.probe_by_hash(b.hash()) {
            if self.tt.use_tt_for_eval {
                let lookup_score = entry.eval;
                if lookup_score != b.static_eval(&self.eval) {
                    println!(
                        "\n\n\n\n{lookup_score} != {s} for board {b} node {n}\n\n\n\n\n",
                        s = b.static_eval(&self.eval)
                    )
                }

                return lookup_score;
            } else {
                return b.static_eval(&self.eval);
            }
            // // let lookup_score = entry.score.as_score(n.ply);
            // if entry.depth >= self.tt.tt_for_eval_depth {
            //     if entry.nt == NodeType::ExactPv {
            //         Metrics::incr_node(n, Event::TtHitEvalNode);
            //         score = lookup_score;
            //     } else if entry.nt == NodeType::LowerCut && lookup_score > score {
            //         Metrics::incr_node(n, Event::TtHitEvalNode);
            //         score = lookup_score;
            //     } else if entry.nt == NodeType::UpperAll && lookup_score < score {
            //         Metrics::incr_node(n, Event::TtHitEvalNode);
            //         score = lookup_score;
            //     } else {
            //         Metrics::incr_node(n, Event::TtMissEvalNode);
            //     }
            //     // }
            //     // eval = eval.clamp_score();
            // }
        }
        b.static_eval(&self.eval)
    }

    #[instrument(target="tree",level="debug", "", skip_all, fields(k=kind,n=%n,t=?trail))]
    pub fn alphabeta(
        &mut self,
        kind: &str,
        trail: &mut Trail,
        b: &mut Board,
        mut n: Node,
        last_move: Move,
    ) -> Result<(Score, Event), Event> {
        debug_assert!(n.alpha < n.beta);
        debug_assert!(n.ply >= 0);

        self.report_progress();

        // if n.alpha.is_numeric() && n.beta.is_numeric() && n.zw != n.alpha.is_numeric() && n.beta.is_numeric() && 
        // n.alpha.as_i16() + 1 == n.beta.as_i16() {
        //     println!("Node {n}")
        // }


        debug_assert!(n.alpha < n.beta, "alpha={}, beta={}", n.alpha, n.beta,);

        let (cancelled, mut cat) = self.time_up_or_cancelled(n.ply, false);
        if cancelled {
            return Err(cat);
        }

        Metrics::incr_node(&n, Event::NodeTotal);
        self.clock.inc_int_nodes();
        if n.is_zw() {
            Metrics::incr_node(&n, Event::NodeZw);
        }
        // self.results.set_seldepth(&n);

        if let Some(s) = self.mate_distance(&mut n) {
            return Ok((s, Event::MateDistSuccess));
        }

        if n.is_qs() {
            let t = Metrics::timing_start();
            let mut qs = RunQs {
                controller: &self.controller,
                eval: &self.eval,
                clock: &self.clock,
                tt: &self.tt,
                trail,
                config: &self.qs,
            };
            let s = qs.qsearch(&n, b, Some(last_move)).unwrap_or_else(|e| e);
            Metrics::profile(t, Timing::TimingQs);
            return Ok((s, Event::NodeQsLeaf));
        }

        Metrics::incr_node(&n, Event::NodeInterior);

        let mut score = -Score::INFINITY;
        let mut category = Event::Unknown;
        let mut bm = None;
        let mut nt = NodeType::UpperAll;

        // we dont draw at root, as otherwise it wont play a move if insufficient-material draw [v32]
        if n.ply > 0 && b.draw_outcome().is_some() {
            Metrics::incr_node(&n, Event::NodeLeafDraw);
            return Ok((b.eval_draw(&mut self.eval, &n), Event::NodeLeafDraw)); // will return a draw score
        }

        let mut tt_mv = Move::new_null();
        match self.lookup(trail, b, &mut n) {
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

        if let Some(s) = self.reverse_fut(b, eval, &n, 0) {
            trail.prune_node(&n, s, Event::RevFutSuccess);
            return Ok((s, Event::RevFutSuccess));
        }
        if let Some(s) = self.razor_node(trail, last_move, b, eval, &n)? {
            trail.prune_node(&n, s, Event::RazorSuccess);
            return Ok((s, Event::RazorSuccess));
        }
        if let Some(s) = self.nmp_node(trail, b, &n, eval)? {
            trail.prune_node(&n, s, Event::NmpSuccess);
            return Ok((s, Event::NmpSuccess));
        }

        let mut sorted_moves = self
            .move_orderer
            .create_sorted_moves(n, b, tt_mv, last_move);
        if trail.path().len() < 2 {
            event!(target:"ab", Level::DEBUG, "{var} generating moves...", var = trail.path(), );
        }
        let mut count = 0;
        let mut quiets = 0;
        while let Some((mt, mv)) = sorted_moves.next_move(b, self) {
            if trail.path().len() < 2 {
                event!(target:"ab",Level::DEBUG, "Trying move {mv} of kind {mt}");
            }
            if self.restrictions.skip_move(n.ply, &mv) {
                continue;
            }
            Metrics::incr_node(&n, Event::MoveCount);
            Metrics::classify_move(&n, mv, mt, b);
            let mut child_board = b.make_move(mv);
            let ext = self.extend(b, &child_board, mv, mt, count, &n, last_move);
            let is_quiet = self.is_quiet(b, mv, mt, &child_board, &n, ext);
            if is_quiet {
                quiets += 1;
            }

            if bm.is_some() {
                if let Some(est) =
                    self.can_futility_prune_move(mv, count, mt, b, &child_board, eval, &n, ext)
                {
                    // dont actually want to make move - but want to record it
                    if self.can_prune_remaining_moves(b, mt, &n) {
                        trail.prune_move(&n, est, mv, Event::FutilitySuccessRemaining);
                        break;
                    } else {
                        trail.prune_move(&n, est, mv, Event::FutilitySuccessRemaining);
                        // dont actually want to make move - but want to record it
                        continue;
                    }
                }
                if self.can_lmp_move(b, count, is_quiet, quiets, &n, mv) {
                    continue;
                }
            }

            self.repetition.push_move(mv, &b);
            self.current_variation.push(mv);
            trail.push_move(&n, mv);
            // self.explainer.start(&n, &self.current_variation);
            child_board.set_repetition_count(self.repetition.count(&child_board));

            let mut s; //= n.alpha + Score::from_cp(1);
                       // debug_assert!(s > n.alpha);
            let mut ev; //  = Event::Unknown;

            count += 1;
            let lmr = self.lmr(b, mv, count, quiets, mt, &child_board, &n, nt, ext, tt_mv);
            if n.is_fw() && !self.pvs_permitted(nt, b, &n, count) {
                Metrics::incr_node(&n, Event::SearchFwFd);
                (s, ev) =
                    self.alphabeta("!pvs", trail, &mut child_board, n.new_child().ext(ext), mv)?;
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
                            zw:    true,
                            ply:   n.ply + 1,
                            depth: n.depth + ext - lmr - 1,
                            alpha: n.alpha,
                            beta:  n.alpha + Score::from_cp(1),
                        },
                        ext,
                    ) {
                        s = est;
                    } else {
                        Metrics::incr_node(&n, Event::SearchZwRd);
                        (s, ev) = self.alphabeta(
                            "zwrd",
                            trail,
                            &mut child_board,
                            n.new_child().ext(ext - lmr).zw(),
                            mv,
                        )?;
                        s = -s;
                    }
                } else {
                    Metrics::incr_node(&n, Event::SearchZwFd);
                    (s, ev) = self.alphabeta(
                        "zwfd",
                        trail,
                        &mut child_board,
                        n.new_child().ext(ext).zw(),
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
                        "zwfd",
                        trail,
                        &mut child_board,
                        n.new_child().ext(ext).zw(),
                        mv,
                    )?;
                    s = -s;
                }

                // research at full window if fw != zw
                if s > n.alpha && !(lmr == 0 && n.is_fw_equal_zw() ) {
                    Metrics::incr_node(&n, Event::ReSearchFwFd);
                    (s, ev) = self.alphabeta(
                        "fwfd",
                        trail,
                        &mut child_board,
                        n.new_child().ext(ext),
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
            trail.pop_move(&n, mv);
            self.repetition.pop();
            self.explain_move(&b, mv, s, cat, &n, count, ext, lmr);

            if s > score {
                score = s;
                category = cat;
                bm = Some(mv);
            }
            if s >= n.beta {
                trail.fail(&n, s, mv, Event::MoveScoreHigh);
                nt = NodeType::LowerCut;
                // self.stats.inc_node_cut(ply, move_type, (count - 1) as i32);
                self.killers.store(&n, &mv, b);
                // self.history.beta_cutoff(&n, b, &mv);
                self.history
                    .beta_variation(&n, b, &self.current_variation, mv, mt);
                self.counter_move.store(b.color_us(), last_move, mv, &n, b);
                self.report_refutation(n.ply);
                break;
            }
            if s > n.alpha {
                trail.alpha_raised(&n, s, mv, Event::AlphaRaised);
                n.alpha = s;
                nt = NodeType::ExactPv;
                debug_assert!(
                    bm.unwrap().to_inner().validate(b).is_ok(),
                    "bm {} on board {}",
                    bm.unwrap(),
                    b
                );
                self.history.raised_alpha(&n, b, mv, mt);
            } else {
                trail.ignore_move(&n, s, mv, Event::MoveScoreLow);
                self.history.duff(&n, b, mv, mt);
            }
        }
        // if trail.path().len() < 2 {
        //     debug!(target:"ab", "Node: {n} finished moves... with score {score}");
        // }
        if count == 0 {
            if n.is_qs() {
                Metrics::incr_node(&n, Event::NodeLeafQuietEval);
                let sc = b.eval_with_outcome(&self.eval, &n);
                trail.terminal(&n, sc, Event::NodeLeafQuietEval);
                return Ok((sc, Event::NodeLeafQuietEval));
            } else {
                Metrics::incr_node(&n, Event::NodeLeafStalemate);
                let sc = b.eval_with_outcome(&self.eval, &n);
                trail.terminal(&n, sc, Event::NodeLeafStalemate);
                return Ok((sc, Event::NodeLeafStalemate));
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
                score: TtScore::new(score, n.ply),
                depth: n.depth,
                nt,
                bm: bm.unwrap_or_default().to_inner(),
                eval,
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
        // self.explain_node(
        //     &b,
        //     bm.unwrap_or_default(),
        //     nt,
        //     score,
        //     eval,
        //     &n,
        //     cat,
        //     &self.pv_table.extract_pv_for(ply),
        // );
        Ok((score, category))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog::*, domain::engine::Engine, search::timecontrol::*};
    use anyhow::Result;

    #[test]
    fn test_2_mates() -> Result<()> {
        let positions = Catalog::mate_in_2();
        for (i, pos) in positions.iter().enumerate() {
            let mut algo = Algo::new();
            algo.set_callback(crate::comms::uci_server::UciServer::uci_info);
            // search.tt.enabled = false;
            algo.set_timing_method(TimeControl::Depth(7));
            algo.set_position(pos.clone()).run_search();
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
    fn test_mate_in_3() -> Result<()> {
        let positions = Catalog::mate_in_3();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            let expected_pv = pos.pv()?;
            let res = search.search(pos.clone(), TimeControl::Depth(5)).unwrap();
            println!("{}", search);

            assert_eq!(res.score().unwrap().mate_in(), Some(3), "#{} {}", i, pos);
            assert_eq!(res.pv(), expected_pv, "#{} {}", i, pos);
        }
        Ok(())
    }

    #[test]
    fn test_mate_in_4() -> Result<()> {
        let positions = Catalog::mate_in_4();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            let res = search.search(pos.clone(), TimeControl::Depth(9)).unwrap();
            // println!("{}", search);
            if pos.try_get("pv").is_ok() {
                let expected_pv = pos.pv()?;
                assert_eq!(
                    res.pv().to_string(),
                    expected_pv.to_string(),
                    "#{} {}",
                    i,
                    pos
                );
            }
            assert_eq!(
                res.score().unwrap().mate_in(),
                Some(4),
                "res={res}\npos={pos}"
            );
        }
        Ok(())
    }
}
