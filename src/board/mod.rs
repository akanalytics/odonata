use crate::bitboard::Bitboard;
use crate::board::boardbuf::BoardBuf;
use crate::hasher::Hasher;
use crate::types::{CastlingRights, Color, Piece};
use std::fmt::{self, Write};
use std::iter::*;

pub mod boardbuf;
pub mod makemove;
pub mod movegen;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Board {
    pieces: [Bitboard; Piece::ALL.len()],
    colors: [Bitboard; 2],
    castling: CastlingRights,
    en_passant: Bitboard,
    turn: Color,
    fifty_clock: u16,
    fullmove_number: u16,
    hash: u64,
    // interior mutability (precludes copy trait)
    // moves: MoveList,
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Board").field("fen", &self.to_fen()).finish()
    }
}

impl Board {
    /// white to move, no castling rights or en passant
    #[inline]
    pub fn new_empty() -> Board {
        Default::default()
    }

    fn calculate_internals(&mut self) {
        self.hash = Hasher::default().hash_board(self);
    }


    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn castling(&self) -> CastlingRights {
        self.castling
    }

    #[inline]
    pub fn pieces(&self, p: Piece) -> Bitboard {
        self.pieces[p as usize]
    }

    #[inline]
    pub fn pawns(&self) -> Bitboard {
        self.pieces(Piece::Pawn)
    }

    #[inline]
    pub fn knights(&self) -> Bitboard {
        self.pieces(Piece::Knight)
    }

    #[inline]
    pub fn bishops(&self) -> Bitboard {
        self.pieces(Piece::Bishop)
    }

    #[inline]
    pub fn rooks(&self) -> Bitboard {
        self.pieces(Piece::Rook)
    }

    #[inline]
    pub fn queens(&self) -> Bitboard {
        self.pieces(Piece::Queen)
    }

    #[inline]
    pub fn kings(&self) -> Bitboard {
        self.pieces(Piece::King)
    }

    #[inline]
    pub fn color(&self, c: Color) -> Bitboard {
        self.colors[c]
    }

    #[inline]
    pub fn white(&self) -> Bitboard {
        self.colors[Color::White]
    }

    #[inline]
    pub fn black(&self) -> Bitboard {
        self.colors[Color::Black]
    }

    #[inline]
    pub fn color_us(&self) -> Color {
        self.turn
    }

    #[inline]
    pub fn color_them(&self) -> Color {
        self.turn.opposite()
    }

    #[inline]
    pub fn them(&self) -> Bitboard {
        self.color(self.turn.opposite())
    }

    #[inline]
    pub fn us(&self) -> Bitboard {
        self.color(self.turn)
    }

    #[inline]
    pub fn en_passant(&self) -> Bitboard {
        self.en_passant
    }

    #[inline]
    pub fn fifty_halfmove_clock(&self) -> i32 {
        self.fifty_clock.into()
    }

    #[inline]
    pub fn fullmove_number(&self) -> i32 {
        self.fullmove_number as i32
    }

    #[inline]
    pub fn ply(&self) -> i32 {
        self.fullmove_number() + self.color_us().chooser_wb(0, 1) as i32
    }

    #[inline]
    pub fn piece_at(&self, sq: Bitboard) -> Piece {
        for &p in &Piece::ALL {
            if self.pieces(p).contains(sq) {
                return p;
            }
        }
        Piece::None
    }

    pub fn to_fen(&self) -> String {
        let b = self.clone();

        let mut fen = Bitboard::RANKS.iter().rev().map(|&r| b.get(r)).collect::<Vec<String>>().join("/");

        // replace continguous empties by a count
        for i in (1..=8).rev() {
            fen = fen.replace(".".repeat(i).as_str(), i.to_string().as_str());
        }
        format!(
            "{fen} {turn} {castle} {ep} {fifty} {count}",
            fen = fen,
            turn = self.color_us(),
            castle = self.castling(),
            ep = if self.en_passant().is_empty() { "-".to_string() } else { self.en_passant().uci() },
            fifty = self.fifty_halfmove_clock(),
            count = self.fullmove_number()
        )
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.clone();
        for &r in Bitboard::RANKS.iter().rev() {
            f.write_str(&b.get(r))?;
            f.write_char('\n')?;
        }
        write!(f, "\nfen: {} \n", self.to_fen())?;
        // write!(fmt, "Moves: {}", self.moves)?;
        if f.alternate() {
            writeln!(f, "Hash: {:x}", self.hash())?;
            writeln!(f, "White:\n{}\nBlack:\n{}\n", self.white(), self.black())?;
            for &p in Piece::ALL.iter() {
                writeln!(
                    f,
                    "Pieces: {}{}\n{}\n",
                    p.to_upper_char(),
                    p.to_lower_char(),
                    self.pieces(p)
                )?;
            }
        }

        Ok(())
    }
}

impl Default for Board {
    #[inline]
    fn default() -> Self {
        let mut b = Board {
            pieces: Default::default(),
            colors: Default::default(),
            castling: Default::default(),
            en_passant: Default::default(),
            turn: Default::default(),
            fifty_clock: Default::default(),
            fullmove_number: 1,
            hash: 0, 
            // moves: MoveList,
        };
        b.hash = Hasher::default().hash_board(&b);
        b
    }
}

#[cfg(test)]
mod tests {
    use super::boardbuf::*;
    use super::*;
    use crate::catalog::*;
    use crate::globals::constants::*;

    #[test]
    fn to_fen() {
        for &fen in &[
            "7k/8/8/8/8/8/8/7K b KQkq - 45 100",
            Catalog::STARTING_POSITION_FEN,
            "8/8/8/8/8/8/8/B7 w - - 0 0",
        ] {
            let b = Board::parse_fen(fen).unwrap().as_board();
            assert_eq!(fen, b.to_fen());
            println!("{:#}", b);
        }
    }

    #[test]
    fn board_bitboards() -> Result<(), String> {
        let board =
            Board::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap().as_board();
        assert_eq!(board.color_us(), Color::White);
        assert_eq!(board.color_them(), Color::Black);
        // assert_eq!(board.en_passant(), Bitboard::empty());
        // assert_eq!(board.move_count(), 0);
        assert_eq!(board.pawns() & board.us(), Bitboard::RANK_2);
        assert_eq!(board.rooks() & board.them(), a8 | h8);
        assert_eq!(board.bishops() & board.us(), c1 | f1);
        assert_eq!(board.them(), Bitboard::RANK_7 | Bitboard::RANK_8);
        Ok(())
    }
}
