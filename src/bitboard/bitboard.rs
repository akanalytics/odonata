use crate::bitboard::square::Square;
use crate::types::Color;
use anyhow::{anyhow, Result};
use std::fmt::{self, Write};
use std::ops;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]

pub struct Dir {
    pub index: u32,
    // pub shift: i8,
    // pub mask: Bitboard, // mask for opposite edge(s)
}

impl Dir {
    pub const N: Self = Dir {
        index: 0,
        // shift: 8,
        // mask: Bitboard::RANK_8,
    };
    pub const NE: Self = Dir {
        index: 1,
        // shift: 9,
        // mask: Bitboard::RANK_8.or(Bitboard::FILE_H),
    };
    pub const E: Self = Dir {
        index: 2,
        // shift: 1,
        // mask: Bitboard::FILE_H,
    };
    pub const SE: Self = Dir {
        index: 3,
        // shift: -7,
        // mask: Bitboard::RANK_1.or(Bitboard::FILE_H),
    };
    pub const S: Self = Dir {
        index: 4,
        // shift: -8,
        // mask: Bitboard::RANK_1,
    };
    pub const SW: Self = Dir {
        index: 5,
        // shift: -9,
        // mask: Bitboard::RANK_1.or(Bitboard::FILE_A),
    };
    pub const W: Self = Dir {
        index: 6,
        // shift: -1,
        // mask: Bitboard::FILE_A,
    };
    pub const NW: Self = Dir {
        index: 7,
        // shift: 7,
        // mask: Bitboard::RANK_8.or(Bitboard::FILE_A),
    };

    pub const ALL: [Self; 8] = [Self::N, Self::NE, Self::E, Self::SE, Self::S, Self::SW, Self::W, Self::NW];

    #[inline]
    pub const fn shift(self) -> i8 {
        // self.shift
        // [ 8,
        //  9,
        //  1,
        //  -7,
        //  -8,
        //  -9,
        //  -1,
        //  7][self.index()]

        match self {
            Self::N => 8,
            Self::NE => 9,
            Self::E => 1,
            Self::SE => -7,
            Self::S => -8,
            Self::SW => -9,
            Self::W => -1,
            Self::NW => 7,
            _ => 0,
        }
    }

    #[inline]
    pub fn bits_to_rotate(self) -> u32 {
        [8, 9, 1, 64 - 7, 64 - 8, 64 - 9, 64 - 1, 7][self]

        // match self {
        //         Self::N => 8,
        //         Self::NE => 9,
        //         Self::E => 1,
        //         Self::SE => 64-7,
        //         Self::S => 64-8,
        //         Self::SW => 64-9,
        //         Self::W => 64-1,
        //         Self::NW => 7,
        //         _ => 0
        //     }
    }

    #[inline]
    pub const fn mask(self) -> Bitboard {
        // self.mask
        [
            Bitboard::RANK_8,
            Bitboard::RANK_8.or(Bitboard::FILE_H),
            Bitboard::FILE_H,
            Bitboard::RANK_1.or(Bitboard::FILE_H),
            Bitboard::RANK_1,
            Bitboard::RANK_1.or(Bitboard::FILE_A),
            Bitboard::FILE_A,
            Bitboard::RANK_8.or(Bitboard::FILE_A),
        ][self.index()]

        // match self {
        //     Self::N => Bitboard::RANK_8,
        //     Self::NE => Bitboard::RANK_8.or(Bitboard::FILE_H),
        //     Self::E => Bitboard::FILE_H,
        //     Self::SE => Bitboard::RANK_1.or(Bitboard::FILE_H),
        //     Self::S => Bitboard::RANK_1,
        //     Self::SW => Bitboard::RANK_1.or(Bitboard::FILE_A),
        //     Self::W => Bitboard::FILE_A,
        //     Self::NW => Bitboard::RANK_8.or(Bitboard::FILE_A),
        //     _ => Bitboard::EMPTY
        // }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self.index as usize
    }

