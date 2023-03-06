use crate::boards::Board;
use crate::domain::NodeType;
use crate::cache::tt2::{TtNode, TtScore};
use crate::domain::Trail;
use crate::eval::score::{Score, ToScore};
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::search::algo::Algo;
use crate::search::node::{Event, Node};
// use crate::eval::score::Score;
use crate::infra::component::Component;
use crate::variation::Variation;
// use crate::{debug, logger::LogInit};
use crate::piece::Ply;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fmt;
use tracing::instrument;

// CLOP
// 75+0.6  a=2.7  b=0.198 c=0.000167
// 10+.08  a=2.62 b=0.231 c=0.00017
// 1+0.01  a=3.04 b=0.272 c=0.000185
//

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NullMovePruning {
    pub enabled: bool,
    pub recursive: bool,
    pub successive: bool,
    pub margin: Score,
    pub min_depth: Ply,
    pub store_tt: bool,
    pub depth_reduction_strat: i64,
    prune_alpha_mate: bool,
    prune_beta_mate: bool,
    pub a: f32,
    pub b: f32,
    pub c: f32,
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
            enabled: true,
            recursive: true,
            successive: true,
            margin: Score::from_cp(-10000),
            min_depth: 2, // 1 means we still prune at frontier (depth=1)
            store_tt: true,
            depth_reduction_strat: 100,
            prune_alpha_mate: false,
            prune_beta_mate: false,
            a: 2.7,
            b: 0.198,
            c: 0.00017,
        }
    }
}

// look for beta cuts by using a null move and null window search around beta
// works for moves that are just "too good to be true"
impl NullMovePruning {
    #[inline]
    pub fn allow(&self, trail: &mut Trail, b: &Board, n: &Node, eval: Score) -> bool {
        if !self.enabled {
            return false;
        }
        if n.ply <= 0 {
            return false; // no null move at root, might mean no moves (with move restrictions too!)
        }

        Metrics::incr_node(n, Event::NmpConsider);

        // definitely no point doing nmp at depth 0 as we do stabding pat in QS anyway
        if n.depth < self.min_depth {
            Metrics::incr_node(n, Event::NmpDeclineDepth);
            return false;
        }
        if (!self.prune_alpha_mate && n.alpha.is_mate())
            || (!self.prune_beta_mate && n.beta.is_mate())
        {
            Metrics::incr_node(n, Event::NmpDeclineMateBound);
            return false;
        }
        if !eval.is_numeric() {
            Metrics::incr_node(n, Event::NmpDeclineEvalNumeric);
            return false;
        }
        if ((b.line_pieces() | b.knights()) & b.us()).is_empty() {
            Metrics::incr_node(n, Event::NmpDeclineMaterial);
            return false;
        }
        // if node.alpha == node.beta - Score::from_cp(1) {
        //     // no NMP in PVS search
        //     return false;
        // }
        if eval < n.beta + self.margin {
            Metrics::incr_node(n, Event::NmpDeclineEvalMargin);
            return false;
        }

        if b.is_in_check(b.color_us()) {
            Metrics::incr_node(n, Event::NmpDeclineInCheck);
            return false;
        }
        let var = trail.path().take(n.ply as usize);
        if self.recursive && !self.successive && Self::last_move_is_null_move(&var) {
            Metrics::incr_node(n, Event::NmpDeclineSuccessive);
            return false;
        }
        if !self.recursive && Self::contains_null_move(&var) {
            Metrics::incr_node(n, Event::NmpDeclineRecursive);
            return false;
        }
        true
    }

    #[inline]
    pub fn contains_null_move(var: &Variation) -> bool {
        var.moves().any(|mv| mv.is_null())
    }

    #[inline]
    pub fn last_move_is_null_move(var: &Variation) -> bool {
        var.last() == Some(Move::new_null())
    }

    #[inline]
    pub fn depth_reduction(&self, eval: Score, b: &Board, n: &Node) -> Ply {
        // let eval = if eval.is_numeric() { eval } else { n.beta };
        match self.depth_reduction_strat {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 2 + n.depth / 4 + min((eval - n.beta).as_i16() as i32 / 128, 3),
            5 => 3 + n.depth / 4 + min((eval - n.beta).as_i16() as i32 / 128, 3),
            100 => f32::round(
                self.a
                    + n.depth as f32 * self.b
                    + f32::min((eval - n.beta).as_i16() as f32 * self.c, 3.0),
            ) as i32,

            // classical adaptive null move pruning reduction
            200 => {
                if n.depth > 8 {
                    3
                } else if n.depth <= 6 {
                    2
                } else if n.depth > 6 && b.us().popcount() >= 3 && b.them().popcount() >= 3 {
                    3
                } else {
                    2
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Algo {
    #[instrument(target="tree", "nmp", skip_all, fields(trail=?trail))]
    #[inline]
    pub fn nmp_node(
        &mut self,
        trail: &mut Trail,
        b: &Board,
        n: &Node,
        eval: Score,
    ) -> Result<Option<Score>, Event> {
        if !self.nmp.allow(trail, b, n, eval) {
            return Ok(None);
        }

        // let span = if n.ply < 2 {
        //     span!(target: "tree", Level::INFO, "nmp", trail=?trail )
        // } else {
        //     Span::none()
        // };
        // let lifetime = span.enter();

        let r = self.nmp.depth_reduction(eval, b, n);
        let mv = Move::new_null();
        let mut child_board = b.make_move(mv);
        self.current_variation.push(mv);
        trail.push_move(n, mv);
        // self.explainer.start(n, &self.current_variation);
        Metrics::incr_node(n, Event::NmpAttempt);
        let reduced_depth = std::cmp::max(n.depth - r - 1, 0);

        // we increment ply so that history tables etc work correctly

        let child_score = -self
            .alphabeta(
                "nmp",
                trail,
                &mut child_board,
                n.ply + 1,
                reduced_depth,
                -n.beta,
                -n.beta + 1.cp(),
                mv,
            )?
            .0;
        // b.undo_move(&mv);
        self.current_variation.pop();
        trail.pop_move(n, mv);
        // self.explainer.start(n, &self.current_variation);
        if child_score >= n.beta {
            // self.stats.inc_node_cut(n.ply, MoveType::Null, -1);
            // self.counts.inc(n, Event::PruneNullMovePrune);
            self.report_refutation(n.ply);
            self.explain_nmp(b, child_score, n);

            if self.nmp.store_tt {
                // score is clamped as you cant mate on a null move. Note reduced depth too
                Metrics::incr_node(n, Event::TtStoreNode);
                let entry = TtNode {
                    score: TtScore::new(child_score.clamp_score(), n.ply),
                    depth: reduced_depth + 1,
                    nt: NodeType::LowerCut,
                    bm: Move::new_null().to_inner(),
                    eval,
                };
                // remember this is the child board hash with child score,
                // but we store it as parent board and negative score and bound,
                // and reduced_depth + 1
                self.tt.store(b.hash(), entry);
            }
            Metrics::incr_node(n, Event::NmpSuccess);
            return Ok(Some(child_score));
        }
        Metrics::incr_node(n, Event::NmpFail);
        // drop(lifetime);
        Ok(None)
    }
}

impl fmt::Display for NullMovePruning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::catalog::*;
    //use crate::comms::uci::*;
    // use crate::eval::eval::*;

    #[test]
    fn test_nmp() {
        let nmp = NullMovePruning::default();
        println!("{:#?}", nmp);
        println!("{}", nmp);
    }
}
