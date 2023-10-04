use std::fmt::Display;

use crate::prelude::*;

#[derive(Clone, Default, Debug)]
pub struct SearchOptions {
    pub root_moves: MoveList,
}

impl SearchOptions {
    pub fn none() -> Self {
        Self::default()
    }
}

impl SearchOptions {
    pub fn to_uci(&self) -> String {
        if !self.root_moves.is_empty() {
            // note preceding space
            format!(" searchmoves {}", self.root_moves.to_uci())
        } else {
            "".to_string()
        }
    }

    pub fn to_san(&self, b: &Board) -> String {
        if !self.root_moves.is_empty() {
            // note preceding space
            format!(" searchmoves {}", self.root_moves.to_san(b))
        } else {
            "".to_string()
        }
    }
}

impl Display for SearchOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.root_moves.iter().format(" "))?;
        Ok(())
    }
}
