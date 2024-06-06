use std::fmt::{self, Debug};
use std::ops;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::bits::bitboard::{Bitboard, Dir, SquaresIterator};
use crate::piece::{Color, FlipVertical};

type USquare = u16;

#[derive(Copy, Hash, Clone, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[rustfmt::skip]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1, 
    A2, B2, C2, D2, E2, F2, G2, H2, 
    A3, B3, C3, D3, E3, F3, G3, H3, 
    A4, B4, C4, D4, E4, F4, G4, H4, 
    A5, B5, C5, D5, E5, F5, G5, H5, 
    A6, B6, C6, D6, E6, F6, G6, H6, 
    A7, B7, C7, D7, E7, F7, G7, H7, 
    A8, B8, C8, D8, E8, F8, G8, H8, 
}

impl Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Square").field(&self.uci()).finish()
    }
}

impl FlipVertical for Square {
    #[inline]
    fn flip_vertical(self) -> Square {
        // flip vertical - https://www.chessprogramming.org/Flipping_Mirroring_and_Rotating
        unsafe { Square::from_u8_unchecked(self as u8 ^ 56) }
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.uci())
    }
}

impl<T> std::ops::Index<Square> for [T; 64] {
    type Output = T;
    #[inline]
    fn index(&self, i: Square) -> &Self::Output {
        #[cfg(not(all(not(feature = "unchecked_indexing"), debug_assertions)))]
        unsafe {
            self.get_unchecked(i.index())
        }

        #[cfg(all(not(feature = "unchecked_indexing"), debug_assertions))]
        self[i.index()]
    }
}

impl<T> std::ops::IndexMut<Square> for [T; 64] {
    #[inline]
    fn index_mut(&mut self, s: Square) -> &mut Self::Output {
        &mut self[s.index()]
    }
}

impl FromStr for Square {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Square::parse(s)
    }
}

use Square::*;

#[rustfmt::skip]
const SQUARES: [Square; 64] = [
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
];

impl Square {
    #[inline]
    #[must_use]
    pub const fn from_u8(i: u8) -> Square {
        assert!(i < 64);
        unsafe { std::mem::transmute(i) }
    }

    #[inline]
    /// # Safety
    /// i must be < 64
    pub const unsafe fn from_u8_unchecked(i: u8) -> Square {
        unsafe { std::mem::transmute(i) }
    }

    #[inline]
    #[must_use]
    pub const fn from_xy(x: u32, y: u32) -> Square {
        debug_assert!(x <= 7 && y <= 7, "Square::from_xy");
        // safe as weve just asserted
        unsafe { Square::from_u8_unchecked(((y << 3) | x) as u8) }
    }

    #[inline]
    #[must_use]
    pub const fn len() -> usize {
        64
    }

    #[inline]
    #[must_use]
    pub const fn all() -> SquaresIterator {
        Bitboard::all().squares()
    }

    #[inline]
    #[must_use]
    pub const fn from_u32(i: u32) -> Square {
        Square::from_u8(i as u8)
    }

    #[inline]
    #[must_use]
    pub const fn from_usize(u: usize) -> Square {
        Square::from_u8(u as u8)
    }

    #[inline]
    pub const fn first(sq1: Self, sq2: Self) -> Self {
        match sq1.index() < sq2.index() {
            true => sq1,
            false => sq2,
        }
    }

    #[inline]
    pub const fn last(sq1: Self, sq2: Self) -> Self {
        match sq1.index() < sq2.index() {
            true => sq2,
            false => sq1,
        }
    }

    #[inline]
    #[must_use]
    pub const fn from_u16(i: u16) -> Square {
        unsafe { Square::from_u8_unchecked(i as u8) }
    }

    #[inline]
    #[must_use]
    pub const fn as_bb(self) -> Bitboard {
        Bitboard::from_sq(self as u16)
    }

