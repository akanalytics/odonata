use crate::bitboard::bitboard::Bitboard;
use crate::board::Board;
use crate::infra::parsed_config::{ParsedConfig, Component};
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
    fn settings(&self, c: &mut ParsedConfig) {
        c.set("history.enabled", &format!("type check default {}", self.enabled));
        c.set("history.scale", &format!("type spin min -1000 max 1000 default {}", self.scale));
        c.set("history.age.factor", &format!("type spin min 0 max 1000 default {}", self.age_factor));
        // c.set("history.clear.every.move", &format!("type check default {}", self.clear_every_move));
    }
    fn configure(&mut self, c: &ParsedConfig) {
        debug!("history.configure");
        // self.clear_every_move = c.bool("killers.clear.every.move").unwrap_or(self.clear_every_move);
        self.enabled = c.bool("history.enabled").unwrap_or(self.enabled);
        self.scale = c.int("history.scale").unwrap_or(self.scale as i64) as i32;
        self.age_factor = c.int("history.age.factor").unwrap_or(self.age_factor as i64) as i32;
    }
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
