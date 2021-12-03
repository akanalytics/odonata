use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::eval::score::Score;
use crate::mv::Move;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::{Event, Node};
// use crate::eval::score::Score;
use crate::infra::component::Component;
use crate::variation::Variation;
// use crate::{debug, logger::LogInit};
use crate::types::{MoveType, Ply};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fmt;

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
    pub depth_reduction_strat: i64,
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
            depth_reduction_strat: 100,
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
    pub fn allow(&self, b: &Board, node: &Node, eval: Score, pv_table: &PvTable) -> bool {
        if !self.enabled {
            return false;
        }
        if node.ply == 0 {
            return false; // no null move at root, might mean no moves (with move restrictions too!)
        }
        if node.depth < self.min_depth {
            return false;
        }
        if !node.beta.is_numeric() {
            return false;
        }
        if ((b.line_pieces() | b.knights()) & b.us()).is_empty() {
            return false;
        }
        // if node.alpha == node.beta - Score::from_cp(1) {
        //     // no NMP in PVS search
        //     return false;
        // }
        if eval < node.beta + self.margin {
            return false;
        }

        if b.is_in_check(b.color_us()) {
            return false;
        }
        let var = pv_table.extract_pv_for(node.ply);
        if self.recursive && !self.successive && Self::last_move_is_null_move(&var) {
            return false;
        }
        if !self.recursive && Self::contains_null_move(&var) {
            return false;
        }
        true
    }

    #[inline]
    pub fn contains_null_move(var: &Variation) -> bool {
        var.iter().any(|mv| mv.is_null())
    }

    #[inline]
    pub fn last_move_is_null_move(var: &Variation) -> bool {
        var.last() == Some(&Move::NULL_MOVE)
    }

    #[inline]
    pub fn depth_reduction(&self, eval: Score, b: &Board, n: &Node) -> Ply {
        match self.depth_reduction_strat {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 2 + n.depth / 4 + min((eval - n.beta).as_i16() as i32 / 128, 3),
            5 => 3 + n.depth / 4 + min((eval - n.beta).as_i16() as i32 / 128, 3),
            100 => f32::round(self.a + n.depth as f32 * self.b + f32::min((eval - n.beta).as_i16() as f32 * self.c, 3.0)) as i32,

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
    #[inline]
    pub fn nmp(&mut self, b: &Board, n: &Node, eval: Score) -> Option<Score> {
        if self.minmax || !self.nmp.allow(&b, &n, eval, &self.pv_table) {
            return None;
        }

        let r = self.nmp.depth_reduction(eval, b, &n);
        let mv = Move::NULL_MOVE;
        let mut child_board = b.make_move(&mv);
        self.current_variation.push(mv);
        self.explainer.start(&self.current_variation);
        self.stats.inc_nmp(n.ply);
        let child_score = -self
            .alphabeta_recursive(
                &mut child_board,
                n.ply + 1,
                n.depth - r - 1,
                -n.beta,
                -n.beta + Score::from_cp(1),
                &mv,
            )
            .0;
        b.undo_move(&mv);
        self.current_variation.pop();
        self.explainer.start(&self.current_variation);
        if child_score >= n.beta {
            self.stats.inc_node_cut(n.ply, MoveType::Null, -1);
            self.counts.inc(n, Event::PruneNullMovePrune);
            self.report_refutation(n.ply);
            self.explain_nmp(child_score, n.beta);
            return Some(child_score);
        }
        None
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