    pub fn parse(s: &str) -> anyhow::Result<Square> {
        let mut parts = s.split_inclusive(|_ch| true);
        let bb = match (parts.next(), parts.next(), parts.next()) {
            (Some(c1), Some(c2), None) => Bitboard::parse_file(c1)? & Bitboard::parse_rank(c2)?,
            _ => anyhow::bail!("invalid square '{}' parsing square", s),
        };
        bb.find_first_square()
            .ok_or_else(|| anyhow::anyhow!("empty bitboard in parse"))
    }

    #[inline]
    #[must_use]
    pub const fn is_in(self, bb: Bitboard) -> bool {
        self.as_bb().intersects(bb)
    }

    #[inline]
    #[must_use]
    pub const fn not_in(self, bb: Bitboard) -> bool {
        !self.as_bb().disjoint(bb)
    }

    #[inline]
    #[must_use]
    pub const fn shift(self, dir: Dir) -> Square {
        debug_assert!(self as i8 + dir.shift() >= 0 && self as i8 + dir.shift() < 64);
        Square::from_u16((self as i8 + dir.shift()) as USquare)
    }

    #[inline]
    #[must_use]
    pub const fn file_char(self) -> char {
        let x = self as u32 % 8;
        (b'a' + x as u8) as char
    }

    #[inline]
    #[must_use]
    pub const fn rank_char(self) -> char {
        let y = self as u32 / 8;
        (b'1' + y as u8) as char
    }

    #[must_use]
    pub fn uci(self) -> &'static str {
        #[rustfmt::skip]
        static UCIS: [&str; 64] = [
            "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1", "a2", "b2", "c2", "d2", "e2", "f2",
            "g2", "h2", "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3", "a4", "b4", "c4", "d4",
            "e4", "f4", "g4", "h4", "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5", "a6", "b6",
            "c6", "d6", "e6", "f6", "g6", "h6", "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
            "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8",
        ];
        UCIS[self]
    }

    #[inline]
    pub const fn file_bitboard(self) -> Bitboard {
        Bitboard::FILES[self.file_index()]
    }

    #[inline]
    pub const fn rank_bitboard(self) -> Bitboard {
        Bitboard::RANKS[self.rank_index()]
    }

    /// either the king or queen side half board
    #[inline]
    pub const fn kq_side(self) -> Bitboard {
        self.as_bb().flood_kq_sides()
    }

    #[inline]
    pub const fn diag(self) -> Bitboard {
        self.as_bb().diag_flood()
    }

    #[inline]
    pub const fn anti_diag(self) -> Bitboard {
        self.as_bb().anti_diag_flood()
    }

    // smallest rectangle containing both squares
    pub(super) const fn calc_bounding_rectangle(s1: Square, s2: Square) -> Bitboard {
        let first = Square::first(s1, s2);
        let last = Square::last(s1, s2);
        let south = first.rank_bitboard().rays(Dir::S);
        let north = last.rank_bitboard().rays(Dir::N);
        let first_last = first.as_bb().or(last.as_bb());
        let west = (first_last.file_flood().rays(Dir::E)).invert().shift(Dir::W);
        let east = (first_last.file_flood().rays(Dir::W)).invert().shift(Dir::E);
        Bitboard::all().sub(north).sub(south).sub(east).sub(west)
    }

    #[inline]
    pub(super) fn slow_between(s1: Square, s2: Square) -> Bitboard {
        Square::slow_line_through(s1, s2) & Square::calc_bounding_rectangle(s1, s2)
    }

    #[inline]
    pub(super) const fn slow_strictly_between(s1: Square, s2: Square) -> Bitboard {
        let between = Square::slow_line_through(s1, s2).and(Square::calc_bounding_rectangle(s1, s2));
        between.exclude(s1).exclude(s2)
    }

    pub(super) const fn slow_within_chebyshev_distance_inclusive(sq: Square, d: i32) -> Bitboard {
        let mut surround = Bitboard::empty();
        let mut s2 = 0;
        while s2 < 64 {
            let sq2 = Square::from_usize(s2);
            // when d = 0, we use distance of 1
            if Square::chebyshev_distance(sq, sq2) <= d {
                surround = surround.or(sq2.as_bb());
            }
            s2 += 1;
        }
        surround
    }

    // returns empty if not on same line. For s1 == s2, returns just the single square
    pub(super) const fn slow_line_through(s1: Square, s2: Square) -> Bitboard {
        if s1.index() == s2.index() {
            s1.as_bb()
        } else if s2.is_in(s1.file_bitboard()) {
            s1.file_bitboard()
        } else if s2.is_in(s1.rank_bitboard()) {
            s1.rank_bitboard()
        } else if s2.is_in(s1.diag()) {
            s1.diag()
        } else if s2.is_in(s1.anti_diag()) {
            s1.anti_diag()
        } else {
            Bitboard::empty()
        }
    }

    // king moves - see https://www.chessprogramming.org/Distance
    pub const fn chebyshev_distance(s1: Square, s2: Square) -> i32 {
        let r = i32::abs(s1.rank_index() as i32 - s2.rank_index() as i32);
        let f = i32::abs(s1.file_index() as i32 - s2.file_index() as i32);
        if r > f {
            r
        } else {
            f
        }
    }

    //  see https://www.chessprogramming.org/Distance
    pub(super) fn manhattan_distance(s1: Square, s2: Square) -> i32 {
        let r = i32::abs(s1.rank_index() as i32 - s2.rank_index() as i32);
        let f = i32::abs(s1.file_index() as i32 - s2.file_index() as i32);
        r + f
    }

    #[inline]
    pub const fn rank_index(self) -> usize {
        (self as u16 / 8) as usize
    }

    /// if white: just the rank NUMBER (1..8) not index
    /// if black: 7 - rank_index (ie 8..1)
    #[inline]
    pub const fn rank_number_as_white(self, c: Color) -> usize {
        (((c as USquare) * 7) ^ (self as u16 >> 3)) as usize + 1
    }

    #[inline]
    pub const fn file_index(self) -> usize {
        (self as u16 % 8) as usize
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }
}

