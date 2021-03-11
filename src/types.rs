use crate::bitboard::{Bitboard, Dir};
use std::fmt;
use std::iter::*;




pub struct Chooser<T> {
    pub black: T,
    pub white: T,
}

// impl<T> Chooser<T> {
#[inline]
pub fn chooser_array<'a, T>(c: Color, white: &'a T, black: &'a T) -> &'a T {
    [&white, &black][c as usize]
}

#[inline]
pub fn chooser_wb_ref<'a, T>(c: Color, white: &'a T, black: &'a T) -> &'a T {
    match c {
        Color::White => { white }, 
        Color::Black => { black }
    }
}

#[inline]
pub fn chooser_wb<T>(c: Color, white: T, black: T) -> T {
    match c {
        Color::White => { white }, 
        Color::Black => { black }
    }
}

// pub fn chooser_wb<T>(c: Color, white: T, black: T) -> T {
//     match c {
//         Col::WHITE => { white }, 
//         Col::BLACK => { black }
//     }
// }


#[inline]
pub fn chooser_struct<'a, T>(c: Color, choices: &'a Chooser<&T>) -> &'a T {
    return [&choices.white, &choices.black][c as usize];
}



bitflags! {
    pub struct CastlingRights: u8 {
        const NONE = 0;
        const WHITE_KING = 1 << 0;
        const WHITE_QUEEN = 1 << 1;
        const BLACK_KING = 1 << 2;
        const BLACK_QUEEN = 1 << 3;
        const ALL = Self::WHITE_KING.bits | Self::WHITE_QUEEN.bits | Self::BLACK_KING.bits | Self::BLACK_QUEEN.bits;
    }
}


impl Default for CastlingRights {
    fn default() -> Self { Self::NONE }
}


impl CastlingRights {
    pub fn parse(s: &str) -> Result<CastlingRights, String> {
        let mut castling = CastlingRights::NONE;
        for ch in s.chars() {
            match ch {
                '-' => break,
                'K' => castling |= CastlingRights::WHITE_KING,
                'Q' => castling |= CastlingRights::WHITE_QUEEN,
                'k' => castling |= CastlingRights::BLACK_KING,
                'q' => castling |= CastlingRights::BLACK_QUEEN,
                _ => return Err(format!("Invalid character '{}' in castling rights '{}'", ch, s)),
            }
        }
        Ok(castling)
    }

    pub fn has_king_side_right(self, c: Color) -> bool {
        self.contains(chooser_wb(c, 
            CastlingRights::WHITE_KING,
            CastlingRights::BLACK_KING
        ))
    } 

    pub fn has_queen_side_right(self, c: Color) -> bool {
        self.contains(chooser_wb(c, 
            CastlingRights::WHITE_QUEEN,
            CastlingRights::BLACK_QUEEN
        ))
    } 

    pub fn king_side_squares(c: Color) -> Bitboard {
        chooser_wb(c, 
            Bitboard::F1.or(Bitboard::G1),
            Bitboard::F8.or(Bitboard::G8)
        )
    }

    pub fn queen_side_squares(c: Color) -> Bitboard {
        chooser_wb(c, 
            Bitboard::D1.or(Bitboard::C1).or(Bitboard::B1),
            Bitboard::D8.or(Bitboard::C8).or(Bitboard::B8)
        )
    }



}

impl fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "{}", '-');
        }
        if self.contains(Self::WHITE_KING) {
            write!(f, "{}", 'K')?
        }
        if self.contains(Self::WHITE_QUEEN) {
            write!(f, "{}", 'Q')?
        }
        if self.contains(Self::BLACK_KING) {
            write!(f, "{}", 'k')?
        }
        if self.contains(Self::BLACK_QUEEN) {
            write!(f, "{}", 'q')?
        }
        Ok(())
    }
}



#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Color { White = 0, Black = 1 }  // numbering as per CPW


impl Default for Color {
    fn default() -> Self { Color::White }
} 


impl Color {

    pub const ALL: [Color; 2] = [Color::White, Color::Black];
        
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    #[inline]
    pub fn chooser_wb<T>(self, white_thing: T, black_thing: T) -> T {
        match self {
            Color::White => { white_thing }, 
            Color::Black => { black_thing }
        }
    }   

