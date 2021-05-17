use std::fmt::{self, Write};
use std::cmp;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Dir {
    pub index: usize,
    pub shift: i32,
    pub mask: Bitboard, // mask for opposite edge(s)
}

impl Dir {
    pub const N: Self = Dir {
        index: 0,
        shift: 8,
        mask: Bitboard::RANK_8,
    };
    pub const NE: Self = Dir {
        index: 1,
        shift: 9,
        mask: Bitboard::RANK_8.or(Bitboard::FILE_H),
    };
    pub const E: Self = Dir {
        index: 2,
        shift: 1,
        mask: Bitboard::FILE_H,
    };
    pub const SE: Self = Dir {
        index: 3,
        shift: -7,
        mask: Bitboard::RANK_1.or(Bitboard::FILE_H),
    };
    pub const S: Self = Dir {
        index: 4,
        shift: -8,
        mask: Bitboard::RANK_1,
    };
    pub const SW: Self = Dir {
        index: 5,
        shift: -9,
        mask: Bitboard::RANK_1.or(Bitboard::FILE_A),
    };
    pub const W: Self = Dir {
        index: 6,
        shift: -1,
        mask: Bitboard::FILE_A,
    };
    pub const NW: Self = Dir {
        index: 7,
        shift: 7,
        mask: Bitboard::RANK_8.or(Bitboard::FILE_A),
    };

    pub const ALL: [Self; 8] = [
        Self::N,
        Self::NE,
        Self::E,
        Self::SE,
        Self::S,
        Self::SW,
        Self::W,
        Self::NW,
    ];

    pub fn opposite(&self) -> Dir {
        Self::ALL[(self.index + 4) % 8]
    }
}

// generated from https://docs.google.com/spreadsheets/d/1TB2TKX04VsR10CLNLDIvrufm6wSJOttXOyPNKndU4N0/edit?usp=sharing
#[rustfmt::skip]
bitflags! {
    #[derive(Default)]
    pub struct Bitboard: u64 {
        const EMPTY = 0;
        const A1 = 1 << 0; 	const B1 = 1 << 1; const	C1 = 1 << 2; const	D1 = 1 << 3; const	E1 = 1 << 4; const	F1 = 1 << 5; const	G1 = 1 << 6; const	H1 = 1 << 7; const
        A2 = 1 << 8; const	B2 = 1 << 9; const	C2 = 1 << 10; const	D2 = 1 << 11; const	E2 = 1 << 12; const	F2 = 1 << 13; const	G2 = 1 << 14; const	H2 = 1 << 15; const
        A3 = 1 << 16; const	B3 = 1 << 17; const	C3 = 1 << 18; const	D3 = 1 << 19; const	E3 = 1 << 20; const	F3 = 1 << 21; const	G3 = 1 << 22; const	H3 = 1 << 23; const
        A4 = 1 << 24; const	B4 = 1 << 25; const	C4 = 1 << 26; const	D4 = 1 << 27; const	E4 = 1 << 28; const	F4 = 1 << 29; const	G4 = 1 << 30; const	H4 = 1 << 31; const
        A5 = 1 << 32; const	B5 = 1 << 33; const	C5 = 1 << 34; const	D5 = 1 << 35; const	E5 = 1 << 36; const	F5 = 1 << 37; const	G5 = 1 << 38; const	H5 = 1 << 39; const
        A6 = 1 << 40; const	B6 = 1 << 41; const	C6 = 1 << 42; const	D6 = 1 << 43; const	E6 = 1 << 44; const	F6 = 1 << 45; const	G6 = 1 << 46; const	H6 = 1 << 47; const
        A7 = 1 << 48; const	B7 = 1 << 49; const	C7 = 1 << 50; const	D7 = 1 << 51; const	E7 = 1 << 52; const	F7 = 1 << 53; const	G7 = 1 << 54; const	H7 = 1 << 55; const
        A8 = 1 << 56; const	B8 = 1 << 57; const	C8 = 1 << 58; const	D8 = 1 << 59; const	E8 = 1 << 60; const	F8 = 1 << 61; const	G8 = 1 << 62; const	H8 = 1 << 63;

        const FILE_A = Self::A1.bits | Self::A2.bits | Self::A3.bits | Self::A4.bits | Self::A5.bits | Self::A6.bits | Self::A7.bits | Self::A8.bits;
        const RANK_1 = Self::A1.bits | Self::B1.bits | Self::C1.bits | Self::D1.bits | Self::E1.bits | Self::F1.bits | Self::G1.bits | Self::H1.bits;
        const FILE_B = Self::FILE_A.bits << 1; const RANK_2 = Self::RANK_1.bits << 8;
        const FILE_C = Self::FILE_A.bits << 2; const RANK_3 = Self::RANK_1.bits << (2*8);
        const FILE_D = Self::FILE_A.bits << 3; const RANK_4 = Self::RANK_1.bits << (3*8);
        const FILE_E = Self::FILE_A.bits << 4; const RANK_5 = Self::RANK_1.bits << (4*8);
        const FILE_F = Self::FILE_A.bits << 5; const RANK_6 = Self::RANK_1.bits << (5*8);
        const FILE_G = Self::FILE_A.bits << 6; const RANK_7 = Self::RANK_1.bits << (6*8);
        const FILE_H = Self::FILE_A.bits << 7; const RANK_8 = Self::RANK_1.bits << (7*8);
    }
}

