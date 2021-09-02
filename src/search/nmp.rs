
use crate::board::Board;
use crate::mv::Move;
use crate::search::node::Node;
use crate::pvtable::PvTable;
// use crate::eval::score::Score;
use crate::config::{Config, Component};
use crate::variation::Variation;
// use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::fmt;






#[derive(Clone, Debug)]
pub struct NullMovePruning {
    pub enabled: bool,
    pub recursive: bool,
    pub successive: bool,
    pub min_leaf_distance: Ply,
    pub depth_reduction_strat: i64, 
}

impl Component for NullMovePruning {
    fn settings(&self, c: &mut Config) {
        c.set("nmp.enabled", &format!("type check default {}", self.enabled));
        c.set("nmp.recursive", &format!("type check default {}", self.recursive));
        c.set("nmp.successive", &format!("type check default {}", self.successive));
        c.set("nmp.min.leaf.distance",  &format!("type spin min 0 max 100 default {}", self.min_leaf_distance));
        c.set("nmp.depth.reduction.strat",  &format!("type spin min 0 max 100 default {}", self.depth_reduction_strat));
    }
    fn configure(&mut self, c: &Config) {
        debug!("nmp.configure");
        self.enabled = c.bool("nmp.enabled").unwrap_or(self.enabled);
        self.recursive = c.bool("nmp.recursive").unwrap_or(self.recursive);
        self.successive = c.bool("nmp.successive").unwrap_or(self.successive);
        self.min_leaf_distance = c.int("nmp.min.leaf.distance").unwrap_or(self.min_leaf_distance as i64) as Ply;
        self.depth_reduction_strat = c.int("nmp.depth.reduction.strat").unwrap_or(self.depth_reduction_strat);

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
            min_leaf_distance: 2, // 1 means we still prune at frontier (depth=1)
            depth_reduction_strat: 2,
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
            return false;  // no null move at root
        } 
        if node.depth < self.min_leaf_distance {
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

    pub fn depth_reduction(&self, _b: &Board, _ply: Ply, depth: Ply) -> Ply {
        match self.depth_reduction_strat {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            237 => if depth >= 7 { 3 } else { 2 },
            347 => if depth >= 7 { 4 } else { 3 },
            236 => if depth >= 6 { 3 } else { 2 },
            346 => if depth >= 6 { 4 } else { 3 },
            _ => 2,
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
