use std::fmt;

use serde::{Deserialize, Serialize};

use super::Position;
use crate::epd::Epd;
use crate::infra::component::Component;
use crate::piece::Repeats;
use crate::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repetition {
    enabled:                     bool,
    never_with_null_moves:       bool,
    include_prior_to_null_moves: bool,
    in_played_threshold:         usize,
    in_search_threshold:         usize,

    #[serde(skip)]
    prior_positions: Vec<Hash>,

    #[serde(skip)]
    root_index: usize,
}

/// logic:
/// if we see a single repeat in the search, then we may as well call it
///  a draw rather than repeat the same moves as before (single repeat = 2 counting current as 1)
///
/// we may however need to play a repeat of a played position to get out of a bad position
/// so dont terminate search on these. 3 matches means the move is the third time in this position => draw
impl Default for Repetition {
    fn default() -> Self {
        Self {
            enabled:                     true,
            never_with_null_moves:       false,
            include_prior_to_null_moves: true, //  +19 elo
            in_played_threshold:         3,
            in_search_threshold:         2,
            root_index:                  0,
            prior_positions:             Vec::new(),
        }
    }
}

impl Configurable for Repetition {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.never_with_null_moves.set(p.get("never_with_null_moves"))?;
        self.include_prior_to_null_moves
            .set(p.get("include_prior_to_null_moves"))?;
        self.in_played_threshold.set(p.get("in_played_threshold"))?;
        self.in_search_threshold.set(p.get("in_search_threshold"))?;
        Ok(p.is_modified())
    }
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

impl fmt::Display for Repetition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl Repetition {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prior_positions(&self) -> usize {
        self.prior_positions.len()
    }

    pub fn push_move(&mut self, mv: Move, pre_move: &Board) {
        if !self.enabled {
            return;
        }
        if mv.is_capture() || mv.mover_piece(pre_move) == Piece::Pawn {
            self.prior_positions.push(0);
        }
        self.prior_positions.push(pre_move.make_move(mv).hash());
    }

    // uses supplied variation
    pub fn capture_all_prior_positions(&mut self, epd: &Epd) {
        self.push_variation(&epd.played(), &epd.setup_board());
        self.root_index = self.prior_positions.len();
    }

    pub fn push_variation(&mut self, moves: &Variation, pre: &Board) {
        if !self.enabled {
            return;
        }

        let mut board = pre.clone();
        for mv in moves.moves() {
            self.push_move(mv, &board);
            board = board.make_move(mv);
        }
    }

    pub fn pop(&mut self) {
        self.prior_positions.pop();
        if self.prior_positions.last() == Some(&0) {
            self.prior_positions.pop();
        }
    }

    pub fn is_draw_by_repetition(&self, p: &Position) -> bool {
        if self.never_with_null_moves && p.has_null_move() {
            return false;
        }
        p.search_reps() >= self.in_search_threshold || p.played_reps() >= self.in_played_threshold
    }

    // pub fn count(&self, p: &Position) -> Repeats {
    //     if p.has_null_move() && self.never_with_null_moves {
    //         return Repeats {
    //             in_search: 0,
    //             in_played: 0,
    //         };
    //     }
    //     // if last played isnt the same color as current move then skip (it cant match)
    //     Repeats {
    //         in_played: self.played_reps(p) as u16,
    //         in_search: self.search_reps(p) as u16,
    //     }
    // }

    // current position counts as 1
    pub fn count_old(&self, b: &Board) -> Repeats {
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

        Repeats {
            in_search,
            in_played: in_total - in_search,
        }
    }

