use crate::infra::parsed_config::Component;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::types::Ply;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Restrictions {
    pub enabled: bool,
    pub multi_pv_count: u32,

    #[serde(skip)]
    pub include_moves: MoveList,


    #[serde(skip)]
    pub exclude_moves: MoveList,
}

impl Component for Restrictions {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.include_moves.clear();
        self.exclude_moves.clear();
    }
}

impl Default for Restrictions {
    fn default() -> Self {
        Self {
            enabled: true,
            multi_pv_count: 1,
            include_moves: MoveList::new(),
            exclude_moves: MoveList::new(),
        }
    }
}

// look for beta cuts by using a null move and null window search around beta
// works for moves that are just "too good to be true"
impl Restrictions {
    #[inline]
    pub fn skip_move(&self, ply: Ply, mv: &Move) -> bool {
        if self.enabled && ply == 0 && (!self.include_moves.is_empty() && !self.include_moves.contains(&mv))
            || (!self.exclude_moves.is_empty() && self.exclude_moves.contains(&mv))
        {
            return true;
        }
        false
    }

    pub fn is_none(&self) -> bool {
        self.exclude_moves.len() == 0 && self.include_moves.is_empty()
    }

    pub fn multi_pv_index(&self) -> u32 {
        self.exclude_moves.len() as u32
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
    fn test_restrictions() {}
}
