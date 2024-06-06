use std::fmt;

use odonata_base::domain::node::Node;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::prelude::*;

use super::algo::Search;

#[derive(Clone, Debug)]
pub struct ReverseFutility {
    pub enabled:      bool,
    in_check:         bool,
    prune_zugzwang:   bool,
    prune_extensions: bool,
    prune_fw_node:    bool,
    prune_eval_mate:  bool,
    prune_alpha_mate: bool,
    prune_beta_mate:  bool,
    max_depth:        Ply,
    min_pieces:       i32,
    min_pieces_depth: Ply,
    margin1:          i32,
    margin2:          i32,
    margin3:          i32,
    margin4:          i32,
    margin_ply:       i32,
}

impl Component for ReverseFutility {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for ReverseFutility {
    fn default() -> Self {
        ReverseFutility {
            enabled:          true,
            in_check:         false,
            prune_zugzwang:   false,
            prune_extensions: false,
            prune_fw_node:    false,
            prune_eval_mate:  false,
            prune_alpha_mate: true,
            prune_beta_mate:  true,
            max_depth:        20,
            min_pieces:       0,
            min_pieces_depth: 1,
            margin1:          90,
            margin2:          180,
            margin3:          300,
            margin4:          400,
            margin_ply:       100,
        }
    }
}

impl Configurable for ReverseFutility {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.in_check.set(p.get("in_check"))?;
        self.prune_zugzwang.set(p.get("prune_zugzwang"))?;
        self.prune_extensions.set(p.get("prune_extensions"))?;
        self.prune_fw_node.set(p.get("prune_fw_node"))?;
        self.prune_eval_mate.set(p.get("prune_eval_mate"))?;
        self.prune_alpha_mate.set(p.get("prune_alpha_mate"))?;
        self.prune_beta_mate.set(p.get("prune_beta_mate"))?;
        self.max_depth.set(p.get("max_depth"))?;
        self.min_pieces.set(p.get("min_pieces"))?;
        self.min_pieces_depth.set(p.get("min_pieces_depth"))?;
        self.margin1.set(p.get("margin1"))?;
        self.margin2.set(p.get("margin2"))?;
        self.margin3.set(p.get("margin3"))?;
        self.margin4.set(p.get("margin4"))?;
        self.margin_ply.set(p.get("margin_ply"))?;
        Ok(p.is_modified())
    }
}

impl fmt::Display for ReverseFutility {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

impl Search {
    #[inline]
    pub fn reverse_fut(&mut self, bd: &Board, eval: Score, n: &Node, ext: Ply) -> Option<Score> {
        if !self.rev_fut.enabled {
            return None;
        }

        Metrics::incr_node(n, Event::RevFutConsider);
        if n.depth > self.max_depth {
            // dont prune too far away from leaf nodes
            Metrics::incr_node(n, Event::RevFutDeclineMaxDepth);
            return None;
        }

        if !self.rev_fut.prune_eval_mate && eval.is_mate() {
            Metrics::incr_node(n, Event::RevFutDeclineEvalMateBound);
            return None;
        }
        if !self.rev_fut.prune_alpha_mate && n.alpha.is_mate() {
            Metrics::incr_node(n, Event::RevFutDeclineAlphaMateBound);
            return None;
        }
        if !self.rev_fut.prune_beta_mate && n.beta.is_mate() {
            Metrics::incr_node(n, Event::RevFutDeclineBetaMateBound);
            return None;
        }

        if !self.rev_fut.prune_fw_node && n.is_fw() {
            Metrics::incr_node(n, Event::RevFutDeclineFwWindow);
            return None;
        }
        if !self.rev_fut.in_check && bd.is_in_check(bd.color_us()) {
            Metrics::incr_node(n, Event::RevFutDeclineInCheck);
            return None;
        }

        if self.rev_fut.min_pieces > 0
            && n.depth >= self.rev_fut.min_pieces_depth
            && bd.occupied().popcount() < self.rev_fut.min_pieces
        {
            Metrics::incr_node(n, Event::RevFutDeclineMinPieces);
            return None;
        }

        if !self.rev_fut.prune_extensions && ext != 0 {
            Metrics::incr_node(n, Event::RevFutDeclineExtension);
            return None;
        }

        // just king + pawns
        if !self.rev_fut.prune_zugzwang && (bd.us() - bd.pawns()).popcount() <= 1 {
            Metrics::incr_node(n, Event::RevFutDeclineZugzwang);
            return None;
        }
        // safety margin depends on how far away we are from leaf node
        let margin = Score::from_cp(match n.depth {
            1 => self.rev_fut.margin1,
            2 => self.rev_fut.margin2,
            3 => self.rev_fut.margin3,
            4 => self.rev_fut.margin4,
            d => self.rev_fut.margin4 + self.rev_fut.margin_ply * (d - 4),
        });
        // if eval - margin >= n.beta
        //     && EndGame::from_board(bd).likely_outcome(bd) != LikelyOutcome::UnknownOutcome
        // {
        //     return None;
        // }

        if eval - margin >= n.beta {
            Metrics::incr_node(n, Event::RevFutSuccess);
            return Some(eval - margin);
        }
        Metrics::incr_node(n, Event::RevFutFail);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::catalog::*;
    // use crate::comms::uci::*;
    // use crate::eval::eval::*;

    #[test]
    fn test_rev_fut() {
        let fut = ReverseFutility::default();
        println!("{:#?}", fut);
        println!("{}", fut);
    }
}
