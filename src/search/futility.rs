
use crate::board::Board;
use crate::pvtable::PvTable;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::fmt;






#[derive(Clone, Debug)]
pub struct FutilityPruning {
    pub enabled: bool,
    pub max_depth: Ply,
}

impl Component for FutilityPruning {
    fn settings(&self, c: &mut Config) {
        c.set("futility.enabled", "type check default true");
        c.set("futility.max.depth",  &format!("type spin min 0 max 100 default {}", self.max_depth));
    }
    fn configure(&mut self, c: &Config) {
        debug!("futility.configure");
        self.enabled = c.bool("futility.enabled").unwrap_or(self.enabled);
        self.max_depth = c.int("futility.min.leaf.distance").unwrap_or(self.max_depth as i64) as Ply;

    }
    fn new_game(&mut self) {
        self.new_search();
    }

    fn new_search(&mut self) {
    }
}

impl Default for FutilityPruning {
    fn default() -> Self {
        Self {
            enabled: true,
            max_depth: 6, 
        }
    }
}


impl FutilityPruning {
    pub fn allow(&self, _b: &Board, _ply: Ply, _depth: Ply) -> bool {
        false
    }
}




impl fmt::Display for FutilityPruning {
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
    fn test_futility() {
        let nmp = FutilityPruning::default();
        println!("{:#?}", nmp);
        println!("{}", nmp);
    }
}
