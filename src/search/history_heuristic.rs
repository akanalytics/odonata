use crate::bitboard::bitboard::Bitboard;
use crate::board::Board;
use crate::infra::parsed_config::{Component};
use crate::mv::Move;
use crate::types::{Ply, Piece, Color};
use std::fmt;
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryHeuristic {
    enabled: bool,
    scale: i32,
    age_factor: i32,

    #[serde(skip)]
    history: [[[i32; 64];Piece::len()];2],
    // clear_every_move: bool,
}

impl Component for HistoryHeuristic {
    fn new_game(&mut self) {
        self.adjust_by_factor(0);
    }

    fn new_position(&mut self) {
        self.adjust_by_factor(self.age_factor);
    }

}




impl Default for HistoryHeuristic {
    fn default() -> Self {
        HistoryHeuristic {
            enabled: true,
            // clear_every_move: true,
            scale: 1,
            age_factor: 4,
            history: [[[0; 64]; Piece::len()];2],
        }
    }
}

impl fmt::Display for HistoryHeuristic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "age.factor       : {}", self.age_factor)?;
        writeln!(f, "scale            : {}", self.scale)?;
        // writeln!(f, "clear.every.move : {}", self.clear_every_move)?;
        for c in Color::ALL {
            for p in Piece::ALL_BAR_NONE {
                writeln!(f, "{} {}", c, p.name())?;
                for sq in Bitboard::all().flip_vertical().squares() {
                    write!(f, "{:>7}", self.history[c][p][sq])?;
                    if sq.file_index() == 7 {
                        writeln!(f)?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl HistoryHeuristic {
    pub fn adjust_by_factor(&mut self, age_factor: i32) {
        for c in Color::ALL {
            for p in Piece::ALL_BAR_NONE {
                for sq in Bitboard::all().squares() {
                    self.history[c][p][sq] *= age_factor / 1024;
                }
            }
        }
    }

    pub fn history_heuristic_bonus(&self, c: Color, mv: &Move) -> i32 {
        if !self.enabled {
            return 0;
        }
        self.history[c][mv.mover_piece()][mv.to()]
    }

    pub fn raised_alpha(&mut self, y: Ply, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() {
            return;
        }
        self.history[b.color_us()][mv.mover_piece()][mv.to()] += y * y;
    }

    pub fn beta_cutoff(&mut self, y: Ply, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() {
            return;
        }
        self.history[b.color_us()][mv.mover_piece()][mv.to()] += 2 << (y / 4); // plus alpha above
    }
}
