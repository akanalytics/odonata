use crate::board::Board;
use crate::config::{Config, Component};
use crate::log_debug;
use crate::movelist::{Move, Variation};
use crate::types::{Hash, Piece};
use crate::board::makemove::MoveMaker;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Repetition {
    pub enabled: bool,
    prior_positions: Vec<Hash>,
}

impl Component for Repetition {
    fn settings(&self, c: &mut Config) {
        c.set("repetition.enabled", "type check default true");
    }

    fn configure(&mut self, c: &Config) {
        log_debug!("repetition.configure with {}", c);
        self.enabled = c.bool("move_orderer.enabled").unwrap_or(self.enabled);
    }
    
    fn new_game(&mut self) {
        self.prior_positions.clear();
    }

    // FIXME!
    fn new_search(&mut self) {
    }

}

impl Default for Repetition {
    fn default() -> Self {
        Self { enabled: true, prior_positions: Vec::new() }
    }
}

impl fmt::Display for Repetition {
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
// reflect the counts -> Outcome in Eval (configurable) (and can test engine1 vs engine2)
// have Game store a repetition so that Draws can be detected (nothing to do with search, no need to be optimised)
// have Search store a repetition (needs to be fast)
// (static) Eval can determine checkmate
//
//
// dont have Board reference a repetition
//

impl Repetition {
    pub fn new() -> Self {
        Self::default()
    }



    pub fn prior_positions(&self) -> usize {
        self.prior_positions.len()
    }

    pub fn push(&mut self, mv: &Move, post_move: &Board) {
        if !self.enabled {
            return;
        }
        if mv.is_capture() || mv.mover_piece() == Piece::Pawn {
            self.prior_positions.push(0);
        }
        self.prior_positions.push(post_move.hash());
    }

    pub fn push_variation(&mut self, moves: &Variation, pre: &Board) {
        if !self.enabled {
            return;
        }

        let mut b = pre.clone();
        for mv in moves.iter() {
            if mv.is_capture() || mv.mover_piece() == Piece::Pawn {
                self.prior_positions.push(0);
            }
            b = b.make_move(mv);
            self.prior_positions.push(pre.hash());
        }
    }

    pub fn pop(&mut self) {
        self.prior_positions.pop();
        if self.prior_positions.last() == Some(&0) {
            self.prior_positions.pop();
        }
    }

    pub fn count(&self, b: &Board) -> u16 {
        if !self.enabled {
            return 0;
        }
        self.prior_positions
            .iter()
            .rev()
            .take_while(|&h| *h != 0)
            //.skip(1)
            .step_by(2)
            .filter(|&h| *h == b.hash())
            .count() as u16
    }

