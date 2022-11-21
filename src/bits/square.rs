use crate::bits::bitboard::{Bitboard, Dir, Squares};
use crate::piece::Color;
use std::cmp;
use std::fmt;

#[derive(Copy, Hash, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Square(u16);

impl Default for Square {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

impl Square {
    pub const A1: Square = Square::from_u8(0);
    pub const B1: Square = Square::from_u8(1);
    pub const C1: Square = Square::from_u8(2);
    pub const D1: Square = Square::from_u8(3);
    pub const E1: Square = Square::from_u8(4);
    pub const F1: Square = Square::from_u8(5);
    pub const G1: Square = Square::from_u8(6);
    pub const H1: Square = Square::from_u8(7);
    pub const A2: Square = Square::from_u8(8);
    pub const B2: Square = Square::from_u8(9);
    pub const C2: Square = Square::from_u8(10);
    pub const D2: Square = Square::from_u8(11);
    pub const E2: Square = Square::from_u8(12);
    pub const F2: Square = Square::from_u8(13);
    pub const G2: Square = Square::from_u8(14);
    pub const H2: Square = Square::from_u8(15);
    pub const A3: Square = Square::from_u8(16);
    pub const B3: Square = Square::from_u8(17);
    pub const C3: Square = Square::from_u8(18);
    pub const D3: Square = Square::from_u8(19);
    pub const E3: Square = Square::from_u8(20);
    pub const F3: Square = Square::from_u8(21);
    pub const G3: Square = Square::from_u8(22);
    pub const H3: Square = Square::from_u8(23);
    pub const A4: Square = Square::from_u8(24);
    pub const B4: Square = Square::from_u8(25);
    pub const C4: Square = Square::from_u8(26);
    pub const D4: Square = Square::from_u8(27);
    pub const E4: Square = Square::from_u8(28);
    pub const F4: Square = Square::from_u8(29);
    pub const G4: Square = Square::from_u8(30);
    pub const H4: Square = Square::from_u8(31);
    pub const A5: Square = Square::from_u8(32);
    pub const B5: Square = Square::from_u8(33);
    pub const C5: Square = Square::from_u8(34);
    pub const D5: Square = Square::from_u8(35);
    pub const E5: Square = Square::from_u8(36);
    pub const F5: Square = Square::from_u8(37);
    pub const G5: Square = Square::from_u8(38);
    pub const H5: Square = Square::from_u8(39);
    pub const A6: Square = Square::from_u8(40);
    pub const B6: Square = Square::from_u8(41);
    pub const C6: Square = Square::from_u8(42);
    pub const D6: Square = Square::from_u8(43);
    pub const E6: Square = Square::from_u8(44);
    pub const F6: Square = Square::from_u8(45);
    pub const G6: Square = Square::from_u8(46);
    pub const H6: Square = Square::from_u8(47);
    pub const A7: Square = Square::from_u8(48);
    pub const B7: Square = Square::from_u8(49);
    pub const C7: Square = Square::from_u8(50);
    pub const D7: Square = Square::from_u8(51);
    pub const E7: Square = Square::from_u8(52);
    pub const F7: Square = Square::from_u8(53);
    pub const G7: Square = Square::from_u8(54);
    pub const H7: Square = Square::from_u8(55);
    pub const A8: Square = Square::from_u8(56);
    pub const B8: Square = Square::from_u8(57);
    pub const C8: Square = Square::from_u8(58);
    pub const D8: Square = Square::from_u8(59);
    pub const E8: Square = Square::from_u8(60);
    pub const F8: Square = Square::from_u8(61);
    pub const G8: Square = Square::from_u8(62);
    pub const H8: Square = Square::from_u8(63);
}
// Bitboard::from_bits_truncate(1 << i)

impl Square {
    #[inline]
    pub const fn from_u8(i: u8) -> Square {
        // debug_assert!(i <= 64);
        Square(i as u16)
    }

    #[inline]
    pub const fn from_xy(x: u32, y: u32) -> Square {
        Square::from_u32((y << 3) | x)
    }

