use std::ops::Deref;

use tracing::{event, Level};

use crate::{
    cache::tt2::{EvalFromTt, TtNode, TtScore},
    search::{algo::Search, qs::RunQs},
};
use odonata_base::{
    boards::Position,
    domain::{
        node::{Counter, Node, SearchType, Timing},
        score::Score,
        staticeval::eval_with_outcome,
        BoundType,
    },
    infra::metric::Metrics,
    mv::Move,
    variation::Variation,
};

use odonata_base::domain::node::Event;

use super::trail::Trail;

pub struct AlphaBeta;

impl Search {
    pub fn alphabeta_root_search(
        &mut self,
        trail: &mut Trail,
        pos: &mut Position,
        n: &mut Node,
    ) -> Result<(Score, Event), Event> {
        self.clock_checks = 0;
        *trail = Trail::new(pos.board().clone());
        self.current_variation = Variation::new();
        self.max_depth = 0;
        trail.clear();
        trail.set_tree_crit(self.explainer.tree_crit.clone());
        debug_assert!(n.alpha < n.beta);

        let t = Metrics::timing_start();
        let depth = n.depth;
        self.max_depth = depth;
        // self.stats.depth = depth;
        debug_assert!(self.current_variation.is_empty());

        let (score, category) =
            self.alphabeta("begin", trail, &mut pos.clone(), *n, Move::new_null())?;

        if self.controller.is_cancelled() {
            Metrics::incr(Counter::SearchTimeUp);
        } else {
            Metrics::incr(Counter::SearchComplete);
        }

        debug_assert!(
            self.current_variation.is_empty()
                || category == Event::UserCancelled
                || category == Event::SearchTimeUp
        );
        let (_pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(pos.board())
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
        Ok((score, category))
    }

    #[inline]
    fn static_eval(&mut self, pos: &Position, n: &Node) -> Score {
        Metrics::incr_node(n, Event::InteriorEvalStatic);
        // let mut score = b.static_eval(&self.eval);

        if let Some(entry) = self.tt.probe_by_hash(pos.board().hash()) {
            match self.tt.eval_from_tt {
                EvalFromTt::Never => self.eval.static_eval(pos).pov_score(),
                EvalFromTt::UseTtEval => entry.eval,
                EvalFromTt::UseTtScore if entry.bt == BoundType::ExactPv => {
                    entry.eval
                    // entry.score.as_score(n.ply)
                }
                EvalFromTt::UseTtScore => entry.eval,
            }
        } else {
            self.eval.static_eval(pos).pov_score()
        }
    }
    // let lookup_score =
    // if entry.depth >= self.tt.tt_for_eval_depth {
    //      {
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
    //                 entry.score.as_score(n.ply),
    //     // }
    //     // eval = eval.clamp_score();
    // }

    pub fn alphabeta(
        &mut self,
        _kind: &str,
        trail: &mut Trail,
        pos: &mut Position,
        mut n: Node,
        last_move: Move,
    ) -> Result<(Score, Event), Event> {
        // score returned will be alpha <= score < beta
        debug_assert!(n.alpha < n.beta);
        debug_assert!(n.ply >= 0);
        self.report_progress();

        let (cancelled, mut cat) = self.time_up_or_cancelled(n.ply, false);
        if cancelled {
            return Err(cat);
        }

        Metrics::incr_node(&n, Event::NodeTotal);
        self.clock.inc_int_nodes();
        match () {
            _ if n.is_null_move() => Metrics::incr_node(&n, Event::NodeNullMove),
            _ if n.is_razor() => Metrics::incr_node(&n, Event::NodeRazor),
            _ if n.is_zw() => Metrics::incr_node(&n, Event::NodeZw),
            _ => {}
        };

        if let Some(s) = self.mate_distance(&mut n) {
            return Ok((s, Event::MateDistSuccess));
        }

        if n.is_qs() {
            let t = Metrics::timing_start();
            let mut qs = RunQs {
                controller: &self.controller,
                eval: self.eval.deref(),
                clock: &self.clock,
                tt: &self.tt,
                trail,
                config: &self.qs,
            };
            let s = qs.qsearch(&n, pos, Some(last_move)).unwrap_or_else(|e| e);
            Metrics::profile(t, Timing::TimingQs);
            return Ok((s, Event::NodeQsLeaf));
        }

        Metrics::incr_node(&n, Event::NodeInterior);

        let mut score = -Score::INFINITY;
        let mut category = Event::Unknown;
        let mut bm = None;
        let mut bt = BoundType::UpperAll;
        // we dont draw at root, as otherwise it wont play a move if handed an illegaly (already drawn) position
        if n.ply > 0
            && (self.repetition.is_draw_by_repetition(pos)
                || pos.board().material().is_insufficient())
        {
            // if n.ply > 0 && pos.board().draw_outcome().is_some() {
            Metrics::incr_node(&n, Event::NodeLeafDraw);
            if n.ply == 0 {
                panic!(
                    "{pos:#} \ndraw = {draw} {ins}",
                    draw = self.repetition.is_draw_by_repetition(pos),
                    ins = pos.board().material().is_insufficient()
                );
            }
            return Ok((
                self.eval.eval_draw(pos.board(), n.ply).pov_score(),
                Event::NodeLeafDraw,
            )); // will return a draw score
        }

        let mut tt_mv = Move::new_null();
        match self.lookup(trail, pos.board(), &mut n) {
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
        let eval = self.static_eval(pos, &n);

        if let Some(s) = self.reverse_fut(pos.board(), eval, &n, 0) {
            trail.prune_node(&n, s, Event::RevFutSuccess);
            return Ok((s, Event::RevFutSuccess));
        }
        if let Some(s) = self.razor_node(trail, last_move, pos, eval, &n)? {
            trail.prune_node(&n, s, Event::RazorSuccess);
            return Ok((s, Event::RazorSuccess));
        }
        if let Some(s) = self.nmp_node(trail, pos, &n, eval)? {
            trail.prune_node(&n, s, Event::NmpSuccess);
            return Ok((s, Event::NmpSuccess));
        }

        let mut sorted_moves =
            self.move_orderer
                .create_sorted_moves(n, pos.board(), tt_mv, last_move);
        if trail.path().len() < 2 {
            event!(target:"ab", Level::DEBUG, "{var} generating moves...", var = trail.path(), );
        }
        let mut count = 0;
        let mut quiets = 0;
        while let Some((mt, mv)) = sorted_moves.next_move(pos.board(), self) {
            if self.restrictions.skip_move(n.ply, mv) {
                continue;
            }
            Metrics::incr_node(&n, Event::MoveCount);
            Metrics::classify_move(&n, mv, mt, pos.board());
            pos.push_move(mv);
            let before = pos.prior_board().unwrap();
            // let mut child_board = b.make_move(mv);
            let ext = self.extend(before, pos.board(), mv, mt, count, &n, last_move);
            let is_quiet = self.is_quiet(before, mv, mt, pos.board(), &n, ext);
            if is_quiet {
                quiets += 1;
            }

            if bm.is_some() {
                if let Some(est) =
                    self.can_futility_prune_move(mv, count, mt, before, pos.board(), eval, &n, ext)
                {
                    // dont actually want to make move - but want to record it
                    if self.can_prune_remaining_moves(before, mt, &n) {
                        trail.prune_move(&n, est, mv, Event::FutilitySuccessRemaining);
                        pos.pop_move();
                        break;
                    } else {
                        trail.prune_move(&n, est, mv, Event::FutilitySuccessRemaining);
                        // dont actually want to make move - but want to record it
                        pos.pop_move();
                        continue;
                    }
                }
                if self.can_lmp_move(before, count, is_quiet, quiets, &n, mv) {
                    pos.pop_move();
                    continue;
                }
            }

            self.repetition.push_move(mv, before);
            self.current_variation.push(mv);
            trail.push_move(&n, mv);
            // self.explainer.start(&n, &self.current_variation);
            // pos.board
            //     .set_repetition_count(self.repetition.count_old(pos.board()));
            // pos.board
            //     .set_repetition_count(child_board.repetition_count());

            let mut s; //= n.alpha + Score::from_cp(1);
                       // debug_assert!(s > n.alpha);
            let mut ev; //  = Event::Unknown;

            count += 1;

            let lmr = self.lmr(
                before,
                mv,
                count,
                quiets,
                mt,
                pos.board(),
                &n,
                bt,
                ext,
                tt_mv,
            );

            // ONLY do fw if
            // we are on PV trail (=fw) otherwise we should narrow/reduce
            // and pvs not permitted perhaps due to depth
            ev = Event::FutilityD0;
            // let cb = &mut child_board;
            s = Score::INFINITY;
            // we check again if we can futility prune this move as the depth has been reduced
            #[allow(clippy::nonminimal_bool)]
            if !(n.is_fw() && !self.pvs_permitted(bt, before, &n, count)) && lmr > 0 {
                if let Some(est) = self.can_futility_prune_move(
                    mv,
                    count,
                    mt,
                    before,
                    pos.board(),
                    eval,
                    &Node {
                        zw:    SearchType::ZeroWindow,
                        ply:   n.ply + 1,
                        depth: n.depth + ext - lmr - 1,
                        alpha: n.alpha,
                        beta:  n.alpha + Score::from_cp(1),
                    },
                    ext,
                ) {
                    s = est;
                }
            }

            if s == Score::INFINITY {
                match (lmr > 0, n.is_fw_equal_zw()) {
                    (..) if n.is_fw() && !self.pvs_permitted(bt, before, &n, count) => {
                        Metrics::incr_node(&n, Event::SearchFwFd);
                        (s, ev) = self.alphabeta("!pvs", trail, pos, n.new_child().ext(ext), mv)?;
                        s = -s;
                    }
                    (true, false) => {
                        Metrics::incr_node(&n, Event::SearchZwRd);
                        (s, ev) = self.alphabeta(
                            "zwrd",
                            trail,
                            pos,
                            n.new_child().ext(ext - lmr).zw(),
                            mv,
                        )?;
                        s = -s;
                        if s > n.alpha {
                            Metrics::incr_node(&n, Event::ReSearchZwFd);
                            (s, ev) = self.alphabeta(
                                "zwfd",
                                trail,
                                pos,
                                n.new_child().ext(ext).zw(),
                                mv,
                            )?;
                            s = -s;
                        }
                        if s > n.alpha {
                            Metrics::incr_node(&n, Event::ReSearchFwFd);
                            (s, ev) =
                                self.alphabeta("fwfd", trail, pos, n.new_child().ext(ext), mv)?;
                            s = -s;
                        }
                    }
                    (true, true) => {
                        Metrics::incr_node(&n, Event::SearchZwRd);
                        (s, ev) =
                            self.alphabeta("fwrd", trail, pos, n.new_child().ext(ext - lmr), mv)?;
                        s = -s;
                        if s > n.alpha {
                            Metrics::incr_node(&n, Event::ReSearchFwFd);
                            (s, ev) =
                                self.alphabeta("fwfd", trail, pos, n.new_child().ext(ext), mv)?;
                            s = -s;
                        }
                    }
                    (false, false) => {
                        // (s, ev) =
                        //     self.alphabeta("zwrd", trail, pos, n.new_child().ext(ext - lmr).zw(), mv)?;
                        // s = -s;
                        Metrics::incr_node(&n, Event::SearchZwFd);
                        (s, ev) =
                            self.alphabeta("zwfd", trail, pos, n.new_child().ext(ext).zw(), mv)?;
                        s = -s;
                        if s > n.alpha {
                            Metrics::incr_node(&n, Event::ReSearchFwFd);
                            (s, ev) =
                                self.alphabeta("fwfd", trail, pos, n.new_child().ext(ext), mv)?;
                            s = -s;
                        }
                    }
                    (false, true) => {
                        Metrics::incr_node(&n, Event::SearchZwFd);
                        (s, ev) = self.alphabeta("fwfd", trail, pos, n.new_child().ext(ext), mv)?;
                        s = -s;
                    }
                };
            }

            let cat = ev;

            // b.undo_move(&mv);
            self.current_variation.pop();
            trail.pop_move(&n, mv);
            self.repetition.pop();
            pos.pop_move();
            self.explain_move(pos.board(), mv, s, cat, &n, count, ext, lmr);

            if s > score {
                score = s;
                category = cat;
                bm = Some(mv);
            }
            // if metrics && futility prune with enabled then print.
            if s >= n.beta {
                trail.fail(&n, s, mv, Event::MoveScoreHigh);
                bt = BoundType::LowerCut;
                // self.stats.inc_node_cut(ply, move_type, (count - 1) as i32);
                self.killers.store(&n, &mv, pos.board());
                // self.history.beta_cutoff(&n, b, &mv);
                self.history
                    .beta_variation(&n, pos.board(), &self.current_variation, mv, mt);
                self.counter_move
                    .store(pos.board().color_us(), last_move, mv, &n, pos.board());
                self.report_refutation(n.ply);
                break;
            }
            if s > n.alpha {
                trail.alpha_raised(&n, s, mv, Event::AlphaRaised);
                n.alpha = s;
                bt = BoundType::ExactPv;
                debug_assert!(
                    bm.unwrap().to_inner().validate(pos.board()).is_ok(),
                    "bm {} on board {}",
                    bm.unwrap(),
                    pos.board()
                );
                self.history.raised_alpha(&n, pos.board(), mv, mt);
            } else {
                trail.ignore_move(&n, s, mv, Event::MoveScoreLow);
                self.history.duff(&n, pos.board(), mv, mt);
            }
        }
        // if trail.path().len() < 2 {
        //     debug!(target:"ab", "Node: {n} finished moves... with score {score}");
        // }
        if count == 0 {
            if n.is_qs() {
                Metrics::incr_node(&n, Event::NodeLeafQuietEval);
                let sc = eval_with_outcome(self.eval.deref(), pos, &n);
                trail.terminal(&n, sc, Event::NodeLeafQuietEval);
                return Ok((sc, Event::NodeLeafQuietEval));
            } else {
                // no moves so either w/d/l
                Metrics::incr_node(&n, Event::NodeLeafStalemate);
                let sc = eval_with_outcome(self.eval.deref(), pos, &n);
                trail.terminal(&n, sc, Event::NodeLeafStalemate);
                return Ok((sc, Event::NodeLeafStalemate));
            }
        }
        if bt == BoundType::UpperAll {
            cat = Event::NodeInteriorAll;
        } else if bt == BoundType::LowerCut {
            cat = Event::NodeInteriorCut;
            // debug_assert!(bm.is_null());
        } else if bt == BoundType::ExactPv {
            cat = Event::NodeInteriorPv;
            debug_assert!(bm.is_some())
        } else {
            panic!("Node type {:?} ", bt);
        }
        Metrics::incr_node(&n, cat);
        // aspiration search fails dont get stored
        if score > -Score::INFINITY && score < Score::INFINITY {
            let entry = TtNode {
                score: TtScore::new(score, n.ply),
                depth: n.depth,
                bt,
                bm: bm.unwrap_or_default().to_inner(),
                eval,
            };
            Metrics::incr_node(&n, Event::TtStoreNode);
            self.tt.store(pos.board().hash(), entry);
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
            bt,
            pos.board(),
        );
        Ok((score, category))
    }
}

#[cfg(test)]
mod tests {
    use crate::{engine::Engine, search::engine::ThreadedSearch};

    use anyhow::Result;
    use odonata_base::{catalog::*, domain::timecontrol::*};

    #[test]
    fn test_2_mates() -> Result<()> {
        let positions = Catalog::mate_in_2();
        for (i, epd) in positions.iter().enumerate() {
            let mut eng = ThreadedSearch::new();
            eng.set_callback(crate::comms::uci_server::UciServer::uci_info);
            // search.tt.enabled = false;
            let res = eng.search(epd.clone(), TimeControl::Depth(7)).unwrap();
            // println!("{}", search);
            assert_eq!(
                res.pv().to_string(),
                epd.var("pv").unwrap().to_string(),
                "#{i} Expected: {epd}\nSearch: {}",
                res.pv()
            );
            assert_eq!(eng.search.score().is_mate(), true);
            println!(
                "#{i} gives mate in {}",
                eng.search.score().mate_in().unwrap()
            );
            assert_eq!(eng.search.score().mate_in(), Some(2), "#{} {}", i, epd);
        }
        Ok(())
    }

    #[test]
    fn test_mate_in_3() -> Result<()> {
        let positions = Catalog::mate_in_3();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = ThreadedSearch::new();
            let expected_pv = pos.var("pv").unwrap();
            let res = search.search(pos.clone(), TimeControl::Depth(5)).unwrap();
            println!("{search}");
            assert_eq!(res.score().unwrap().mate_in(), Some(3), "#{i} {pos}");
            assert_eq!(res.pv(), expected_pv, "#{i} {pos}");
        }
        Ok(())
    }

    #[test]
    fn test_mate_in_4() -> Result<()> {
        let mut search = ThreadedSearch::new();
        for epd in Catalog::mate_in_4().iter() {
            search.start_game()?;
            let res = search.search(epd.clone(), TimeControl::Depth(10)).unwrap();
            if let Some(expected_pv) = epd.var("pv") {
                // some checkmates have multiple solutions so no PV given - only depth
                assert_eq!(res.pv(), expected_pv, "{epd}");
            }
            assert_eq!(res.score().unwrap().mate_in(), Some(4), "{res}\n{epd}");
        }
        Ok(())
    }
}