    pub fn debug_display(&self, f: &mut fmt::Formatter, b: &Board) -> fmt::Result {
        writeln!(f, "colour           : {}", b.color_us())?;
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "prior posn count : {}", self.prior_positions.len())?;
        writeln!(f, "root index       : {}", self.root_index)?;
        for (i, &hash) in self.prior_positions.iter().enumerate() {
            if i == 0 && self.root_index == 0 {
                writeln!(f, "------")?;
            }
            writeln!(f, "{i:>3} {mat}", mat = if b.hash() == hash { "*" } else { "" })?;
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
    use test_log::test;

    use super::*;
    use crate::catalog::*;

    // #[test]
    // fn test_repetition() {
    //     let bds: Vec<Board> = (0..20)
    //         .map(Catalog::chess960)
    //         .map(|p| p.board().clone())
    //         .collect();
    //     let mut rep1 = Repetition::new();
    //     let b = Catalog::starting_board();
    //     let knight_mv = b.parse_uci_move("b1c3").unwrap();
    //     let pawn_mv = b.parse_uci_move("a2a3").unwrap();
    //     rep1.push_move(knight_mv, &bds[0]);
    //     rep1.push_move(knight_mv, &bds[1]);
    //     rep1.push_move(pawn_mv, &bds[2]);
    //     rep1.push_move(knight_mv, &bds[3]);
    //     rep1.push_move(knight_mv, &bds[4]);
    //     rep1.push_move(knight_mv, &bds[5]);
    //     rep1.push_move(knight_mv, &bds[6]);
    //     assert_eq!(rep1.count_old(&bds[4].make_move(knight_mv)).in_total(), 1);
    //     assert_eq!(rep1.count_old(&bds[2].make_move(pawn_mv)).in_total(), 1);
    //     assert_eq!(rep1.count_old(&bds[0].make_move(knight_mv)).in_total(), 0); // pawn move reset the count

    //     rep1.pop(); // 6
    //     rep1.pop(); // 5
    //     rep1.pop(); // 4
    //     rep1.pop(); // 3
    //     rep1.pop(); // 2 the pawn move
    //     rep1.push_move(knight_mv, &bds[2]);
    //     rep1.push_move(knight_mv, &bds[3]);
    //     rep1.push_move(knight_mv, &bds[4]);
    //     rep1.push_move(knight_mv, &bds[5]);
    //     rep1.push_move(knight_mv, &bds[6]);
    //     assert_eq!(rep1.count_old(&bds[4].make_move(knight_mv)).in_total(), 1);
    //     assert_eq!(rep1.count_old(&bds[2].make_move(knight_mv)).in_total(), 1);
    //     assert_eq!(rep1.count_old(&bds[0].make_move(knight_mv)).in_total(), 1); // no pawn move to reset the hmvc
    //     rep1.push_move(knight_mv, &bds[6]);
    //     rep1.push_move(knight_mv, &bds[4]);
    //     assert_eq!(rep1.count_old(&bds[2].make_move(knight_mv)).in_total(), 1);
    // }

    #[test]
    fn test_repetition2() -> anyhow::Result<()> {
        let b = Catalog::starting_board().make_move_str("e2e4")?.make_move_str("e7e6")?;
        let bds: Vec<Board> = (0..20).map(|_| b.clone()).collect();
        let mut rep1 = Repetition::new();
        let knight_mv = b.parse_uci_move("b1c3")?;
        let knight_mv2 = b.parse_uci_move("g1f3")?;
        let knight_mv3 = b.parse_uci_move("g1h3")?;
        let knight_mv4 = b.parse_uci_move("b1a3")?;
        let bishop_mv1 = b.parse_uci_move("f1e2")?;
        let bishop_mv2 = b.parse_uci_move("f1d3")?;
        let pawn_mv = b.parse_uci_move("a2a3")?;
        rep1.push_move(knight_mv, &bds[0]);
        rep1.push_move(knight_mv2, &bds[1]);
        rep1.push_move(pawn_mv, &bds[2]);
        rep1.push_move(knight_mv3, &bds[3]);
        rep1.push_move(knight_mv, &bds[4]);
        rep1.push_move(bishop_mv1, &bds[5]);
        rep1.push_move(bishop_mv2, &bds[6]);
        assert_eq!(rep1.count_old(&bds[4].make_move(knight_mv)).in_total(), 1);
        assert_eq!(rep1.count_old(&bds[2].make_move(pawn_mv)).in_total(), 1);
        // pawn move after this knight move reset the count
        assert_eq!(rep1.count_old(&bds[1].make_move(knight_mv2)).in_total(), 0);

        rep1.pop(); // 6
        rep1.pop(); // 5
        rep1.pop(); // 4
        rep1.pop(); // 3
        rep1.pop(); // 2 the pawn move
        rep1.push_move(knight_mv, &bds[2]);
        rep1.push_move(knight_mv3, &bds[3]);
        rep1.push_move(knight_mv4, &bds[4]);
        rep1.push_move(bishop_mv1, &bds[5]);
        rep1.push_move(bishop_mv2, &bds[6]);
        assert_eq!(rep1.count_old(&bds[4].make_move(knight_mv4)).in_total(), 1);
        assert_eq!(rep1.count_old(&bds[2].make_move(knight_mv)).in_total(), 2); // bd0 and bd2
        rep1.push_move(knight_mv, &bds[6]);
        rep1.push_move(knight_mv, &bds[4]);
        assert_eq!(rep1.count_old(&bds[2].make_move(knight_mv)).in_total(), 3);
        Ok(())
    }

    #[test]
    fn test_rep_position() {
        let mut b = Catalog::starting_board();
        let mut repetition = Repetition::default();
        let s = concat!(
            "e2e4 b8c6 b1c3 g8f6 d2d4 d7d5 e4e5 f6e4 c3e4 d5e4 f1b5 c8d7 b5c6 d7c6 g1e2 e7e6 e1g1 ",
            "f8b4 a2a3 b4a5 c1e3 e8g8 c2c4 f7f6 b2b4 a5b6 c4c5 f6e5 c5b6 e5d4 e2d4 a7b6 d4e6 d8d1 ",
            "f1d1 f8f7 d1d8 a8d8 e6d8 f7f6 a1d1 f6f8 d8e6 f8c8 e3f4 c8a8 f4c7 a8a3 c7b6 a3d3 d1a1 ",
            "h7h6 b6c7 d3d2 c7f4 d2b2 a1a8 g8h7 e6f8 h7g8"
        );
        repetition.new_game();
        let mvs = b.parse_uci_variation(s).unwrap();
        for mv in mvs.moves() {
            repetition.push_move(mv, &b);
            b = b.make_move(mv);
            println!(
                "rep count = {} hash = {:x}",
                repetition.count_old(&b).in_played,
                b.hash()
            );
        }
    }
}