impl ops::BitOr<Square> for Square {
    type Output = Bitboard;

    #[inline]
    fn bitor(self, o: Square) -> Bitboard {
        self.as_bb() | o.as_bb()
    }
}

#[cfg(test)]
mod tests {

    // #[allow(non_upper_case_globals)]
    // const a1b2: Bitboard = Bitboard::A1.or(Bitboard::B2);

    use std::hint::black_box;

    use super::*;
    use crate::infra::profiler::PerfProfiler;

    #[test]
    fn test_square() {
        assert_eq!(Bitboard::A1.find_first_square(), Some(Square::from_u8(0)));
        assert_eq!(Bitboard::A1 < Bitboard::B3, true);
        assert_eq!(Bitboard::A2 < Bitboard::A1, false);
        assert_eq!(Square::A1.rank_index(), 0);
        assert_eq!(Square::B1.file_index(), 1);
        assert_eq!(Square::C7.index(), 6 * 8 + 2);
        assert_eq!(Square::C7.rank_index(), 6);
        assert_eq!(Square::C7.rank_number_as_white(Color::White), 7);
        assert_eq!(Square::C7.rank_number_as_white(Color::Black), 2);
        for y in 0..8 {
            assert_eq!(
                Square::from_xy(0, y).rank_number_as_white(Color::Black),
                9 - Square::from_xy(0, y).rank_number_as_white(Color::White)
            );
        }
        assert_eq!(Square::C7.file_index(), 2);
        assert_eq!(Square::all().count(), 64);
        assert_eq!(Square::all().next(), Some(Square::from_u8(0)));
        assert_eq!(!Bitboard::all(), Bitboard::empty());
        assert_eq!(Square::A1.kq_side().popcount(), 32);
        assert!(Square::A1.kq_side().contains(Bitboard::B2));
        assert!(Square::E8.kq_side().contains(Bitboard::H1));
        assert_eq!(Square::A1.flip_vertical().uci(), "a8");
        assert_eq!(Square::E8.flip_vertical().uci(), "e1");
        // assert_eq!(Square::Null().as_bb(), Bitboard::empty());
    }

