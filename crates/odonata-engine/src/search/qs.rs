use std::fmt;

use odonata_base::boards::Position;
use odonata_base::clock::Clock;
use odonata_base::domain::node::{Event, Node, Timing};
use odonata_base::domain::score::ToScore;
use odonata_base::domain::staticeval::StaticEval;
use odonata_base::domain::BoundType;
use odonata_base::eg::EndGame;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::Metrics;
use odonata_base::prelude::*;
use odonata_base::Epd;
use serde::{Deserialize, Serialize};

use super::controller::Controller;
use super::trail::Trail;
use crate::cache::tt2::{EvalFromTt, TranspositionTable2};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Qs {
    pub enabled:                      bool,
    pub only_on_capture:              bool,
    pub promos:                       bool,
    pub promo_piece:                  Option<Piece>,
    pub checks_max_ply:               Ply,
    pub delta_prune_discovered_check: bool,
    pub delta_prune_gives_check:      bool,
    pub delta_prune_near_promos:      bool,
    pub delta_prune_min_pieces:       i32,
    pub see_prune_discovered_check:   bool,
    pub see_prune_gives_check:        bool,
    pub see_prune_near_promos:        bool,
    pub even_exchange_max_ply:        Ply,
    pub max_ply:                      u16,
    pub delta_prune:                  bool,
    pub delta_prune_move_margin:      Score,
    pub delta_prune_move_pawn_margin: Score,
    pub delta_prune_node_margin:      Score,
    pub recapture_score:              i32,
    pub probe_tt:                     bool,
    pub use_hash_move:                bool,
}

impl Default for Qs {
    fn default() -> Self {
        Self {
            checks_max_ply:               -1,
            delta_prune:                  true,
            delta_prune_discovered_check: true,
            delta_prune_gives_check:      false,
            delta_prune_min_pieces:       0,
            delta_prune_move_margin:      Score::from_cp(50),
            delta_prune_move_pawn_margin: Score::from_cp(100),
            delta_prune_near_promos:      true,
            delta_prune_node_margin:      Score::from_cp(0),
            enabled:                      true,
            even_exchange_max_ply:        2,
            max_ply:                      16,
            only_on_capture:              false,
            probe_tt:                     true,
            promo_piece:                  Some(Piece::Queen),
            promos:                       true,
            recapture_score:              0,
            see_prune_discovered_check:   false,
            see_prune_gives_check:        true,
            see_prune_near_promos:        true,
            use_hash_move:                true,
        }
    }
}

impl Configurable for Qs {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.checks_max_ply.set(p.get("checks_max_ply"))?;
        self.delta_prune.set(p.get("delta_prune"))?;
        self.delta_prune_discovered_check
            .set(p.get("delta_prune_discovered_check"))?;
        self.delta_prune_gives_check.set(p.get("delta_prune_gives_check"))?;
        self.delta_prune_min_pieces.set(p.get("delta_prune_min_pieces"))?;
        self.delta_prune_move_margin.set(p.get("delta_prune_move_margin"))?;
        self.delta_prune_move_pawn_margin
            .set(p.get("delta_prune_move_pawn_margin"))?;
        self.delta_prune_near_promos.set(p.get("delta_prune_near_promos"))?;
        self.delta_prune_node_margin.set(p.get("delta_prune_node_margin"))?;
        self.enabled.set(p.get("enabled"))?;
        self.even_exchange_max_ply.set(p.get("even_exchange_max_ply"))?;
        self.max_ply.set(p.get("max_ply"))?;
        self.only_on_capture.set(p.get("only_on_capture"))?;
        self.probe_tt.set(p.get("probe_tt"))?;
        // self.promo_piece.set(p.get("promo_piece"))?;  // BUG
        self.promos.set(p.get("promos"))?;
        self.recapture_score.set(p.get("recapture_score"))?;
        self.see_prune_discovered_check
            .set(p.get("see_prune_discovered_check"))?;
        self.see_prune_gives_check.set(p.get("see_prune_gives_check"))?;
        self.see_prune_near_promos.set(p.get("see_prune_near_promos"))?;
        self.use_hash_move.set(p.get("use_hash_move"))?;
        Ok(p.is_modified())
    }
}