    #[inline]
    pub const fn null() -> Square {
        Square(64u16)
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
        Square(i as u16)
    }

    #[inline]
    pub const fn from_bb(bb: Bitboard) -> Square {
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
        self.0 == 64u16
    }

    #[inline]
    pub const fn shift(self, dir: Dir) -> Square {
        debug_assert!(self.0 as i8 + dir.shift() >= 0 && self.0 as i8 + dir.shift() < 64);
        Square((self.0 as i8 + dir.shift()) as u16)
    }

    #[inline]
    pub const fn file_char(self) -> char {
        let x = self.0 % 8;
        (b'a' + x as u8) as char
    }

    #[inline]
    pub const fn rank_char(self) -> char {
        let y = self.0 / 8;
        (b'1' + y as u8) as char
    }


    pub fn uci(self) -> &'static str {
        // if self.is_null() {
        //     "-".to_string()
        // } else {
        //     [self.file_char(), self.rank_char()].iter().collect()
        // }
        if self.is_null() {
            "-"
        } else {
            static UCIS: [&str; 64] = [
                "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1", "a2", "b2", "c2", "d2", "e2", "f2",
                "g2", "h2", "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3", "a4", "b4", "c4", "d4",
                "e4", "f4", "g4", "h4", "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5", "a6", "b6",
                "c6", "d6", "e6", "f6", "g6", "h6", "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
                "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8",
            ];
            UCIS[self.index()]
        }
    }

    #[inline]
    pub const fn file(self) -> Bitboard {
        Bitboard::FILES[self.file_index()]
    }

    #[inline]
    pub const fn rank(self) -> Bitboard {
        Bitboard::RANKS[self.rank_index()]
    }

    // either the king or queen side
    #[inline]
    pub const fn kq_side(self) -> Bitboard {
        self.as_bb().flood_kq_sides()
    }


    #[inline]
    pub fn diag(self) -> Bitboard {
        self.as_bb().diag_flood()
    }

