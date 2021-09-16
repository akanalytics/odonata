use crate::board::Board;
use crate::config::{Component, Config};
use crate::material::Material;
use crate::Piece;

use std::cmp;
use std::fmt;
// use static_init::{dynamic};

#[derive(Clone, Debug)]
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
            half_way_score: 6500 / 2,
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
    fn settings(&self, c: &mut Config) {
        c.set("phaser.enabled", &format!("type check default {}", self.enabled));
        c.set("phaser.method", &format!("type string default {}", self.method));
        c.set(
            "phaser.half.way.score",
            &format!("type spin min 0 max 12000 default {}", self.half_way_score),
        );
    }

    fn configure(&mut self, c: &Config) {
        debug!("phaser.configure");
        self.enabled = c.bool("phaser.enabled").unwrap_or(self.enabled);
        self.method = c.string("phaser.method").unwrap_or(self.method.clone());
        self.half_way_score = c.int("phaser.half.way.score").unwrap_or(self.half_way_score as i64).clamp(1,6400) as i32;
    }

    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl Phaser {
    const ALL_PIECES: i32 = 6500;

    pub fn phase(&self, mat: &Material) -> i32 {
        match self.method.as_str() {
            "CPW" => self.cpw_method(mat),
            "CO" => self.complex_method(mat),
            "SO" | _ => self.simple_method(mat),
        }
    }

    fn simple_method(&self, mat: &Material) -> i32 {
        let cp = cmp::min(
            Self::ALL_PIECES,
            mat.white().minors_and_majors().centipawns() - mat.black().minors_and_majors().centipawns(),
        );

        let percentage = cp * 100 / Self::ALL_PIECES;
        100 - percentage
    }

    fn complex_method(&self, mat: &Material) -> i32 {
        let cp = cmp::min(
            Self::ALL_PIECES,
            mat.white().minors_and_majors().centipawns() - mat.black().minors_and_majors().centipawns(),
        );
        let percentage = if cp > self.half_way_score {
            // interpolate between 50 and 100
            50 + (cp - self.half_way_score) * 50 / (Self::ALL_PIECES - self.half_way_score)
        } else {
            cp * 50 / self.half_way_score
        };

        100 - percentage
    }

    fn cpw_method(&self, mat: &Material) -> i32 {
        const PAWN_PHASE: i32 = 0;
        const KNIGHT_PHASE: i32 = 1;
        const BISHOP_PHASE: i32 = 1;
        const ROOK_PHASE: i32 = 2;
        const QUEEN_PHASE: i32 = 4;

        const TOTAL_PHASE: i32 =
            PAWN_PHASE * 16 + KNIGHT_PHASE * 4 + BISHOP_PHASE * 4 + ROOK_PHASE * 4 + QUEEN_PHASE * 2;

        let mut phase = TOTAL_PHASE;

        phase -= mat.counts_piece(Piece::Pawn) * PAWN_PHASE;
        phase -= mat.counts_piece(Piece::Knight) * KNIGHT_PHASE;
        phase -= mat.counts_piece(Piece::Bishop) * BISHOP_PHASE;
        phase -= mat.counts_piece(Piece::Rook) * ROOK_PHASE;
        phase -= mat.counts_piece(Piece::Queen) * QUEEN_PHASE;

        if phase < 0 {
            phase = 0;
        }

        debug_assert!(phase >= 0 && phase <= TOTAL_PHASE);

        // convert to 0..100 range
        phase = (phase * 100 + (TOTAL_PHASE / 2)) / TOTAL_PHASE;
        phase
    }
}

impl Material {
    // phase = % endgame, 0 is start, 100 is end game with just pawns
    #[inline]
    pub fn phase(&self, phaser: &Phaser) -> i32 {
        phaser.phase(self)
    }
}

impl Board {
    // phase = % endgame, 0 is start, 100 is end game with just pawns
    #[inline]
    pub fn phase(&self, phaser: &Phaser) -> i32 {
        self.material().phase(phaser)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;

    #[test]
    fn test_phase() -> Result<(), String> {
        let mut phaser = Phaser::default();
        assert_eq!(Phaser::ALL_PIECES, 6500);
        assert_eq!(Catalog::starting_board().phase(&phaser), 0);
        let board100 = Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 45 100")?;
        let board100_too = Board::parse_fen("7k/pppppppp/8/8/8/8/PPPPPPPP/7K b - - 45 100")?;
        let board73 = Board::parse_fen("6qk/8/8/8/8/8/8/6QK b - - 45 100")?;
        let board42 = Board::parse_fen("4rrqk/8/8/8/8/8/8/4RRQK b - - 45 100")?;
        let board0 = Board::parse_fen("rrbbnnqk/8/8/8/8/8/8/BBNNRRQK b - - 45 100")?;

        // odonata's simple method
        assert_eq!(board100.phase(&phaser), 100);
        assert_eq!(board100_too.phase(&phaser), 100);
        assert_eq!(board73.phase(&phaser), 73);
        assert_eq!(board42.phase(&phaser), 42);
        assert_eq!(board0.phase(&phaser), 0);

        // cpw's method
        assert_eq!(phaser.cpw_method(&board100.material()), 100);
        assert_eq!(phaser.cpw_method(&board100_too.material()), 100);
        assert_eq!(phaser.cpw_method(&board73.material()), 67);
        assert_eq!(phaser.cpw_method(&board42.material()), 33);
        assert_eq!(phaser.cpw_method(&board0.material()), 0);

        // complex method method
        assert_eq!(phaser.complex_method(&board100.material()), 100);
        assert_eq!(phaser.complex_method(&board100_too.material()), 100);
        assert_eq!(phaser.complex_method(&board73.material()), 73);
        assert_eq!(phaser.complex_method(&board42.material()), 42);
        assert_eq!(phaser.complex_method(&board0.material()), 0);

        // complex method method
        phaser.half_way_score = 6500/4; // rapoid decline at first
        assert_eq!(phaser.complex_method(&board100.material()), 100);
        assert_eq!(phaser.complex_method(&board100_too.material()), 100);
        assert_eq!(phaser.complex_method(&board73.material()), 49);  // 73 -> 49
        assert_eq!(phaser.complex_method(&board42.material()), 28);  // 42 -> 28
        assert_eq!(phaser.complex_method(&board0.material()), 0);

        Ok(())
    }
}