impl Bitboard {
    // const EDGES:Self = Self::FILE_A.or(Self::FILE_H).or(Self::RANK_1).or(Self::RANK_8);
    pub const PROMO_RANKS: Self = Self::RANK_1.or(Self::RANK_8);
    pub const RANKS: [Self; 8] = [
        Self::RANK_1,
        Self::RANK_2,
        Self::RANK_3,
        Self::RANK_4,
        Self::RANK_5,
        Self::RANK_6,
        Self::RANK_7,
        Self::RANK_8,
    ];
    pub const FILES: [Self; 8] = [
        Self::FILE_A,
        Self::FILE_B,
        Self::FILE_C,
        Self::FILE_D,
        Self::FILE_E,
        Self::FILE_F,
        Self::FILE_G,
        Self::FILE_H,
    ];

    // insert,
    // remove,
    // set,
    // toggle,
    // all(),
    // empty(),
    // is_all,
    // is_empty,
    // intersects,
    // contains
    // ... come for free

    #[inline]
    pub fn from_xy(x: u32, y: u32) -> Bitboard {
        let bit = 1 << (y * 8 + x);
        Bitboard::from_bits_truncate(bit)
    }

    #[inline]
    pub const fn from_sq(index: u32) -> Bitboard {
        let bit = 1 << index;
        Bitboard::from_bits_truncate(bit)
    }

    // a niche value that never occurs "in real life"
    #[inline]
    pub const fn niche() -> Bitboard {
        Bitboard::all()
    }

    #[inline]
    pub fn disjoint(self, other: Bitboard) -> bool {
        (self & other).is_empty()
    }

    #[inline]
    pub fn any(self) -> bool {
        !self.is_empty()
    }


    #[inline]
    pub fn shift(self, dir: Dir) -> Bitboard {
        let bb = self - dir.mask;
        if dir.shift > 0 {
            Bitboard::from_bits_truncate(bb.bits << dir.shift)
        } else {
            Bitboard::from_bits_truncate(bb.bits >> -dir.shift)
        }
    }


    // excludes the src squares themselves, but includes edge squares
    pub fn rays(&self, dir: Dir) -> Bitboard {
        let mut sqs = *self;
        let mut bb = Bitboard::EMPTY;
        while !sqs.is_empty() {
            sqs = sqs.shift(dir);
            bb |= sqs;
        }
        bb
    }

