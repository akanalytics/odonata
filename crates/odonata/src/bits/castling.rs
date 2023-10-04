use crate::{
    bits::{bitboard::Bitboard, square::Square},
    piece::Color,
};
use anyhow::{bail, Result};
use std::{fmt};

bitflags! {
    #[derive(Clone,Copy, PartialEq, Debug, Eq)]
    pub struct CastlingRights: u8 {
        const NONE = 0;
        const WHITE_KING = 1 << 0;
        const WHITE_QUEEN = 1 << 1;
        const BLACK_KING = 1 << 2;
        const BLACK_QUEEN = 1 << 3;
        const ALL = Self::WHITE_KING.bits() | Self::WHITE_QUEEN.bits() | Self::BLACK_KING.bits() | Self::BLACK_QUEEN.bits();
    }
}

impl CastlingRights {
    #[inline]
    pub fn values_array() -> &'static [Self] {
        &[
            Self::WHITE_KING,
            Self::WHITE_QUEEN,
            Self::BLACK_KING,
            Self::BLACK_QUEEN,
        ]
    }

    pub fn parse(s: &str) -> Result<CastlingRights> {
        let mut castling = CastlingRights::NONE;
        for ch in s.chars() {
            match ch {
                '-' => break,
                'K' => castling |= Self::WHITE_KING,
                'Q' => castling |= Self::WHITE_QUEEN,
                'k' => castling |= Self::BLACK_KING,
                'q' => castling |= Self::BLACK_QUEEN,
                _ => bail!("invalid character '{}' in castling rights '{}'", ch, s),
            }
        }
        Ok(castling)
    }

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
        Self::from_bits_truncate(self.bits() | other.bits())
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
    pub fn king_side_move_squares(c: Color) -> Bitboard {
        c.chooser_wb(Bitboard::F1.or(Bitboard::G1), Bitboard::F8.or(Bitboard::G8))
    }

    #[inline]
    pub fn queen_side_move_squares(c: Color) -> Bitboard {
        c.chooser_wb(
            Bitboard::D1.or(Bitboard::C1).or(Bitboard::B1),
            Bitboard::D8.or(Bitboard::C8).or(Bitboard::B8),
        )
    }

    #[inline]
    pub fn is_castling(from: Square, to: Square) -> bool {
        from == Square::E1 && (to == Square::C1 || to == Square::G1)
            || from == Square::E8 && (to == Square::C8 || to == Square::G8)
    }

    #[inline]
    pub fn from_king_move(to: Square) -> CastlingRights {
        match to {
            Square::G1 => CastlingRights::WHITE_KING,
            Square::C1 => CastlingRights::WHITE_QUEEN,
            Square::G8 => CastlingRights::BLACK_KING,
            Square::C8 => CastlingRights::BLACK_QUEEN,
            _ => CastlingRights::NONE,
        }
    }

    pub fn rights_lost(from: Square, to: Square) -> CastlingRights {
        const fn pop_castling_rights() -> [CastlingRights; 64] {
            let mut cr = [CastlingRights::NONE; 64];
            cr[Square::A1.index()] = CastlingRights::WHITE_QUEEN;
            cr[Square::A8.index()] = CastlingRights::BLACK_QUEEN;
            cr[Square::H1.index()] = CastlingRights::WHITE_KING;
            cr[Square::H8.index()] = CastlingRights::BLACK_KING;
            cr[Square::E1.index()] = CastlingRights::WHITE_KING.or(CastlingRights::WHITE_QUEEN);
            cr[Square::E8.index()] = CastlingRights::BLACK_KING.or(CastlingRights::BLACK_QUEEN);
            cr
        }

        const RIGHTS_LOST: [CastlingRights; Square::len()] = pop_castling_rights();
        RIGHTS_LOST[from] ^ RIGHTS_LOST[to]
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