    #[inline]
    pub const fn rotate_clockwise(self) -> Dir {
        Dir::ALL[(self.index() + 1) % 8]
    }

    #[inline]
    pub const fn opposite(self) -> Dir {
        Self::ALL[(self.index() + 4) % 8]
    }
}

impl<T> std::ops::Index<Dir> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, i: Dir) -> &Self::Output {
        #[cfg(feature = "unchecked_indexing")]
        unsafe {
            &self.get_unchecked(i.index())
        }

        #[cfg(not(feature = "unchecked_indexing"))]
        &self[(i.index())]
    }
}

impl<T> std::ops::IndexMut<Dir> for [T] {
    #[inline]
    fn index_mut(&mut self, d: Dir) -> &mut Self::Output {
        &mut self[d.index()]
    }
}

#[derive(Copy, Clone, Default, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct Bitboard(u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const A1: Bitboard = Bitboard(1 << 0);
    pub const B1: Bitboard = Bitboard(1 << 1);
    pub const C1: Bitboard = Bitboard(1 << 2);
    pub const D1: Bitboard = Bitboard(1 << 3);
    pub const E1: Bitboard = Bitboard(1 << 4);
    pub const F1: Bitboard = Bitboard(1 << 5);
    pub const G1: Bitboard = Bitboard(1 << 6);
    pub const H1: Bitboard = Bitboard(1 << 7);
    pub const A2: Bitboard = Bitboard(1 << 8);
    pub const B2: Bitboard = Bitboard(1 << 9);
    pub const C2: Bitboard = Bitboard(1 << 10);
    pub const D2: Bitboard = Bitboard(1 << 11);
    pub const E2: Bitboard = Bitboard(1 << 12);
    pub const F2: Bitboard = Bitboard(1 << 13);
    pub const G2: Bitboard = Bitboard(1 << 14);
    pub const H2: Bitboard = Bitboard(1 << 15);
    pub const A3: Bitboard = Bitboard(1 << 16);
    pub const B3: Bitboard = Bitboard(1 << 17);
    pub const C3: Bitboard = Bitboard(1 << 18);
    pub const D3: Bitboard = Bitboard(1 << 19);
    pub const E3: Bitboard = Bitboard(1 << 20);
    pub const F3: Bitboard = Bitboard(1 << 21);
    pub const G3: Bitboard = Bitboard(1 << 22);
    pub const H3: Bitboard = Bitboard(1 << 23);
    pub const A4: Bitboard = Bitboard(1 << 24);
    pub const B4: Bitboard = Bitboard(1 << 25);
    pub const C4: Bitboard = Bitboard(1 << 26);
    pub const D4: Bitboard = Bitboard(1 << 27);
    pub const E4: Bitboard = Bitboard(1 << 28);
    pub const F4: Bitboard = Bitboard(1 << 29);
    pub const G4: Bitboard = Bitboard(1 << 30);
    pub const H4: Bitboard = Bitboard(1 << 31);
    pub const A5: Bitboard = Bitboard(1 << 32);
    pub const B5: Bitboard = Bitboard(1 << 33);
    pub const C5: Bitboard = Bitboard(1 << 34);
    pub const D5: Bitboard = Bitboard(1 << 35);
    pub const E5: Bitboard = Bitboard(1 << 36);
    pub const F5: Bitboard = Bitboard(1 << 37);
    pub const G5: Bitboard = Bitboard(1 << 38);
    pub const H5: Bitboard = Bitboard(1 << 39);
    pub const A6: Bitboard = Bitboard(1 << 40);
    pub const B6: Bitboard = Bitboard(1 << 41);
    pub const C6: Bitboard = Bitboard(1 << 42);
    pub const D6: Bitboard = Bitboard(1 << 43);
    pub const E6: Bitboard = Bitboard(1 << 44);
    pub const F6: Bitboard = Bitboard(1 << 45);
    pub const G6: Bitboard = Bitboard(1 << 46);
    pub const H6: Bitboard = Bitboard(1 << 47);
    pub const A7: Bitboard = Bitboard(1 << 48);
    pub const B7: Bitboard = Bitboard(1 << 49);
    pub const C7: Bitboard = Bitboard(1 << 50);
    pub const D7: Bitboard = Bitboard(1 << 51);
    pub const E7: Bitboard = Bitboard(1 << 52);
    pub const F7: Bitboard = Bitboard(1 << 53);
    pub const G7: Bitboard = Bitboard(1 << 54);
    pub const H7: Bitboard = Bitboard(1 << 55);
    pub const A8: Bitboard = Bitboard(1 << 56);
    pub const B8: Bitboard = Bitboard(1 << 57);
    pub const C8: Bitboard = Bitboard(1 << 58);
    pub const D8: Bitboard = Bitboard(1 << 59);
    pub const E8: Bitboard = Bitboard(1 << 60);
    pub const F8: Bitboard = Bitboard(1 << 61);
    pub const G8: Bitboard = Bitboard(1 << 62);
    pub const H8: Bitboard = Bitboard(1 << 63);

    pub const FILE_A: Bitboard = Bitboard(
        Self::A1.bits()
            | Self::A2.bits()
            | Self::A3.bits()
            | Self::A4.bits()
            | Self::A5.bits()
            | Self::A6.bits()
            | Self::A7.bits()
            | Self::A8.bits(),
    );
    pub const RANK_1: Bitboard = Bitboard(
        Self::A1.bits()
            | Self::B1.bits()
            | Self::C1.bits()
            | Self::D1.bits()
            | Self::E1.bits()
            | Self::F1.bits()
            | Self::G1.bits()
            | Self::H1.bits(),
    );
    pub const FILE_B: Bitboard = Bitboard(Self::FILE_A.bits() << 1);
    pub const RANK_2: Bitboard = Bitboard(Self::RANK_1.bits() << 8);
    pub const FILE_C: Bitboard = Bitboard(Self::FILE_A.bits() << 2);
    pub const RANK_3: Bitboard = Bitboard(Self::RANK_1.bits() << (2 * 8));
    pub const FILE_D: Bitboard = Bitboard(Self::FILE_A.bits() << 3);
    pub const RANK_4: Bitboard = Bitboard(Self::RANK_1.bits() << (3 * 8));
    pub const FILE_E: Bitboard = Bitboard(Self::FILE_A.bits() << 4);
    pub const RANK_5: Bitboard = Bitboard(Self::RANK_1.bits() << (4 * 8));
    pub const FILE_F: Bitboard = Bitboard(Self::FILE_A.bits() << 5);
    pub const RANK_6: Bitboard = Bitboard(Self::RANK_1.bits() << (5 * 8));
    pub const FILE_G: Bitboard = Bitboard(Self::FILE_A.bits() << 6);
    pub const RANK_7: Bitboard = Bitboard(Self::RANK_1.bits() << (6 * 8));
    pub const FILE_H: Bitboard = Bitboard(Self::FILE_A.bits() << 7);
    pub const RANK_8: Bitboard = Bitboard(Self::RANK_1.bits() << (7 * 8));

    // https://gekomad.github.io/Cinnamon/BitboardCalculator/
    pub const WHITE_SQUARES: Bitboard = Bitboard(0x55aa55aa55aa55aa_u64);
    pub const BLACK_SQUARES: Bitboard = Bitboard(0xaa55aa55aa55aa55_u64);
    pub const RIM: Bitboard = Bitboard::FILE_A.or(Bitboard::FILE_H);
    pub const RANKS_18: Bitboard = Bitboard::RANK_1.or(Bitboard::RANK_8);
    pub const RANKS_27: Bitboard = Bitboard::RANK_2.or(Bitboard::RANK_7);
    pub const RANKS_36: Bitboard = Bitboard::RANK_3.or(Bitboard::RANK_6);
    pub const RANKS_45: Bitboard = Bitboard::RANK_4.or(Bitboard::RANK_5);
    pub const CENTER_4_SQ: Bitboard = Bitboard::RANKS_45.and(Bitboard::FILE_D.or(Bitboard::FILE_E));
    pub const CENTER_16_SQ: Bitboard =
        (Bitboard::RANKS_45.or(Bitboard::RANKS_36)).and(Bitboard::FILE_C.or(Bitboard::FILE_D).or(Bitboard::FILE_E).or(Bitboard::FILE_F));
}

