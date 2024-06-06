use std::fmt::{self, Write};
use std::ops::{self, Index};
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Result};
use crossbeam_utils::atomic::AtomicCell;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::bits::square::Square;
use crate::piece::Color;
use crate::FlipVertical;

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
    #[must_use]
    pub const fn rotate_clockwise(self) -> Dir {
        Dir::ALL[(self.index() + 1) % 8]
    }

    #[inline]
    #[must_use]
    pub const fn opposite(self) -> Dir {
        Self::ALL[(self.index() + 4) % 8]
    }
}

impl<T> std::ops::Index<Dir> for [T; Dir::ALL.len()] {
    type Output = T;
    #[inline]
    fn index(&self, i: Dir) -> &Self::Output {
        #[cfg(not(all(not(feature = "unchecked_indexing"), debug_assertions)))]
        unsafe {
            self.get_unchecked(i.index())
        }

        #[cfg(all(not(feature = "unchecked_indexing"), debug_assertions))]
        self[i.index()]
    }
}

impl<T> std::ops::IndexMut<Dir> for [T; Dir::ALL.len()] {
    #[inline]
    fn index_mut(&mut self, d: Dir) -> &mut Self::Output {
        &mut self[d.index()]
    }
}

#[derive(Copy, Clone, Default, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct Bitboard(u64);

// impl std::iter::Sum for Bitboard {
//     fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
//         iter.fold(Bitboard::EMPTY, |a, b| a | b)
//     }
// }

// #[derive(Clone, Copy, PartialEq, Eq)]
// pub struct NonZeroBitboard(pub NonZeroU64);
// impl NonZeroBitboard {
//     #[inline(always)]
//     pub fn new(b: Bitboard) -> Self {
//         NonZeroBitboard(unsafe { NonZeroU64::new_unchecked(b.bits()) })
//     }
// }

#[derive(Serialize, Deserialize)]
pub struct LazyBitboard<const NICHE: u64> {
    #[serde(skip)]
    cell: AtomicCell<Bitboard>,
}

impl<const NICHE: u64> PartialEq for LazyBitboard<NICHE> {
    fn eq(&self, other: &Self) -> bool {
        self.cell.load() == other.cell.load()
    }
}

impl<const NICHE: u64> Clone for LazyBitboard<NICHE> {
    fn clone(&self) -> Self {
        Self {
            cell: AtomicCell::new(self.cell.load()),
        }
    }
}

impl<const NICHE: u64> Default for LazyBitboard<NICHE> {
    fn default() -> Self {
        Self {
            cell: AtomicCell::new(Bitboard::from_u64(NICHE)),
        }
    }
}

impl<const NICHE: u64> LazyBitboard<NICHE> {
    #[inline(always)]
    pub fn get_or_init(&self, f: impl FnOnce() -> Bitboard) -> Bitboard {
        let mut bb = self.cell.load();
        if bb.bits() == NICHE {
            bb = f();
            debug_assert!(bb.bits() != NICHE, "Lazy bb {bb} == niche value {NICHE}");
            self.cell.store(bb);
        }
        bb
    }
    pub fn get(&self) -> Bitboard {
        self.cell.load()
    }

    pub fn set(&self, v: Bitboard) {
        self.cell.store(v)
    }

    pub const fn niche(&self) -> Bitboard {
        Bitboard(NICHE)
    }
}

