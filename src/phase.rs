use crate::board::Board;
use crate::board::boardbuf::BoardBuf;
use crate::catalog::Catalog;
use crate::material::Material;
use std::cmp;
use std::fmt;
use once_cell::sync::Lazy;

// #[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]


pub struct Phase;


static ALL_PIECES: Lazy<i32> = Lazy::new(|| 2 * Catalog::starting_position().material().white().non_pawn().centipawns());
static KINGS: Lazy<i32> = Lazy::new(|| 2 * Material::from_piece_str("K").unwrap().centipawns());

impl Board {
    
    // pahse = % endgame, 0 is start, 100 is end game with just pawns
    pub fn phase(&self) -> i32 {
        let material = self.material();
        let cp = cmp::min(*ALL_PIECES, material.white().non_pawn().centipawns() -  material.black().non_pawn().centipawns());

        // KINGS <= cp <= ALL_PIECES
        let percentage = (cp - *KINGS) * 100 / (*ALL_PIECES - *KINGS);
        100-percentage
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::makemove::MoveMaker;
    use crate::board::movegen::MoveGen;
    use crate::catalog::Catalog;

    #[test]
    fn test_phase() -> Result<(), String> {
        assert_eq!( *ALL_PIECES, 26500);
        assert_eq!( *KINGS, 20000);
        assert_eq!( Catalog::starting_position().phase(), 0);
        assert_eq!( Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 45 100")?.phase(), 100);
        assert_eq!( Board::parse_fen("7k/pppppppp/8/8/8/8/PPPPPPPP/7K b - - 45 100")?.phase(), 100);
        assert_eq!( Board::parse_fen("6qk/8/8/8/8/8/8/6QK b - - 45 100")?.phase(), 73);
        assert_eq!( Board::parse_fen("4rrqk/8/8/8/8/8/8/4RRQK b - - 45 100")?.phase(), 42);
        assert_eq!( Board::parse_fen("rrbbnnqk/8/8/8/8/8/8/BBNNRRQK b - - 45 100")?.phase(), 0);
        Ok(())
    }
}