    // inclusive, faster than ray - works on empty set
    pub fn fill_north(self) -> Bitboard {
        let mut bb = self;
        bb |= Bitboard::from_bits_truncate(bb.bits << 32);
        bb |= Bitboard::from_bits_truncate(bb.bits << 16);
        bb |= Bitboard::from_bits_truncate(bb.bits << 8);
        bb
    }

    // all points south inclusive - works on empty set
    pub fn fill_south(self) -> Bitboard {
        let mut bb = self;
        bb |= Bitboard::from_bits_truncate(bb.bits >> 32);
        bb |= Bitboard::from_bits_truncate(bb.bits >> 16);
        bb |= Bitboard::from_bits_truncate(bb.bits >> 8);
        bb
    }

    pub fn file_flood(self) -> Bitboard {
        self.fill_north() | self.fill_south() | self
    }

    pub fn diag_flood(self) -> Bitboard {
        self.rays(Dir::NE) | self.rays(Dir::SW) | self
    }

    pub fn anti_diag_flood(self) -> Bitboard {
        self.rays(Dir::NW) | self.rays(Dir::SE) | self
    }
    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        Self::from_bits_truncate(self.bits | other.bits)
    }

    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn xor(self, other: Self) -> Self {
        Self::from_bits_truncate(self.bits ^ other.bits)
    }

    #[inline]
    pub const fn popcount(self) -> i32 {
        self.bits.count_ones() as i32
    }

    /// flip vertical - https://www.chessprogramming.org/Flipping_Mirroring_and_Rotating
    /// named flip_vertical rather than swap_bytes to match square ^ 56
    #[inline]
    pub const fn flip_vertical(self) -> Self {
        Bitboard::from_bits_truncate(self.bits.swap_bytes())
    }

    // 
    #[inline]
    pub const fn wrapping_sub(self, other: Bitboard) -> Self {
        Bitboard::from_bits_truncate(self.bits.wrapping_sub(other.bits))
    }

    // #[inline]
    // pub fn includes(self, sq: Square) -> bool {
    //     self.intersects(sq.as_bb())
    // }


    #[inline]
    pub fn exclude(self, sq: Square) -> Bitboard {
        self - sq.as_bb()
    }

    #[inline]
    pub fn include(self, sq: Square) -> Bitboard {
        self | sq.as_bb()
    }

    #[inline]
    pub const fn square(self) -> Square {
        // debug_assert_eq!(self.popcount(), 1, "Attempt to convert bb {} to a square", self);
        let sq = self.bits.trailing_zeros();
        // debug_assert!(sq < 64);
        Square::from_u32(sq)
    }

    #[inline]
    pub fn last_square(self) -> Square {
        debug_assert!(!self.is_empty(), "bb.last_square on empty");
        let msb = self.bits.leading_zeros();
        debug_assert!(msb < 64);
        Square::from_u32(63 - msb)
    }

    #[inline]
    pub fn first_square(self) -> Square {
        debug_assert!(!self.is_empty(), "bb.first_square on empty");
        // LSB
        let sq = self.bits.trailing_zeros();
        debug_assert!(sq < 64);
        Square::from_u32(sq)
    }

    #[inline]
    pub fn last(self) -> Self {
        debug_assert!(!self.is_empty(), "bb.last on empty");
        Bitboard::from_bits_truncate(1 << self.last_square().index()) // MSb
    }

    #[inline]
    pub fn first(self) -> Self {
        debug_assert!(!self.is_empty(), "bb.first on empty");
        Bitboard::from_bits_truncate(1 << self.first_square().index()) // LSb
    }



    #[inline]
    pub const fn iter(self) -> BitIterator {
        BitIterator { bb: self }
    }

    #[inline]
    pub const fn squares(self) -> Squares {
        Squares { bb: self }
    }

    // carry rippler from https://www.chessprogramming.org/Traversing_Subsets_of_a_Set
    #[inline]
    pub const fn power_set_iter(self) -> PowerSetIterator {
        PowerSetIterator::new(self)
    }

    pub fn files_string(self) -> String {
        let mut files: Vec<char> = self.iter().map(|bb| bb.first_square().file_char()).collect();
        files.sort_unstable();
        files.dedup();
        files.iter().collect()
    }

    pub fn ranks_string(self) -> String {
        let mut ranks: Vec<char> = self.iter().map(|bb| bb.first_square().rank_char()).collect();
        ranks.sort_unstable();
        ranks.dedup();
        ranks.iter().collect()
    }

    // pub fn sq_as_file(sq: Square) -> char {
    //     let x = sq.index() % 8;
    //     char::from(b'a' + x as u8)
    // }

    // pub fn sq_as_rank(sq: Square) -> char {
    //     let y = sq.index() / 8;
    //     char::from(b'1' + y as u8)
    // }

    pub fn sq_as_uci(self) -> String {
        let s = self.first_square();
        format!("{}{}", s.file_char(), s.rank_char())
    }

    pub fn uci(self) -> String {
        let strings: Vec<String> = self.iter().map(Self::sq_as_uci).collect();
        strings.join("+")
    }

    pub fn parse_rank(s: &str) -> Result<Bitboard, String> {
        match s.chars().next() {
            Some(ch) if ('1'..='8').contains(&ch) => Ok(Self::RANKS[ch as usize - b'1' as usize]),
            _ => Err(format!("Invalid rank '{}'", s)),
        }
    }

    pub fn parse_file(s: &str) -> Result<Bitboard, String> {
        match s.chars().next() {
            Some(ch) if ('a'..='h').contains(&ch) => Ok(Self::FILES[ch as usize - b'a' as usize]),
            _ => Err(format!("Invalid file '{}'", s)),
        }
    }

    pub fn parse_squares(s: &str) -> Result<Bitboard, String> {
        let s = s.replace(",", " ");
        let mut bb = Bitboard::empty();
        for sq_str in s.split_ascii_whitespace() {
            let sq = Self::parse_square(sq_str)?;
            bb |= sq.as_bb()
        }
        Ok(bb)
    }

    pub fn parse_square(s: &str) -> Result<Square, String> {
        if s.len() != 2 {
            return Err(format!("Invalid square '{}'", s));
        }
        let chars: Vec<&str> = s.split("").collect(); // gives empty [0]
        let bb = Self::parse_file(chars[1])? & Self::parse_rank(chars[2])?;
        Ok(bb.square())
    }
}