impl fmt::Binary for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut elements = Vec::new();
        for e in Square::all() {
            if e.is_in(*self) {
                elements.push(e.to_string().to_ascii_uppercase());
            }
        }
        for (i, &e) in Self::FILES.iter().enumerate() {
            if self.contains(e) {
                elements.push(format!("FILE_{}", char::from(b'A' + i as u8)));
            }
        }
        for (i, &e) in Self::RANKS.iter().enumerate() {
            if self.contains(e) {
                elements.push(format!("RANK_{}", i + 1));
            }
        }
        write!(f, "{}", elements.join(" | "))
    }
}

impl ops::Shl<u8> for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn shl(self, s: u8) -> Bitboard {
        Bitboard(self.0 << s)
    }
}

impl ops::Shr<u8> for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn shr(self, s: u8) -> Bitboard {
        Bitboard(self.0 >> s)
    }
}

impl ops::BitOr for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn bitor(self, o: Bitboard) -> Bitboard {
        Bitboard(self.0 | o.0)
    }
}

impl ops::BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, o: Bitboard) {
        self.0 |= o.0;
    }
}

impl ops::Not for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
}

impl ops::BitAnd for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn bitand(self, o: Bitboard) -> Bitboard {
        Bitboard(self.0 & o.0)
    }
}

