use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::log_debug;
use crate::movelist::{Move, MoveValidator};
use crate::types::{Hash, Piece};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Repitition {
    enabled: bool,
    prior_positions: Vec<Hash>,
}

impl Configurable for Repitition {
    fn settings(&self, c: &mut Config) {
        c.set("repitition.enabled", "type check default true");
    }

    fn configure(&mut self, c: &Config) {
        log_debug!("repitition.configure with {}", c);
        self.enabled = c.bool("move_orderer.enabled").unwrap_or(self.enabled);
    }
}

impl Default for Repitition {
    fn default() -> Self {
        Self { enabled: true, prior_positions: Vec::new() }
    }
}

impl fmt::Display for Repitition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "prior posn count : {}", self.prior_positions.len())?;
        Ok(())
    }
}
//
// some use cases
//
// a. pos1, pos2, pos3 outside of search => draw, and search returns outcome of draw
// b. pos1, pos2 [search] pos3 => score-of-draw inside search (no choice) - but don't return outcome as a draw
// c. [search] pos1, pos2 => score-of-draw inside of search (probably dont want cycles) but don't return outcome as a draw
// d. pos1 [search] pos2 => score-of-draw inside of search (probably dont want cycles) but don't return outcome as a draw
//
// a & b we must return draw score
// c & d we can return draw score, but if time on out side, and losing, then maybe play it?
//
// so logic is
// count 3 before search => outcome draw
// count 3 return draw score
// count 2 probably return draw score
//
impl Repitition {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mv: &Move, b: &Board) {
        if mv.is_capture() || mv.mover_piece() == Piece::Pawn {
            self.prior_positions.push(0);
        }
        self.prior_positions.push(b.hash());
    }

    pub fn pop(&mut self) {
        self.prior_positions.pop();
        if self.prior_positions.last() == Some(&0) {
            self.prior_positions.pop();
        }
    }

    pub fn count(&self, b: &Board) -> usize {
        self.prior_positions
            .iter()
            .rev()
            .take_while(|&h| *h != 0)
            .skip(1)
            .step_by(2)
            .filter(|&h| *h == b.hash())
            .count()
    }

    pub fn is_repeated(&self, b: &Board) -> bool {
        self.prior_positions.iter().rev().take_while(|&h| *h != 0).step_by(2).any(|&h| h == b.hash())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;

    #[test]
    fn test_repitition() {
        let boards: Vec<Board> =
            (0..20).into_iter().map(|i| Catalog::chess960(i)).map(|p| *p.board()).collect();
        let mut rep1 = Repitition::new();
        let b = Catalog::starting_position();
        let knight_mv = b.parse_uci_move("b1c3").unwrap();
        let pawn_mv = b.parse_uci_move("a2a3").unwrap();
        rep1.push(&knight_mv, &boards[0]);
        rep1.push(&knight_mv, &boards[1]);
        rep1.push(&pawn_mv, &boards[2]);
        rep1.push(&knight_mv, &boards[3]);
        rep1.push(&knight_mv, &boards[4]);
        rep1.push(&knight_mv, &boards[5]);
        assert_eq!(rep1.count(&boards[4]), 1);
        assert_eq!(rep1.count(&boards[2]), 1);
        assert_eq!(rep1.count(&boards[0]), 0);  // pawn move reset the count

        rep1.pop(); // 5
        rep1.pop(); // 4
        rep1.pop(); // 3
        rep1.pop(); // 2 the pawn move
        rep1.push(&knight_mv, &boards[2]);
        rep1.push(&knight_mv, &boards[3]);
        rep1.push(&knight_mv, &boards[4]);
        rep1.push(&knight_mv, &boards[5]);
        assert_eq!(rep1.count(&boards[4]), 1);
        assert_eq!(rep1.count(&boards[2]), 1);
        assert_eq!(rep1.count(&boards[0]), 1); // no pawn move to reset the hmvc
        rep1.push(&knight_mv, &boards[6]);
        rep1.push(&knight_mv, &boards[4]);
        assert_eq!(rep1.count(&boards[2]), 1);
    }
}
