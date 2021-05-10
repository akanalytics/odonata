use crate::bitboard::bitboard::Bitboard;
use crate::types::Color;
use std::fmt;
use std::iter::*;

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
    fn default() -> Self {
        Self::NONE
    }
}

impl CastlingRights {
    #[inline]
    pub fn iter() -> &'static [Self] {
        &[
            Self::WHITE_KING,
            Self::WHITE_QUEEN,
            Self::BLACK_KING,
            Self::BLACK_QUEEN,
        ]
    }

    #[inline]
    pub fn index(&self) -> usize {
        match *self {
            Self::WHITE_KING => 0,
            Self::WHITE_QUEEN => 1,
            Self::BLACK_KING => 2,
            Self::BLACK_QUEEN => 3,
            _ => panic!("Castling index called on {}", self),
        }
    }

    pub fn parse(s: &str) -> Result<CastlingRights, String> {
        let mut castling = CastlingRights::NONE;
        for ch in s.chars() {
            match ch {
                '-' => break,
                'K' => castling |= Self::WHITE_KING,
                'Q' => castling |= Self::WHITE_QUEEN,
                'k' => castling |= Self::BLACK_KING,
                'q' => castling |= Self::BLACK_QUEEN,
                _ => return Err(format!("Invalid character '{}' in castling rights '{}'", ch, s)),
            }
        }
        Ok(castling)
    }

    // #[inline]
    // pub fn has_rights(&self, cr: CastlingRights) -> bool {
    //     self.contains(cr)
    // }

    #[inline]
    pub fn king_side_right(c: Color) -> Self {
        c.chooser_wb(Self::WHITE_KING, Self::BLACK_KING)
    }

    #[inline]
    pub fn queen_side_right(c: Color) -> Self {
        c.chooser_wb(Self::WHITE_QUEEN, Self::BLACK_QUEEN)
    }

    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        Self::from_bits_truncate(self.bits | other.bits)
    }

    #[inline]
    pub const fn is_queen_side(&self) -> bool {
        self.intersects(Self::WHITE_QUEEN.or(Self::BLACK_QUEEN))
    }

    #[inline]
    pub const fn is_king_side(&self) -> bool {
        self.intersects(Self::WHITE_KING.or(Self::BLACK_KING))
    }

    #[inline]
    pub fn king_side_squares(c: Color) -> Bitboard {
        c.chooser_wb(Bitboard::F1.or(Bitboard::G1), Bitboard::F8.or(Bitboard::G8))
    }

    #[inline]
    pub fn queen_side_squares(c: Color) -> Bitboard {
        c.chooser_wb(
            Bitboard::D1.or(Bitboard::C1).or(Bitboard::B1),
            Bitboard::D8.or(Bitboard::C8).or(Bitboard::B8),
        )
    }

    #[inline]
    pub const fn rook_and_king_squares() -> Bitboard {
        Bitboard::A1.or(Bitboard::A8.or(Bitboard::H1.or(Bitboard::H8.or(Bitboard::E1.or(Bitboard::E8)))))
    }

    #[inline]
    pub fn adjust(&mut self, squares_changed: Bitboard) {
        if squares_changed.intersects(Self::rook_and_king_squares()) {
            if squares_changed.intersects(Bitboard::FILE_E) {
                if squares_changed.intersects(Bitboard::E1) {
                    *self -= Self::WHITE_KING.or(Self::WHITE_QUEEN);
                }
                if squares_changed.intersects(Bitboard::E8) {
                    *self -= Self::BLACK_KING.or(Self::BLACK_QUEEN);
                }
            }
            if squares_changed.intersects(Bitboard::FILE_A) {
                if squares_changed.intersects(Bitboard::A1) {
                    *self -= Self::WHITE_QUEEN;
                }
                if squares_changed.intersects(Bitboard::A8) {
                    *self -= Self::BLACK_QUEEN;
                }
            }
            if squares_changed.intersects(Bitboard::FILE_H) {
                if squares_changed.intersects(Bitboard::H1) {
                    *self -= Self::WHITE_KING;
                }
                if squares_changed.intersects(Bitboard::H8) {
                    *self -= Self::BLACK_KING;
                }
            }
        }
    }
}

impl fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "-");
        }
        if self.contains(Self::WHITE_KING) {
            write!(f, "K")?
        }
        if self.contains(Self::WHITE_QUEEN) {
            write!(f, "Q")?
        }
        if self.contains(Self::BLACK_KING) {
            write!(f, "k")?
        }
        if self.contains(Self::BLACK_QUEEN) {
            write!(f, "q")?
        }
        Ok(())
    }
}