impl ops::BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, o: Bitboard) {
        self.0 &= o.0;
    }
}

impl ops::BitXor for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn bitxor(self, o: Bitboard) -> Bitboard {
        Bitboard(self.0 ^ o.0)
    }
}

impl ops::Sub for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn sub(self, o: Bitboard) -> Bitboard {
        Bitboard(self.0 & (Bitboard::all().0 ^ o.0))
    }
}

impl ops::SubAssign for Bitboard {
    #[inline]
    fn sub_assign(&mut self, o: Bitboard) {
        *self &= Bitboard::all() ^ o
    }
}

impl ops::BitXorAssign for Bitboard {
    #[inline]
    fn bitxor_assign(&mut self, o: Bitboard) {
        self.0 ^= o.0;
    }
}

impl Bitboard {
    // const EDGES:Self = Self::FILE_A.or(Self::FILE_H).or(Self::RANK_1).or(Self::RANK_8);

    /// All of RANK 1 plus RANK 8
    ///```
    /// use odonata::bitboard::bitboard::Bitboard;
    /// assert!(Bitboard::PROMO_RANKS.contains(Bitboard::A1));
    /// assert!(Bitboard::PROMO_RANKS.contains(Bitboard::H8));
    ///```
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

    #[inline]
    pub const fn all() -> Bitboard {
        Bitboard(u64::MAX)
    }

    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn is_all(self) -> bool {
        self.0 == Self::all().0
    }

    #[inline]
    pub const fn empty() -> Bitboard {
        Bitboard::EMPTY
    }

    #[inline]
    pub fn insert(&mut self, o: Bitboard) {
        *self |= o;
    }

    #[inline]
    pub fn remove(&mut self, o: Bitboard) {
        *self -= o;
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == Self::EMPTY.0
    }

    #[inline]
    pub const fn intersects(self, o: Bitboard) -> bool {
        self.0 & o.0 != 0
    }

