
use crate::board::Board;
use crate::eval::score::Score;
use crate::mv::Move;
use crate::search::node::Node;
use crate::pvtable::PvTable;
// use crate::eval::score::Score;
use crate::infra::parsed_config::{ParsedConfig, Component};
use crate::variation::Variation;
// use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::cmp::min;
use std::fmt;


// CLOP
// 75+0.6  a=2.7  b=0.198 c=0.000167
// 10+.08  a=2.62 b=0.231 c=0.00017
// 1+0.01  a=3.04 b=0.272 c=0.000185
// 
 
#[derive(Clone, Debug)]
pub struct NullMovePruning {
    pub enabled: bool,
    pub recursive: bool,
    pub successive: bool,
    pub min_depth: Ply,
    pub depth_reduction_strat: i64, 
    pub a: f32, 
    pub b: f32, 
    pub c: f32, 
}

impl Component for NullMovePruning {
    fn settings(&self, c: &mut ParsedConfig) {
        c.set("nmp.enabled", &format!("type check default {}", self.enabled));
        c.set("nmp.recursive", &format!("type check default {}", self.recursive));
        c.set("nmp.successive", &format!("type check default {}", self.successive));
        c.set("nmp.min.depth",  &format!("type spin min 0 max 100 default {}", self.min_depth));
        c.set("nmp.strat",  &format!("type spin min 0 max 100 default {}", self.depth_reduction_strat));
        c.set("nmp.a",  &format!("type string default {}", format!("{}", self.a)));
        c.set("nmp.b",  &format!("type string default {}", format!("{}", self.a)));
        c.set("nmp.c",  &format!("type string default {}", format!("{}", self.a)));
    }
    fn configure(&mut self, c: &ParsedConfig) {
        debug!("nmp.configure");
        self.enabled = c.bool("nmp.enabled").unwrap_or(self.enabled);
        self.recursive = c.bool("nmp.recursive").unwrap_or(self.recursive);
        self.successive = c.bool("nmp.successive").unwrap_or(self.successive);
        self.min_depth = c.int("nmp.min.depth").unwrap_or(self.min_depth as i64) as Ply;
        self.depth_reduction_strat = c.int("nmp.strat").unwrap_or(self.depth_reduction_strat);
        self.a = c.string("nmp.a").unwrap_or(format!("{}", self.a)).parse().unwrap();
        self.b = c.string("nmp.b").unwrap_or(format!("{}", self.b)).parse().unwrap();
        self.c = c.string("nmp.c").unwrap_or(format!("{}", self.c)).parse().unwrap();

    }
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
    }
}

impl Default for NullMovePruning {
    fn default() -> Self {
        Self {
            enabled: true,
            recursive: true,
            successive: true,
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
    pub fn allow(&self, b: &Board, node: &Node, pv_table: &PvTable) -> bool {
        if !self.enabled {
            return false;
        } 
        if node.ply == 0 {
            return false;  // no null move at root, might mean no moves (with move restrictions too!)
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
    pub fn depth_reduction(&self, eval: Score, n: &Node) -> Ply {
        match self.depth_reduction_strat {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 2 + n.depth / 4 + min((eval - n.beta).as_i16() as i32 / 128, 3),
            5 => 3 + n.depth / 4 + min((eval - n.beta).as_i16() as i32 / 128, 3),
            100 => f32::round(self.a + n.depth as f32 * self.b + f32::min((eval - n.beta).as_i16() as f32 * self.c, 3.0)) as i32,
            _ => unreachable!(),
        }
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
