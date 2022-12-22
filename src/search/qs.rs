use crate::bits::Square;
use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::TranspositionTable2;
use crate::domain::Trail;
use crate::eval::endgame::EndGame;
use crate::eval::eval::Eval;
use crate::eval::score::{Score, ToScore};
use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::piece::Ply;
use crate::search::node::Node;
use crate::{Bitboard, Piece};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::{Event, Timing};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Qs {
    pub enabled: bool,
    pub only_on_capture: bool,
    pub promos: bool,
    pub promo_piece: Option<Piece>,
    pub checks_max_ply: Ply,
    pub delta_prune_discovered_check: bool,
    pub delta_prune_gives_check: bool,
    pub delta_prune_near_promos: bool,
    pub delta_prune_min_pieces: i32,
    pub see_prune_discovered_check: bool,
    pub see_prune_gives_check: bool,
    pub see_prune_near_promos: bool,
    pub even_exchange_max_ply: Ply,
    pub max_ply: u16,
    pub delta_prune: bool,
    pub delta_prune_move_margin: Score,
    pub delta_prune_node_margin: Score,
    pub recapture_score: i32,
    pub probe_tt: bool,
    pub use_hash_move: bool,
}

impl Default for Qs {
    fn default() -> Self {
        Qs {
            enabled: true,
            only_on_capture: false,
            delta_prune_discovered_check: true,
            delta_prune_gives_check: false,
            delta_prune_near_promos: true,
            see_prune_discovered_check: false,
            see_prune_gives_check: true,
            see_prune_near_promos: true,
            delta_prune_min_pieces: 0,
            even_exchange_max_ply: 2,
            promos: false,
            promo_piece: Some(Piece::Queen),
            max_ply: 10,
            delta_prune: true,
            delta_prune_move_margin: Score::from_cp(1000),
            delta_prune_node_margin: Score::from_cp(2000),
            checks_max_ply: 2,
            recapture_score: 0,
            probe_tt: true,
            use_hash_move: true,
        }
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
    pub eval: &'a Eval,
    pub tt: &'a TranspositionTable2,
    pub config: &'a Qs,
    pub trail: &'a mut Trail,
}

impl RunQs<'_> {
    // if the move resulted in checkmate, we should return a mate score
    // if the move results in a position which after quiesce, is potentially a mate,
    // we should not return a mate score, as only captures have been considered,
    // and a mate score might cut a genuine mate score elsewhere
    // since we only consider captures, repeats aren't an issue

