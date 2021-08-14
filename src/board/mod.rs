use crate::bitboard::bitboard::Bitboard;
use crate::material::Material;
use crate::board::boardbuf::BoardBuf;
use crate::hasher::Hasher;
use std::cell::Cell;
use crate::types::{Color, Piece, Hash, Ply, Repeats};
use crate::bitboard::castling::CastlingRights;
use std::fmt::{self, Write};
use std::iter::*;
use std::str::FromStr;
use serde::{Serialize, Serializer};
use serde_with::{DeserializeFromStr};

pub mod boardbuf;
pub mod makemove;
pub mod movegen;
pub mod boardcalcs;
pub mod rules;



#[derive(Clone, PartialEq, Eq, DeserializeFromStr)]
pub struct Board {
    pieces: [Bitboard; Piece::len()],
    colors: [Bitboard; Color::len()],
    castling: CastlingRights,
    en_passant: Bitboard,
    turn: Color,
    fifty_clock: u16,
    fullmove_number: u16,
    repetition_count: Cell<Repeats>,
    hash: Hash,
    threats_to: [Cell<Bitboard>; Color::len()],
    checkers_of: [Cell<Bitboard>; Color::len()],
    pinned: Cell<Bitboard>,
    // material: Cell<Material>,
    // interior mutability (precludes copy trait)
    // moves: MoveList,
}



impl Serialize for Board {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_fen())
    }
}

// impl<'de> Deserialize<'de> for Board {
//     fn deserialize<D>(deserializer: D) -> Result<Board, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_str()
//         Ok(Board::new_empty())
//     }
// }

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Board").field("fen", &self.to_fen()).finish()
    }
}

impl FromStr for Board {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Board::parse_fen(s)
    }
}

impl Board {
    /// white to move, no castling rights or en passant
    #[inline]
    pub fn new_empty() -> Board {
        Default::default()
    }

    #[inline]
    pub fn repetition_count(&self) -> Repeats {
        self.repetition_count.get()
    } 

    pub fn set_repetition_count(&self, reps: Repeats) {
        self.repetition_count.set(reps);
    } 

    #[inline]
    fn calculate_internals(&mut self) {
        self.hash = Hasher::default().hash_board(self);
        // self.material.set(Material::niche());
        self.pinned.set(Bitboard::niche());
        self.threats_to = [Cell::<_>::new(Bitboard::niche()), Cell::<_>::new(Bitboard::niche())];
        self.checkers_of = [Cell::<_>::new(Bitboard::niche()), Cell::<_>::new(Bitboard::niche())];
}


    #[inline]
    pub fn hash(&self) -> Hash {
        self.hash
    }

    #[inline]
    pub fn castling(&self) -> CastlingRights {
        self.castling
    }

    #[inline]
    pub fn pieces(&self, p: Piece) -> Bitboard {
        self.pieces[p]
    }

    // #[inline]
    // pub fn pieces_mut(&mut self, p: Piece) -> &mut Bitboard {
    //     &self.pieces[p as usize]
    // }

    #[inline]
    pub fn line_pieces(&self) -> Bitboard {
        self.rooks() | self.bishops() | self.queens()
    }