impl Component for Qs {
    fn new_game(&mut self) {}
    fn new_position(&mut self) {}
}

impl fmt::Display for Qs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

pub struct RunQs<'a> {
    pub controller: &'a Controller,
    pub eval:       &'a dyn StaticEval,
    pub clock:      &'a Clock,
    pub tt:         &'a TranspositionTable2,
    pub config:     &'a Qs,
    pub trail:      &'a mut Trail,
}

impl RunQs<'_> {
    // if the move resulted in checkmate, we should return a mate score
    // if the move results in a position which after quiesce, is potentially a mate,
    // we should not return a mate score, as only captures have been considered,
    // and a mate score might cut a genuine mate score elsewhere
    // since we only consider captures, repeats aren't an issue

    pub fn qsearch(&mut self, n: &Node, pos: &mut Position, lm: Option<Move>) -> Result<Score, Score> {
        debug_assert_eq!(n.depth, 0);
        Metrics::incr_node(n, Event::NodeQsLeaf);
        if n.is_zw() {
            Metrics::incr_node(n, Event::NodeQsLeafZw);
        }

        let q_nodes = self.clock.q_nodes();
        let res = self.qs(*n, pos, lm);
        if cfg!(debug_assertions) && false {
            let mut pos = Epd::from_board(pos.board().clone());
            pos.set_tag("pv", &self.trail.pv(n).to_uci());
            pos.set_tag("sv", &self.trail.path().to_uci());
            pos.set_tag("acn", &(self.clock.q_nodes() - q_nodes).to_string());

            match res {
                Ok(score) => {
                    pos.set_tag("c0", "Pv");
                    pos.set_tag("ce", &score.as_i16().to_string());
                }
                Err(score) => {
                    pos.set_tag("c0", "Cut");
                    pos.set_tag("ce", &score.as_i16().to_string());
                }
            };
            // if self.trail.pv(n).len() > self.trail.path().len() {
            //     pos.set(Tag::Comment(0, "******".into()));
            // }
            self.trail.record(pos);
        }
        res
    }

    pub fn qs(&mut self, mut n: Node, pos: &mut Position, lm: Option<Move>) -> Result<Score, Score> {
        debug_assert!(n.alpha < n.beta && n.ply >= 0 && n.depth <= 0, "{n}");
        // if n.beta.is_mate() && n.beta < 0.cp()  {
        //     let s = n.beta; // self.eval.static_evaldb).pov_score()eval);
        //     trail.prune_node(&n, s, Event::QsEvalStatic);
        //     return s;
        //     // || n.beta.is_mate() && n.beta > 0.cp()
        // }
        let orig_alpha = n.alpha;

        Metrics::incr_node(&n, Event::NodeQs);
        self.clock.inc_q_nodes();
        if EndGame::is_insufficient_material(pos.board()) {
            self.trail.terminal(&n, Score::DRAW, Event::QsCatInsufficientMaterial);
            return Err(Score::DRAW);
        }

        Metrics::incr_node(&n, Event::QsEvalStatic);
        let t = Metrics::timing_start();
        // let mut pat = self.eval.static_evaldb).pov_score()eval);
        // if self.tt.use_tt_for_eval {
        //     if let Some(entry) = self.tt.probe_by_hash(b.hash()) {

        Metrics::profile(t, Timing::TimingQsEval);

        let in_check = pos.board().is_in_check(pos.board().color_us());

        let mut pat = Score::zero();
        let hm = self.probe_tt(&mut n, pos, &mut pat)?;

        if !self.config.enabled {
            self.trail.terminal(&n, pat, Event::QsEvalStatic);
            return Err(pat);
        }

        if !in_check {
            if pat >= n.beta {
                Metrics::incr_node(&n, Event::QsCatCutStandingPat);
                self.trail.prune_node(&n, pat, Event::QsCatCutStandingPat);
                return Err(pat.clamp_score());
            }
            // TODO: zugawang check
            // ?? you cant stand pat unless theres already a move/pv (alpha=finite)
            if pat > n.alpha {
                // && n.alpha.is_finite() && n.ply >= 1 {
                self.trail.terminal(&n, pat, Event::QsStandingPatAlphaRaised);
                n.alpha = pat;
            }
            // coarse delta prune - where margin bigger than any possible move
            // b.most_valuable_piece_except_king(b.them());
            let p = pos
                .board()
                .most_valuable_piece_except_king(pos.board().occupied())
                .map(|(p, _sq)| p)
                .unwrap_or(Piece::Pawn);
            // if p != Piece::Queen {
            //     p = Piece::Rook;
            // }
            // calculate value of best enemy + pawn + static_margin (=0) => most_gain = most you can gain.
            // if min pieces && pat + most_gain < alpha   -> prune
            let pawn = self.eval.piece_material_eval(Piece::Pawn, pos.board());
            let mvp = self.eval.piece_material_eval(p, pos.board()) + pawn;
            // let ph = bd.phase(&self.eval.phaser);
            // let pawn = self.eval.mb.piece_weights[Piece::Pawn].interpolate(ph);
            // let mvp = self.eval.mb.piece_weights[p].interpolate(ph) + pawn;

            // .. but if pawn on rank2/7 then promo possible to widen margin by (Q-P)
            let mut margin = self.config.delta_prune_node_margin + Score::from_f64(mvp);
            if (pos.board().pawns() & pos.board().white() & Bitboard::RANK_7
                | pos.board().pawns() & pos.board().black() & Bitboard::RANK_2)
                .any()
            {
                let queen = self.eval.piece_material_eval(Piece::Queen, pos.board());
                // let queen = self.eval.mb.piece_weights[Piece::Queen].interpolate(ph);
                margin = margin + Score::from_f64(queen - pawn);
            }
            if pos.board().occupied().popcount() >= self.config.delta_prune_min_pieces
                && pat.is_numeric()
                && pat + margin <= n.alpha
            {
                Metrics::incr_node(&n, Event::QsCatAllDeltaPruneNode);
                self.trail.prune_node(&n, pat + margin, Event::QsCatAllDeltaPruneNode);
                return Err((pat + margin).clamp_score());
            }
        } else {
            Metrics::incr_node(&n, Event::NodeQsInCheck);
        }

        Metrics::incr_node(&n, Event::QsMoveGen);
        let mut moves = MoveList::new();
        self.gen_sorted_moves(in_check, &n, pos.board(), lm, hm, &mut moves);

        // if in_check && moves.is_empty() {
        //     return Ok(Score::from_mate_in_moves(0).clamp_score());
        // }

        let mut unpruned_move_count = 0;
        let mut bs = None; // Some(pat);
        for &mv in moves.iter() {
            Metrics::incr_node(&n, Event::QsMoveCount);
            if !in_check && self.can_see_prune_move(mv, &n, pat, pos.board()) {
                continue;
            }
            if !in_check && self.can_delta_prune_move(mv, &n, pat, pos.board()) {
                continue;
            }

            unpruned_move_count += 1;
            self.child_move(&mut n, pos, mv, &mut bs, unpruned_move_count)?;
        }

        if bs <= Some(orig_alpha) {
            self.trail.terminal(&n, n.alpha, Event::QsCatAll);
            Metrics::incr_node(&n, Event::QsCatAll);
            Metrics::add_node(&n, Event::QsCountMovesAtAllNode, unpruned_move_count);
            if orig_alpha.is_numeric() && bs < Some(orig_alpha - 200.cp()) {
                Metrics::incr_node(&n, Event::QsCatAllCp200);
            }
        } else {
            Metrics::incr_node(&n, Event::QsCatNodePv);
            Metrics::add_node(&n, Event::QsCountMovesAtPvNode, unpruned_move_count);
        }
        trace!("leaving qs: orig alpha {orig_alpha} and new {}", n.alpha);
        Ok(bs.unwrap_or(n.alpha).clamp_score())
    }

    // sort moves
    //
    fn gen_sorted_moves(&self, in_check: bool, n: &Node, bd: &Board, lm: Option<Move>, hm: Move, moves: &mut MoveList) {
        let t = Metrics::timing_start();
        match (in_check, self.config.promos, self.config.promo_piece) {
            (true, ..) => bd.legal_moves_with(|mv| moves.push(mv)),
            _ if -n.depth <= self.config.checks_max_ply => bd.legal_moves_with(|mv| {
                if mv.is_capture() || (mv.is_promo() && self.config.promos) || bd.gives_check(mv) {
                    moves.push(mv);
                }
            }),
            (_, false, _) => bd.legal_moves_to_with(bd.them(), |mv| moves.push(mv)),
            (_, true, None) => bd.legal_moves_to_with(bd.them() | Bitboard::RANKS_18, |mv| {
                if mv.is_capture() || mv.is_promo() {
                    moves.push(mv)
                }
            }),
            (_, true, Some(_)) => bd.legal_moves_to_with(bd.them() | Bitboard::RANKS_18, |mv| {
                if mv.is_capture() || mv.promo_piece() == self.config.promo_piece {
                    moves.push(mv)
                }
            }),
        };

        moves.sort_by_cached_key(|m| {
            Move::mvv_lva_score(m, bd)
                + if let Some(lm) = lm {
                    if m.to() == lm.to() {
                        self.config.recapture_score
                    } else {
                        0
                    }
                } else {
                    0
                }
                + if hm == *m { 10000 } else { 0 }
        });
        moves.reverse();
        Metrics::profile(t, Timing::TimingQsMoveSort);
    }

    // delta prune caters for non-captures during evasions
    //
    fn can_delta_prune_move(&self, mv: Move, n: &Node, pat: Score, bd: &Board) -> bool {
        let margin = if mv.from().is_in(bd.pawns()) {
            self.config.delta_prune_move_pawn_margin
        } else {
            self.config.delta_prune_move_margin
        };
        if self.config.delta_prune
            && pat.is_numeric()
            && !mv.is_capture()
            && (self.config.delta_prune_discovered_check || !bd.maybe_gives_discovered_check(mv))
            && (self.config.delta_prune_gives_check || !bd.gives_check(mv))
            && (self.config.delta_prune_near_promos || !mv.is_near_promo(bd))
            && bd.occupied().popcount() >= self.config.delta_prune_min_pieces
            && pat + margin <= n.alpha
        {
            Metrics::incr_node(n, Event::QsMovePruneDelta);
            true
        } else {
            false
        }
    }

    // see prune
    //
    fn can_see_prune_move(&self, mv: Move, n: &Node, _pat: Score, bd: &Board) -> bool {
        if mv.is_capture()
            && (self.config.see_prune_discovered_check || !bd.maybe_gives_discovered_check(mv))
            && (self.config.see_prune_gives_check || !bd.gives_check(mv))
            && (self.config.see_prune_near_promos || !mv.is_near_promo(bd))
            && bd.occupied().popcount() >= self.config.delta_prune_min_pieces
        {
            let t = Metrics::timing_start();
            let score = self.eval.eval_move_see(mv, bd);
            Metrics::profile(t, Timing::TimingQsSee);

            if score == 0.cp() && n.depth >= -self.config.even_exchange_max_ply || score < 0.cp() {
                Metrics::incr_node(n, Event::QsMovePruneSee);
                true
            // } else {
            //     if pat + score + self.config.see_prune_margin <= n.alpha {
            //         Metrics::incr_node(&n, Event::QsMovePruneSeeMargin);
            //         true
            } else {
                false
            }
        } else {
            false
        }
    }

    // Either:
    //   sets score, hash move, narrows window
    // Or:
    //   returns fail low/high with score
    //

    // SearchResult
    // enum Score, HintMove, Eval
    // new Window

    fn probe_tt(&mut self, n: &mut Node, pos: &Position, pat: &mut Score) -> Result<Move, Score> {
        if !self.config.probe_tt {
            *pat = self.eval.static_eval(pos).pov_score();
            return Ok(Move::new_null());
        };
        if let Some(ttn) = self.tt.probe_by_hash(pos.hash()) {
            Metrics::incr_node(n, Event::QsTtHit);
            match self.tt.eval_from_tt {
                EvalFromTt::Never => {
                    *pat = self.eval.static_eval(pos).pov_score();
                }
                EvalFromTt::UseTtEval => {
                    *pat = ttn.eval;
                    // if *pat != self.eval.static_eval(pos).pov_score() {
                    //     println!(
                    //         "\n\n\n\n{tt} != {s} for board {pos} node {n}\n\n\n\n\n",
                    //         tt = *pat,
                    //         s = self.eval.static_eval(pos).pov_score()
                    //     )
                    // }
                }
                // EvalFromTt::UseTtScore if ttn.nt == BoundType::ExactPv => {
                //     *pat = ttn.score.as_score(n.ply);
                //     if *pat != self.eval.static_eval(pos).pov_score() {
                //         println!(
                //             "\n\n\n\n{tt} != {s} for board {pos} node {n}\n\n\n\n\n",
                //             tt = *pat,
                //             s = self.eval.static_eval(pos).pov_score()
                //         )
                //     }
                // },
                EvalFromTt::UseTtScore => {
                    *pat = ttn.eval;

                    // if *pat != self.eval.static_eval(pos).pov_score() {
                    //     println!(
                    //         "\n\n\n\n{tt} != {s} for board {pos} node {n}\n\n\n\n\n",
                    //         tt = *pat,
                    //         s = self.eval.static_eval(pos).pov_score()
                    //     )
                    // }
                }
            };
            let s = ttn.score.as_score(n.ply);
            debug_assert!(s.is_finite());
            match ttn.bt {
                BoundType::ExactPv => {
                    if self.tt.allow_truncated_pv && !self.controller.analyse_mode {
                        // let mv = tt.validate_move(&pos);
                        self.trail.terminal(n, s, Event::TtPv);
                        return Err(s);
                    }
                }
                BoundType::UpperAll => {
                    if s <= n.alpha {
                        self.trail.prune_node(n, s, Event::TtAll);
                        return Err(s);
                    };
                    *pat = Score::min(*pat, s);
                }
                BoundType::LowerCut => {
                    if s >= n.beta {
                        self.trail.prune_node(n, s, Event::TtCut);
                        return Err(s);
                    };
                    *pat = Score::max(*pat, s);
                }
                BoundType::Unused => unreachable!(),
            }
            if self.config.use_hash_move {
                return Ok(ttn.validate_move(pos.board()));
            }
        } else {
            Metrics::incr_node(n, Event::QsTtMiss);
            *pat = self.eval.static_eval(pos).pov_score();
        }
        Ok(Move::new_null())
    }

    #[inline(always)]
    fn child_move(
        &mut self,
        n: &mut Node,
        pos: &mut Position,
        mv: Move,
        bs: &mut Option<Score>,
        unpruned_move_count: u64,
    ) -> Result<(), Score> {
        pos.push_move(mv);
        self.trail.push_move(n, mv);
        // self.current_variation.push(mv);
        let qsn = Node {
            zw:    n.zw,
            ply:   n.ply + 1,
            depth: n.depth - 1,
            alpha: -n.beta,
            beta:  -n.alpha,
        };
        let s = -self.qs(qsn, pos, Some(mv)).unwrap_or_else(|e| e);
        self.trail.pop_move(n, mv);
        pos.pop_move();

        // self.current_variation.pop();
        // bd.undo_move(&mv);
        if bs.is_none() || s > bs.unwrap() {
            *bs = Some(s);
        }
        // cutoffs before any pv recording
        if s >= n.beta {
            Metrics::incr_node(n, Event::QsCatCutMoveEval);
            Metrics::add_node(n, Event::QsCountMovesAtCutNode, unpruned_move_count);
            self.trail.fail(n, s, mv, Event::QsCatCutMoveEval);
            return Err(s.clamp_score());
        }
        if s > n.alpha {
            self.trail.alpha_raised(n, s, mv, Event::QsAlphaRaised);
            // self.record_move(n.ply, &mv);
            n.alpha = s;
            *bs = Some(s);
        } else {
            self.trail.ignore_move(n, s, mv, Event::QsMoveScoreLow);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::catalog::Catalog;
    use odonata_base::infra::profiler::PerfProfiler;
    use odonata_base::other::tags::EpdOps as _;

    use super::*;
    use crate::search::engine::ThreadedSearch;

    #[test]
    fn qsearch_serde_test() {
        info!("toml\n{}", toml::to_string(&Qs::default()).unwrap());
    }

    #[test]
    fn bugs_qs() {
        fn invoke(s: &str, depth: Ply) {
            let pos = Epd::parse_epd(s).unwrap();
            let mut eng = ThreadedSearch::new();
            let res = eng.search(pos.clone(), TimeControl::Depth(depth)).unwrap();
            println!("{}", res);
        }
        dbg!(log::log_enabled!(log::Level::Info));
        dbg!(log::log_enabled!(log::Level::Trace));
        dbg!(log::log_enabled!(target: "tree", log::Level::Error));
        dbg!(log::log_enabled!(target: "tree", log::Level::Info));
        dbg!(log::log_enabled!(target: "tree", log::Level::Trace));
        invoke(&Catalog::quiesce()[0].board().to_fen(), 2);

        invoke("8/1p4PR/1k6/3pNK2/5P2/r7/2p2n2/8 w - - 0 74", 1);
        invoke(&Catalog::bratko_kopec()[4].board().to_fen(), 7);
        invoke("rnbq1rk1/ppp1ppbp/3p1np1/8/2PPP3/2NB1N2/PP3PPP/R1BQK2R b KQ - 2 6", 1);
    }

    #[test]
    fn bench_qs() {
        // PROFD: qs  13 cyc=37,091  ins=29,170 br=2,676  304  978
        // PROFD: qs     13          48,295          76,501           7,917             563           1,284

        let catalog = Catalog::quiesce();
        // let mut flame = ProfProfiler::new("qs".to_string());
        let mut prof = PerfProfiler::new("qs");
        for epd in catalog {
            let mut trail = Trail::new(epd.board().clone());
            let eng = ThreadedSearch::new();
            let node = Node::root(0);
            let mut qs = RunQs {
                controller: &eng.search.controller,
                eval:       &eng.search.eval,
                clock:      &eng.search.clock,
                tt:         &eng.search.tt,
                config:     &eng.search.qs,
                trail:      &mut trail,
            };
            let mut pos = Position::from_board(epd.board().clone());
            let _score = prof.bench(|| qs.qsearch(&node, &mut pos, None));
            trace!("{pos}\n{trail}\n");
        }
    }

    #[test]
    fn metrics_qs() {
        // let pos = Position::parse_epd("1k6/p7/4p3/8/8/8/Q7/K7 w - - 0 1").unwrap();
        let pos = Catalog::test_position();
        let mut eng = ThreadedSearch::new();
        let mut prof = PerfProfiler::new("qs: metrics");
        let _res = prof.bench(|| eng.search(pos.clone(), TimeControl::Depth(6)).unwrap());
        // println!("{mets}", mets = res.metrics.unwrap().summary("Qs"));
    }

    #[test]
    fn test_qs_catalog() -> Result<()> {
        let catalog = Catalog::quiesce();
        // let pos = Position::find_by_id("pawn fork", &positions ).unwrap();
        for epd in catalog.into_iter() {
            let mut eng = ThreadedSearch::new();
            eng.search.qs.probe_tt = false;
            let res = eng.search(epd.clone(), TimeControl::Depth(0))?;
            // debug!("{}", search);
            println!(
                "search:  {pv_act:<20}  {pos_act}\nexpected:{pv_exp:<20}  {pos_exp}\n",
                pv_act = res.pv().to_string(),
                pos_act = res.to_results_epd(),
                pv_exp = epd.var("pv").unwrap().to_string(),
                pos_exp = epd,
                // res = eng.algo.results,
            );

            // position wil be POV of mover at end of PV line
            let q_pos = Position::from_epd(epd.clone());
            let mut static_eval = eng.search.eval.static_eval(&q_pos).pov_score().cp().unwrap();
            if q_pos.board().color_us() != epd.board().color_us() {
                static_eval = -static_eval;
            }
            assert_eq!(static_eval, static_eval);
            // assert_eq!(
            //     static_eval,
            //     engine.algo.results_as_position().ce().unwrap() as i16,
            //     "{}\n{}",
            //     engine.algo.results_as_position(), engine.algo.eval.w_eval_explain(&qboard, false)
            // );
        }
        Ok(())
    }
}