// impl<const NICHE: Bitboard> LazyBitboard<NICHE> {
//     pub fn get_or_init(&self, f: impl FnOnce() -> Bitboard) -> Bitboard {
//         let nzbb = self.cell.get();
//         if  let Some(nzbb) = nzbb {
//             return Bitboard(nzbb.0.get())
//         }
//         let bb = f();
//         self.cell.set(Some(NonZeroBitboard::new(bb)));
//         bb
//     }
// }

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

    // const EDGES:Self = Self::FILE_A.or(Self::FILE_H).or(Self::RANK_1).or(Self::RANK_8);

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
    pub const FILE_C: Bitboard = Bitboard(Self::FILE_A.bits() << 2);
    pub const FILE_D: Bitboard = Bitboard(Self::FILE_A.bits() << 3);
    pub const FILE_E: Bitboard = Bitboard(Self::FILE_A.bits() << 4);
    pub const FILE_F: Bitboard = Bitboard(Self::FILE_A.bits() << 5);
    pub const FILE_G: Bitboard = Bitboard(Self::FILE_A.bits() << 6);
    pub const FILE_H: Bitboard = Bitboard(Self::FILE_A.bits() << 7);
    pub const RANK_2: Bitboard = Bitboard(Self::RANK_1.bits() << 8);
    pub const RANK_3: Bitboard = Bitboard(Self::RANK_1.bits() << (2 * 8));
    pub const RANK_4: Bitboard = Bitboard(Self::RANK_1.bits() << (3 * 8));
    pub const RANK_5: Bitboard = Bitboard(Self::RANK_1.bits() << (4 * 8));
    pub const RANK_6: Bitboard = Bitboard(Self::RANK_1.bits() << (5 * 8));
    pub const RANK_7: Bitboard = Bitboard(Self::RANK_1.bits() << (6 * 8));
    pub const RANK_8: Bitboard = Bitboard(Self::RANK_1.bits() << (7 * 8));

    // https://gekomad.github.io/Cinnamon/BitboardCalculator/
    pub const WHITE_SQUARES: Bitboard = Bitboard(0x55aa55aa55aa55aa_u64);
    pub const BLACK_SQUARES: Bitboard = Bitboard(0xaa55aa55aa55aa55_u64);

    pub const ALL: Bitboard = Self::WHITE_SQUARES.or(Self::BLACK_SQUARES);

    pub const RIM: Bitboard = Bitboard::FILE_A.or(Bitboard::FILE_H);

    pub const FILES_CDEF: Bitboard = Bitboard::FILE_C
        .or(Bitboard::FILE_D)
        .or(Bitboard::FILE_E)
        .or(Bitboard::FILE_F);

    pub const QUEENS_SIDE: Bitboard = Bitboard::FILE_A
        .or(Bitboard::FILE_B)
        .or(Bitboard::FILE_C)
        .or(Bitboard::FILE_D);
    pub const KINGS_SIDE: Bitboard = Bitboard::FILE_E
        .or(Bitboard::FILE_F)
        .or(Bitboard::FILE_G)
        .or(Bitboard::FILE_H);

    /// All of RANK 1 plus RANK 8
    /// ```
    /// use odonata_base::bits::Bitboard;
    /// assert!(Bitboard::RANKS_18.contains(Bitboard::A1));
    /// assert!(Bitboard::RANKS_18.contains(Bitboard::H8));
    /// ```
    pub const RANKS_18: Bitboard = Bitboard::RANK_1.or(Bitboard::RANK_8);
    pub const RANKS_27: Bitboard = Bitboard::RANK_2.or(Bitboard::RANK_7);
    pub const RANKS_36: Bitboard = Bitboard::RANK_3.or(Bitboard::RANK_6);
    pub const RANKS_45: Bitboard = Bitboard::RANK_4.or(Bitboard::RANK_5);
    pub const RANKS_3456: Bitboard = Bitboard::RANKS_45.or(Bitboard::RANKS_36);

    pub const CENTER_4_SQ: Bitboard = Bitboard::RANKS_45.and(Bitboard::FILE_D.or(Bitboard::FILE_E));
    pub const CENTER_16_SQ: Bitboard = (Bitboard::RANKS_3456).and(Self::FILES_CDEF);
    pub const RANKS_234567: Self = Self::RANK_2.or(Self::RANKS_3456).or(Self::RANK_7);
    pub const RANKS_1_3456_8: Self = Self::RANKS_18.or(Self::RANKS_3456);

    pub const EDGE: Self = Self::RANKS_18.or(Self::RIM);
    pub const CORNERS: Bitboard = Self::RANKS_18.and(Self::RIM);
    // king from/to squares
    pub const CASTLING_WHITE: Bitboard = Self::E1.or(Self::C1).or(Self::G1);
    pub const CASTLING_BLACK: Bitboard = Self::E8.or(Self::C8).or(Self::G8);
    pub const CASTLING: Bitboard = Self::CASTLING_WHITE.or(Self::CASTLING_BLACK);
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

