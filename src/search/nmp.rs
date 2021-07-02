
use crate::board::Board;
use crate::pvtable::PvTable;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::fmt;






#[derive(Clone, Debug)]
pub struct NullMovePruning {
    pub enabled: bool,
    pub min_leaf_distance: Ply,
    pub depth_reduction_strat: i64, 
}

impl Component for NullMovePruning {
    fn settings(&self, c: &mut Config) {
        c.set("nmp.enabled", "type check default true");
        c.set("nmp.min.leaf.distance",  &format!("type spin min 0 max 100 default {}", self.min_leaf_distance));
        c.set("nmp.depth.reduction.strat",  &format!("type spin min 0 max 100 default {}", self.depth_reduction_strat));
    }
    fn configure(&mut self, c: &Config) {
        debug!("nmp.configure");
        self.enabled = c.bool("nmp.enabled").unwrap_or(self.enabled);
        self.min_leaf_distance = c.int("nmp.min.leaf.distance").unwrap_or(self.min_leaf_distance as i64) as Ply;
        self.depth_reduction_strat = c.int("nmp.depth.reduction.strat").unwrap_or(self.depth_reduction_strat);

    }
    fn new_game(&mut self) {
        self.new_search();
    }

    fn new_search(&mut self) {
    }
}

impl Default for NullMovePruning {
    fn default() -> Self {
        Self {
            enabled: true,
            min_leaf_distance: 2, // 1 means we still prune at frontier
            depth_reduction_strat: 2,
        }
    }
}


impl NullMovePruning {
    pub fn allow(&self, b: &Board, ply: Ply, depth: Ply, variation: &PvTable) -> bool {
        if !self.enabled 
            || ply + self.min_leaf_distance > depth
            || ((b.line_pieces() | b.knights()) & b.us()).is_empty()     
            || b.is_in_check(b.color_us())
            || variation.extract_pv_for(ply).contains_null_move() {
            return false;
        }
        true
    }

    pub fn depth_reduction(&self, _b: &Board, ply: Ply, depth: Ply) -> Ply {
        match self.depth_reduction_strat {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            237 => if depth-ply >= 7 { 3 } else { 2 },
            347 => if depth-ply >= 7 { 4 } else { 3 },
            236 => if depth-ply >= 6 { 3 } else { 2 },
            346 => if depth-ply >= 6 { 4 } else { 3 },
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
