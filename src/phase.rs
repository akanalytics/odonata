use crate::board::Board;
use crate::catalog::Catalog;
// use crate::types::{Color, Piece};
use once_cell::sync::Lazy;
use std::cmp;

// #[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]

pub struct Phase;

static ALL_PIECES: Lazy<i32> = Lazy::new(|| {
    2 * Catalog::starting_position()
        .material()
        .white()
        .minors_and_majors()
        .centipawns()
});

impl Board {
    // phase = % endgame, 0 is start, 100 is end game with just pawns
    pub fn phase(&self) -> i32 {
        let m = self.material();

        let cp = cmp::min(
            *ALL_PIECES,
            m.white().minors_and_majors().centipawns() - m.black().minors_and_majors().centipawns(),
        );

        // let mut cp = 0;
        // for &p in &[Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        //     cp += m.counts(Color::White, p) * p.centipawns();
        //     cp += m.counts(Color::Black, p) * p.centipawns();
        // }
        // let cp = cmp::min(*ALL_PIECES, cp);  // unlikely but can promote without losing a piece

        // KINGS <= cp <= ALL_PIECES
        let percentage = cp * 100 / *ALL_PIECES;
        100 - percentage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;

    #[test]
    fn test_phase() -> Result<(), String> {
        assert_eq!(*ALL_PIECES, 6500);
        assert_eq!(Catalog::starting_position().phase(), 0);
        assert_eq!(Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 45 100")?.phase(), 100);
        assert_eq!(
            Board::parse_fen("7k/pppppppp/8/8/8/8/PPPPPPPP/7K b - - 45 100")?.phase(),
            100
        );
        assert_eq!(Board::parse_fen("6qk/8/8/8/8/8/8/6QK b - - 45 100")?.phase(), 73);
        assert_eq!(
            Board::parse_fen("4rrqk/8/8/8/8/8/8/4RRQK b - - 45 100")?.phase(),
            42
        );
        assert_eq!(
            Board::parse_fen("rrbbnnqk/8/8/8/8/8/8/BBNNRRQK b - - 45 100")?.phase(),
            0
        );
        Ok(())
    }
}
