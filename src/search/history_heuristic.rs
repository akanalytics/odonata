use crate::bitboard::bitboard::Bitboard;
use crate::board::Board;
use crate::config::{Config, Component};
use crate::mv::Move;
use crate::types::{Ply, Piece, Color};
use std::fmt;

#[derive(Clone, Debug)]
pub struct HistoryHeuristic {
    enabled: bool,
    clear_every_move: bool,
    scale: i32,
    history: [[[i32; 64];Piece::len()];2],
}

impl Component for HistoryHeuristic {
    fn settings(&self, c: &mut Config) {
        c.set("history.enabled", &format!("type check default {}", self.enabled));
        c.set("history.scale", &format!("type spin min -1000 max 1000 default {}", self.scale));
        c.set("history.clear.every.move", &format!("type check default {}", self.clear_every_move));
    }
    fn configure(&mut self, c: &Config) {
        debug!("history.configure");
        self.clear_every_move = c.bool("killers.clear.every.move").unwrap_or(self.clear_every_move);
        self.enabled = c.bool("history.enabled").unwrap_or(self.enabled);
        self.scale = c.int("history.scale").unwrap_or(self.scale as i64) as i32;
    }
    fn new_game(&mut self) {
        self.clear();
    }

    fn new_position(&mut self) {
        if self.clear_every_move {
            self.clear();
        }
    }

}




impl Default for HistoryHeuristic {
    fn default() -> Self {
        HistoryHeuristic {
            enabled: true,
            clear_every_move: true,
            scale: 10,
            history: [[[0; 64]; Piece::len()];2],
        }
    }
}

impl fmt::Display for HistoryHeuristic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "clear.every.move : {}", self.clear_every_move)?;
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
    pub fn clear(&mut self) {
        for c in Color::ALL {
            for p in Piece::ALL_BAR_NONE {
                for sq in Bitboard::all().squares() {
                    self.history[c][p][sq] = 0;
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
        self.history[b.color_us()][mv.mover_piece()][mv.to()] += y * y; // plus alpha above
    }
}
