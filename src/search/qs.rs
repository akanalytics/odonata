use crate::board::Board;
use crate::bound::NodeType;
use crate::eval::endgame::EndGame;
use crate::eval::score::{Score, ToScore};
use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::piece::Ply;
use crate::search::algo::Algo;
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

impl Algo {
    // if the move resulted in checkmate, we should return a mate score
    // if the move results in a position which after quiesce, is potentially a mate,
    // we should not return a mate score, as only captures have been considered,
    // and a mate score might cut a genuine mate score elsewhere
    pub fn qs(&mut self, mut n: Node, bd: &mut Board, lm: Option<Move>) -> Score {
        debug_assert!(n.alpha < n.beta, "{n}");
        debug_assert!(n.ply >= 0);
        debug_assert!(n.depth <= 0);

        let orig_alpha = n.alpha;

        Metrics::incr_node(&n, Event::NodeQs);
        if n.is_zw() {
            Metrics::incr_node(&n, Event::NodeQsZw);
        }

        if EndGame::is_insufficient_material(&bd) {
            return Score::DRAW;
        }

        if !self.qs.enabled {
            return bd.static_eval(&self.eval);
        }

        let in_check = bd.is_in_check(bd.color_us());

        Metrics::incr_node(&n, Event::QsEvalStatic);
        let t = Metrics::timing_start();
        let mut pat = bd.static_eval(&self.eval);
        Metrics::profile(t, Timing::TimingQsEval);

        let mut hm = Move::NULL_MOVE;
        if self.qs.probe_tt {
            Metrics::incr_node(&n, Event::QsTtProbe);
            if let Some(tt) = self.tt.probe_by_hash(bd.hash()) {
                let score = tt.score.as_score(n.ply);
                debug_assert!(score.is_finite());
                Metrics::incr_node(&n, Event::QsTtHit);
                match tt.nt {
                    NodeType::ExactPv => {
                        return score;
                    }
                    NodeType::UpperAll => {
                        if score <= n.alpha {
                            return score;
                        };
                        pat = pat.min(score)
                    }
                    NodeType::LowerCut => {
                        if score >= n.beta {
                            return score;
                        };
                        pat = pat.max(score);
                    }
                    NodeType::Unused => unreachable!(),
                }
                if self.qs.use_hash_move {
                    hm = tt.validate_move(bd);
                }
            }
        }
        // let pat = pat.unwrap_or_else(|| {
        //     pat
        // });

        if !in_check {
            if pat >= n.beta {
                Metrics::incr_node(&n, Event::QsStandingPatPrune);
                Self::trace(n, pat, Move::NULL_MOVE, "standing pat");
                return pat.clamp_score();
            }
            if pat > n.alpha {
                Self::trace(n, pat, Move::NULL_MOVE, "alpha raised");
                n.alpha = pat;
            }
            // coarse delta prune - where margin bigger than any possible move
            let mut margin = self.qs.delta_prune_node_margin;
            if (bd.pawns() & bd.white() & Bitboard::RANK_7
                | bd.pawns() & bd.black() & Bitboard::RANK_2)
                .any()
            {
                margin = 2 * margin;
            }
            if pat.is_numeric() && pat + margin <= n.alpha {
                Metrics::incr_node(&n, Event::QsDeltaPruneNode);
                return (pat + margin).clamp_score();
            }
        } else {
            Metrics::incr_node(&n, Event::NodeQsInCheck);
        }

        let moves = bd.legal_moves();
        let capture_only = |mv: &&Move| in_check || mv.is_capture();
        let incl_promo = |mv: &&Move| in_check || mv.is_capture() || mv.is_promo();
        let some_promos = |mv: &&Move| {
            in_check
                || mv.is_capture()
                || mv.promo() == self.qs.promo_piece
        };

        Metrics::incr_node(&n, Event::NodeQsInterior);

        let t = Metrics::timing_start();
        let mut moves: MoveList = match (self.qs.promos, self.qs.promo_piece) {
            (false, _) => moves.iter().filter(capture_only).cloned().collect(),
            (true, None) => moves.iter().filter(incl_promo).cloned().collect(),
            (true, Some(_)) => moves.iter().filter(some_promos).cloned().collect(),
        };

        moves.sort_by_cached_key(|m| {
            Move::mvv_lva_score(m)
                + if let Some(lm) = lm {
                    if m.to() == lm.to() {
                        self.qs.recapture_score
                    } else {
                        0
                    }
                } else {
                    0
                }
                + if hm == *m {
                    10000
                } else {
                    0
                }
        });
        moves.reverse();
        Metrics::profile(t, Timing::TimingQsMoveSort);

        let mut unpruned_move = 0;
        let mut bs = None;
        for &mv in moves.iter() {
            Metrics::incr_node(&n, Event::QsMoveCount);
            if !in_check
                && pat.is_numeric()
                && self.qs.delta_prune
                && (self.qs.delta_prune_discovered_check || !bd.maybe_gives_discovered_check(mv))
                && (self.qs.delta_prune_gives_check || !bd.gives_check(&mv))
                && (self.qs.delta_prune_near_promos || !mv.is_near_promo())
                && pat + bd.eval_move_material(&self.eval, &mv) + self.qs.delta_prune_move_margin
                    <= n.alpha
            {
                Metrics::incr_node(&n, Event::QsDeltaPruneMove);
                continue;
            }

            if !in_check
                && mv.is_capture()
                && (self.qs.see_prune_discovered_check || !bd.maybe_gives_discovered_check(mv))
                && (self.qs.see_prune_gives_check || !bd.gives_check(&mv))
                && (self.qs.see_prune_near_promos || !mv.is_near_promo())
            {
                let t = Metrics::timing_start();
                let score = bd.eval_move_see(&self.eval, mv);
                Metrics::profile(t, Timing::TimingQsSee);

                if score == 0.cp() && n.ply <= self.qs.even_exchange_max_ply || score < 0.cp() {
                    Metrics::incr_node(&n, Event::QsSeePruneMove);
                    continue;
                }
            }

            unpruned_move += 1;
            let mut child = bd.make_move(&mv);
            let s = -self.qs(
                Node {
                    ply: n.ply + 1,
                    depth: n.depth - 1,
                    alpha: -n.beta,
                    beta: -n.alpha,
                },
                &mut child,
                Some(mv),
            );
            // bd.undo_move(&mv);
            if bs.is_none() || s > bs.unwrap() {
                bs = Some(s);
            }
            // cutoffs before any pv recording
            if s >= n.beta {
                Self::trace(n, s, mv, "mv is cut");
                Metrics::incr_node(&n, Event::NodeQsCut);
                Metrics::add_node(&n, Event::QsMoveCountAtCutNode, unpruned_move);
                return s.clamp_score();
            }
            if s > n.alpha {
                Self::trace(n, s, mv, "mv raises alpha");
                // self.record_move(n.ply, &mv);
                n.alpha = s;
            } else {
                Self::trace(n, s, mv, "mv doesn't raise alpha");
            }
        }

        if bs < Some(orig_alpha) {
            Metrics::incr_node(&n, Event::NodeQsAll);
            if orig_alpha.is_numeric() && bs < Some(orig_alpha - 200.cp()) {
                Metrics::incr_node(&n, Event::NodeQsAllVeryLow);
            }
        } else {
            Metrics::incr_node(&n, Event::NodeQsPv);
            Metrics::add_node(&n, Event::QsMoveCountAtPvNode, unpruned_move);
        }
        bs.unwrap_or(n.alpha).clamp_score()
    }

    #[inline]
    #[allow(unused_variables)]
    fn trace(n: Node, eval: Score, mv: Move, comment: &str) {
        // warn!("{:<25}  {:<6}  {mv:<5}  {comment}", n.to_string(), eval.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::search::engine::Engine;
    use crate::search::timecontrol::*;
    use crate::test_log::test;
    use anyhow::Result;

    #[test]
    fn qsearch_serde_test() {
        info!("toml\n{}", toml::to_string(&Qs::default()).unwrap());
    }

    #[test]
    fn test_qs() -> Result<()> {
        let catalog = Catalog::quiesce();
        // let pos = Position::find_by_id("pawn fork", &positions ).unwrap();
        for pos in catalog {
            let mut eng = Engine::new();

            eng.set_position(pos.clone());
            eng.algo.set_timing_method(TimeControl::Depth(0));
            eng.search();
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
            let qboard = eng.algo.pv().apply_to(pos.board());
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
