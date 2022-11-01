use crate::board::Board;
use crate::infra::component::Component;
use crate::infra::utils::Displayable;
use crate::mv::Move;
use crate::piece::{Hash, Piece, Repeats};
use crate::position::Position;
use crate::variation::Variation;
// use crate::{debug, logger::LogInit};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Repetition {
    pub enabled: bool,
    pub avoid_tt_on_repeats: bool,

    #[serde(skip)]
    prior_positions: Vec<Hash>,

    #[serde(skip)]
    root_index: usize,
}

impl Component for Repetition {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.prior_positions.clear();
        self.root_index = 0;
    }
}

impl Default for Repetition {
    fn default() -> Self {
        Self {
            enabled: true,
            avoid_tt_on_repeats: false,
            prior_positions: Vec::new(),
            root_index: 0,
        }
    }
}

impl fmt::Display for Repetition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "avoid_tt_on_rep  : {}", self.avoid_tt_on_repeats)?;
        writeln!(f, "prior posn count : {}", self.prior_positions.len())?;
        writeln!(f, "root index       : {}", self.root_index)?;
        Ok(())
    }
}

//
// some use cases
//
// 2 repetitions before root, 1 after => would create a certain draw -> score as 0
// 0 repetitions before root, 1 after => score as zero to avoid cycles
// 1 repetition  before root, 1 after => score as zero to avoid cycles
// 2 repetitions before root, 0 after => score as normal (but avoid tt?) (otherwise everything looks like a draw)
// 0 repetitions before root, 3 after = > score as zero

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

    pub fn push_move(&mut self, mv: &Move, post_move: &Board) {
        if !self.enabled {
            return;
        }
        if mv.is_capture() || mv.mover_piece() == Piece::Pawn {
            self.prior_positions.push(0);
        }
        self.prior_positions.push(post_move.hash());
    }

    // uses supplied variation
    pub fn capture_all_prior_positions(&mut self, pos: &Position) {
        self.push_variation(pos.supplied_variation(), pos.board());
        self.root_index = self.prior_positions.len();
    }

    pub fn push_variation(&mut self, moves: &Variation, pre: &Board) {
        if !self.enabled {
            return;
        }

        let mut b = pre.clone();
        self.prior_positions.push(b.hash());
        for mv in moves.iter() {
            if mv.is_capture() || mv.mover_piece() == Piece::Pawn {
                self.prior_positions.push(0);
            }
            b = b.make_move(mv);
            self.prior_positions.push(b.hash());
        }
    }

    pub fn pop(&mut self) {
        self.prior_positions.pop();
        if self.prior_positions.last() == Some(&0) {
            self.prior_positions.pop();
        }
    }

    // current position counts as 1
    pub fn count(&self, b: &Board) -> Repeats {
        if !self.enabled {
            return Repeats::default();
        }
        // root index will be even or odd and match the board ply
        let in_total = self
            .prior_positions
            .iter()
            .rev()
            .take_while(|&h| *h != 0)
            //.skip(1)
            .step_by(2)
            .filter(|&h| *h == b.hash())
            .count() as u16;
        let in_search = self
            .prior_positions
            .iter()
            .skip(self.root_index)
            .rev()
            .take_while(|&h| *h != 0)
            //.skip(1)
            .step_by(2)
            .filter(|&h| *h == b.hash())
            .count() as u16;

        let reps = Repeats {
            in_search,
            in_total,
        };
        debug_assert_eq!(
            reps.in_total,
            reps.in_game() + reps.in_search,
            "{rep}",
            rep = Displayable(|f| self.debug_display(f, &b))
        );
        reps
    }

    pub fn debug_display(&self, f: &mut fmt::Formatter, b: &Board) -> fmt::Result {
        writeln!(f, "colour           : {}", b.color_us())?;
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "avoid_tt_on_rep  : {}", self.avoid_tt_on_repeats)?;
        writeln!(f, "prior posn count : {}", self.prior_positions.len())?;
        writeln!(f, "root index       : {}", self.root_index)?;
        for (i, &hash) in self.prior_positions.iter().enumerate() {
            if i == 0 && self.root_index == 0 {
                writeln!(f, "------")?;
            }
            writeln!(
                f,
                "{i:>3} {mat}",
                mat = if b.hash() == hash { "*" } else { "" }
            )?;
            if i + 1 == self.root_index {
                writeln!(f, "------\n\n\n")?;
            }
        }
        Ok(())
    }

    // pub fn is_repeated(&self, b: &Board) -> bool {
    //     if !self.enabled {
    //         return false;
    //     }
    //     self.prior_positions
    //         .iter()
    //         .rev()
    //         .take_while(|&h| *h != 0)
    //         .step_by(2)
    //         .any(|&h| h == b.hash())
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci_server::UciServer;
    use crate::eval::eval::Attr;
    use crate::eval::weight::Weight;
    use crate::search::algo::*;
    use crate::search::engine::*;
    use crate::search::timecontrol::*;
    use crate::tags::Tag;
    use test_log::test;

    #[test]
    fn test_repetition() {
        let boards: Vec<Board> = (0..20)
            .into_iter()
            .map(|i| Catalog::chess960(i))
            .map(|p| p.board().clone())
            .collect();
        let mut rep1 = Repetition::new();
        let b = Catalog::starting_board();
        let knight_mv = b.parse_uci_move("b1c3").unwrap();
        let pawn_mv = b.parse_uci_move("a2a3").unwrap();
        rep1.push_move(&knight_mv, &boards[0]);
        rep1.push_move(&knight_mv, &boards[1]);
        rep1.push_move(&pawn_mv, &boards[2]);
        rep1.push_move(&knight_mv, &boards[3]);
        rep1.push_move(&knight_mv, &boards[4]);
        rep1.push_move(&knight_mv, &boards[5]);
        rep1.push_move(&knight_mv, &boards[6]);
        assert_eq!(rep1.count(&boards[4]).in_total, 1);
        assert_eq!(rep1.count(&boards[2]).in_total, 1);
        assert_eq!(rep1.count(&boards[0]).in_total, 0); // pawn move reset the count

        rep1.pop(); // 6
        rep1.pop(); // 5
        rep1.pop(); // 4
        rep1.pop(); // 3
        rep1.pop(); // 2 the pawn move
        rep1.push_move(&knight_mv, &boards[2]);
        rep1.push_move(&knight_mv, &boards[3]);
        rep1.push_move(&knight_mv, &boards[4]);
        rep1.push_move(&knight_mv, &boards[5]);
        rep1.push_move(&knight_mv, &boards[6]);
        assert_eq!(rep1.count(&boards[4]).in_total, 1);
        assert_eq!(rep1.count(&boards[2]).in_total, 1);
        assert_eq!(rep1.count(&boards[0]).in_total, 1); // no pawn move to reset the hmvc
        rep1.push_move(&knight_mv, &boards[6]);
        rep1.push_move(&knight_mv, &boards[4]);
        assert_eq!(rep1.count(&boards[2]).in_total, 1);
    }

    #[test]
    fn test_rep_position() {
        let mut b = Catalog::starting_board();
        let mut algo = Algo::new();
        algo.set_timing_method(TimeControl::Depth(5));
        algo.set_callback(UciServer::uci_info);

        let s = concat!(
            "e2e4 b8c6 b1c3 g8f6 d2d4 d7d5 e4e5 f6e4 c3e4 d5e4 f1b5 c8d7 b5c6 d7c6 g1e2 e7e6 e1g1 ",
            "f8b4 a2a3 b4a5 c1e3 e8g8 c2c4 f7f6 b2b4 a5b6 c4c5 f6e5 c5b6 e5d4 e2d4 a7b6 d4e6 d8d1 ",
            "f1d1 f8f7 d1d8 a8d8 e6d8 f7f6 a1d1 f6f8 d8e6 f8c8 e3f4 c8a8 f4c7 a8a3 c7b6 a3d3 d1a1 ",
            "h7h6 b6c7 d3d2 c7f4 d2b2 a1a8 g8h7 e6f8 h7g8"
        );
        algo.repetition.new_game();
        let mvs = b.parse_uci_variation(s).unwrap();
        for mv in mvs.iter() {
            b = b.make_move(&mv);
            algo.repetition.push_move(&mv, &b);
            println!(
                "rep count = {} hash = {:x}",
                algo.repetition.count(&b).in_total,
                b.hash()
            );
        }
    }

    #[test]
    fn test_rep_bug1() {
        let s = concat!(
            "e2e4 b8c6 b1c3 e7e5 g1f3 g8f6 d2d4 e5d4 f3d4 f8b4 c1g5 d8e7 f2f3 e8g8 ",
            "d4c6 b4c3 b2c3 d7c6 d1d2 h7h6 g5e3 c8e6 f1d3 e7a3 e3f4 a3a5 c3c4 a5d2 e1d2 f8c8 d2c1 ",
            "f6d7 c1b1 c6c5 f4e3 d7e5 f3f4 e5d3 c2d3 c8d8 b1c2 b7b6 h1d1 e6g4 d1d2 d8e8 a1e1 a8d8 ",
            "f4f5 a7a6 h2h3 g4f5 e4f5 e8e5 d2f2 d8d6 f2f3 b6b5 c4b5 a6b5 e1c1 d6d8 c1h1 d8d7 h1d1 ",
            "d7d5 g2g4 e5e8 c2b1 d5e5 e3f2 e8a8 f3e3 e5e3 f2e3 c5c4 e3f4 a8a3 d3c4 b5c4 f4c7 a3h3 ",
            "d1c1 h6h5 c1c4 h3h1 b1b2 h1h4 c7e5 h4g4 c4c8 g8h7 c8c7 h7g8 c7c8"
        );
        let mut b = Catalog::starting_board();
        let mut eng = ThreadedSearch::new();
        eng.algo.set_callback(UciServer::uci_info);
        eng.algo.repetition.new_game();
        let mvs = b.parse_uci_variation(s).unwrap();
        for mv in mvs.iter() {
            b = b.make_move(&mv);
            eng.algo.repetition.push_move(&mv, &b);
        }
        eng.algo.set_position(Position::from_board(b));
        // eng.algo.explainer.enabled = true;
        // eng.algo.explainer.add_variation_to_explain(Variation::new());
        eng.algo.set_timing_method(TimeControl::Depth(5));
        eng.search_sync();
        println!("{}", eng.algo.results_as_position());
    }

    #[test]
    #[ignore]
    fn test_rep_bug2() {
        let mut engine = ThreadedSearch::new();
        engine
            .algo
            .eval
            .set_weight(Attr::ContemptPenalty.into(), Weight::from_i32(-30, -30));

        let pos3 = Catalog::draws()[3].clone();

        let board = pos3.board().make_moves_old(pos3.supplied_variation());
        engine.new_game();
        engine.set_position(pos3.clone());
        print!(
            "\n\npos3 = {}\nboard={}\nhash={}\nrep=\n{}\nRepeats{:?}\n",
            pos3,
            board,
            board.hash(),
            engine.algo.repetition,
            engine.algo.repetition.count(&board)
        );
        println!("rep = {:?}", engine.algo.repetition);
        engine.search_sync();
        let res = engine.algo.results_as_position();
        println!("res3: {}", res);

        let mut pos2 = pos3.clone();
        let mut var2 = pos3.supplied_variation().clone();
        let len = var2.len();
        var2.truncate(len - 4);
        pos2.set(Tag::SuppliedVariation(var2));
        engine.new_game();
        engine.set_position(pos2.clone());
        print!(
            "\n\npos2 = {}\nboard={}\nhash={}\nrep=\n{}\nRepeats{:?}\n",
            pos2,
            board,
            board.hash(),
            engine.algo.repetition,
            engine.algo.repetition.count(&board)
        );
        println!("rep = {:?}", engine.algo.repetition);
        engine.search_sync();
        let res = engine.algo.results_as_position();
        println!("res2: {}", res);

        let mut pos1 = pos3.clone();
        let mut var1 = pos3.supplied_variation().clone();
        let len = var1.len();
        var1.truncate(len - 8);
        pos1.set(Tag::SuppliedVariation(var1));
        engine.new_game();
        engine.set_position(pos1.clone());
        print!(
            "\n\npos1 = {}\nboard={}\nhash={}\nrep=\n{}\nRepeats{:?}\n",
            pos1,
            board,
            board.hash(),
            engine.algo.repetition,
            engine.algo.repetition.count(&board)
        );
        println!("rep = {:?}", engine.algo.repetition);
        engine.search_sync();
        let res = engine.algo.results_as_position();
        println!("res1: {}", res);
    }
}
