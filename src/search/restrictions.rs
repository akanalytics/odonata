use crate::mv::Move;
use crate::infra::parsed_config::{ParsedConfig, Component};
use crate::movelist::MoveList;
use crate::types::Ply;
use std::fmt;
use serde::{Deserialize, Serialize};




#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Restrictions {
    pub enabled: bool,

    #[serde(skip)]
    pub search_moves: MoveList,
}

impl Component for Restrictions {
    fn settings(&self, c: &mut ParsedConfig) {
        c.set("restrictions.enabled", &format!("type check default {}", self.enabled));
    }
    fn configure(&mut self, c: &ParsedConfig) {
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

    #[inline]
    pub fn skip_move(&self, ply: Ply, mv: &Move) -> bool {
        if self.enabled && ply == 0 && !self.search_moves.is_empty() && !self.search_moves.contains(&mv)  {
            return true;
        } 
        false
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