    pub fn is_repeated(&self, b: &Board) -> bool {
        if !self.enabled {
            return false;
        }
        self.prior_positions.iter().rev().take_while(|&h| *h != 0).step_by(2).any(|&h| h == b.hash())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::movelist::*;
    use crate::comms::uci::Uci;
    use crate::search::timecontrol::*;
    use crate::search::algo::*;


    #[test]
    fn test_repetition() {
        let boards: Vec<Board> =
            (0..20).into_iter().map(|i| Catalog::chess960(i)).map(|p| p.board().clone()).collect();
        let mut rep1 = Repetition::new();
        let b = Catalog::starting_position();
        let knight_mv = b.parse_uci_move("b1c3").unwrap();
        let pawn_mv = b.parse_uci_move("a2a3").unwrap();
        rep1.push(&knight_mv, &boards[0]);
        rep1.push(&knight_mv, &boards[1]);
        rep1.push(&pawn_mv, &boards[2]);
        rep1.push(&knight_mv, &boards[3]);
        rep1.push(&knight_mv, &boards[4]);
        rep1.push(&knight_mv, &boards[5]);
        rep1.push(&knight_mv, &boards[6]);
        assert_eq!(rep1.count(&boards[4]), 1);
        assert_eq!(rep1.count(&boards[2]), 1);
        assert_eq!(rep1.count(&boards[0]), 0);  // pawn move reset the count

        rep1.pop(); // 6
        rep1.pop(); // 5
        rep1.pop(); // 4
        rep1.pop(); // 3
        rep1.pop(); // 2 the pawn move
        rep1.push(&knight_mv, &boards[2]);
        rep1.push(&knight_mv, &boards[3]);
        rep1.push(&knight_mv, &boards[4]);
        rep1.push(&knight_mv, &boards[5]);
        rep1.push(&knight_mv, &boards[6]);
        assert_eq!(rep1.count(&boards[4]), 1);
        assert_eq!(rep1.count(&boards[2]), 1);
        assert_eq!(rep1.count(&boards[0]), 1); // no pawn move to reset the hmvc
        rep1.push(&knight_mv, &boards[6]);
        rep1.push(&knight_mv, &boards[4]);
        assert_eq!(rep1.count(&boards[2]), 1);
    }

    #[test]
    fn test_rep_position() {
        let mut b = Catalog::starting_position();
        let mut algo = Algo::new().set_timing_method(TimeControl::Depth(5)).set_callback(Uci::uci_info).build();

        let s = concat!("e2e4 b8c6 b1c3 g8f6 d2d4 d7d5 e4e5 f6e4 c3e4 d5e4 f1b5 c8d7 b5c6 d7c6 g1e2 e7e6 e1g1 ",
        "f8b4 a2a3 b4a5 c1e3 e8g8 c2c4 f7f6 b2b4 a5b6 c4c5 f6e5 c5b6 e5d4 e2d4 a7b6 d4e6 d8d1 ",
        "f1d1 f8f7 d1d8 a8d8 e6d8 f7f6 a1d1 f6f8 d8e6 f8c8 e3f4 c8a8 f4c7 a8a3 c7b6 a3d3 d1a1 ",
        "h7h6 b6c7 d3d2 c7f4 d2b2 a1a8 g8h7 e6f8 h7g8");
        algo.repetition.new_game();
        let mvs = b.parse_uci_moves(s).unwrap();
        for mv in mvs.iter() {
            b = b.make_move(&mv);
            algo.repetition.push(&mv, &b);
            println!("rep count = {} hash = {:x}", algo.repetition.count(&b), b.hash());
        }
    }

    #[test]
    fn test_rep_bug1() {

        let s = concat!("e2e4 b8c6 b1c3 e7e5 g1f3 g8f6 d2d4 e5d4 f3d4 f8b4 c1g5 d8e7 f2f3 e8g8 ",
        "d4c6 b4c3 b2c3 d7c6 d1d2 h7h6 g5e3 c8e6 f1d3 e7a3 e3f4 a3a5 c3c4 a5d2 e1d2 f8c8 d2c1 ",
        "f6d7 c1b1 c6c5 f4e3 d7e5 f3f4 e5d3 c2d3 c8d8 b1c2 b7b6 h1d1 e6g4 d1d2 d8e8 a1e1 a8d8 ",
        "f4f5 a7a6 h2h3 g4f5 e4f5 e8e5 d2f2 d8d6 f2f3 b6b5 c4b5 a6b5 e1c1 d6d8 c1h1 d8d7 h1d1 ",
        "d7d5 g2g4 e5e8 c2b1 d5e5 e3f2 e8a8 f3e3 e5e3 f2e3 c5c4 e3f4 a8a3 d3c4 b5c4 f4c7 a3h3 ",
        "d1c1 h6h5 c1c4 h3h1 b1b2 h1h4 c7e5 h4g4 c4c8 g8h7 c8c7 h7g8 c7c8");
        let mut b = Catalog::starting_position();
        let mut algo = Algo::new().set_timing_method(TimeControl::Depth(5)).set_callback(Uci::uci_info).build();
        algo.repetition.new_game();
        let mvs = b.parse_uci_moves(s).unwrap();
        for mv in mvs.iter() {
            b = b.make_move(&mv);
            algo.repetition.push(&mv, &b);
        }
        algo.search(&b);
        println!("{}", algo);
    }

}
