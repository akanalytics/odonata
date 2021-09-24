
use crate::board::Board;
// use crate::mv::Move;
use crate::search::node::Node;
use crate::pvtable::PvTable;
use crate::config::{Config, Component};
// use crate::variation::Variation;
use crate::movelist::MoveList;
// use crate::types::Ply;
use std::fmt;




#[derive(Clone, Debug)]
pub struct Restrictions {
    pub enabled: bool,
    pub search_moves: MoveList,
}

impl Component for Restrictions {
    fn settings(&self, c: &mut Config) {
        c.set("restrictions.enabled", &format!("type check default {}", self.enabled));
    }
    fn configure(&mut self, c: &Config) {
        debug!("restrictions.configure");
        self.enabled = c.bool("restrictions.enabled").unwrap_or(self.enabled);
    }
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.search_moves.clear();
    }
}

impl Default for Restrictions {
    fn default() -> Self {
        Self {
            enabled: true,
            search_moves: MoveList::new(),
        }
    }
}

// look for beta cuts by using a null move and null window search around beta
// works for moves that are just "too good to be true"
impl Restrictions {
    pub fn allow(&self, _b: &Board, _node: &Node, _pv_table: &PvTable) -> bool {
        if !self.enabled {
            return false;
        } 
        true
    }


}


impl fmt::Display for Restrictions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restrictions() {
    }
}