impl FromIterator<Square> for Bitboard {
    fn from_iter<T: IntoIterator<Item = Square>>(iter: T) -> Self {
        let mut bb = Self::empty();
        for sq in iter {
            bb |= sq.as_bb()
        }
        bb
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

impl ops::BitOr<Square> for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn bitor(self, o: Square) -> Bitboard {
        self | o.as_bb()
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
    #[inline]
    pub const fn from_xy(x: u32, y: u32) -> Bitboard {
        let bit = 1 << (y * 8 + x);
        Bitboard(bit)
    }

    #[inline]
    pub const fn from_sq(index: u16) -> Bitboard {
        debug_assert!(index < 64);
        let bit = 1 << index;
        Bitboard(bit)
    }

    #[inline]
    pub const fn from_u64(bits: u64) -> Bitboard {
        Bitboard(bits)
    }

    #[inline]
    pub const fn all() -> Bitboard {
        Bitboard(u64::MAX)
    }

    #[inline]
    pub const fn empty() -> Bitboard {
        Bitboard::EMPTY
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
    /// self if and only iff true else Empty
    pub const fn iff(&self, b: bool) -> Bitboard {
        Bitboard(self.0 * (b as u64))
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
    /// bits shifted off the board are zeroed / disappear
    pub const fn shift(self, dir: Dir) -> Bitboard {
        // let bb = self.sub(dir.mask());
        // Bitboard(bb.0.rotate_left(dir.bits_to_rotate()))

        let (rotation, mask) = match dir {
            Dir::N => (8, Bitboard::RANK_8),
            Dir::NE => (9, Bitboard::RANK_8.or(Bitboard::FILE_H)),
            Dir::E => (1, Bitboard::FILE_H),
            Dir::SE => (64 - 7, Bitboard::RANK_1.or(Bitboard::FILE_H)),
            Dir::S => (64 - 8, Bitboard::RANK_1),
            Dir::SW => (64 - 9, Bitboard::RANK_1.or(Bitboard::FILE_A)),
            Dir::W => (64 - 1, Bitboard::FILE_A),
            Dir::NW => (7, Bitboard::RANK_8.or(Bitboard::FILE_A)),
            _ => unreachable!(),
        };
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

    /// rays exclude the src squares themselves, but includes edge squares
    #[inline]
    pub const fn rays(self, dir: Dir) -> Bitboard {
        let mut sqs = self;
        let mut bb = Bitboard::EMPTY;
        while !sqs.is_empty() {
            sqs = sqs.shift(dir);
            bb = bb.or(sqs);
        }
        bb
    }

    /// fills are inclusive of source square, f/aster than ray - works on empty set
    #[inline]
    pub const fn fill_north(self) -> Bitboard {
        let mut bb = self;
        bb = bb.or(Bitboard(bb.0 << 32));
        bb = bb.or(Bitboard(bb.0 << 16));
        bb = bb.or(Bitboard(bb.0 << 8));
        bb
    }

    /// fills are inclusive of source square, f/aster than ray - works on empty set
    #[inline]
    pub const fn fill_south(self) -> Bitboard {
        let mut bb = self;
        bb = bb.or(Bitboard(bb.0 >> 32));
        bb = bb.or(Bitboard(bb.0 >> 16));
        bb = bb.or(Bitboard(bb.0 >> 8));
        bb
    }

    #[inline]
    /// forward wrt pawn advance for that color
    pub fn fill_forward(self, c: Color) -> Self {
        if c == Color::White {
            self.fill_north()
        } else {
            self.fill_south()
        }
    }

    #[inline]
    /// all squares of the same colour as any sq in the bitboard
    /// if bitboard comtains both black and white whole board is returned
    pub fn color_flood(self) -> Bitboard {
        Bitboard::WHITE_SQUARES.iff(self.intersects(Bitboard::WHITE_SQUARES))
            | Bitboard::BLACK_SQUARES.iff(self.intersects(Bitboard::BLACK_SQUARES))
    }

    #[inline]
    /// if all squares within same color
    pub fn same_color(self) -> bool {
        self.disjoint(Self::WHITE_SQUARES) || self.disjoint(Self::BLACK_SQUARES)
    }

    // the set of files containing the bitboard
    #[inline]
    pub const fn file_flood(self) -> Bitboard {
        self.fill_north().or(self.fill_south()).or(self)
    }

    #[inline]
    pub const fn diag_flood(self) -> Bitboard {
        self.rays(Dir::NE).or(self.rays(Dir::SW)).or(self)
    }

    #[inline]
    pub const fn anti_diag_flood(self) -> Bitboard {
        self.rays(Dir::NW).or(self.rays(Dir::SE)).or(self)
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
            Bitboard::RANK_1
                .or(Bitboard::RANK_2)
                .or(Bitboard::RANK_3)
                .or(Bitboard::RANK_4),
            Bitboard::RANK_5
                .or(Bitboard::RANK_6)
                .or(Bitboard::RANK_7)
                .or(Bitboard::RANK_8),
        )
    }

    /// returns king or queens or both sides of the board depending on where region sits
    #[inline]
    pub const fn flood_kq_sides(self) -> Self {
        let is_queens_side = self.intersects(Self::QUEENS_SIDE);
        let is_kings_side = self.intersects(Self::KINGS_SIDE);
        Self::QUEENS_SIDE
            .iff(is_queens_side)
            .or(Self::KINGS_SIDE.iff(is_kings_side))
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

    /// named flip_vertical rather than swap_bytes to match square ^ 56
    #[inline]
    pub const fn flip_vertical(self) -> Self {
        // flip vertical - https://www.chessprogramming.org/Flipping_Mirroring_and_Rotating
        Bitboard(self.0.swap_bytes())
    }

    /// flip horizontal
    #[inline]
    pub const fn flip_horizontal(self) -> Self {
        // https://www.chessprogramming.org/Flipping_Mirroring_and_Rotating
        // using algo mirrorHorizontal
        let k1 = 0x5555_5555_5555_5555_u64;
        let k2 = 0x3333_3333_3333_3333_u64;
        let k4 = 0x0f0f_0f0f_0f0f_0f0f_u64;
        let mut x = self.0;
        x = ((x >> 1) & k1) | ((x & k1) << 1);
        x = ((x >> 2) & k2) | ((x & k2) << 2);
        x = ((x >> 4) & k4) | ((x & k4) << 4);
        Bitboard(x)
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

    // #[inline]
    // pub const fn square(self) -> Square {
    //     debug_assert!(
    //         self.popcount() == 1,
    //         "attempt to convert bb to square where popcount != 1"
    //     );
    //     let sq = self.0.trailing_zeros();
    //     // debug_assert!(sq < 64);
    //     unsafe { Square::from_u8_unchecked(sq as u8) }
    // }

    // #[inline]
    // pub const fn first_square(self) -> Square {
    //     debug_assert!(!self.is_empty(), "bb.first_square on empty");
    //     // LSB
    //     let sq = self.0.trailing_zeros();
    //     debug_assert!(sq < 64);
    //     Square::from_u32(sq)
    // }

    // // last square in the block of bits containing square s. If s is not in a block then just s.
    // // so for (RANK_1 | RANK_8).last_square_from(A2) = A8.
    // #[inline]
    // pub fn last_square_from(self, s: Square) -> Square {
    //     let bb = self.include(s) >> s.index() as u8;
    //     let first_empty = (!bb).first_square();
    //     let i = first_empty.index() - 1 + s.index();
    //     debug_assert!(i < 64);
    //     Square::from_u32(i as u32)
    // }

    #[inline]
    pub const fn find_first_square(self) -> Option<Square> {
        match self.0.trailing_zeros() {
            64.. => None,
            i => Some(unsafe { Square::from_u8_unchecked(i as u8) }),
        }
    }

    #[inline]
    pub const fn find_last_square(self) -> Option<Square> {
        match self.0.leading_zeros() {
            64.. => None,
            i => Some(unsafe { Square::from_u8_unchecked(63 - i as u8) }),
        }
    }

    #[inline]
    pub const fn bits_iter(self) -> BitIterator {
        BitIterator { bb: self }
    }

    #[inline]
    pub const fn squares(self) -> SquaresIterator {
        SquaresIterator { bb: self }
    }

    // carry rippler from https://www.chessprogramming.org/Traversing_Subsets_of_a_Set
    #[inline]
    pub const fn power_set_iter(self) -> PowerSetIterator {
        PowerSetIterator::new(self)
    }

    /// FILES_ABC => "abc"
    pub fn files_string(self) -> String {
        self.squares().map(Square::file_char).sorted().dedup().collect()
    }

    /// RANKS_2345 => "2345"
    pub fn ranks_string(self) -> String {
        self.squares().map(Square::rank_char).sorted().dedup().collect()
    }

    pub fn uci(self) -> String {
        self.squares().map(Square::uci).join("+")
    }

    pub fn parse_rank(s: &str) -> Result<Bitboard> {
        match s.chars().next() {
            Some(ch @ '1'..='8') => Ok(Self::RANKS[ch as usize - b'1' as usize]),
            _ => Err(anyhow!("invalid rank '{s}' parsing square")),
        }
    }

    pub fn parse_file(s: &str) -> Result<Bitboard> {
        match s.chars().next() {
            Some(ch @ 'a'..='h') => Ok(Self::FILES[ch as usize - b'a' as usize]),
            _ => Err(anyhow!("invalid file '{s}' parsing square")),
        }
    }

    pub fn parse_squares(s: &str) -> Result<Bitboard> {
        let s = s.replace(',', " ");
        let s = s.replace('+', " ");
        let s = s.replace('|', " ");
        let mut bb = Bitboard::empty();
        for sq_str in s.split_ascii_whitespace() {
            let sq = Square::parse(sq_str)?;
            bb |= sq.as_bb()
        }
        Ok(bb)
    }
}

impl FlipVertical for Bitboard {
    fn flip_vertical(self) -> Self {
        self.flip_vertical()
    }
}

impl FromStr for Bitboard {
    type Err = anyhow::Error;

    /// parse a fen-like bitboard consistening of 1..8 for numbers of 0's (or '.' or '0' for a 0) and X's for 1's
    /// eg "8/8/8/8/8/8/8/5XXX"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.to_string();
        for i in 1..=8 {
            s = s.replace(i.to_string().as_str(), "0".repeat(i).as_str());
        }
        s = s.replace('.', "0").replace('X', "1");

        let mut r: Vec<&str> = s.rsplit('/').collect();
        if r.iter().any(|r| r.chars().count() != 8) || r.len() != 8 {
            bail!("Expected 8 ranks of 8 bits in bitboard string {}", s);
        }
        let bin = r.iter_mut().map(|s| s.chars().rev().collect::<String>()).rev().join("");
        let bits = u64::from_str_radix(&bin, 2).with_context(|| format!("with contents {}", bin))?;
        Ok(Bitboard::from_u64(bits))
    }
}

// https://www.chessprogramming.org/Traversing_Subsets_of_a_Set
#[derive(Copy, Clone, Debug)]
pub struct PowerSetIterator {
    d:         Bitboard, // we're iterating subsets of d
    n:         Bitboard, // next subset
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
            let sq = self.bb.find_first_square().expect("bit iter").as_bb();
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
}

impl Index<Square> for Bitboard {
    type Output = bool;

    fn index(&self, index: Square) -> &Self::Output {
        if index.is_in(*self) {
            &true
        } else {
            &false
        }
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in (0..8).rev() {
            for f in 0..8 {
                let bit = 1 << (r * 8 + f);
                fmt.write_str(if self.contains(Bitboard(bit)) { "1 " } else { ". " })?;
            }
            if r > 0 {
                // no trailing newline after bitboard
                fmt.write_char('\n')?;
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SquaresIterator {
    bb: Bitboard,
}

impl Iterator for SquaresIterator {
    type Item = Square;

    #[inline]
    fn next(&mut self) -> Option<Square> {
        if self.bb.is_empty() {
            None
        } else {
            let i = self.bb.0.trailing_zeros();
            self.bb.0 ^= 1 << i;
            // this is safe as trailing_zeros < 64
            Some(unsafe { Square::from_u8_unchecked(i as u8) })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let bitcount = self.bb.popcount() as usize;
        (bitcount, Some(bitcount))
    }
}

impl ExactSizeIterator for SquaresIterator {
    #[inline]
    fn len(&self) -> usize {
        self.bb.popcount() as usize
    }
}

#[cfg(test)]
mod tests {

    const A1B2: Bitboard = Bitboard::A1.or(Bitboard::B2);

    use std::hint::black_box;

    use super::*;
    use crate::infra::profiler::PerfProfiler;

    #[test]
    fn test_bitwise() {
        assert!(A1B2.contains(Bitboard::A1));
        assert!(A1B2 & Bitboard::C1 == Bitboard::A1 - Bitboard::A1);
        assert!(A1B2 - Bitboard::A1 == Bitboard::B2);
        assert!(!A1B2.is_empty());
        assert!(A1B2.intersects(Bitboard::B2));
        assert_eq!(Bitboard::all(), !Bitboard::empty());
        assert!(Bitboard::FILE_A.contains(Bitboard::A4));
        assert_eq!(Bitboard::FILE_A.popcount(), 8);
        assert_eq!(Bitboard::all().popcount(), 64);
        assert_eq!(
            (Bitboard::FILE_A | Bitboard::RANK_1).flip_vertical(),
            (Bitboard::FILE_A | Bitboard::RANK_8)
        );
        assert_eq!((Bitboard::FILE_A).flip_horizontal(), Bitboard::FILE_H);
        assert_eq!((Bitboard::B6).flip_horizontal(), Bitboard::G6);
        assert_eq!((Bitboard::RANK_1).flip_horizontal(), Bitboard::RANK_1);
        assert_eq!((1u64 << 63) >> 63, 1);
        assert_eq!(Bitboard::BLACK_SQUARES | Bitboard::WHITE_SQUARES, Bitboard::all());
        assert_eq!(Bitboard::BLACK_SQUARES.popcount(), Bitboard::WHITE_SQUARES.popcount(),);
        assert_eq!(Bitboard::CORNERS.popcount(), 4,);
        assert!(Bitboard::BLACK_SQUARES.contains(Square::A1.as_bb()));
        assert_eq!(Bitboard::A1.color_flood(), Bitboard::BLACK_SQUARES);
        assert_eq!(Bitboard::B1.color_flood(), Bitboard::WHITE_SQUARES);
        assert_eq!((Bitboard::A1 | Bitboard::B1).color_flood(), Bitboard::all());
        assert_eq!(1_u64.wrapping_shl(64), 1_u64);
        assert_eq!(Bitboard::A1.flood_kq_sides().popcount(), 32);
        assert!(Bitboard::A1.flood_kq_sides().contains(Bitboard::D8));
        assert_eq!((Bitboard::A1 | Bitboard::H1).flood_kq_sides().popcount(), 64);
        // from iterator
        assert_eq!(
            [Square::A1, Square::B1].into_iter().collect::<Bitboard>(),
            Bitboard::A1 | Bitboard::B1
        );
        assert_eq!(Some(Square::A1).into_iter().collect::<Bitboard>(), Bitboard::A1);
        assert_eq!(None::<Square>.into_iter().collect::<Bitboard>(), Bitboard::empty());
        // assert_eq!(Bitboard::from_sq(64), Bitboard::EMPTY);
    }

    #[test]
    fn test_rays() {
        let c3 = Square::C3.as_bb();
        let north = c3.rays(Dir::N);
        use Square::*;
        assert_eq!(north, C4 | C5 | C6 | C7 | C8);
        assert_eq!(north.popcount(), 5);

        assert_eq!(c3.rays(Dir::NE), D4 | E5 | F6 | G7 | H8);
        assert_eq!(c3.rays(Dir::SW), A1 | B2);
        assert_eq!(c3.rays(Dir::S), C1 | C2);
        assert_eq!(c3.rays(Dir::NW), A5 | B4);
    }

    #[test]
    fn test_floods_and_fills() {
        use Square::*;
        assert_eq!(A1B2.fill_north(), (Bitboard::FILE_A | Bitboard::FILE_B) - Bitboard::B1);
        assert_eq!(A1B2.fill_south(), A1B2 | Square::B1);
        assert_eq!(A1B2.file_flood(), Bitboard::FILE_A | Bitboard::FILE_B);
        let main_diag = A1 | B2 | C3 | D4 | E5 | F6 | G7 | H8;
        assert_eq!(A1B2.diag_flood(), main_diag);
        assert_eq!(main_diag.file_flood(), Bitboard::all());
        assert_eq!(A1B2.anti_diag_flood(), A1 | B2 | A3 | C1);
    }

    #[test]
    fn test_froms() {
        assert_eq!(Bitboard::from_xy(4, 7), Bitboard::E8);
        assert_eq!(Bitboard::from_sq(63), Bitboard::H8);
        assert_eq!(Bitboard::from_sq(8), Bitboard::A2);
    }

    #[test]
    fn test_parse() {
        assert_eq!(Bitboard::parse_file("a").unwrap(), Bitboard::FILE_A);
        assert_eq!(Bitboard::parse_file("h").unwrap(), Bitboard::FILE_H);
        assert_eq!(Bitboard::parse_rank("1").unwrap(), Bitboard::RANK_1);
        assert_eq!(Bitboard::parse_rank("8").unwrap(), Bitboard::RANK_8);
        assert_eq!(Square::parse("a1").unwrap(), Square::A1);
        assert_eq!(Square::parse("a8").unwrap(), Square::A8);
        assert_eq!(Square::parse("h8").unwrap(), Square::H8);

        use Square::*;
        assert_eq!(Bitboard::parse_squares("h8 h1").unwrap(), H8 | H1);
        assert_eq!(Bitboard::parse_squares("a1, a2,a3  ").unwrap(), A1 | A2 | A3);
        assert_eq!(Bitboard::parse_squares("").unwrap(), Bitboard::empty());
        assert_eq!(Bitboard::from_str("8/8/8/8/8/8/8/8").unwrap(), Bitboard::EMPTY);
        assert_eq!(
            Bitboard::from_str("X7/8/8/8/8/8/8/7X").unwrap(),
            Bitboard::H1 | Bitboard::A8
        );
    }

    #[test]
    fn test_parse_fail() {
        assert_eq!(
            Bitboard::parse_file("9").unwrap_err().to_string(),
            "invalid file '9' parsing square"
        );
        assert_eq!(
            Bitboard::parse_file("").unwrap_err().to_string(),
            "invalid file '' parsing square"
        );
        assert_eq!(
            Bitboard::parse_rank("a").unwrap_err().to_string(),
            "invalid rank 'a' parsing square"
        );
        assert_eq!(
            "aa".parse::<Square>().unwrap_err().to_string(),
            "invalid rank 'a' parsing square"
        );
        assert_eq!(
            Square::parse("11").unwrap_err().to_string(),
            "invalid file '1' parsing square"
        );
        assert_eq!(
            Square::parse("").unwrap_err().to_string(),
            "invalid square '' parsing square"
        );
        assert_eq!(
            Square::parse("abc").unwrap_err().to_string(),
            "invalid square 'abc' parsing square"
        );
    }

    #[test]
    fn test_firsts_and_lasts() {
        use Square as Sq;
        assert_eq!(Bitboard::RANK_2.popcount(), 8);
        assert_eq!(A1B2.popcount(), 2);
        assert_eq!(A1B2.find_first_square(), Some(Sq::A1));
        assert_eq!(A1B2.find_last_square(), Some(Sq::B2));
        assert_eq!((Bitboard::A1 | Bitboard::A2).find_last_square(), Some(Sq::A2));

        // let bb = Bitboard::C1 | Bitboard::D1 | Bitboard::E1;
        // assert_eq!(bb.last_square_from(Sq::A1), Sq::A1);
        // assert_eq!(bb.last_square_from(Sq::C1), Sq::E1);
        // assert_eq!(bb.last_square_from(Sq::D1), Sq::E1);
        // assert_eq!(bb.last_square_from(Sq::E1), Sq::E1);
        // assert_eq!(bb.last_square_from(Sq::F1), Sq::F1);
        // assert_eq!(bb.last_square_from(Sq::H8), Sq::H8);

        // let bb = Bitboard::RANK_1 | Bitboard::RANK_8;
        // assert_eq!(bb.last_square_from(Sq::A1), Sq::H1);
        // assert_eq!(bb.last_square_from(Sq::H1), Sq::H1);
        // assert_eq!(bb.last_square_from(Sq::A2), Sq::A2);
        // assert_eq!(bb.last_square_from(Sq::A8), Sq::H8);
        // assert_eq!(bb.last_square_from(Sq::H8), Sq::H8);

        // FIXME : calling first_square on empty board (show panic)!
        // assert_eq!(Bitboard::EMPTY.first_square(), 64);
        // assert_eq!(Bitboard::EMPTY.last_square(), 64);
    }
    // let result = std::panic::catch_unwind(|| Bitboard::EMPTY.as_sq() );
    // assert!(result.is_err());
    #[test]
    fn test_shifts() {
        let a2b3 = A1B2.shift(Dir::N);
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
        use Square::*;
        assert_eq!(Bitboard::A1.files_string(), "a");
        assert_eq!((A1 | B1 | C1).files_string(), "abc");
        assert_eq!(Bitboard::all().files_string(), "abcdefgh");

        assert_eq!(Bitboard::A1.ranks_string(), "1");
        assert_eq!((A1 | B5 | E5).ranks_string(), "15");
        assert_eq!(Bitboard::all().ranks_string(), "12345678");

        assert_eq!(Square::A1.uci(), "a1");
        assert_eq!(Square::H1.uci(), "h1");
        assert_eq!(Square::A8.uci(), "a8");
        assert_eq!(A1B2.uci(), "a1+b2");
        assert_eq!(format!("{A1B2}"), ". . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. 1 . . . . . . \n1 . . . . . . . ");
        assert_eq!(format!("{A1B2:?}"), "A1 | B2");
        assert_eq!(
            format!("{:?}", Bitboard::FILE_A),
            "A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 | FILE_A"
        );
        // assert_eq!(format!("{:?}", Bitboard::EDGES), "");
        assert_eq!(format!("{A1B2:b}"), "1000000001");
    }

    #[test]
    fn test_directions() {
        let dir = Dir::N;
        assert_eq!(dir.shift(), 8);
        assert_eq!(Dir::ALL[0], Dir::N);
    }

    #[test]
    fn test_iterators() {
        let a1b1g5 = Bitboard::A1 | Bitboard::C1 | Bitboard::G5;
        let mut i = a1b1g5.bits_iter();
        assert_eq!(i.next(), Some(Bitboard::A1));
        assert_eq!(i.next(), Some(Bitboard::C1));
        assert_eq!(i.next(), Some(Bitboard::G5));
        assert_eq!(i.next(), None);
        assert_eq!(a1b1g5.bits_iter().count(), 3);

        let mut sqs = a1b1g5.squares();
        assert_eq!(sqs.next(), Some(Square::A1));
        assert_eq!(sqs.next(), Some(Square::C1));
        assert_eq!(sqs.next(), Some(Square::G5));
        assert_eq!(sqs.next(), None);
        assert_eq!(a1b1g5.squares().count(), 3);

        let power_sets = a1b1g5.power_set_iter();
        power_sets.for_each(|bb| println!("{bb:?}"));
        assert_eq!(power_sets.reduce(|a, b| a | b), Some(a1b1g5));
        assert_eq!(power_sets.count(), 1 << 3);
        assert_eq!(power_sets.max(), Some(a1b1g5));

        let power_sets = Bitboard::FILE_A.power_set_iter();
        assert_eq!(power_sets.count(), 1 << 8);
        assert_eq!(power_sets.fold(Bitboard::EMPTY, |acc, bb| acc | bb), Bitboard::FILE_A);
        assert_eq!(power_sets.filter(|bb| bb.popcount() == 2).count(), 7 * 8 / 2);
        assert_eq!(power_sets.filter(|bb| bb.popcount() == 7).count(), 8);
    }

    #[test]
    fn bench_bitboard() {
        use crate::bits::Bitboard;
        let mut pr = PerfProfiler::new("bb.popcount");
        let _count = pr.bench(|| {
            let bb = Bitboard::RANK_1;
            let count1 = (bb & Bitboard::FILE_A).popcount();
            let count2 = (black_box(Bitboard::RANK_3) & Bitboard::FILE_A).popcount();
            count1 + count2
        });

        let mut pr = PerfProfiler::new("bb.iter-avg4");
        let mut pr8 = PerfProfiler::new("bb.iter8");
        let mut pr4 = PerfProfiler::new("bb.iter4");
        let mut n = black_box(0);
        for b in Bitboard::RANK_1.power_set_iter() {
            pr.bench(|| {
                black_box(b).squares().for_each(|sq| {
                    let _ = black_box(sq);
                })
            });
            pr8.bench(|| {
                black_box(Bitboard::RANK_1)
                    .squares()
                    .for_each(|sq| n += sq.file_index())
            });
            pr4.bench(|| {
                black_box(Bitboard::CORNERS)
                    .squares()
                    .for_each(|sq| n += sq.file_index())
            });
        }

        assert_eq!(n, 10752);
        let mut p_or = PerfProfiler::new("bb.or");
        let mut p_and = PerfProfiler::new("bb.and");
        let mut p_in = PerfProfiler::new("bb.sq_in");
        let mut p_index = PerfProfiler::new("bb.sq_index");
        let mut p_assign_or = PerfProfiler::new("bb.assign_or");
        let mut p_flip = PerfProfiler::new("bb.flip");
        let mut p_next = PerfProfiler::new("bb.next");
        let mut p_file_flood = PerfProfiler::new("bb.file_flood");
        let mut p_fill_fwd = PerfProfiler::new("bb.fill_fwd");
        let sq = Square::A1;
        let mut n = 0;
        let mut m = 0;
        let mut t = 0;
        let mut bb1 = Bitboard::empty();
        for bb in Bitboard::diag_flood(Bitboard::A1).power_set_iter() {
            n += p_in.bench(|| black_box(sq).is_in(black_box(bb))) as i32;
            n += p_index.bench(|| black_box(bb)[black_box(sq)]) as i32;
            m += p_and.bench(|| black_box(bb) & Bitboard::FILE_A).popcount();
            m += p_or.bench(|| black_box(bb) | Bitboard::FILE_A).popcount();
            p_assign_or.bench(|| bb1 |= black_box(bb));
            t += p_flip.bench(|| black_box(bb).flip_vertical()).popcount();
            let iter = bb.squares();
            t += p_next.bench(|| black_box(iter).next()).is_some() as i32;
            t += p_file_flood.bench(|| black_box(bb).file_flood()).popcount();
            t += p_fill_fwd.bench(|| black_box(bb).fill_forward(Color::White)).popcount();
            t += p_fill_fwd.bench(|| black_box(bb).fill_forward(Color::Black)).popcount();
        }
        assert_eq!(n, 256);
        assert_eq!(m, 3072);
        assert_eq!(t, 18687);
    }
}
