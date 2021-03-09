use crate::bitboard::{Bitboard, Dir};
use std::fmt;
use std::iter::*;


#[derive(Copy, Clone, Debug)]
pub enum Col { BLACK = 0, WHITE = 1 }

pub struct Chooser<T> {
    pub black: T,
    pub white: T,
}

// impl<T> Chooser<T> {
#[inline]
pub fn chooser_array<'a, T>(c: Col, white: &'a T, black: &'a T) -> &'a T {
    [&white, &black][c as usize]
}

#[inline]
pub fn chooser_wb<'a, T>(c: Col, white: &'a T, black: &'a T) -> &'a T {
    match c {
        Col::WHITE => { white }, 
        Col::BLACK => { black }
    }
}



#[inline]
pub fn chooser_struct<'a, T>(c: Col, choices: &'a Chooser<&T>) -> &'a T {
    return [&choices.white, &choices.black][c as usize];
}



#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
    pub is_white: bool,
    pub index: usize,
    pub pawn_move: Dir,
    pub pawn_capture_east: Dir,
    pub pawn_capture_west: Dir,
    pub kingside_castle_sqs: Bitboard,
    pub queenside_castle_sqs: Bitboard,
    pub double_push_dest_rank: Bitboard,
    pub castle_rights_queen: CastlingRights,
    pub castle_rights_king: CastlingRights,
    pub back_rank: Bitboard,
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

impl Color {
    pub const WHITE: Self = Color {
        is_white: true,
        index: 0,
        pawn_move: Dir::N,
        pawn_capture_east: Dir::NE,
        pawn_capture_west: Dir::NW,
        kingside_castle_sqs: Bitboard::F1.or(Bitboard::G1),
        queenside_castle_sqs: Bitboard::D1.or(Bitboard::C1).or(Bitboard::B1),
        double_push_dest_rank: Bitboard::RANK_4,
        castle_rights_queen: CastlingRights::WHITE_QUEEN,
        castle_rights_king: CastlingRights::WHITE_KING,
        back_rank: Bitboard::RANK_1,
    };
    pub const BLACK: Self = Color {
        is_white: false,
        index: 1,
        pawn_move: Dir::S,
        pawn_capture_east: Dir::SE,
        pawn_capture_west: Dir::SW,
        kingside_castle_sqs: Bitboard::F8.or(Bitboard::G8),
        queenside_castle_sqs: Bitboard::D8.or(Bitboard::C8).or(Bitboard::B8),
        double_push_dest_rank: Bitboard::RANK_5,
        castle_rights_queen: CastlingRights::BLACK_QUEEN,
        castle_rights_king: CastlingRights::BLACK_KING,
        back_rank: Bitboard::RANK_8,
    };

    pub fn opposite(&self) -> Color {
        [Color::BLACK, Color::WHITE][self.index]
    }

    pub fn parse(s: &str) -> Result<Color, String> {
        match s {
            "w" => Ok(Color::WHITE),
            "b" => Ok(Color::BLACK),
            _ => Err(format!("Invalid color: '{}'", s)),
        }
    }

    pub fn from_piece_char(ch: char) -> Result<Color, String> {
        if ch.is_lowercase() {
            return Ok(Color::BLACK);
        } else if ch.is_uppercase() {
            return Ok(Color::WHITE);
        }
        Err(format!("Cannot get color for char '{}'", ch))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", ['w', 'b'][self.index])
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Piece {
    None = 0,
    Pawn = 1,
    Knight = 2,
    Bishop = 3,
    Rook = 4,
    Queen = 5,
    King = 6,
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
            Some(c) if c.is_white => self.to_upper_char(),
            Some(_) => self.to_upper_char().to_ascii_lowercase(),
        }
    }
}




#[cfg(test)]
mod tests {
    use crate::catalog::*;
    use super::*;
    use crate::globals::constants::*;

    #[test]
    fn color() {
        assert_eq!(Color::parse("w"), Ok(Color::WHITE));
        assert_eq!(Color::parse("b"), Ok(Color::BLACK));
        assert_eq!(Color::parse("B"), Err("Invalid color: 'B'".to_string()));
        assert_eq!(Piece::King.to_char(Some(Color::BLACK)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');
    }

    #[test]
    fn piece() {
        assert_eq!(Piece::Pawn.to_upper_char(), 'P');
        assert_eq!(Piece::King.to_char(Some(Color::BLACK)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');
    }

    #[test]
    fn choose() {
        let c = Col::WHITE;
        const choice: Chooser<&Bitboard> = Chooser { 
            white: &Bitboard::RANK_4, 
            black: &Bitboard::RANK_5,
        };

        let bb = chooser_array(c, &Bitboard::RANK_4, &Bitboard::RANK_5 );
        assert_eq!(bb, &Bitboard::RANK_4);
        let bb = chooser_wb(c, &Bitboard::RANK_4, &Bitboard::RANK_5 );
        let bb = chooser_struct(c, &choice );
    }
}