    #[inline]
    pub fn pawn_move(self) -> Dir {
        self.chooser_wb(Dir::N, Dir::S)
    }

    #[inline]
    pub fn double_push_dest_rank(self) -> Bitboard {
        self.chooser_wb(Bitboard::RANK_4, Bitboard::RANK_5)
    }

    #[inline]
    pub fn pawn_capture_east(self) -> Dir {
        self.chooser_wb(Dir::NE, Dir::SE)
    }

    #[inline]
    pub fn pawn_capture_west(self) -> Dir {
        self.chooser_wb(Dir::NW, Dir::SW)
    } 

    #[inline]
    pub fn back_rank(self) -> Bitboard {
        self.chooser_wb(Bitboard::RANK_1, Bitboard::RANK_8)
    }

    #[inline]
    pub fn opposite(self) -> Color {
        self.chooser_wb(Color::Black, Color::White)
    }

    pub fn parse(s: &str) -> Result<Color, String> {
        match s {
            "w" => Ok(Color::White),
            "b" => Ok(Color::Black),
            _ => Err(format!("Invalid color: '{}'", s)),
        }
    }

    pub fn from_piece_char(ch: char) -> Result<Color, String> {
        if ch.is_lowercase() {
            return Ok(Color::Black);
        } else if ch.is_uppercase() {
            return Ok(Color::White);
        }
        Err(format!("Cannot get color for char '{}'", ch))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", chooser_wb(*self, 'w', 'b'))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Piece {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
    None = 6,
}


impl Default for Piece {
    fn default() -> Self {
        Piece::None
    }
}

impl Piece {
    pub const ALL: [Piece; 6] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];

    #[inline]
    // pub fn to_upper_char(self) -> &char {
    //     ".PNBRQK".as_bytes()[self as usize] as char
    // }
    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }

    #[inline]
    pub fn to_upper_char(&self) -> char {
        match self {
            Piece::None => '.',
            Piece::Pawn => 'P',
            Piece::Knight => 'N',
            Piece::Bishop => 'B',
            Piece::Rook => 'R',
            Piece::Queen => 'Q',
            Piece::King => 'K',
        }
    }

    #[inline]
    pub fn from_char(ch: char) -> Result<Piece, String> {
        Ok(match ch.to_ascii_uppercase() {
            '.' | ' ' => Piece::None,
            'P' => Piece::Pawn,
            'N' => Piece::Knight,
            'B' => Piece::Bishop,
            'R' => Piece::Rook,
            'Q' => Piece::Queen,
            'K' => Piece::King,
            _ => return Err(format!("Unknown piece '{}'", ch)),
        })
    }

    #[inline]
    pub fn to_char(&self, c: Option<Color>) -> char {
        match c {
            None => self.to_upper_char(),
            Some(c) if c == Color::White => self.to_upper_char(),
            Some(_) => self.to_upper_char().to_ascii_lowercase(),
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color() {
        assert_eq!(Color::parse("w"), Ok(Color::White));
        assert_eq!(Color::parse("b"), Ok(Color::Black));
        assert_eq!(Color::parse("B"), Err("Invalid color: 'B'".to_string()));
        assert_eq!(Piece::King.to_char(Some(Color::Black)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');
    }

    #[test]
    fn piece() {
        assert_eq!(Piece::Pawn.to_upper_char(), 'P');
        assert_eq!(Piece::King.to_char(Some(Color::Black)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');
    }

    #[test]
    fn choose() {
        let c = Color::White;
        const CHOICE: Chooser<&Bitboard> = Chooser { 
            white: &Bitboard::RANK_4, 
            black: &Bitboard::RANK_5,
        };

        let bb = chooser_array(c, &Bitboard::RANK_4, &Bitboard::RANK_5 );
        assert_eq!(bb, &Bitboard::RANK_4);
        chooser_wb(c, &Bitboard::RANK_4, &Bitboard::RANK_5 );
        chooser_struct(c, &CHOICE );
    }
}