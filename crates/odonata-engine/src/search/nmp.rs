use std::fmt;

use odonata_base::boards::Position;
use odonata_base::domain::node::{Node, SearchType};
use odonata_base::domain::score::ToScore;
use odonata_base::domain::BoundType;
use odonata_base::eg::Zugzwang;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::prelude::*;
use odonata_base::Epd;
use strum_macros::EnumString;
use tracing::instrument;

use super::algo::Search;
use super::trail::Trail;
use crate::cache::tt2::{EvalFromTt, TtNode, TtScore};

// CLOP
// 75+0.6  a=2.7  b=0.198 c=0.000167
// 10+.08  a=2.62 b=0.231 c=0.00017
// 1+0.01  a=3.04 b=0.272 c=0.000185
//

#[derive(Clone, Debug, EnumString)]
pub enum NmpDepthReductionStrategy {
    Always1,
    Always2,
    Always3,
    ClassicalAdaptive,
    DepthEvalWeighted,
    DepthEvalWeightedPlus2,
    DepthEvalWeightedPlus3,
}

impl NmpDepthReductionStrategy {
    pub fn depth_reduction(&self, eval: Score, bd: &Board, n: &Node, a: f32, b: f32, c: f32) -> Ply {
        // let eval = if eval.is_numeric() { eval } else { n.beta };
        match self {
            Self::Always1 => 1,
            Self::Always2 => 2,
            Self::Always3 => 3,
            Self::DepthEvalWeightedPlus2 => 2 + n.depth / 4 + ((eval - n.beta).as_i16() as i32 / 128).clamp(0, 3),

            Self::DepthEvalWeightedPlus3 => 3 + n.depth / 4 + ((eval - n.beta).as_i16() as i32 / 128).clamp(0, 3),
            Self::DepthEvalWeighted => {
                f32::round(a + n.depth as f32 * b + f32::min((eval - n.beta).as_i16() as f32 * c, 3.0)) as i32
            }

            Self::ClassicalAdaptive => match n.depth {
                9.. => 3,
                ..=6 => 2,
                7 | 8 if bd.us().popcount() >= 3 && bd.them().popcount() >= 3 => 3,
                7 | 8 => 2,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct NullMovePruning {
    pub enabled:           bool,
    logging:               bool,
    zugzwang_check:        bool,
    pv_nodes:              bool,
    recursive:             bool,
    successive:            bool,
    pub eval_margin:       Score,
    min_depth:             Ply,
    min_ply:               Ply,
    store_tt:              bool,
    depth_reduction_strat: NmpDepthReductionStrategy,
    prune_alpha_mate:      bool,
    prune_beta_mate:       bool,
    a:                     f32,
    b:                     f32,
    c:                     f32,
    zugzwang:              Zugzwang,
}

impl Configurable for NullMovePruning {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.logging.set(p.get("logging"))?;
        self.zugzwang_check.set(p.get("zugzwang_check"))?;
        self.pv_nodes.set(p.get("pv_nodes"))?;
        self.recursive.set(p.get("recursive"))?;
        self.successive.set(p.get("successive"))?;
        self.eval_margin.set(p.get("eval_margin"))?;
        self.min_depth.set(p.get("min_depth"))?;
        self.min_ply.set(p.get("min_ply"))?;
        self.store_tt.set(p.get("store_tt"))?;
        self.depth_reduction_strat.set(p.get("depth_reduction_strat"))?;
        self.prune_alpha_mate.set(p.get("prune_alpha_mate"))?;
        self.prune_alpha_mate.set(p.get("prune_alpha_mate"))?;
        self.prune_beta_mate.set(p.get("prune_beta_mate"))?;
        self.a.set(p.get("a"))?;
        self.b.set(p.get("b"))?;
        self.c.set(p.get("c"))?;
        Ok(p.is_modified())
    }
}

impl fmt::Display for NullMovePruning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

impl Component for NullMovePruning {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for NullMovePruning {
    fn default() -> Self {
        Self {
            enabled:               true,
            logging:               false,
            zugzwang_check:        true,
            recursive:             true,
            successive:            true,
            pv_nodes:              true,
            eval_margin:           0.cp(), // eval needs to be >= beta + eval_margin
            min_depth:             2,      // 1 means we still prune at frontier (depth=1)
            min_ply:               1,
            store_tt:              true,
            depth_reduction_strat: NmpDepthReductionStrategy::DepthEvalWeighted,
            prune_alpha_mate:      false,
            prune_beta_mate:       false,
            a:                     2.7,
            b:                     0.198,
            c:                     0.00017,
            zugzwang:              Zugzwang::NonPawnNonPinned,
        }
    }
}

// look for beta cuts by using a null move and null window search around beta
// works for moves that are just "too good to be true"
impl NullMovePruning {
    #[inline]
    pub fn allow(&self, trail: &mut Trail, b: &Board, n: &Node, eval: Score) -> (bool, &str) {
        if !self.enabled {
            return (false, "disabled");
        }
        // this also covers no null move at root, as that might mean no moves (with move restrictions too!)
        if n.ply < self.min_ply {
            return (false, "min ply");
        }

        Metrics::incr_node(n, Event::NmpConsider);

        // definitely no point doing nmp at depth 0 as we do stabding pat in QS anyway
        if n.depth < self.min_depth {
            Metrics::incr_node(n, Event::NmpDeclineDepth);
            return (false, "depth");
        }
        if !self.pv_nodes && n.is_fw() {
            Metrics::incr_node(n, Event::NmpDeclinePvNode);
            return (false, "pv node");
        }

        if (!self.prune_alpha_mate && n.alpha.is_mate()) || (!self.prune_beta_mate && n.beta.is_mate()) {
            Metrics::incr_node(n, Event::NmpDeclineMateBound);
            return (false, "mate bound");
        }

        if !eval.is_numeric() {
            Metrics::incr_node(n, Event::NmpDeclineEvalNumeric);
            return (false, "non numeric eval");
        }

        if eval < n.beta + self.eval_margin {
            Metrics::incr_node(n, Event::NmpDeclineEvalMargin);
            return (false, "margin");
        }

        if self.zugzwang_check && self.zugzwang.is_maybe_zugzwang(b) {
            Metrics::incr_node(n, Event::NmpDeclineZugzwang);
            return (false, "zugzwang");
        }

        if b.is_in_check(b.color_us()) {
            Metrics::incr_node(n, Event::NmpDeclineInCheck);
            return (false, "in check");
        }
        let var = trail.path().take(n.ply as usize);
        if self.recursive && !self.successive && Self::last_move_is_null_move(&var) {
            Metrics::incr_node(n, Event::NmpDeclineSuccessive);
            return (false, "successive");
        }
        if !self.recursive && Self::contains_null_move(&var) {
            Metrics::incr_node(n, Event::NmpDeclineRecursive);
            return (false, "recursive");
        }
        (true, "")
    }

    #[inline]
    pub fn contains_null_move(var: &Variation) -> bool {
        var.moves().any(|mv| mv.is_null())
    }

    #[inline]
    pub fn last_move_is_null_move(var: &Variation) -> bool {
        var.last() == Some(Move::new_null())
    }
}

impl Search {
    #[instrument(target="tree", "nmp", skip_all, fields(trail=?trail))]
    #[inline]
    pub fn nmp_node(
        &mut self,
        trail: &mut Trail,
        pos: &mut Position,
        n: &Node,
        eval: Score,
    ) -> Result<Option<Score>, Event> {
        let eval = match self.tt.eval_from_tt {
            EvalFromTt::UseTtScore => {
                if let Some(entry) = self.tt.probe_by_hash(pos.board().hash()) {
                    if entry.bt == BoundType::ExactPv {
                        entry.score.as_score(n.ply)
                    } else {
                        eval
                    }
                } else {
                    eval
                }
            }
            _ => eval,
        };
        let (allow, reason) = self.nmp.allow(trail, pos.board(), n, eval);

        if self.nmp.logging {
            let reason = reason.to_string();
            // real.score > beta (but performed null move) => Good nmp
            // real.score <= beta (but performed null move) => Wasted Null Move Search
            // real.score > beta (but skipped null move) => Should Allow
            // real.score <= beta (but skipped null move) => Good skip
            enum NmpOutcome {
                PrunedCorrectly,
                WastedNullSearch,
                PrunedIncorrectly,
            }

            self.nmp.logging = false;
            let real_score = self
                .alphabeta(
                    "real",
                    &mut Trail::new(pos.board().clone()),
                    pos,
                    Node {
                        zw:    SearchType::ZeroWindow,
                        ply:   0,
                        depth: 0.max(n.depth - 1),
                        alpha: n.beta - 1.cp(),
                        beta:  n.beta,
                    },
                    Move::new_null(),
                )?
                .0;
            self.nmp.logging = true; // TODO

            let event = match allow {
                true if real_score > n.beta => Event::NmpGoodNullSearch,
                true if real_score <= n.beta => Event::NmpBadNullSearch,
                false if real_score <= n.beta => Event::NmpGoodSkip,
                false if real_score > n.beta => Event::NmpBadSkip,
                _ => unreachable!(),
            };

            Metrics::incr_node(n, event);

            let r = self.nmp.depth_reduction_strat.depth_reduction(
                eval,
                pos.board(),
                n,
                self.nmp.a,
                self.nmp.b,
                self.nmp.c,
            );
            let mut pos = Epd::from_board(pos.board().clone());
            pos.set_tag("c0", event.as_ref());
            pos.set_tag("c1", &reason);
            pos.set_tag("c1", &format!("ply:{p} reduce:{r}", p = n.ply));
            pos.set_tag("acd", &n.depth.to_string());
            pos.set_tag("ce", &eval.as_i16().to_string());
            pos.set_tag("Cpl", &n.beta.as_i16().to_string());
            if n.depth > 4 {
                warn!("{pos}");
            }
        }

        if !allow {
            return Ok(None);
        }

        // let span = if n.ply < 2 {
        //     span!(target: "tree", Level::INFO, "nmp", trail=?trail )
        // } else {
        //     Span::none()
        // };
        // let lifetime = span.enter();

        let r =
            self.nmp
                .depth_reduction_strat
                .depth_reduction(eval, pos.board(), n, self.nmp.a, self.nmp.b, self.nmp.c);
        let mv = Move::new_null();
        pos.push_move(mv);
        self.current_variation.push(mv);
        trail.push_move(n, mv);
        // self.explainer.start(n, &self.current_variation);
        Metrics::incr_node(n, Event::NmpAttempt);
        let reduced_depth = std::cmp::max(n.depth - r - 1, 0);

        // we increment ply so that history tables etc work correctly

        let ab = self.alphabeta(
            "nmp",
            trail,
            pos,
            Node {
                zw:    SearchType::ZeroWindow | SearchType::NullMove,
                ply:   n.ply + 1,
                depth: reduced_depth,
                alpha: -n.beta,
                beta:  -n.beta + 1.cp(),
            },
            mv,
        );
        pos.pop_move();
        let child_score = -ab?.0;
        // b.undo_move(&mv);
        self.current_variation.pop();
        trail.pop_move(n, mv);
        // self.explainer.start(n, &self.current_variation);
        if child_score >= n.beta {
            // self.stats.inc_node_cut(n.ply, MoveType::Null, -1);
            // self.counts.inc(n, Event::PruneNullMovePrune);
            Metrics::incr_node(n, Event::NmpSuccess);
            self.report_refutation(n.ply);
            // self.explain_nmp(pos.board(), child_score, n);

            // dont allow a mate score on a null move
            // let tt_score = child_score.clamp_score();
            if self.nmp.store_tt {
                // TODO: Score here does not involve a null move, so permit a mate score. Note reduced depth too
                Metrics::incr_node(n, Event::TtStoreNode);
                let entry = TtNode {
                    score: TtScore::new(child_score.clamp_score(), n.ply),
                    depth: reduced_depth + 1,
                    bt: BoundType::LowerCut,
                    bm: Move::new_null().to_inner(),
                    eval,
                };
                // remember this is the child board hash with child score,
                // but we store it as parent board and negative score and bound,
                // and reduced_depth + 1
                self.tt.store(pos.board().hash(), entry);
            }
            // TODO: score is clamped as you cant mate on a null move.
            return Ok(Some(child_score));
        }
        Metrics::incr_node(n, Event::NmpFail);
        // drop(lifetime);
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::infra::profiler::PerfProfiler;
    use test_log::test;

    use super::*;
    use crate::search::engine::ThreadedSearch;

    fn fixture() -> Board {
        Board::parse_diagram(
            r"
                k.......
                .p......
                p...p...
                P.......
                .P......
                ....P...
                ........
                K....... w - - 1 1",
        )
        .unwrap()
    }

    #[test]
    fn test_nmp_logging() {
        let board = Board::parse_diagram(
            r"
                kq.r....
                .p......
                p...p...
                P.......
                .P......
                ....P...
                ........
                KNQ..... w - - 1 1",
        )
        .unwrap();
        let mut eng = ThreadedSearch::new();
        eng.search.nmp.logging = true;
        let pos = Epd::from_board(board);
        let _sr = eng.search(pos, TimeControl::Depth(10)).unwrap();
        // warn!(target: "metrics",
        //     "\n{metrics}",
        //     metrics = sr.metrics.to_string_or("-")
        // );
    }

    #[test]
    fn test_nmp_metrics() {
        let pos = Epd::from_board(fixture());
        // let pos = Position::parse_epd("4q3/5r1k/7p/6p1/8/P2p2P1/7P/6K1 w - - 0 59").unwrap();
        //    let pos = Catalog::test_position();
        let mut eng = ThreadedSearch::new();
        // eng.nmp.zugzwang = Zugzwang::NonPawnOrPawnMove1;
        let mut p = PerfProfiler::new("null move pruning");
        let _sr = p.bench(|| {
            eng.set_option("Clear Hash", "").unwrap();
            eng.search(pos.clone(), TimeControl::Depth(15)).unwrap()
        });
        drop(p);
        info!(target: "metrics",
            "{fen}\n{diag}\n{metrics}",
            fen = pos.board().to_fen(),
            diag = pos.board().to_diagram(),
            metrics = eng.metrics("Nmp").unwrap(),
        );
    }
}
