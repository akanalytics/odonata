use std::fmt::{self, Write};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Dir {
    pub index: usize,
    pub shift: i32,
    pub mask: Bitboard, // mask for opposite edge(s)
}

impl Dir {
    pub const N: Self = Dir { index: 0, shift: 8, mask: Bitboard::RANK_8 };
    pub const NE: Self = Dir { index: 1, shift: 9, mask: Bitboard::RANK_8.or(Bitboard::FILE_H) };
    pub const E: Self = Dir { index: 2, shift: 1, mask: Bitboard::FILE_H };
    pub const SE: Self = Dir { index: 3, shift: -7, mask: Bitboard::RANK_1.or(Bitboard::FILE_H) };
    pub const S: Self = Dir { index: 4, shift: -8, mask: Bitboard::RANK_1 };
    pub const SW: Self = Dir { index: 5, shift: -9, mask: Bitboard::RANK_1.or(Bitboard::FILE_A) };
    pub const W: Self = Dir { index: 6, shift: -1, mask: Bitboard::FILE_A };
    pub const NW: Self = Dir { index: 7, shift: 7, mask: Bitboard::RANK_8.or(Bitboard::FILE_A) };

    pub const ALL: [Self; 8] = [Self::N, Self::NE, Self::E, Self::SE, Self::S, Self::SW, Self::W, Self::NW];

    pub fn opposite(&self) -> &Dir {
        &Self::ALL[(self.index + 4) % 8]
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
    // all, 
    // empty, 
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
    pub fn from_sq(index: u32) -> Bitboard {
        let bit = 1 << index;
        Bitboard::from_bits_truncate(bit)
    }

    #[inline]
    pub fn shift(self, dir: &Dir) -> Bitboard {
        let bb = self - dir.mask;
        if dir.shift > 0 {
            Bitboard::from_bits_truncate(bb.bits << dir.shift)
        } else {
            Bitboard::from_bits_truncate(bb.bits >> -dir.shift)
        }
    }


    pub fn fill_north(self) -> Bitboard {
        let mut bb = self;
        bb |= Bitboard::from_bits_truncate(bb.bits << 32);
        bb |= Bitboard::from_bits_truncate(bb.bits << 16);
        bb |= Bitboard::from_bits_truncate(bb.bits << 8);
        bb
    }
    
    pub fn fill_south(self) -> Bitboard {
        let mut bb = self;
        bb |= Bitboard::from_bits_truncate(bb.bits >> 32);
        bb |= Bitboard::from_bits_truncate(bb.bits >> 16);
        bb |= Bitboard::from_bits_truncate(bb.bits >> 8);
        bb
    }


    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn or(self, other: Bitboard) -> Bitboard {
        Bitboard::from_bits_truncate(self.bits | other.bits)
    }

    #[inline]
    pub fn popcount(self) -> i32 {
        self.bits.count_ones() as i32
    }

    /// flip vertical
    #[inline]
    pub fn swap_bytes(self) -> Self {
        Bitboard::from_bits_truncate(self.bits.swap_bytes())
    }

    // pub fn as_sq(self) -> usize {
    //     // LSB
    //     self.bits.trailing_zeros() as usize
    // }

    #[inline]
    pub fn last_square(self) -> usize {
        let msb = self.bits.leading_zeros() as usize;
        if msb < 64 {
            63 - msb
        } else {
            panic!("last_square of 0");
        }
    }

    #[inline]
    pub fn first_square(self) -> usize {
        // LSB
        self.bits.trailing_zeros() as usize
    }

    #[inline]
    pub fn last(self) -> Self {
        Bitboard::from_bits_truncate(1 << self.last_square()) // MSb
    }

    #[inline]
    pub fn first(self) -> Self {
        Bitboard::from_bits_truncate(1 << self.first_square()) // LSb
    }

    #[inline]
    pub fn iter(self) -> BitIterator {
        BitIterator { bb: self }
    }

    pub fn files(self) -> String {
        let mut files: Vec<char> = self.iter().map(|bb| Self::sq_as_file(bb.first_square())).collect();
        files.sort_unstable();
        files.dedup();
        files.iter().collect()
    }

    pub fn ranks(self) -> String {
        let mut ranks: Vec<char> = self.iter().map(|bb| Self::sq_as_rank(bb.first_square())).collect();
        ranks.sort_unstable();
        ranks.dedup();
        ranks.iter().collect()
    }

    pub fn sq_as_file(sq: usize) -> char {
        let x = sq % 8;
        char::from(b'a' + x as u8)
    }

    pub fn sq_as_rank(sq: usize) -> char {
        let y = sq / 8;
        char::from(b'1' + y as u8)
    }

    pub fn sq_as_uci(self) -> String {
        let s = self.first_square();
        let f = Self::sq_as_file(s);
        let r = Self::sq_as_rank(s);
        format!("{}{}", f, r)
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

    pub fn parse_square(s: &str) -> Result<Bitboard, String> {
        if s.len() != 2 {
            return Err(format!("Invalid square '{}'", s));
        }
        let chars: Vec<&str> = s.split("").collect(); // gives empty [0]
        Ok(Self::parse_file(chars[1])? & Self::parse_rank(chars[2])?)
    }
}

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
                fmt.write_str(if self.contains(Bitboard::from_bits_truncate(bit)) { "1 " } else { ". " })?;
            }
            fmt.write_char('\n')?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[allow(non_upper_case_globals)]
    const a1b2: Bitboard = Bitboard::A1.or(Bitboard::B2);

