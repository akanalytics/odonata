
use crate::board::Board;
use crate::pvtable::PvTable;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::fmt;






#[derive(Clone, Debug)]
pub struct NullMovePruning {
    pub enabled: bool,
}

impl Component for NullMovePruning {
    fn settings(&self, c: &mut Config) {
        c.set("nmp.enabled", "type check default true");
    }
    fn configure(&mut self, c: &Config) {
        debug!("nmp.configure");
        self.enabled = c.bool("nmp.enabled").unwrap_or(self.enabled);
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
        }
    }
}


impl NullMovePruning {
    pub fn allow(&self, b: &Board, ply: Ply, variation: &PvTable) -> bool {
        if !self.enabled 
            || ((b.line_pieces() | b.knights()) & b.us()).is_empty()     
            || b.is_in_check(b.color_us())
            || variation.extract_pv_for(ply).contains_null_move() {
            return false;
        }
        true
    }

    pub fn depth_reduction(&self, _b: &Board) -> Ply {
        2
    }
}





impl fmt::Display for NullMovePruning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:?}", self)?;
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
    }
}