    #[inline]
    pub const fn contains(self, o: Bitboard) -> bool {
        self.0 & o.0 == o.0
    }

    // insert,
    // remove,
    // set,
    // toggle,

    #[inline]
    pub const fn from_xy(x: u32, y: u32) -> Bitboard {
        let bit = 1 << (y * 8 + x);
        Bitboard(bit)
    }

    #[inline]
    pub const fn from_sq(index: u8) -> Bitboard {
        let bit = 1 << index;
        Bitboard(bit)
    }

    #[inline]
    pub const fn from_u64(bits: u64) -> Bitboard {
        Bitboard(bits)
    }

    // a niche value that never occurs "in real life"
    #[inline]
    pub const fn niche() -> Bitboard {
        Bitboard::all()
    }

    #[inline]
    pub const fn disjoint(self, other: Bitboard) -> bool {
        (self.0 & other.0) == 0
    }

    #[inline]
    pub const fn any(self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub const fn two_or_more(self) -> bool {
        self.popcount() >= 2
    }

    #[inline]
    pub const fn exactly_one(self) -> bool {
        self.popcount() == 1
    }

    #[inline]
    pub const fn shift(self, dir: Dir) -> Bitboard {
        // let bb = self.sub(dir.mask());
        // Bitboard(bb.0.rotate_left(dir.bits_to_rotate()))

        let (rotation,mask) = match dir {
            Dir::N => (8,Bitboard::RANK_8),
            Dir::NE => (9,Bitboard::RANK_8.or(Bitboard::FILE_H)),
            Dir::E => (1,Bitboard::FILE_H),
            Dir::SE => (64-7,Bitboard::RANK_1.or(Bitboard::FILE_H)),
            Dir::S => (64-8,Bitboard::RANK_1),
            Dir::SW => (64-9,Bitboard::RANK_1.or(Bitboard::FILE_A)),
            Dir::W => (64-1,Bitboard::FILE_A),
            Dir::NW => (7,Bitboard::RANK_8.or(Bitboard::FILE_A)),
            _ => unreachable!()
        } ;
        let bb = self.sub(mask);
        Bitboard(bb.0.rotate_left(rotation))

        // let bb = self.sub(dir.mask());
        // Bitboard(bb.0.rotate_left(dir.bits_to_rotate()))
        // if dir.shift() > 0 {
        //     Bitboard(bb.0 << dir.shift())
        // } else {
        //     Bitboard(bb.0 >> -dir.shift())
        // }
    }

    // excludes the src squares themselves, but includes edge squares
    #[inline]
    pub fn rays(self, dir: Dir) -> Bitboard {
        let mut sqs = self;
        let mut bb = Bitboard::EMPTY;
        while !sqs.is_empty() {
            sqs = sqs.shift(dir);
            bb |= sqs;
        }
        bb
    }

    // inclusive, faster than ray - works on empty set
    #[inline]
    pub fn fill_north(self) -> Bitboard {
        let mut bb = self;
        bb |= Bitboard(bb.0 << 32);
        bb |= Bitboard(bb.0 << 16);
        bb |= Bitboard(bb.0 << 8);
        bb
        // let bb32 = self.0 | self.0 << 32;
        // let bb16 = bb32 | bb32 << 16;
        // let bb8 = bb16 | bb16 << 8;
        // Bitboard(bb8)
    }

    // all points south inclusive - works on empty set
    #[inline]
    pub fn fill_south(self) -> Bitboard {
        let mut bb = self;
        bb |= Bitboard(bb.0 >> 32);
        bb |= Bitboard(bb.0 >> 16);
        bb |= Bitboard(bb.0 >> 8);
        bb
    }

    // the set of files containing the bitboard
    #[inline]
    pub fn file_flood(self) -> Bitboard {
        self.fill_north() | self.fill_south() | self
    }

    #[inline]
    pub fn diag_flood(self) -> Bitboard {
        self.rays(Dir::NE) | self.rays(Dir::SW) | self
    }

    #[inline]
    pub fn anti_diag_flood(self) -> Bitboard {
        self.rays(Dir::NW) | self.rays(Dir::SE) | self
    }
    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    pub const fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    #[inline]
    pub const fn invert(self) -> Self {
        Self(!self.0)
    }

    #[inline]
    pub fn home_half(c: Color) -> Self {
        c.chooser_wb(
            Bitboard::RANK_1.or(Bitboard::RANK_2).or(Bitboard::RANK_3).or(Bitboard::RANK_4),
            Bitboard::RANK_5.or(Bitboard::RANK_6).or(Bitboard::RANK_7).or(Bitboard::RANK_8),
        )
    }

    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn xor(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }

    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn sub(self, other: Self) -> Self {
        Bitboard(self.0 & (Self::all().bits() ^ other.bits()))
    }

    #[must_use]
    #[inline]
    pub const fn popcount(self) -> i32 {
        self.0.count_ones() as i32
    }

    /// flip vertical - https://www.chessprogramming.org/Flipping_Mirroring_and_Rotating
    /// named flip_vertical rather than swap_bytes to match square ^ 56
    #[inline]
    pub const fn flip_vertical(self) -> Self {
        Bitboard(self.0.swap_bytes())
    }

    //
    #[inline]
    pub const fn wrapping_sub(self, other: Bitboard) -> Self {
        Bitboard(self.0.wrapping_sub(other.0))
    }

    // #[inline]
    // pub fn includes(self, sq: Square) -> bool {
    //     self.intersects(sq.as_bb())
    // }

    #[inline]
    pub const fn exclude(self, sq: Square) -> Bitboard {
        self.sub(sq.as_bb())
    }

    #[inline]
    pub const fn include(self, sq: Square) -> Bitboard {
        self.or(sq.as_bb())
    }

    #[inline]
    pub const fn square(self) -> Square {
        // debug_assert_eq!(self.popcount(), 1, "Attempt to convert bb {} to a square", self);
        let sq = self.0.trailing_zeros();
        // debug_assert!(sq < 64);
        Square::from_u32(sq)
    }

    #[inline]
    pub const fn last_square(self) -> Square {
        debug_assert!(!self.is_empty(), "bb.last_square on empty");
        let msb = self.0.leading_zeros();
        debug_assert!(msb < 64);
        Square::from_u32(63 - msb)
    }

    #[inline]
    pub const fn first_square(self) -> Square {
        debug_assert!(!self.is_empty(), "bb.first_square on empty");
        // LSB
        let sq = self.0.trailing_zeros();
        debug_assert!(sq < 64);
        Square::from_u32(sq)
    }

    // last square in the block of bits containing square s. If s is not in a block then just s.
    // so for (RANK_1 | RANK_8).last_square_from(A2) = A8.
    #[inline]
    pub fn last_square_from(self, s: Square) -> Square {
        let bb = self.include(s) >> s.index() as u8;
        let first_empty = (!bb).first_square();
        let i = first_empty.index() - 1 + s.index();
        debug_assert!(i < 64);
        Square::from_u32(i as u32)
    }

    #[inline]
    pub const fn last(self) -> Self {
        debug_assert!(!self.is_empty(), "bb.last on empty");
        Bitboard(1 << self.last_square().index()) // MSb
    }

    #[inline]
    pub const fn first(self) -> Self {
        debug_assert!(!self.is_empty(), "bb.first on empty");
        Bitboard(1 << self.first_square().index()) // LSb
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

    pub fn sq_as_uci(self) -> String {
        let s = self.first_square();
        format!("{}{}", s.file_char(), s.rank_char())
    }

    pub fn uci(self) -> String {
        let strings: Vec<String> = self.iter().map(Self::sq_as_uci).collect();
        strings.join("+")
    }

    pub fn parse_rank(s: &str) -> Result<Bitboard> {
        match s.chars().next() {
            Some(ch) if ('1'..='8').contains(&ch) => Ok(Self::RANKS[ch as usize - b'1' as usize]),
            _ => Err(anyhow!("Invalid rank '{}'", s)),
        }
    }

    pub fn parse_file(s: &str) -> Result<Bitboard> {
        match s.chars().next() {
            Some(ch) if ('a'..='h').contains(&ch) => Ok(Self::FILES[ch as usize - b'a' as usize]),
            _ => Err(anyhow!("Invalid file '{}'", s)),
        }
    }

    pub fn parse_squares(s: &str) -> Result<Bitboard> {
        let s = s.replace(",", " ");
        let mut bb = Bitboard::empty();
        for sq_str in s.split_ascii_whitespace() {
            let sq = Self::parse_square(sq_str)?;
            bb |= sq.as_bb()
        }
        Ok(bb)
    }

    pub fn parse_square(s: &str) -> Result<Square> {
        if s.len() != 2 {
            return Err(anyhow!("Invalid square '{}'", s));
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
    #[inline]
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
        self.n = Bitboard(self.n.0.wrapping_sub(self.d.0)) & self.d;
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
            self.bb ^= sq;
            Some(sq)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bitcount = self.bb.popcount() as usize;
        (bitcount, Some(bitcount))
    }
}

impl ExactSizeIterator for BitIterator {

    #[inline]
    fn len(&self) -> usize { 
        self.bb.popcount() as usize
    }

    // #[inline]
    // fn is_empty(&self) -> bool {
    //     self.bb.is_empty()
    // }
    // #[inline]
    // fn count(self) -> usize {
    //     self.bb.popcount() as usize
    // }
}




impl fmt::Display for Bitboard {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in (0..8).rev() {
            for f in 0..8 {
                let bit = 1 << (r * 8 + f);
                fmt.write_str(if self.contains(Bitboard(bit)) { "1 " } else { ". " })?;
            }
            fmt.write_char('\n')?;
        }
        Ok(())
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
            let sq = self.bb.0.trailing_zeros();
            self.bb.0 ^= 1 << sq;
            Some(Square::from_u32(sq))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bitcount = self.bb.popcount() as usize;
        (bitcount, Some(bitcount))
    }
}

impl ExactSizeIterator for Squares {

    #[inline]
    fn len(&self) -> usize { 
        self.bb.popcount() as usize
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
        assert_eq!(Bitboard::all(), !Bitboard::empty());
        assert!(Bitboard::FILE_A.contains(a4));
        assert_eq!(Bitboard::FILE_A.popcount(), 8);
        assert_eq!(Bitboard::all().popcount(), 64);
        assert_eq!(
            (Bitboard::FILE_A | Bitboard::RANK_1).flip_vertical(),
            (Bitboard::FILE_A | Bitboard::RANK_8)
        );
        assert_eq!((1u64 << 63) >> 63, 1);
        assert_eq!(Bitboard::BLACK_SQUARES | Bitboard::WHITE_SQUARES, Bitboard::all());
        assert!(Bitboard::BLACK_SQUARES.contains(a1));
        assert_eq!(1_u64.wrapping_shl(64), 1_u64);
        // assert_eq!(Bitboard::from_sq(64), Bitboard::EMPTY);
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

        assert_eq!(Bitboard::parse_squares("h8 h1").unwrap(), h8 | h1);
        assert_eq!(Bitboard::parse_squares("a1, a2,a3  ").unwrap(), a1 | a2 | a3);
        assert_eq!(Bitboard::parse_squares("").unwrap(), Bitboard::empty());
    }

    #[test]
    fn test_parse_fail() {
        assert_eq!(Bitboard::parse_file("9").unwrap_err().to_string(), "Invalid file '9'");
        assert_eq!(Bitboard::parse_file("").unwrap_err().to_string(), "Invalid file ''");
        assert_eq!(Bitboard::parse_rank("a").unwrap_err().to_string(), "Invalid rank 'a'");
        assert_eq!(Bitboard::parse_square("aa").unwrap_err().to_string(), "Invalid rank 'a'");
        assert_eq!(Bitboard::parse_square("11").unwrap_err().to_string(), "Invalid file '1'");
        assert_eq!(Bitboard::parse_square("").unwrap_err().to_string(), "Invalid square ''");
        assert_eq!(Bitboard::parse_square("abc").unwrap_err().to_string(), "Invalid square 'abc'");
    }

    #[test]
    fn test_firsts_and_lasts() {
        assert_eq!(Bitboard::RANK_2.popcount(), 8);
        assert_eq!(a1b2.popcount(), 2);
        assert_eq!(a1b2.first_square().index(), 0);
        assert_eq!(a1b2.last_square().index(), 9);
        assert_eq!((Bitboard::A1 | Bitboard::A2).last_square().index(), 8);

        let bb = Bitboard::C1 | Bitboard::D1 | Bitboard::E1;
        assert_eq!(bb.last_square_from(Square::A1), Square::A1);
        assert_eq!(bb.last_square_from(Square::C1), Square::E1);
        assert_eq!(bb.last_square_from(Square::D1), Square::E1);
        assert_eq!(bb.last_square_from(Square::E1), Square::E1);
        assert_eq!(bb.last_square_from(Square::F1), Square::F1);
        assert_eq!(bb.last_square_from(Square::H8), Square::H8);

        let bb = Bitboard::RANK_1 | Bitboard::RANK_8;
        assert_eq!(bb.last_square_from(Square::A1), Square::H1);
        assert_eq!(bb.last_square_from(Square::H1), Square::H1);
        assert_eq!(bb.last_square_from(Square::A2), Square::A2);
        assert_eq!(bb.last_square_from(Square::A8), Square::H8);
        assert_eq!(bb.last_square_from(Square::H8), Square::H8);

        // FIXME!
        // assert_eq!(Bitboard::EMPTY.first_square(), 64);
        // assert_eq!(Bitboard::EMPTY.last_square(), 64);
    }
    // let result = std::panic::catch_unwind(|| Bitboard::EMPTY.as_sq() );
    // assert!(result.is_err());
    #[test]
    fn test_shifts() {
        let a2b3 = a1b2.shift(Dir::N);
        assert_eq!(Bitboard::H8.bits().wrapping_shl(1), Bitboard::EMPTY.bits());
        assert_eq!(a2b3, Bitboard::A2 | Bitboard::B3);
        assert!(Bitboard::D8.shift(Dir::N).is_empty());
        assert_eq!(Bitboard::D8.shift(Dir::E), Bitboard::E8);
        assert_eq!(Bitboard::A8.shift(Dir::N), Bitboard::EMPTY);
        assert_eq!(Bitboard::H8.shift(Dir::E), Bitboard::EMPTY);
        assert_eq!(Bitboard::A1.shift(Dir::W), Bitboard::EMPTY);
        assert_eq!(Bitboard::A1.shift(Dir::S), Bitboard::EMPTY);
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
        assert_eq!(format!("{:?}", Bitboard::FILE_A), "A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 | FILE_A");
        // assert_eq!(format!("{:?}", Bitboard::EDGES), "");
        assert_eq!(format!("{:b}", a1b2), "1000000001");
    }

    #[test]
    fn test_directions() {
        let dir = Dir::N;
        assert_eq!(dir.shift(), 8);
        assert_eq!(Dir::ALL[0], Dir::N);
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
        assert_eq!(power_sets.clone().fold(Bitboard::EMPTY, |acc, bb| acc | bb), Bitboard::FILE_A);
        assert_eq!(power_sets.clone().filter(|bb| bb.popcount() == 2).count(), 7 * 8 / 2);
        assert_eq!(power_sets.clone().filter(|bb| bb.popcount() == 7).count(), 8);
    }
}
