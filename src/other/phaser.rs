use crate::boards::Board;
use crate::domain::Material;
use crate::infra::component::Component;
use crate::Piece;

use std::cmp;
use std::fmt;
// use static_init::{dynamic};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Default)]
pub struct Phase(pub i32);

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Phaser {
    enabled: bool,
    method: String,
    half_way_score: i32,
}

impl Default for Phaser {
    fn default() -> Self {
        Self {
            enabled: true,
            method: "SO".to_string(),
            half_way_score: Self::ALL_PIECES / 2,
        }
    }
}

impl fmt::Display for Phaser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}

impl Component for Phaser {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl Phaser {
    const ALL_PIECES: i32 = 6400;

    pub fn phase(&self, mat: &Material) -> Phase {
        match self.method.as_str() {
            "CPW" => self.cpw_method(mat),
            "CO" => self.complex_method(mat),
            "SO" => self.simple_method(mat),
            _ => panic!("Unexpected phase method type {}", self.method),
        }
    }

    fn simple_method(&self, mat: &Material) -> Phase {
        let cp = cmp::min(
            Self::ALL_PIECES,
            mat.white().minors_and_majors().centipawns()
                - mat.black().minors_and_majors().centipawns(),
        );

        let percentage = cp * 100 / Self::ALL_PIECES;
        Phase(100 - percentage)
    }

    fn complex_method(&self, mat: &Material) -> Phase {
        let cp = cmp::min(
            Self::ALL_PIECES,
            mat.white().minors_and_majors().centipawns()
                - mat.black().minors_and_majors().centipawns(),
        );
        let percentage = if cp > self.half_way_score {
            // interpolate between 50 and 100
            50 + (cp - self.half_way_score) * 50 / (Self::ALL_PIECES - self.half_way_score)
        } else {
            cp * 50 / self.half_way_score
        };

        Phase(100 - percentage)
    }

    fn cpw_method(&self, mat: &Material) -> Phase {
        const PAWN_PHASE: i32 = 0;
        const KNIGHT_PHASE: i32 = 1;
        const BISHOP_PHASE: i32 = 1;
        const ROOK_PHASE: i32 = 2;
        const QUEEN_PHASE: i32 = 4;

        const TOTAL_PHASE: i32 = PAWN_PHASE * 16
            + KNIGHT_PHASE * 4
            + BISHOP_PHASE * 4
            + ROOK_PHASE * 4
            + QUEEN_PHASE * 2;

        let mut phase = TOTAL_PHASE;

        phase -= mat.count_piece(Piece::Pawn) * PAWN_PHASE;
        phase -= mat.count_piece(Piece::Knight) * KNIGHT_PHASE;
        phase -= mat.count_piece(Piece::Bishop) * BISHOP_PHASE;
        phase -= mat.count_piece(Piece::Rook) * ROOK_PHASE;
        phase -= mat.count_piece(Piece::Queen) * QUEEN_PHASE;

        if phase < 0 {
            phase = 0;
        }

        debug_assert!((0..=TOTAL_PHASE).contains(&phase));

        // convert to 0..100 range
        phase = (phase * 100 + (TOTAL_PHASE / 2)) / TOTAL_PHASE;
        Phase(phase)
    }
}

impl Material {
    // phase = % endgame, 0 is start, 100 is end game with just pawns
    #[inline]
    pub fn phase(&self, phaser: &Phaser) -> Phase {
        phaser.phase(self)
    }
}

impl Board {
    // phase = % endgame, 0 is start, 100 is end game with just pawns
    #[inline]
    pub fn phase(&self, phaser: &Phaser) -> Phase {
        self.material().phase(phaser)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use anyhow::Result;

    #[test]
    fn test_phase() -> Result<()> {
        let mut phaser = Phaser::default();
        assert_eq!(Phaser::ALL_PIECES, 6400);
        assert_eq!(Catalog::starting_board().phase(&phaser), Phase(0));
        let board100 = Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 45 100")?;
        let board100_too = Board::parse_fen("7k/pppppppp/8/8/8/8/PPPPPPPP/7K b - - 45 100")?;
        let board73 = Board::parse_fen("6qk/8/8/8/8/8/8/6QK b - - 45 100")?;
        let board42 = Board::parse_fen("4rrqk/8/8/8/8/8/8/4RRQK b - - 45 100")?;
        let board0 = Board::parse_fen("rrbbnnqk/8/8/8/8/8/8/BBNNRRQK b - - 45 100")?;

        // odonata's simple method
        assert_eq!(board100.phase(&phaser), Phase(100));
        assert_eq!(board100_too.phase(&phaser), Phase(100));
        assert_eq!(board73.phase(&phaser), Phase(72));
        assert_eq!(board42.phase(&phaser), Phase(41));
        assert_eq!(board0.phase(&phaser), Phase(0));

        // cpw's method
        assert_eq!(phaser.cpw_method(&board100.material()), Phase(100));
        assert_eq!(phaser.cpw_method(&board100_too.material()), Phase(100));
        assert_eq!(phaser.cpw_method(&board73.material()), Phase(67));
        assert_eq!(phaser.cpw_method(&board42.material()), Phase(33));
        assert_eq!(phaser.cpw_method(&board0.material()), Phase(0));

        // complex method
        assert_eq!(phaser.complex_method(&board100.material()), Phase(100));
        assert_eq!(phaser.complex_method(&board100_too.material()), Phase(100));
        assert_eq!(phaser.complex_method(&board73.material()), Phase(72));
        assert_eq!(phaser.complex_method(&board42.material()), Phase(41));
        assert_eq!(phaser.complex_method(&board0.material()), Phase(0));

        // complex method
        phaser.half_way_score = 6500 / 4; // rapid decline at first
        assert_eq!(phaser.complex_method(&board100.material()), Phase(100));
        assert_eq!(phaser.complex_method(&board100_too.material()), Phase(100));
        assert_eq!(phaser.complex_method(&board73.material()), Phase(49)); // 73 -> 49
        assert_eq!(phaser.complex_method(&board42.material()), Phase(28)); // 42 -> 28
        assert_eq!(phaser.complex_method(&board0.material()), Phase(0));

        Ok(())
    }
}
