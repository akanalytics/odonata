use crate::bitboard::bitboard::{Bitboard, Dir, Squares};
use std::cmp;
use std::fmt;



#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Square(u8);

impl Default for Square {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

// Bitboard::from_bits_truncate(1 << i)

impl Square {
    #[inline]
    pub fn from_u8(i: u8) -> Square {
        debug_assert!(i <= 64);
        Square(i)
    }

    #[inline]
    pub const fn null() -> Square {
        Square(64u8)
    }

    #[inline]
    pub const fn len() -> usize {
        64
    }

    #[inline]
    pub const fn all() -> Squares {
        Bitboard::all().squares()
    }

    #[inline]
    pub const fn from_u32(i: u32) -> Square {
        Square(i as u8)
    }

    #[inline]
    pub fn from_bb(bb: Bitboard) -> Square {
        bb.square()
    }

    #[inline]
    pub const fn as_bb(self) -> Bitboard {
        Bitboard::from_sq(self.0)
    }

    #[inline]
    pub const fn is_in(self, bb: Bitboard) -> bool {
        self.as_bb().intersects(bb)
    }

    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 64u8
    }

    #[inline]
    pub fn shift(self, dir: Dir) -> Square {
        debug_assert!(self.0 as i8 + dir.shift >= 0 && self.0 as i8 + dir.shift < 64);
        Square((self.0 as i8 + dir.shift) as u8)
    }

    #[inline]
    pub fn file_char(self) -> char {
        let x = self.0 % 8;
        char::from(b'a' + x as u8)
    }

    #[inline]
    pub fn rank_char(self) -> char {
        let y = self.0 / 8;
        char::from(b'1' + y as u8)
    }

    pub fn uci(self) -> String {
        format!("{}{}", self.file_char(), self.rank_char())
    }

    #[inline]
    pub fn file(self) -> Bitboard {
        Bitboard::FILES[self.file_index()]
    }

    #[inline]
    pub fn rank(self) -> Bitboard {
        Bitboard::RANKS[self.rank_index()]
    }

    #[inline]
    pub fn diag(self) -> Bitboard {
        // FIXME: slow
        self.as_bb().diag_flood()
    }

    #[inline]
    pub fn anti_diag(self) -> Bitboard {
        // FIXME: slow
        self.as_bb().anti_diag_flood()
    }

    // smallest rectangle containing both squares
    pub fn bounding_rectangle(s1: Square, s2: Square) -> Bitboard {
        let first = cmp::min(s1, s2);
        let last = cmp::max(s1, s2);
        let south = first.rank().rays(Dir::S);
        let north = last.rank().rays(Dir::N);
        let west = (!((first.as_bb() | last.as_bb()).file_flood().rays(Dir::E))).shift(Dir::W);
        let east = (!((first.as_bb() | last.as_bb()).file_flood().rays(Dir::W))).shift(Dir::E);
        Bitboard::all() - north - south - east - west
    }

    // returns empty if not on same line. For s1 == s2, returns just the single square
    pub fn calc_line_through(s1: Square, s2: Square) -> Bitboard {
        if s1 == s2 {
            s1.as_bb()
        } else if s2.is_in(s1.file()) {
            s1.file()
        } else if s2.is_in(s1.rank()) {
            s1.rank()
        } else if s2.is_in(s1.diag()) {
            s1.diag()
        } else if s2.is_in(s1.anti_diag()) {
            s1.anti_diag()
        } else {
            Bitboard::empty()
        }
    }

    /// flip vertical - https://www.chessprogramming.org/Flipping_Mirroring_and_Rotating
    #[inline]
    pub const fn flip_vertical(self) -> Square {
        Square(self.0 ^ 56)
    }

    #[inline]
    pub const fn rank_index(self) -> usize {
        (self.0 / 8) as usize
    }

    #[inline]
    pub const fn file_index(self) -> usize {
        (self.0 % 8) as usize
    }

    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.uci())
    }
}

impl<T> std::ops::Index<Square> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, s: Square) -> &Self::Output {
        unsafe { &self.get_unchecked(s.index()) }
        // &self[s.index()]
    }
}

impl<T> std::ops::IndexMut<Square> for [T] {
    #[inline]
    fn index_mut(&mut self, s: Square) -> &mut Self::Output {
        &mut self[s.index()]
    }
}



#[cfg(test)]
mod tests {

    // #[allow(non_upper_case_globals)]
    // const a1b2: Bitboard = Bitboard::A1.or(Bitboard::B2);

    use super::*;
    use crate::globals::constants::*;

    #[test]
    fn test_square() {
        assert_eq!(a1.first_square(), Square(0));
        assert_eq!(a1.first_square().rank_index(), 0);
        assert_eq!(b1.first_square().file_index(), 1);
        assert_eq!(c7.first_square().index(), 6 * 8 + 2);
        assert_eq!(c7.first_square().rank_index(), 6);
        assert_eq!(c7.first_square().file_index(), 2);
        assert_eq!(Square::all().count(), 64);
        assert_eq!(Square::all().next(), Some(Square(0)));
        assert_eq!(!Bitboard::all(), Bitboard::empty());
    }

    #[test]
    fn test_bounding_rectangle() {
        assert_eq!(cmp::min(a1.square(), b2.square()), a1.square());
        assert_eq!(Bitboard::all() - b2.square().rank().rays(Dir::N), RANK_1 | RANK_2);
        assert_eq!(
            Square::bounding_rectangle(a1.square(), b2.square()),
            a1 | a2 | b1 | b2
        );
        assert_eq!(
            Square::bounding_rectangle(b2.square(), a1.square()),
            a1 | a2 | b1 | b2
        );
        assert_eq!(Square::bounding_rectangle(a1.square(), a1.square()), a1);
        assert_eq!(Square::bounding_rectangle(c3.square(), c3.square()), c3);
        assert_eq!(
            Square::bounding_rectangle(a1.square(), h8.square()),
            Bitboard::all()
        );
        assert_eq!(
            Square::bounding_rectangle(b2.square(), b5.square()),
            b2 | b3 | b4 | b5
        );
        assert_eq!(
            Square::bounding_rectangle(b5.square(), b2.square()),
            b2 | b3 | b4 | b5
        );
        assert_eq!(Square::bounding_rectangle(c5.square(), e5.square()), c5 | d5 | e5);
        assert_eq!(Square::bounding_rectangle(e5.square(), c5.square()), c5 | d5 | e5);
    }

    #[test]
    fn test_line_though() {
        assert_eq!(Square::calc_line_through(b6.square(), b8.square()), FILE_B);
        assert_eq!(Square::calc_line_through(b5.square(), d5.square()), RANK_5);
        assert_eq!(Square::calc_line_through(a2.square(), b1.square()), a2 | b1);
        assert_eq!(Square::calc_line_through(f1.square(), g2.square()), f1 | g2 | h3);
        assert_eq!(Square::calc_line_through(f1.square(), f1.square()), f1);
        assert_eq!(
            Square::calc_line_through(f1.square(), g3.square()),
            Bitboard::empty()
        );
    }


}