// https://www.chessprogramming.org/Traversing_Subsets_of_a_Set
#[derive(Copy, Clone, Debug)]
pub struct PowerSetIterator {
    d: Bitboard, // we're iterating subsets of d
    n: Bitboard, // next subset
    completed: bool,
}

impl PowerSetIterator {
    const fn new(d: Bitboard) -> Self {
        Self {
            n: Bitboard::EMPTY,
            d,
            completed: false,
        }
    }
}

impl Iterator for PowerSetIterator {
    type Item = Bitboard;

    #[inline]
    fn next(&mut self) -> Option<Bitboard> {
        if self.completed {
            return None;
        }
        let last = self.n;
        self.n = Bitboard::from_bits_truncate(self.n.bits().wrapping_sub(self.d.bits())) & self.d;
        self.completed = self.n.is_empty();
        Some(last)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = 1 << self.d.popcount() as usize;
        (n, Some(n))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BitIterator {
    bb: Bitboard,
}

impl Iterator for BitIterator {
    type Item = Bitboard;

    #[inline]
    fn next(&mut self) -> Option<Bitboard> {
        if self.bb.is_empty() {
            None
        } else {
            let sq = self.bb.first();
            self.bb.remove(sq);
            Some(sq)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bitcount = self.bb.popcount() as usize;
        (bitcount, Some(bitcount))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Squares {
    bb: Bitboard,
}

impl Iterator for Squares {
    type Item = Square;

    #[inline]
    fn next(&mut self) -> Option<Square> {
        if self.bb.is_empty() {
            None
        } else {
            let sq = self.bb.bits.trailing_zeros();
            self.bb.bits ^= 1 << sq;
            Some(Square::from_u32(sq))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bitcount = self.bb.popcount() as usize;
        (bitcount, Some(bitcount))
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in (0..8).rev() {
            for f in 0..8 {
                let bit = 1 << (r * 8 + f);
                fmt.write_str(if self.contains(Bitboard::from_bits_truncate(bit)) {
                    "1 "
                } else {
                    ". "
                })?;
            }
            fmt.write_char('\n')?;
        }
        Ok(())
    }
}

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
        Bitboard::from_bits_truncate(1 << self.0)
    }

    #[inline]
    pub fn is_in(self, bb: Bitboard) -> bool {
        self.as_bb().intersects(bb)
    }

    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 64u8
    }

    #[inline]
    pub fn shift(self, dir: Dir) -> Square {
        debug_assert!(self.0 as i32 + dir.shift >=0 && self.0 as i32 + dir.shift < 64 );
        Square((self.0 as i32 + dir.shift) as u8)
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
        } else 
        if s2.is_in(s1.file()) {
            s1.file()
        } else
        if s2.is_in(s1.rank()) {
            s1.rank()
        } else 
        if s2.is_in(s1.diag()) {
            s1.diag()
        } else
        if s2.is_in(s1.anti_diag()) {
            s1.anti_diag()
        } else {
            Bitboard::empty()
        }
    }


    /// flip vertical - https://www.chessprogramming.org/Flipping_Mirroring_and_Rotating
    #[inline]
    pub const fn flip_vertical(self) -> Square {
        Square( self.0 ^ 56 )
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
        &self[s.index()]
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

    #[allow(non_upper_case_globals)]
    const a1b2: Bitboard = Bitboard::A1.or(Bitboard::B2);

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
    }

    #[test]
    fn test_bitwise() {
        assert!(a1b2.contains(a1));
        assert!(a1b2 & c1 == a1 - a1);
        assert!(a1b2 - a1 == b2);
        assert!(!a1b2.is_empty());
        assert!(a1b2.intersects(b2));
        assert_eq!(Bitboard::all(), !Bitboard::empty());
        assert!(Bitboard::FILE_A.contains(a4));
        assert_eq!(Bitboard::FILE_A.popcount(), 8);
        assert_eq!(Bitboard::all().popcount(), 64);
        assert_eq!(
            (Bitboard::FILE_A | Bitboard::RANK_1).flip_vertical(),
            (Bitboard::FILE_A | Bitboard::RANK_8)
        );
    }

    #[test]
    fn test_rays() {
        let north = c3.rays(Dir::N);
        assert_eq!(north, c4 | c5 | c6 | c7 | c8);
        assert_eq!(north.popcount(), 5);

        assert_eq!(c3.rays(Dir::NE), d4 | e5 | f6 | g7 | h8);
        assert_eq!(c3.rays(Dir::SW), a1 | b2);
        assert_eq!(c3.rays(Dir::S), c1 | c2);
        assert_eq!(c3.rays(Dir::NW), a5 | b4);
    }

    #[test]
    fn test_floods_and_fills() {
        assert_eq!(a1b2.fill_north(), (FILE_A | FILE_B) - b1);
        assert_eq!(a1b2.fill_south(), a1b2 | b1);
        assert_eq!(a1b2.file_flood(), FILE_A | FILE_B);
        let main_diag = a1 | b2 | c3 | d4 | e5 | f6 | g7 | h8;
        assert_eq!(a1b2.diag_flood(), main_diag);
        assert_eq!(main_diag.file_flood(), Bitboard::all());
        assert_eq!(a1b2.anti_diag_flood(), a1 | b2 | a3 | c1);
    }

    #[test]
    fn test_bounding_rectangle() {
        assert_eq!(cmp::min(a1.square(), b2.square()), a1.square());
        assert_eq!(Bitboard::all() - b2.square().rank().rays(Dir::N), RANK_1 | RANK_2);
        assert_eq!(Square::bounding_rectangle(a1.square(), b2.square()), a1 | a2 | b1 | b2);
        assert_eq!(Square::bounding_rectangle(b2.square(), a1.square()), a1 | a2 | b1 | b2);
        assert_eq!(Square::bounding_rectangle(a1.square(), a1.square()), a1);
        assert_eq!(Square::bounding_rectangle(c3.square(), c3.square()), c3);
        assert_eq!(Square::bounding_rectangle(a1.square(), h8.square()), Bitboard::all());
        assert_eq!(Square::bounding_rectangle(b2.square(), b5.square()), b2| b3| b4| b5);
        assert_eq!(Square::bounding_rectangle(b5.square(), b2.square()), b2| b3| b4| b5);
        assert_eq!(Square::bounding_rectangle(c5.square(), e5.square()), c5| d5| e5);
        assert_eq!(Square::bounding_rectangle(e5.square(), c5.square()), c5| d5| e5);
    }


    #[test]
    fn test_line_though() {
        assert_eq!(Square::calc_line_through(b6.square(), b8.square()), FILE_B);
        assert_eq!(Square::calc_line_through(b5.square(), d5.square()), RANK_5);
        assert_eq!(Square::calc_line_through(a2.square(), b1.square()), a2 | b1);
        assert_eq!(Square::calc_line_through(f1.square(), g2.square()), f1|g2|h3);
        assert_eq!(Square::calc_line_through(f1.square(), f1.square()), f1);
        assert_eq!(Square::calc_line_through(f1.square(), g3.square()), Bitboard::empty());
    }

    #[test]
    fn test_froms() {
        assert_eq!(Bitboard::from_xy(4, 7), e8);
        assert_eq!(Bitboard::from_sq(63), h8);
        assert_eq!(Bitboard::from_sq(8), a2);
    }

    #[test]
    fn test_parse() {
        assert_eq!(Bitboard::parse_file("a").unwrap(), Bitboard::FILE_A);
        assert_eq!(Bitboard::parse_file("h").unwrap(), Bitboard::FILE_H);
        assert_eq!(Bitboard::parse_rank("1").unwrap(), Bitboard::RANK_1);
        assert_eq!(Bitboard::parse_rank("8").unwrap(), Bitboard::RANK_8);
        assert_eq!(Bitboard::parse_square("a1").unwrap(), a1.square());
        assert_eq!(Bitboard::parse_square("a8").unwrap(), a8.square());
        assert_eq!(Bitboard::parse_square("h8").unwrap(), h8.square());

        assert_eq!(Bitboard::parse_squares("h8 h1").unwrap(), h8|h1);
        assert_eq!(Bitboard::parse_squares("a1, a2,a3  ").unwrap(), a1|a2|a3);
        assert_eq!(Bitboard::parse_squares("").unwrap(), Bitboard::empty());
    }

    #[test]
    fn test_parse_fail() {
        assert_eq!(Bitboard::parse_file("9").err(), Some("Invalid file '9'".into()));
        assert_eq!(Bitboard::parse_file("").err(), Some("Invalid file ''".into()));
        assert_eq!(Bitboard::parse_rank("a").err(), Some("Invalid rank 'a'".into()));
        assert_eq!(
            Bitboard::parse_square("aa").err(),
            Some("Invalid rank 'a'".into())
        );
        assert_eq!(
            Bitboard::parse_square("11").err(),
            Some("Invalid file '1'".into())
        );
        assert_eq!(Bitboard::parse_square("").err(), Some("Invalid square ''".into()));
        assert_eq!(
            Bitboard::parse_square("abc").err(),
            Some("Invalid square 'abc'".into())
        );
    }

    #[test]
    fn test_firsts_and_lasts() {
        assert_eq!(Bitboard::RANK_2.popcount(), 8);
        assert_eq!(a1b2.popcount(), 2);
        assert_eq!(a1b2.first_square().index(), 0);
        assert_eq!(a1b2.last_square().index(), 9);
        assert_eq!((Bitboard::A1 | Bitboard::A2).last_square().index(), 8);
        // FIXME!
        // assert_eq!(Bitboard::EMPTY.first_square(), 64);
        // assert_eq!(Bitboard::EMPTY.last_square(), 64);
    }
    // let result = std::panic::catch_unwind(|| Bitboard::EMPTY.as_sq() );
    // assert!(result.is_err());
    #[test]
    fn test_shifts() {
        let a2b3 = a1b2.shift(Dir::N);
        assert_eq!(a2b3, Bitboard::A2 | Bitboard::B3);
        assert!(Bitboard::D8.shift(Dir::N).is_empty());
        assert_eq!(Bitboard::D8.shift(Dir::E), Bitboard::E8);
    }

    #[test]
    fn test_formats() {
        assert_eq!(a1.files_string(), "a");
        assert_eq!((a1 | b1 | c1).files_string(), "abc");
        assert_eq!(Bitboard::all().files_string(), "abcdefgh");

        assert_eq!(a1.ranks_string(), "1");
        assert_eq!((a1 | b5 | e5).ranks_string(), "15");
        assert_eq!(Bitboard::all().ranks_string(), "12345678");

        assert_eq!(a1.sq_as_uci(), "a1");
        assert_eq!(h1.sq_as_uci(), "h1");
        assert_eq!(a8.sq_as_uci(), "a8");
        assert_eq!(a1b2.uci(), "a1+b2");
        assert_eq!(format!("{}", a1b2), ". . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. 1 . . . . . . \n1 . . . . . . . \n");
        assert_eq!(format!("{:?}", a1b2), "A1 | B2");
        assert_eq!(
            format!("{:?}", Bitboard::FILE_A),
            "A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 | FILE_A"
        );
        // assert_eq!(format!("{:?}", Bitboard::EDGES), "");
        assert_eq!(format!("{:b}", a1b2), "1000000001");
    }

    #[test]
    fn test_directions() {
        let dir = Dir::N;
        assert_eq!(dir.shift, 8);
        assert_eq!(Dir::ALL[0], Dir::N);
        assert_eq!(
            format!("{:?}", Dir::N),
            "Dir { index: 0, shift: 8, mask: A8 | B8 | C8 | D8 | E8 | F8 | G8 | H8 | RANK_8 }"
        );
    }

    #[test]
    fn test_iterators() {
        let a1b1g5 = a1 | c1 | g5;
        let mut i = a1b1g5.iter();
        assert_eq!(i.next(), Some(a1));
        assert_eq!(i.next(), Some(c1));
        assert_eq!(i.next(), Some(g5));
        assert_eq!(i.next(), None);
        assert_eq!(a1b1g5.iter().count(), 3);

        let mut sqs = a1b1g5.squares();
        assert_eq!(sqs.next(), Some(a1.square()));
        assert_eq!(sqs.next(), Some(c1.square()));
        assert_eq!(sqs.next(), Some(g5.square()));
        assert_eq!(sqs.next(), None);
        assert_eq!(a1b1g5.squares().count(), 3);

        let power_sets = a1b1g5.power_set_iter();
        power_sets.clone().for_each(|bb| println!("{:?}", bb));
        assert_eq!(power_sets.clone().reduce(|a, b| a | b), Some(a1b1g5));
        assert_eq!(power_sets.clone().count(), 1 << 3);
        assert_eq!(power_sets.clone().max(), Some(a1b1g5));

        let power_sets = Bitboard::FILE_A.power_set_iter();
        assert_eq!(power_sets.clone().count(), 1 << 8);
        assert_eq!(
            power_sets.clone().fold(Bitboard::EMPTY, |acc, bb| acc | bb),
            Bitboard::FILE_A
        );
        assert_eq!(
            power_sets.clone().filter(|bb| bb.popcount() == 2).count(),
            7 * 8 / 2
        );
        assert_eq!(power_sets.clone().filter(|bb| bb.popcount() == 7).count(), 8);
    }
}
