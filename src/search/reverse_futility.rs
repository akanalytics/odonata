use crate::board::Board;
use crate::eval::score::Score;
use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::search::node::{Event, Node};
use crate::Algo;
use crate::piece::Ply;
use serde::{Deserialize, Serialize};
use std::fmt;


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReverseFutility {
    pub enabled: bool,
    in_check: bool,
    prune_zugzwang: bool,
    prune_extensions: bool,
    prune_fw_node: bool,
    prune_alpha_mate: bool,
    prune_beta_mate: bool,
    max_depth: Ply,
    margin1: i32,
    margin2: i32,
    margin3: i32,
    margin_ply: i32,
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
            enabled: true,
            in_check: false,
            prune_zugzwang: false,
            prune_extensions: false,
            prune_fw_node: false,
            prune_alpha_mate: false,
            prune_beta_mate: false,
            max_depth: 6,
            margin1: 100,
            margin2: 200,
            margin3: 300,
            margin_ply: 100,
        }
    }
}

impl Algo {
    #[inline]
    pub fn reverse_fut(&mut self, bd: &Board, eval: Score, n: &Node, ext: Ply) -> Option<Score> {
        if !self.rev_fut.enabled {
            return None;
        }

        Metrics::incr_node(n, Event::RevFutConsider);
        if n.depth > self.max_depth {
            // dont prune too far away from leaf nodes
            Metrics::incr_node(&n, Event::RevFutDeclineMaxDepth);
            return None;
        }

        if (!self.rev_fut.prune_alpha_mate && n.alpha.is_mate())
            || (!self.rev_fut.prune_beta_mate && n.beta.is_mate())
        {
            Metrics::incr_node(&n, Event::RevFutDeclineMateBound);
            return None;
        }
        if !self.rev_fut.prune_fw_node && n.is_fw() {
            Metrics::incr_node(&n, Event::RevFutDeclineFwWindow);
            return None;
        }
        if !self.rev_fut.in_check && bd.is_in_check(bd.color_us()) {
            Metrics::incr_node(&n, Event::RevFutDeclineInCheck);
            return None;
        }

        if !self.rev_fut.prune_extensions && ext != 0 {
            Metrics::incr_node(n, Event::RevFutDeclineExt);
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
            d => self.rev_fut.margin3 + self.rev_fut.margin_ply * (d - 3),
        });

        if eval - margin >= n.beta {
            Metrics::incr_node(n, Event::RevFutPrune);
            return Some(eval - margin);
        }
        Metrics::incr_node(n, Event::RevFutFail);
        None
    }
}

impl fmt::Display for ReverseFutility {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
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
    fn test_rev_fut() {
        let fut = ReverseFutility::default();
        println!("{:#?}", fut);
        println!("{}", fut);
    }
}