    #[inline]
    pub fn non_line_pieces(&self) -> Bitboard {
        self.pawns() | self.knights() | self.kings()
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
        self.colors[c.index()]
    }

    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.black() | self.white()
    }

    #[inline]
    pub fn white(&self) -> Bitboard {
        self.colors[Color::White.index()]
    }

    #[inline]
    pub fn black(&self) -> Bitboard {
        self.colors[Color::Black.index()]
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
    pub fn total_halfmoves(&self) -> Ply {
        2 * self.fullmove_number() as Ply + self.color_us().chooser_wb(0, 1) - 2
    }

    #[inline]
    pub fn material(&self) -> Material {
        // let mut mat = self.material.get();
        // if mat == Material::niche() {
        // mat = Material::from_board(self);
        //     self.material.set(mat);
        // }
        // mat
        Material::from_board(self)
    }


    #[inline]
    pub fn piece_at(&self, sq: Bitboard) -> Piece {
        debug_assert!(sq != Bitboard::EMPTY);
        for &p in &Piece::ALL_BAR_NONE {
            if self.pieces(p).contains(sq) {
                return p;
            }
        }
        Piece::None
    }

    #[inline]
    pub fn least_valuable_piece(&self, region: Bitboard) -> Bitboard {
        for &p in &Piece::ALL_BAR_NONE {
            if self.pieces(p).intersects(region) {
                return (self.pieces(p) & region).first();
            }
        }
        Bitboard::EMPTY
    }


    // https://www.chessprogramming.org/Color_Flipping
    pub fn color_flip(&self) -> Board {
        let mut b = self.clone();
        b.colors = [self.colors[1].flip_vertical(), self.colors[0].flip_vertical()];
        b.pieces.iter_mut().for_each(|bb| *bb = bb.flip_vertical() );
        b.turn = self.turn.opposite();
        b.en_passant = self.en_passant().flip_vertical();
        b.castling = self.castling.color_flip();
        b.calculate_internals();
        debug_assert!(b.validate().is_ok());
        b
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
        f.write_char('\n')?;
        let b = self.clone();
        for &r in Bitboard::RANKS.iter().rev() {
            f.write_str(&b.get(r))?;
            f.write_char('\n')?;
        }
        write!(f, "\nfen: {} \n", self.to_fen())?;
        // write!(fmt, "Moves: {}", self.moves)?;
        if f.alternate() {
            writeln!(f, "Hash: {:x}", self.hash())?;
            writeln!(f, "Rep count: {:x}", self.repetition_count().total)?;
            writeln!(f, "White:\n{}\nBlack:\n{}\n", self.white(), self.black())?;
            for &p in Piece::ALL_BAR_NONE.iter() {
                writeln!(
                    f,
                    "Pieces: {}{}\n{}\n",
                    p.to_upper_char(),
                    p.to_lower_char(),
                    self.pieces(p)
                )?;
            }
            writeln!(f, "Pinned:\n{}\n", self.pinned.get())?;
            writeln!(f, "Checkers of white:\n{}\n", self.checkers_of[Color::White].get())?;
            writeln!(f, "Checkers of black:\n{}\n", self.checkers_of[Color::Black].get())?;
            writeln!(f, "Threats to white:\n{}\n", self.threats_to[Color::White].get())?;
            writeln!(f, "Threats to black:\n{}\n", self.threats_to[Color::Black].get())?;
        }

        Ok(())
    }
}

impl Default for Board {
    #[inline]
    fn default() -> Self {
        Board {
            pieces: Default::default(),
            colors: Default::default(),
            castling: CastlingRights::NONE,
            en_passant: Default::default(),
            turn: Default::default(),
            fifty_clock: Default::default(),
            fullmove_number: 1,
            repetition_count: Cell::<_>::new(Repeats::default()), 
            threats_to: [Cell::<_>::new(Bitboard::niche()), Cell::<_>::new(Bitboard::niche())],
            checkers_of: [Cell::<_>::new(Bitboard::niche()), Cell::<_>::new(Bitboard::niche())],
            pinned: Cell::<_>::new(Bitboard::niche()),
            // material: Cell::<_>::new(Material::niche()),
            hash: 0, 
            // moves: MoveList,
        }
        // b.hash = Hasher::default().hash_board(&b);
    }
}

#[cfg(test)]
mod tests {
    use super::boardbuf::*;
    use super::*;
    use crate::catalog::*;
    use crate::globals::constants::*;


    
    #[test]
    fn test_serde() {
        let board1 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap().as_board();
        assert_eq!(serde_json::to_string(&board1).unwrap(), "\"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\""); 
        assert_eq!(serde_json::from_str::<Board>("\"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\"").unwrap(), board1); 
    }


    #[test]
    fn test_color_flip() {
        let board1 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap().as_board();
        let board2 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap().as_board();
        assert_eq!(board1.color_flip().to_fen(), board2.to_fen(), "{:#}\n{:#}", board1.color_flip(), board2);
        assert_eq!(board2.color_flip().to_fen(), board1.to_fen());
    

        let board1 = Board::parse_fen("rnb1k2r/pp3ppp/4p3/3pB3/2pPn3/2P1PN2/q1P1QPPP/2KR1B1R b kq - 1 11").unwrap();
        let board2 = Board::parse_fen("2kr1b1r/Q1p1qppp/2p1pn2/2PpN3/3Pb3/4P3/PP3PPP/RNB1K2R w KQ - 1 11").unwrap();
        assert_eq!(board1.color_flip().to_fen(), board2.to_fen(), "{:#}\n{:#}", board1.color_flip(), board2);
        assert_eq!(board2.color_flip().to_fen(), board1.to_fen());
    }

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