    #[test]
    #[ignore]
    #[should_panic]
    fn test_out_of_bounds() {
        let array: [u32; 64] = [0; 64];
        assert_eq!(array[Square::from_u8(63)], 0);
        assert_eq!(array[Square::from_u8(64)], 0);
    }

    #[test]
    fn test_bounding_rectangle() {
        assert_eq!(Square::first(A1, B2), A1);
        assert_eq!(
            Bitboard::all() - B2.rank_bitboard().rays(Dir::N),
            Bitboard::RANK_1 | Bitboard::RANK_2
        );
        assert_eq!(Square::calc_bounding_rectangle(A1, B2), A1 | A2 | B1 | B2);
        assert_eq!(Square::calc_bounding_rectangle(B2, A1), A1 | A2 | B1 | B2);
        assert_eq!(Square::calc_bounding_rectangle(A1, A1), A1.as_bb());
        assert_eq!(Square::calc_bounding_rectangle(C3, C3), C3.as_bb());
        assert_eq!(Square::calc_bounding_rectangle(A1, H8), Bitboard::all());
        assert_eq!(Square::calc_bounding_rectangle(B2, B5), B2 | B3 | B4 | B5);
        assert_eq!(Square::calc_bounding_rectangle(B5, B2), B2 | B3 | B4 | B5);
        assert_eq!(Square::calc_bounding_rectangle(C5, E5), C5 | D5 | E5);
        assert_eq!(Square::calc_bounding_rectangle(E5, C5), C5 | D5 | E5);
    }

    #[test]
    fn test_line_though() {
        assert_eq!(Square::slow_line_through(B6, B8), Bitboard::FILE_B);
        assert_eq!(Square::slow_line_through(B5, D5), Bitboard::RANK_5);
        assert_eq!(Square::slow_line_through(A2, B1), A2 | B1);
        assert_eq!(Square::slow_line_through(F1, G2), F1 | G2 | H3);
        assert_eq!(Square::slow_line_through(F1, F1), F1.as_bb());
        assert_eq!(Square::slow_line_through(F1, G3), Bitboard::empty());
    }

    #[test]
    fn test_betweens() {
        assert_eq!(Square::slow_between(B6, B6), Bitboard::B6);
        assert_eq!(Square::slow_strictly_between(B6, B6), Bitboard::empty());

        assert_eq!(Square::slow_between(B6, B8), B6 | B7 | B8);
        assert_eq!(Square::slow_strictly_between(B6, B8), Bitboard::B7);

        assert_eq!(Square::slow_between(F1, H3), F1 | G2 | H3);
        assert_eq!(Square::slow_strictly_between(F1, H3), Bitboard::G2);
    }

    #[test]
    fn test_square_uci() {
        let mut s = String::new();
        for sq in Bitboard::all().squares() {
            s.push_str(&format!("\"{}{}\", ", sq.file_char(), sq.rank_char()));
        }
        println!("s = [ {s} ]");
        assert_eq!(Square::A1.uci(), "a1");
        assert_eq!(Square::B2.uci(), "b2");
        assert_eq!(Square::C3.uci(), "c3");
        assert_eq!(Square::B5.uci(), "b5");
        assert_eq!(Square::C5.uci(), "c5");
        assert_eq!(Square::E5.uci(), "e5");
        assert_eq!(Square::H8.uci(), "h8");
        let bad = ["a11", "", "a", "i1", "a9"];
        for sq in &bad {
            assert!(Square::parse(sq).is_err());
            println!("{}", Square::parse(sq).unwrap_err());
        }
    }

    #[test]
    fn bench_square() {
        let mut p = PerfProfiler::new("square.uci");
        for sq in Bitboard::all().squares() {
            p.bench(|| black_box(sq.uci()));
        }

        let mut p = PerfProfiler::new("square.parse");
        for sq in Bitboard::all().squares() {
            let s = sq.uci();
            let parsed_sq = p.bench(|| black_box(Square::parse(s))).unwrap();
            assert_eq!(parsed_sq, sq);
        }
    }
}
