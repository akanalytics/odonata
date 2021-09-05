use crate::board::Board;
use crate::config::{Component, Config};
use crate::material::Material;

use std::cmp;
use std::fmt;
// use static_init::{dynamic};

#[derive(Clone, Debug)]
pub struct Phaser {
    enabled: bool,
}

impl Default for Phaser {
    fn default() -> Self {
        Self { enabled: true }
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
    }

    fn configure(&mut self, c: &Config) {
        debug!("phaser.configure");
        self.enabled = c.bool("phaser.enabled").unwrap_or(self.enabled);
    }

    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl Phaser {
    const ALL_PIECES: i32 = 6500;

    pub fn phase(&self, mat: &Material) -> i32 {
        let cp = cmp::min(
            Self::ALL_PIECES,
            mat.white().minors_and_majors().centipawns() - mat.black().minors_and_majors().centipawns(),
        );
        
        let percentage = cp * 100 / Self::ALL_PIECES;
        100 - percentage
    }
}



impl Material {
    // phase = % endgame, 0 is start, 100 is end game with just pawns
    pub fn phase(&self, phaser: &Phaser) -> i32 {
        phaser.phase(self)
    }
}

impl Board {
    // phase = % endgame, 0 is start, 100 is end game with just pawns
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
        let phaser = Phaser::default();
        assert_eq!(Phaser::ALL_PIECES, 6500);
        assert_eq!(Catalog::starting_board().phase(&phaser), 0);
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 45 100")?.phase(&phaser),
            100
        );
        assert_eq!(
            Board::parse_fen("7k/pppppppp/8/8/8/8/PPPPPPPP/7K b - - 45 100")?.phase(&phaser),
            100
        );
        assert_eq!(
            Board::parse_fen("6qk/8/8/8/8/8/8/6QK b - - 45 100")?.phase(&phaser),
            73
        );
        assert_eq!(
            Board::parse_fen("4rrqk/8/8/8/8/8/8/4RRQK b - - 45 100")?.phase(&phaser),
            42
        );
        assert_eq!(
            Board::parse_fen("rrbbnnqk/8/8/8/8/8/8/BBNNRRQK b - - 45 100")?.phase(&phaser),
            0
        );
        Ok(())
    }
}