    use super::*;
    use crate::globals::constants::*;

    #[test]
    fn test_bitwise() {
        assert!(a1b2.contains(a1));
        assert!(a1b2 & c1 == a1 - a1);
        assert!(a1b2 - a1 == b2);
        assert!(!a1b2.is_empty());
        assert!(a1b2.intersects(b2));
        assert_eq!(a1b2.fill_north(), (FILE_A | FILE_B) - b1);
        assert_eq!(a1b2.fill_south(), a1b2 | b1);
        assert_eq!(Bitboard::all(), !Bitboard::empty());
        assert!(Bitboard::FILE_A.contains(a4));
        assert_eq!(Bitboard::FILE_A.popcount(), 8);
        assert_eq!(Bitboard::all().popcount(), 64);
        assert_eq!((Bitboard::FILE_A | Bitboard::RANK_1).swap_bytes(), (Bitboard::FILE_A | Bitboard::RANK_8));
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
        assert_eq!(Bitboard::parse_square("a1").unwrap(), a1);
        assert_eq!(Bitboard::parse_square("a8").unwrap(), a8);
        assert_eq!(Bitboard::parse_square("h8").unwrap(), h8);
    }

    #[test]
    fn test_parse_fail() {
        assert_eq!(Bitboard::parse_file("9").err(), Some("Invalid file '9'".into()));
        assert_eq!(Bitboard::parse_file("").err(), Some("Invalid file ''".into()));
        assert_eq!(Bitboard::parse_rank("a").err(), Some("Invalid rank 'a'".into()));
        assert_eq!(Bitboard::parse_square("aa").err(), Some("Invalid rank 'a'".into()));
        assert_eq!(Bitboard::parse_square("11").err(), Some("Invalid file '1'".into()));
        assert_eq!(Bitboard::parse_square("").err(), Some("Invalid square ''".into()));
        assert_eq!(Bitboard::parse_square("abc").err(), Some("Invalid square 'abc'".into()));
    }

    #[test]
    fn test_firsts_and_lasts() {
        assert_eq!(Bitboard::RANK_2.popcount(), 8);
        assert_eq!(a1b2.popcount(), 2);
        assert_eq!(a1b2.first_square(), 0);
        assert_eq!(a1b2.last_square(), 9);
        assert_eq!((Bitboard::A1 | Bitboard::A2).last_square(), 8);
        // FIXME!
        // assert_eq!(Bitboard::EMPTY.first_square(), 64);
        // assert_eq!(Bitboard::EMPTY.last_square(), 64);
    }
    // let result = std::panic::catch_unwind(|| Bitboard::EMPTY.as_sq() );
    // assert!(result.is_err());
    #[test]
    fn test_shifts() {
        let a2b3 = a1b2.shift(&Dir::N);
        assert_eq!(a2b3, Bitboard::A2 | Bitboard::B3);
        assert!(Bitboard::D8.shift(&Dir::N).is_empty());
        assert_eq!(Bitboard::D8.shift(&Dir::E), Bitboard::E8);
    }

    #[test]
    fn test_formats() {
        assert_eq!(a1.files(), "a");
        assert_eq!((a1 | b1 | c1).files(), "abc");
        assert_eq!(Bitboard::all().files(), "abcdefgh");

        assert_eq!(a1.ranks(), "1");
        assert_eq!((a1 | b5 | e5).ranks(), "15");
        assert_eq!(Bitboard::all().ranks(), "12345678");

        assert_eq!(a1.sq_as_uci(), "a1");
        assert_eq!(h1.sq_as_uci(), "h1");
        assert_eq!(a8.sq_as_uci(), "a8");
        assert_eq!(a1b2.uci(), "a1+b2");
        assert_eq!(format!("{}", a1b2), ". . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. 1 . . . . . . \n1 . . . . . . . \n");
        assert_eq!(format!("{:?}", a1b2), "A1 | B2");
        assert_eq!(format!("{:?}", Bitboard::FILE_A), "A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 | FILE_A");
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

    // #[test]
    // fn test_iterator() {
    //     let mut a1b1c1 = a1 | b1 | c1;
    //     assert_eq!(a1b1c1.next(), Some(a1));
    //     assert_eq!(a1b1c1.next(), Some(b1));
    //     assert_eq!(a1b1c1.next(), Some(c1));
    //     assert_eq!(a1b1c1.next(), None);
    // }

    #[test]
    fn test_iterator() {
        let a1b1c1 = a1 | b1 | c1;
        let mut i = a1b1c1.iter();
        assert_eq!(i.next(), Some(a1));
        assert_eq!(i.next(), Some(b1));
        assert_eq!(i.next(), Some(c1));
        assert_eq!(i.next(), None);
        assert_eq!(a1b1c1.iter().count(), 3);
    }
}
