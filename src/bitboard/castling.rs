use crate::bitboard::bitboard::Bitboard;
use crate::types::Color;
use crate::bitboard::square::Square;
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
    #[inline]
    fn default() -> Self {
        Self::NONE
    }
}

impl<T> std::ops::Index<CastlingRights> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, i: CastlingRights) -> &Self::Output {
        #[cfg(feature = "unchecked_indexing")]
        unsafe {
            &self.get_unchecked(i.index())
        }

        #[cfg(not(feature = "unchecked_indexing"))]
        &self[(i.index())]
    }
}

impl<T> std::ops::IndexMut<CastlingRights> for [T] {
    #[inline]
    fn index_mut(&mut self, cr: CastlingRights) -> &mut Self::Output {
        &mut self[cr.index()]
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
    pub const fn len() -> usize {
        4
    }

    // #[inline]
    // pub fn index(&self) -> usize {
    //     self.bits() as usize
    // }

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
    pub fn from_king_move(to: Square) -> CastlingRights {
        match to.as_bb() {
            Bitboard::G1 => CastlingRights::WHITE_QUEEN,
            Bitboard::C1 => CastlingRights::WHITE_QUEEN,
            Bitboard::G8 => CastlingRights::BLACK_KING,
            Bitboard::C8 => CastlingRights::BLACK_KING,
            _ => {
                unreachable!("king move-to for castling");
            }
        }
    }

    #[inline]
    pub fn rights_lost(squares_changed: Bitboard) -> CastlingRights {
        let mut loss = CastlingRights::NONE;
        if squares_changed.intersects(Self::rook_and_king_squares()) {
            if squares_changed.intersects(Bitboard::FILE_E) {
                if squares_changed.intersects(Bitboard::E1) {
                    loss |= Self::WHITE_KING.or(Self::WHITE_QUEEN);
                }
                if squares_changed.intersects(Bitboard::E8) {
                    loss |= Self::BLACK_KING.or(Self::BLACK_QUEEN);
                }
            }
            if squares_changed.intersects(Bitboard::FILE_A) {
                if squares_changed.intersects(Bitboard::A1) {
                    loss |= Self::WHITE_QUEEN;
                }
                if squares_changed.intersects(Bitboard::A8) {
                    loss |= Self::BLACK_QUEEN;
                }
            }
            if squares_changed.intersects(Bitboard::FILE_H) {
                if squares_changed.intersects(Bitboard::H1) {
                    loss |= Self::WHITE_KING;
                }
                if squares_changed.intersects(Bitboard::H8) {
                    loss |= Self::BLACK_KING;
                }
            }
        }
        loss
    }

    #[inline]
    pub fn color_flip(&self) -> CastlingRights {
        let mut cr = Self::NONE;
        if self.contains(Self::BLACK_KING) {
            cr |= Self::WHITE_KING;
        }
        if self.contains(Self::BLACK_QUEEN) {
            cr |= Self::WHITE_QUEEN;
        }
        if self.contains(Self::WHITE_KING) {
            cr |= Self::BLACK_KING;
        }
        if self.contains(Self::WHITE_QUEEN) {
            cr |= Self::BLACK_QUEEN;
        }
        cr
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
