use crate::infra::component::Component;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::piece::Ply;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Restrictions {
    pub enabled: bool,
    pub multi_pv_count: usize,

    #[serde(skip)]
    pub include_moves: MoveList,

    #[serde(skip)]
    pub exclude_moves: MoveList,
}

impl Component for Restrictions {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_iter(&mut self) {
        self.exclude_moves.clear();
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
        if self.enabled
            && ply == 0
            && (!self.include_moves.is_empty() && !self.include_moves.contains(mv))
            || (!self.exclude_moves.is_empty() && self.exclude_moves.contains(mv))
        {
            return true;
        }
        false
    }

    pub fn is_none(&self) -> bool {
        self.exclude_moves.is_empty() && self.include_moves.is_empty()
    }

    pub fn multi_pv_index(&self) -> usize {
        self.exclude_moves.len()
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

    #[test]
    fn test_restrictions() {}
}