    #[instrument(target="tree", "qs", skip_all, fields(t=?self.trail,%n))]
    pub fn qs(&mut self, mut n: Node, bd: &mut Board, lm: Option<Move>) -> Result<Score, Score> {
        debug_assert!(n.alpha < n.beta && n.ply >= 0 && n.depth <= 0, "{n}");
        // if n.beta.is_mate() && n.beta < 0.cp()  {
        //     let s = n.beta; // bd.static_eval(&self.eval);
        //     trail.prune_node(&n, s, Event::QsEvalStatic);
        //     return s;
        //     // || n.beta.is_mate() && n.beta > 0.cp()
        // }
        let orig_alpha = n.alpha;

        Metrics::incr_node(&n, Event::NodeQs);
        if n.is_zw() {
            Metrics::incr_node(&n, Event::NodeQsZw);
        }

        if EndGame::is_insufficient_material(&bd) {
            self.trail
                .terminal(&n, Score::DRAW, Event::QsInsufficientMaterial);
            return Err(Score::DRAW);
        }

        Metrics::incr_node(&n, Event::QsEvalStatic);
        let t = Metrics::timing_start();
        let mut pat = bd.static_eval(&self.eval);
        Metrics::profile(t, Timing::TimingQsEval);

        if !self.config.enabled {
            self.trail.terminal(&n, pat, Event::QsEvalStatic);
            return Err(pat);
        }

        let in_check = bd.is_in_check(bd.color_us());

        let hm = self.probe_tt(&mut n, bd, &mut pat)?;

        if !in_check {
            if pat >= n.beta {
                Metrics::incr_node(&n, Event::QsStandingPatFailHigh);
                self.trail.prune_node(&n, pat, Event::QsStandingPatFailHigh);
                return Err(pat.clamp_score());
            }
            // TODO: zugawang check
            // you cant stand pat unless theres already a move/pv (alpha=finite)
            if pat > n.alpha && n.alpha.is_finite() && n.ply >= 1 {
                self.trail
                    .terminal(&n, pat, Event::QsStandingPatAlphaRaised);
                n.alpha = pat;
            }
            // coarse delta prune - where margin bigger than any possible move
            // b.most_valuable_piece_except_king(b.them());
            let mut p = bd
                .most_valuable_piece_except_king(bd.occupied())
                .unwrap_or((Piece::Queen, Square::A1))
                .0;
            if p != Piece::Queen {
                p = Piece::Rook;
            }
            let ph = bd.phase(&self.eval.phaser);
            let pawn = self.eval.mb.piece_weights[Piece::Pawn].interpolate(ph);
            let mvp = self.eval.mb.piece_weights[p].interpolate(ph) + pawn;

            let mut margin = self.config.delta_prune_node_margin + Score::from_f32(mvp);
            if (bd.pawns() & bd.white() & Bitboard::RANK_7
                | bd.pawns() & bd.black() & Bitboard::RANK_2)
                .any()
            {
                let queen = self.eval.mb.piece_weights[Piece::Queen].interpolate(ph);
                margin = margin + Score::from_f32(queen);
            }
            if bd.occupied().popcount() >= self.config.delta_prune_min_pieces
                && pat.is_numeric()
                && pat + margin <= n.alpha
            {
                Metrics::incr_node(&n, Event::QsDeltaPruneNode);
                self.trail
                    .prune_node(&n, pat + margin, Event::QsDeltaPruneNode);
                return Err((pat + margin).clamp_score());
            }
        } else {
            Metrics::incr_node(&n, Event::NodeQsInCheck);
        }

        let moves = bd.legal_moves();
        let moves = self.sort_moves(&moves, in_check, &n, bd, lm, hm);

        let mut unpruned_move_count = 0;
        let mut bs = None;
        for &mv in moves.iter() {
            Metrics::incr_node(&n, Event::QsMoveCount);
            if !in_check && self.can_delta_prune_move(mv, &n, pat, bd) {
                continue;
            }

            if !in_check && self.can_see_prune_move(mv, &n, pat, bd) {
                continue;
            }

            unpruned_move_count += 1;
            self.child_move(&mut n, bd, mv, &mut bs, unpruned_move_count)?;
        }

        if bs < Some(orig_alpha) {
            Metrics::incr_node(&n, Event::NodeQsAll);
            if orig_alpha.is_numeric() && bs < Some(orig_alpha - 200.cp()) {
                Metrics::incr_node(&n, Event::NodeQsAllVeryLow);
            }
        } else {
            Metrics::incr_node(&n, Event::NodeQsPv);
            Metrics::add_node(&n, Event::QsMoveCountAtPvNode, unpruned_move_count);
        }
        trace!("leaving qs: orig alpha {orig_alpha} and new {}", n.alpha);

        Ok(bs.unwrap_or(n.alpha).clamp_score())
    }

