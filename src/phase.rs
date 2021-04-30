use crate::board::Board;
use crate::catalog::Catalog;
use once_cell::sync::Lazy;
use std::cmp;

// #[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]


pub struct Phase;


static ALL_PIECES: Lazy<i32> = Lazy::new(|| 2 * Catalog::starting_position().material().white().minors_and_majors().centipawns());

impl Board {
    
    // pahse = % endgame, 0 is start, 100 is end game with just pawns
    pub fn phase(&self) -> i32 {
        let material = self.material();
        let cp = cmp::min(*ALL_PIECES, material.white().minors_and_majors().centipawns() -  material.black().minors_and_majors().centipawns());

        // KINGS <= cp <= ALL_PIECES
        let percentage = cp * 100 / *ALL_PIECES;
        100-percentage
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::board::boardbuf::BoardBuf;

    #[test]
    fn test_phase() -> Result<(), String> {
        assert_eq!( *ALL_PIECES, 6500);
        assert_eq!( Catalog::starting_position().phase(), 0);
        assert_eq!( Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 45 100")?.phase(), 100);
        assert_eq!( Board::parse_fen("7k/pppppppp/8/8/8/8/PPPPPPPP/7K b - - 45 100")?.phase(), 100);
        assert_eq!( Board::parse_fen("6qk/8/8/8/8/8/8/6QK b - - 45 100")?.phase(), 73);
        assert_eq!( Board::parse_fen("4rrqk/8/8/8/8/8/8/4RRQK b - - 45 100")?.phase(), 42);
        assert_eq!( Board::parse_fen("rrbbnnqk/8/8/8/8/8/8/BBNNRRQK b - - 45 100")?.phase(), 0);
        Ok(())
    }
}