    #[inline]
    pub fn anti_diag(self) -> Bitboard {
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

    #[inline]
    pub fn calc_between(s1: Square, s2: Square) -> Bitboard {
        Square::calc_line_through(s1, s2) & Square::bounding_rectangle(s1, s2)
    }

    #[inline]
    pub fn calc_strictly_between(s1: Square, s2: Square) -> Bitboard {
        let between = Square::calc_line_through(s1, s2) & Square::bounding_rectangle(s1, s2);
        between.exclude(s1).exclude(s2)
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

    // king moves - see https://www.chessprogramming.org/Distance
    pub fn calc_chebyshev_distance(s1: Square, s2: Square) -> i32 {
        i32::max(
            i32::abs(s1.rank_index() as i32 - s2.rank_index() as i32),
            i32::abs(s1.file_index() as i32 - s2.file_index() as i32),
        )
    }

    //  see https://www.chessprogramming.org/Distance
    pub fn calc_manhattan_distance(s1: Square, s2: Square) -> i32 {
        i32::abs(s1.rank_index() as i32 - s2.rank_index() as i32)
            + i32::abs(s1.file_index() as i32 - s2.file_index() as i32)
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

    /// if white: just the rank NUMBER (1..8) not index
    /// if black: 7 - rank_index (ie 8..1)
    #[inline]
    pub const fn rank_number_as_white(self, c: Color) -> usize {
        (((c as u16) * 7) ^ (self.0 >> 3)) as usize + 1
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
    fn index(&self, i: Square) -> &Self::Output {
        #[cfg(feature = "unchecked_indexing")]
        unsafe {
            &self.get_unchecked(i.index())
        }

        #[cfg(not(feature = "unchecked_indexing"))]
        &self[(i.index())]
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
    use crate::infra::black_box;
    use crate::infra::profiler::*;

    #[test]
    fn test_square() {
        assert_eq!(a1.first_square(), Square(0));
        assert_eq!(a1 < b3, true);
        assert_eq!(a2 < a1, false);
        assert_eq!(a1.first_square().rank_index(), 0);
        assert_eq!(b1.first_square().file_index(), 1);
        assert_eq!(c7.first_square().index(), 6 * 8 + 2);
        assert_eq!(c7.first_square().rank_index(), 6);
        assert_eq!(c7.first_square().rank_number_as_white(Color::White), 7);
        assert_eq!(c7.first_square().rank_number_as_white(Color::Black), 2);
        for y in 0..8 {
            assert_eq!(
                Square::from_xy(0, y).rank_number_as_white(Color::Black),
                9 - Square::from_xy(0, y).rank_number_as_white(Color::White)
            );
        }
        assert_eq!(c7.first_square().file_index(), 2);
        assert_eq!(Square::all().count(), 64);
        assert_eq!(Square::all().next(), Some(Square(0)));
        assert_eq!(!Bitboard::all(), Bitboard::empty());
        assert_eq!(Square::A1.kq_side().popcount(), 32);
        assert!(Square::A1.kq_side().contains(Bitboard::B2));
        assert!(Square::E8.kq_side().contains(Bitboard::H1));
        // assert_eq!(Square::null().as_bb(), Bitboard::empty());
    }

    #[test]
    #[ignore]
    #[should_panic]
    fn test_out_of_bounds() {
        let array: [u32; 64] = [0; 64];
        assert_eq!(array[Square(63)], 0);
        assert_eq!(array[Square(64)], 0);
    }

    #[test]
    fn test_bounding_rectangle() {
        assert_eq!(cmp::min(a1.square(), b2.square()), a1.square());
        assert_eq!(
            Bitboard::all() - b2.square().rank().rays(Dir::N),
            RANK_1 | RANK_2
        );
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
        assert_eq!(
            Square::bounding_rectangle(c5.square(), e5.square()),
            c5 | d5 | e5
        );
        assert_eq!(
            Square::bounding_rectangle(e5.square(), c5.square()),
            c5 | d5 | e5
        );
    }

    #[test]
    fn test_line_though() {
        assert_eq!(Square::calc_line_through(b6.square(), b8.square()), FILE_B);
        assert_eq!(Square::calc_line_through(b5.square(), d5.square()), RANK_5);
        assert_eq!(Square::calc_line_through(a2.square(), b1.square()), a2 | b1);
        assert_eq!(
            Square::calc_line_through(f1.square(), g2.square()),
            f1 | g2 | h3
        );
        assert_eq!(Square::calc_line_through(f1.square(), f1.square()), f1);
        assert_eq!(
            Square::calc_line_through(f1.square(), g3.square()),
            Bitboard::empty()
        );
    }

    #[test]
    fn test_betweens() {
        assert_eq!(Square::calc_between(b6.square(), b6.square()), b6);
        assert_eq!(
            Square::calc_strictly_between(b6.square(), b6.square()),
            Bitboard::empty()
        );

        assert_eq!(Square::calc_between(b6.square(), b8.square()), b6 | b7 | b8);
        assert_eq!(Square::calc_strictly_between(b6.square(), b8.square()), b7);

        assert_eq!(Square::calc_between(f1.square(), h3.square()), f1 | g2 | h3);
        assert_eq!(Square::calc_strictly_between(f1.square(), h3.square()), g2);
    }

    #[test]
    fn test_square_uci() {
        let mut s = String::new();
        for sq in Bitboard::all().squares() {
            s.push_str(&format!("\"{}{}\", ", sq.file_char(), sq.rank_char()));
        }
        println!("s = [ {s} ]");
        assert_eq!(Square::null().uci(), "-");
        assert_eq!(a1.square().uci(), "a1");
        assert_eq!(b2.square().uci(), "b2");
        assert_eq!(c3.square().uci(), "c3");
        assert_eq!(b5.square().uci(), "b5");
        assert_eq!(c5.square().uci(), "c5");
        assert_eq!(e5.square().uci(), "e5");
        assert_eq!(h8.square().uci(), "h8");
    }

    #[test]
    fn bench_square_uci() {
        let mut p = PerfProfiler::new("square_uci".into());
        p.start();
        for sq in Bitboard::all().squares() {
            black_box(&sq.uci());
        }
        p.stop();
    }
}
