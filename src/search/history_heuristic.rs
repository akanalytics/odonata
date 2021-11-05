use crate::bitboard::bitboard::Bitboard;
use crate::board::Board;
use crate::infra::parsed_config::{Component};
use crate::mv::Move;
use crate::types::{Ply, Piece, Color};
use std::fmt;
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum HistoryMethod { PlyPower, PlySquared, Zero }


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum HistoryBoard { FromTo, PieceTo, PieceFromTo }


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryHeuristic {
    enabled: bool,
    scale: i32,
    age_factor: i32,
    malus_factor: i32,
    alpha_method: HistoryMethod,
    beta_method: HistoryMethod,
    duff_method: HistoryMethod,
    board: HistoryBoard, 

    #[serde(skip)]
    history: Box<[[[[i32; 64]; 64];Piece::len()];2]>,
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
            malus_factor: 10,
            alpha_method: HistoryMethod::PlySquared,
            beta_method: HistoryMethod::PlySquared,
            duff_method: HistoryMethod::PlySquared,            
            board: HistoryBoard::PieceTo,
            history: Box::new([[[[0; 64]; 64]; Piece::len()];2]),
        }
    }
}

impl fmt::Display for HistoryHeuristic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "age.factor       : {}", self.age_factor)?;
        writeln!(f, "scale            : {}", self.scale)?;
        // writeln!(f, "clear.every.move : {}", self.clear_every_move)?;
        // for c in Color::ALL {
        //     for p in Piece::ALL_BAR_NONE {
        //         writeln!(f, "{} {}", c, p.name())?;
        //         for sq in Bitboard::all().flip_vertical().squares() {
        //             write!(f, "{:>7}", self.history[c][p][sq])?;
        //             if sq.file_index() == 7 {
        //                 writeln!(f)?;
        //             }
        //         }
        //     }
        // }
        Ok(())
    }
}

impl HistoryHeuristic {
    pub fn adjust_by_factor(&mut self, age_factor: i32) {
        for c in Color::ALL {
            for p in Piece::ALL_BAR_NONE {
                for fr in Bitboard::all().squares() {
                    for to in Bitboard::all().squares() {
                        self.history[c][p][fr][to] *= age_factor / 1024;
                    }
                }
            }
        }
    }

    #[inline]
    pub fn history_heuristic_bonus(&self, c: Color, mv: &Move) -> i32 {
        if !self.enabled {
            return 0;
        }
        use HistoryBoard::*;
        match self.board {
            PieceTo => self.history[c][mv.mover_piece()][0][mv.to()],
            FromTo => self.history[c][0][mv.from()][mv.to()],
            PieceFromTo => self.history[c][mv.mover_piece()][mv.from()][mv.to()],
        }
    }

    #[inline]
    pub fn get_mut(&mut self, c: Color, mv: &Move) -> &mut i32 {
        if !self.enabled {
            return &mut self.history[c][0][0][0];
        }
        use HistoryBoard::*;
        match self.board {
            PieceTo => &mut self.history[c][mv.mover_piece()][0][mv.to()],
            FromTo => &mut self.history[c][0][mv.from()][mv.to()],
            PieceFromTo => &mut self.history[c][mv.mover_piece()][mv.from()][mv.to()],
        }
    }


    #[inline]
    pub fn raised_alpha(&mut self, y: Ply, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() {
            return;
        }
        use HistoryMethod::*;
        *self.get_mut(b.color_us(), mv) += match self.alpha_method {
            PlyPower =>  2 << (y / 4),
            PlySquared =>  y * y,
            Zero =>  0,
        };
    }

    #[inline]
    pub fn beta_cutoff(&mut self, y: Ply, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() {
            return;
        }
        use HistoryMethod::*;
        *self.get_mut(b.color_us(), mv) += match self.alpha_method {
            PlyPower =>  2 << (y / 4),
            PlySquared =>  y * y,
            Zero =>  0,
        };
    }

    #[inline]
    pub fn duff(&mut self, y: Ply, b: &Board, mv: &Move) {
        if !self.enabled || mv.is_capture() {
            return;
        }
        use HistoryMethod::*;
        *self.get_mut(b.color_us(), mv) -= match self.alpha_method {
            PlyPower =>  2 << (y / 4) / self.malus_factor,
            PlySquared =>  y * y  / self.malus_factor,
            Zero =>  0,
        };
    }
}


#[cfg(test)]
mod tests {
    use test_env_log::test;
    use crate::bitboard::square::Square;

    use super::*;

    #[test]
    fn hh_serde_test() {
        let hh = HistoryHeuristic::default();
        let text = toml::to_string(&hh).unwrap();
        info!("toml\n{}", text);
        let hh2: HistoryHeuristic = toml::from_str(&text).unwrap();
        info!("from toml\n{}", hh2);
    }

    #[test]
    fn hh_test() {
        let mut hh = HistoryHeuristic::default();
        hh.get_mut(Color::White, &Move::new_quiet(Piece::Pawn, Square::A1, Square::H8));
        *hh.get_mut(Color::White, &Move::new_quiet(Piece::Pawn, Square::A1, Square::H8)) = 1;
        hh.new_position();
        hh.new_game();
    }


}