    //
    // sort moves
    //
    fn sort_moves(
        &self,
        moves: &MoveList,
        in_check: bool,
        n: &Node,
        bd: &Board,
        lm: Option<Move>,
        hm: Move,
    ) -> MoveList {
        let capture_only = |mv: &&Move| in_check || mv.is_capture();
        let incl_promo = |mv: &&Move| in_check || mv.is_capture() || mv.is_promo();
        let some_promos =
            |mv: &&Move| in_check || mv.is_capture() || mv.promo() == self.config.promo_piece;

        Metrics::incr_node(&n, Event::NodeQsInterior);

        let t = Metrics::timing_start();
        let mut moves: MoveList = match (self.config.promos, self.config.promo_piece) {
            (false, _) => moves.iter().filter(capture_only).cloned().collect(),
            (true, None) => moves.iter().filter(incl_promo).cloned().collect(),
            (true, Some(_)) => moves.iter().filter(some_promos).cloned().collect(),
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
        moves
    }

    //
    // delta prune
    //
    fn can_delta_prune_move(&self, mv: Move, n: &Node, pat: Score, bd: &Board) -> bool {
        if pat.is_numeric()
            && self.config.delta_prune
            && (self.config.delta_prune_discovered_check || !bd.maybe_gives_discovered_check(mv))
            && (self.config.delta_prune_gives_check || !bd.gives_check(mv))
            && (self.config.delta_prune_near_promos || !mv.is_near_promo(&bd))
            && bd.occupied().popcount() >= self.config.delta_prune_min_pieces
            && pat + bd.eval_move_material(&self.eval, mv) + self.config.delta_prune_move_margin
                <= n.alpha
        {
            Metrics::incr_node(&n, Event::QsDeltaPruneMove);
            true
        } else {
            false
        }
    }

    //
    // see prune
    //
    fn can_see_prune_move(&self, mv: Move, n: &Node, _pat: Score, bd: &Board) -> bool {
        if mv.is_capture()
            && (self.config.see_prune_discovered_check || !bd.maybe_gives_discovered_check(mv))
            && (self.config.see_prune_gives_check || !bd.gives_check(mv))
            && (self.config.see_prune_near_promos || !mv.is_near_promo(&bd))
            && bd.occupied().popcount() >= self.config.delta_prune_min_pieces
        {
            let t = Metrics::timing_start();
            let score = bd.eval_move_see(&self.eval, mv);
            Metrics::profile(t, Timing::TimingQsSee);

            if score == 0.cp() && n.depth >= -self.config.even_exchange_max_ply || score < 0.cp() {
                Metrics::incr_node(&n, Event::QsSeePruneMove);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn probe_tt(&mut self, n: &mut Node, bd: &Board, pat: &mut Score) -> Result<Move, Score> {
        if !self.config.probe_tt {
            return Ok(Move::NULL_MOVE);
        };
        Metrics::incr_node(&n, Event::QsTtProbe);
        if let Some(ttn) = self.tt.probe_by_hash(bd.hash()) {
            let s = ttn.score.as_score(n.ply);
            debug_assert!(s.is_finite());
            Metrics::incr_node(&n, Event::QsTtHit);
            match ttn.nt {
                NodeType::ExactPv => {
                    if self.tt.allow_truncated_pv {
                        // let mv = tt.validate_move(&bd);
                        self.trail.terminal(&n, s, Event::TtPv);
                        return Err(s);
                    }
                }
                NodeType::UpperAll => {
                    if s <= n.alpha {
                        self.trail.prune_node(&n, s, Event::TtAll);
                        return Err(s);
                    };
                    *pat = Score::min(*pat, s);
                }
                NodeType::LowerCut => {
                    if s >= n.beta {
                        self.trail.prune_node(&n, s, Event::TtCut);
                        return Err(s);
                    };
                    *pat = Score::max(*pat, s);
                }
                NodeType::Unused => unreachable!(),
            }
            if self.config.use_hash_move {
                return Ok(ttn.validate_move(bd));
            }
        }
        Ok(Move::NULL_MOVE)
    }

    fn child_move(
        &mut self,
        n: &mut Node,
        bd: &Board,
        mv: Move,
        bs: &mut Option<Score>,
        unpruned_move_count: u64,
    ) -> Result<(), Score> {
        let mut child = bd.make_move(mv);
        self.trail.push_move(&n, mv.clone());
        // self.current_variation.push(mv);
        let qsn = Node {
            ply: n.ply + 1,
            depth: n.depth - 1,
            alpha: -n.beta,
            beta: -n.alpha,
        };
        let s = -self.qs(qsn, &mut child, Some(mv)).unwrap_or_else(|e| e);
        self.trail.pop_move(&n, mv);
        // self.current_variation.pop();
        // bd.undo_move(&mv);
        if bs.is_none() || s > bs.unwrap() {
            *bs = Some(s);
        }
        // cutoffs before any pv recording
        if s >= n.beta {
            Metrics::incr_node(&n, Event::NodeQsCut);
            Metrics::add_node(&n, Event::QsMoveCountAtCutNode, unpruned_move_count);
            self.trail.fail(&n, s, mv, Event::NodeQsCut);
            return Err(s.clamp_score());
        }
        if s > n.alpha {
            self.trail.alpha_raised(&n, s, mv, Event::QsAlphaRaised);
            // self.record_move(n.ply, &mv);
            n.alpha = s;
        } else {
            self.trail.ignore_move(&n, s, mv, Event::QsMoveScoreLow);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::engine::Engine;
    use crate::infra::profiler::PerfProfiler;
    use crate::search::engine::ThreadedSearch;
    use crate::search::timecontrol::*;
    use crate::test_log::test;
    use crate::{catalog::*, Algo, Position};
    use anyhow::Result;

    #[test]
    fn qsearch_serde_test() {
        info!("toml\n{}", toml::to_string(&Qs::default()).unwrap());
    }

    #[test]
    fn bugs_qs() {
        fn invoke(s: &str, depth: Ply) {
            let pos = Position::parse_epd(s).unwrap();
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
        invoke(
            "rnbq1rk1/ppp1ppbp/3p1np1/8/2PPP3/2NB1N2/PP3PPP/R1BQK2R b KQ - 2 6",
            1,
        );
    }

    #[test]
    fn bench_qs() {
        // PROFD: qs  13 cyc=37,091  ins=29,170 br=2,676  304  978
        // PROFD: qs     13          48,295          76,501           7,917             563           1,284

        let catalog = Catalog::quiesce();
        // let mut flame = ProfProfiler::new("qs".to_string());
        let mut prof = PerfProfiler::new("qs".to_string());
        for pos in catalog {
            let mut trail = Trail::new(pos.board().clone());
            let eng = Algo::new();
            let node = Node::root(0);
            let mut board = pos.board().clone();
            let mut qs = RunQs {
                eval: &eng.eval,
                tt: &eng.tt,
                config: &eng.qs,
                trail: &mut trail,
            };
            let _score = prof.benchmark(|| qs.qs(node, &mut board, None));
            trace!("{pos}\n{trail}\n");
        }
    }

    #[test]
    fn test_qs() -> Result<()> {
        let catalog = Catalog::quiesce();
        // let pos = Position::find_by_id("pawn fork", &positions ).unwrap();
        for pos in catalog {
            let mut eng = ThreadedSearch::new();

            eng.set_position(pos.clone());
            eng.algo.set_timing_method(TimeControl::Depth(0));
            eng.search_sync();
            // debug!("{}", search);
            println!(
                "search:  {:<20}  {}\nexpected:{:<20}  {}\n",
                eng.algo.pv().to_string(),
                eng.algo.results_as_position(),
                pos.pv()?.to_string(),
                pos,
            );
            // assert_eq!(
            //     eng.algo.pv().to_string(),
            //     pos.pv()?.to_string(),
            //     "\n\n{} {}\n",
            //     pos.id()?,
            //     pos.board().to_san_variation(eng.algo.pv(), None),
            // );
            // forward score is from POV of mover at end of PV line
            let qboard = pos.board().make_moves_old(&eng.algo.pv());
            let mut static_eval = qboard.static_eval(&mut eng.algo.eval).cp().unwrap();
            if qboard.color_us() != pos.board().color_us() {
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
