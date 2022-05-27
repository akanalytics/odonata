use crate::board::Board;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::types::{Color, Piece};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CounterMove {
    enabled: bool,
    clear_every_move: bool,

    #[serde(skip)]
    counter_moves: Box<[[[Option<Move>; 64]; Piece::len()]; 2]>,
}

impl Component for CounterMove {
    fn new_game(&mut self) {
        self.counter_moves = Box::new([[[None; 64]; Piece::len()]; 2]);
    }

    fn new_position(&mut self) {
        if self.clear_every_move {
            self.counter_moves = Box::new([[[None; 64]; Piece::len()]; 2]);
        }
    }
}

impl Default for CounterMove {
    fn default() -> Self {
        CounterMove {
            enabled: true,
            clear_every_move: false,
            counter_moves: Box::new([[[None; 64]; Piece::len()]; 2]),
        }
    }
}

impl fmt::Display for CounterMove {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        Ok(())
    }
}

impl CounterMove {
    #[inline]
    pub fn counter_move_for(&self, b: &Board, parent: Move) -> Option<Move> {
        if let Some(m) = self.counter_move_unchecked(b.color_us(), parent) {
            if !m.is_null() && b.is_pseudo_legal_move(&m) && b.is_legal_move(&m) {
                return Some(m);
            }
        }
        None
    }

    #[inline]
    pub fn counter_move_unchecked(&self, c: Color, parent: Move) -> Option<Move> {
        if !self.enabled {
            return None;
        }
        if let Some(m) = self.counter_moves[c][parent.mover_piece()][parent.to()] {
            return Some(m);
        }
        None
    }

    /// The color is the player of the move being stored
    #[inline]
    pub fn store(&mut self, c: Color, parent: Move, mv: Move) {
        if !self.enabled || mv.is_capture() || mv.is_castle() {
            return;
        }

        self.counter_moves[c][parent.mover_piece()][parent.to()] = Some(mv);
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;

    #[test]
    fn hh_serde_test() {
        let hh = CounterMove::default();
        let text = toml::to_string(&hh).unwrap();
        info!("toml\n{}", text);
        let hh2: CounterMove = toml::from_str(&text).unwrap();
        info!("from toml\n{}", hh2);
    }
}
