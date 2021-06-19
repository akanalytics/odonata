// /*
//  * The MIT License (MIT)
//  *
//  * Copyright (c) 2016 Dan Ravensloft <dan.ravensloft@gmail.com>
//  *
//  * Permission is hereby granted, free of charge, to any person obtaining a copy
//  * of this software and associated documentation files (the "Software"), to deal
//  * in the Software without restriction, including without limitation the rights
//  * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//  * copies of the Software, and to permit persons to whom the Software is
//  * furnished to do so, subject to the following conditions:
//  *
//  * The above copyright notice and this permission notice shall be included in all
//  * copies or substantial portions of the Software.
//  *
//  * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//  * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//  * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//  * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//  * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//  * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//  * SOFTWARE.
//  */
//
// Using the code kindly provided by Volker Annuss:
// http://www.talkchess.com/forum/viewtopic.php?topic_view=threads&p=670709&t=60065

//
//
// This implementation a direct Rust "port" of C++ magics at  https://github.com/Ravenslofty/Hoarfrost which is a
// nearly direct copy of a talkchess discussion from
// http://www.talkchess.com/forum/viewtopic.php?topic_view=threads&p=670709&t=60065 by Volker Annuss.
// Licensed as MIT
//
// I've not made any effort to port this to idiomatic Rust. Its here only for benchmarking purposes.
//

#![allow(non_upper_case_globals)]

use crate::bitboard::attacks::BitboardAttacks;
use crate::bitboard::bb_hyperbola::Hyperbola;
use crate::bitboard::bitboard::Bitboard;
use crate::bitboard::square::Square;
// use once_cell::sync::Lazy;
use static_init::{dynamic};

// #[ctor]
// fn init_module() {
//     Magic::init();
// }

// static mut STATIC_INSTANCE: *const Magic = std::ptr::null();

// impl Magic {
//     // doesnt impl Default as too large to copy by value
//     #[inline]
//     pub fn default() -> &'static Self {
//         unsafe {
//             &*STATIC_INSTANCE
//         }
//     }

//     pub fn init() {
//         let me = Self::new();
//         unsafe {
//             // leak the value, so it will never be dropped or freed
//             STATIC_INSTANCE = Box::leak(me) as *const Self;
//         }
//     }
// }

#[dynamic(lazy)]
static STATIC_INSTANCE: Box<Magic> = Magic::new();

// #[dynamic]
// static STATIC_INSTANCE: Box<Magic> = Magic::new();

impl Magic {
        
    // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn default() -> &'static Self {
        &STATIC_INSTANCE
    }

}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct Magic {
    MagicTable: [u64; 89524],
    PawnMask: [[u64; 64]; 2],
    KnightMask: [u64; 64],
    BishopMask: [u64; 64],
    RookMask: [u64; 64],
    KingMask: [u64; 64],
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
const FileAMask: u64 = 0x0101010101010101_u64;
const FileBMask: u64 = 0x0202020202020202_u64;
// const FileCMask: u64 = 0x0404040404040404_u64;
// const FileDMask: u64 = 0x0808080808080808_u64;
// const FileEMask: u64 = 0x1010101010101010_u64;
// const FileFMask: u64 = 0x2020202020202020_u64;
const FileGMask: u64 = 0x4040404040404040_u64;
const FileHMask: u64 = 0x8080808080808080_u64;

// const Rank1Mask: u64 = 0x00000000000000FF_u64;
// const Rank2Mask: u64 = 0x000000000000FF00_u64;
// const Rank3Mask: u64 = 0x0000000000FF0000_u64;
// const Rank4Mask: u64 = 0x00000000FF000000_u64;
// const Rank5Mask: u64 = 0x000000FF00000000_u64;
// const Rank6Mask: u64 = 0x0000FF0000000000_u64;
// const Rank7Mask: u64 = 0x00FF000000000000_u64;
// const Rank8Mask: u64 = 0xFF00000000000000_u64;

#[rustfmt::skip]
#[allow(non_upper_case_globals)]
const BISHOP_MAGIC: [u64; 64] = [
    0x404040404040_u64, 0xa060401007fc_u64, 0x401020200000_u64, 0x806004000000_u64,
    0x440200000000_u64, 0x80100800000_u64, 0x104104004000_u64, 0x20020820080_u64,
    0x40100202004_u64, 0x20080200802_u64, 0x10040080200_u64, 0x8060040000_u64,
    0x4402000000_u64, 0x21c100b200_u64, 0x400410080_u64, 0x3f7f05fffc0_u64,
    0x4228040808010_u64, 0x200040404040_u64, 0x400080808080_u64, 0x200200801000_u64,
    0x240080840000_u64, 0x18000c03fff8_u64, 0xa5840208020_u64, 0x58408404010_u64,
    0x2022000408020_u64, 0x402000408080_u64, 0x804000810100_u64, 0x100403c0403ff_u64,
    0x78402a8802000_u64, 0x101000804400_u64, 0x80800104100_u64, 0x400480101008_u64,
    0x1010102004040_u64, 0x808090402020_u64, 0x7fefe08810010_u64, 0x3ff0f833fc080_u64,
    0x7fe08019003042_u64, 0x202040008040_u64, 0x1004008381008_u64, 0x802003700808_u64,
    0x208200400080_u64, 0x104100200040_u64, 0x3ffdf7f833fc0_u64, 0x8840450020_u64,
    0x20040100100_u64, 0x7fffdd80140028_u64, 0x202020200040_u64, 0x1004010039004_u64,
    0x40041008000_u64, 0x3ffefe0c02200_u64, 0x1010806000_u64, 0x08403000_u64,
    0x100202000_u64, 0x40100200800_u64, 0x404040404000_u64, 0x6020601803f4_u64,
    0x3ffdfdfc28048_u64, 0x820820020_u64, 0x10108060_u64, 0x00084030_u64,
    0x01002020_u64, 0x40408020_u64, 0x4040404040_u64, 0x404040404040_u64
];

#[rustfmt::skip]
#[allow(non_upper_case_globals)]
const  BISHOP_OFFSET: [usize; 64] = [
    33104, 4094, 24764, 13882,
    23090, 32640, 11558, 32912,
    13674, 6109, 26494, 17919,
    25757, 17338, 16983, 16659,
    13610, 2224, 60405, 7983,
    17, 34321, 33216, 17127,
    6397, 22169, 42727, 155,
    8601, 21101, 29885, 29340,
    19785, 12258, 50451, 1712,
    78475, 7855, 13642, 8156,
    4348, 28794, 22578, 50315,
    85452, 32816, 13930, 17967,
    33200, 32456, 7762, 7794,
    22761, 14918, 11620, 15925,
    32528, 12196, 32720, 26781,
    19817, 24732, 25468, 10186
];

#[rustfmt::skip]
#[allow(non_upper_case_globals)]
const ROOK_MAGIC: [u64; 64] = [
    0x280077ffebfffe_u64, 0x2004010201097fff_u64, 0x10020010053fff_u64, 0x30002ff71ffffa_u64,
    0x7fd00441ffffd003_u64, 0x4001d9e03ffff7_u64, 0x4000888847ffff_u64, 0x6800fbff75fffd_u64,
    0x28010113ffff_u64, 0x20040201fcffff_u64, 0x7fe80042ffffe8_u64, 0x1800217fffe8_u64,
    0x1800073fffe8_u64, 0x7fe8009effffe8_u64, 0x1800602fffe8_u64, 0x30002fffffa0_u64,
    0x300018010bffff_u64, 0x3000c0085fffb_u64, 0x4000802010008_u64, 0x2002004002002_u64,
    0x2002020010002_u64, 0x1002020008001_u64, 0x4040008001_u64, 0x802000200040_u64,
    0x40200010080010_u64, 0x80010040010_u64, 0x4010008020008_u64, 0x40020200200_u64,
    0x10020020020_u64, 0x10020200080_u64, 0x8020200040_u64, 0x200020004081_u64,
    0xfffd1800300030_u64, 0x7fff7fbfd40020_u64, 0x3fffbd00180018_u64, 0x1fffde80180018_u64,
    0xfffe0bfe80018_u64, 0x1000080202001_u64, 0x3fffbff980180_u64, 0x1fffdff9000e0_u64,
    0xfffeebfeffd800_u64, 0x7ffff7ffc01400_u64, 0x408104200204_u64, 0x1ffff01fc03000_u64,
    0xfffe7f8bfe800_u64, 0x8001002020_u64, 0x3fff85fffa804_u64, 0x1fffd75ffa802_u64,
    0xffffec00280028_u64, 0x7fff75ff7fbfd8_u64, 0x3fff863fbf7fd8_u64, 0x1fffbfdfd7ffd8_u64,
    0xffff810280028_u64, 0x7ffd7f7feffd8_u64, 0x3fffc0c480048_u64, 0x1ffffafd7ffd8_u64,
    0xffffe4ffdfa3ba_u64, 0x7fffef7ff3d3da_u64, 0x3fffbfdfeff7fa_u64, 0x1fffeff7fbfc22_u64,
    0x20408001001_u64, 0x7fffeffff77fd_u64, 0x3ffffbf7dfeec_u64, 0x1ffff9dffa333_u64,
];

#[rustfmt::skip]
#[allow(non_upper_case_globals)]
const RookOffset: [usize; 64] = [
    41305, 14326, 24477, 8223,
    49795, 60546, 28543, 79282,
    6457, 4125, 81021, 42341,
    14139, 19465, 9514, 71090,
    75419, 33476, 27117, 85964,
    54915, 36544, 71854, 37996,
    30398, 55939, 53891, 56963,
    77451, 12319, 88500, 51405,
    72878, 676, 83122, 22206,
    75186, 681, 36453, 20369,
    1981, 13343, 10650, 57987,
    26302, 58357, 40546, 0,
    14967, 80361, 40905, 58347,
    20381, 81868, 59381, 84404,
    45811, 62898, 45796, 66994,
    67204, 32448, 62946, 17005
];

const WHITE: i32 = 0;
const BLACK: i32 = 1;

impl BitboardAttacks for Magic {
    // inclusive of end points
    #[inline]
    fn strictly_between(&self, s1: Square, s2: Square) -> Bitboard {
        Hyperbola::default().strictly_between(s1, s2)
    }

    #[inline]
    fn line_through(&self, s1: Square, s2: Square) -> Bitboard {
        Hyperbola::default().line_through(s1, s2)
    }

    #[inline]
    fn knight_attacks(&self, sq: Square) -> Bitboard {
        Bitboard::from_u64(self.KnightMask[sq])
    }

    #[inline]
    fn bishop_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let occ = occ.bits();
        let bits = self.MagicTable[(BISHOP_OFFSET[sq]
            + (((occ & self.BishopMask[sq]).wrapping_mul(BISHOP_MAGIC[sq])) >> 55) as usize)];
        Bitboard::from_u64(bits)
    }

    #[inline]
    fn rook_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let occ = occ.bits();
        // debug_assert!(sq >= 0 && sq <= 63);
        let bits = self.MagicTable
            [(RookOffset[sq] + (((occ & self.RookMask[sq]).wrapping_mul(ROOK_MAGIC[sq])) >> 52) as usize)];
        Bitboard::from_u64(bits)
    }

    #[inline]
    fn king_attacks(&self, sq: Square) -> Bitboard {
        Bitboard::from_u64(self.KingMask[sq])
    }
}

impl Magic {


    // #[inline]
    // pub fn pawn_attacks(&self, side: i32, sq: i32) -> u64 {
    //     debug_assert!(sq >= 0 && sq <= 63);
    //     debug_assert!(side == WHITE || side == BLACK);
    //     return self.PawnMask[side as usize][sq as usize];
    // }

    // #[inline]
    // fn queen_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
    //     // debug_assert!(sq >= 0 && sq <= 63);
    //     return self.rook_attacks(occ, sq) | self.bishop_attacks(occ, sq);
    // }



    // Steffan Westcott's innovation.
    #[inline]
    fn snoob(set: u64, subset: u64) -> u64 {
        return (subset.wrapping_sub(set)) & set;
    }

    // Taken from Tord Romstad's example Looking for Magics code.
    fn calc_rook_mask(sq: i32) -> u64 {
        let mut result = 0_u64;
        let rk = sq / 8;
        let fl = sq % 8;
        for r in (rk + 1)..=6 {
            result |= 1_u64 << (fl + r * 8);
        }
        for r in (1..=(rk - 1)).rev() {
            result |= 1_u64 << (fl + r * 8);
        }
        for f in fl + 1..=6 {
            result |= 1_u64 << (f + rk * 8);
        }
        for f in (1..=(fl - 1)).rev() {
            result |= 1_u64 << (f + rk * 8);
        }
        return result;
    }

    // Also taken from Tord Romstad's example Looking for Magics code.
    fn calc_bishop_mask(sq: i32) -> u64 {
        let mut result = 0_u64;
        let rk = sq / 8;
        let fl = sq % 8;
        let mut r = rk + 1;
        let mut f = fl + 1;
        while r <= 6 && f <= 6 {
            result |= 1_u64 << (f + r * 8);
            r += 1;
            f += 1;
        }
        let mut r = rk + 1;
        let mut f = fl - 1;
        while r <= 6 && f >= 1 {
            result |= 1_u64 << (f + r * 8);
            r += 1;
            f -= 1;
        }
        let mut r = rk - 1;
        let mut f = fl + 1;
        while r >= 1 && f <= 6 {
            result |= 1_u64 << (f + r * 8);
            r -= 1;
            f += 1;
        }
        let mut r = rk - 1;
        let mut f = fl - 1;
        while r >= 1 && f >= 1 {
            result |= 1_u64 << (f + r * 8);
            r -= 1;
            f -= 1;
        }
        return result;
    }

    // Likewise.
    fn calc_rook_attacks(sq: i32, block: u64) -> u64 {
        let mut result = 0_u64;
        let rk = sq / 8;
        let fl = sq % 8;

        let mut r = rk + 1;
        while r <= 7 {
            result |= 1_u64 << (fl + r * 8);
            if block & (1_u64 << (fl + r * 8)) > 0 {
                break;
            }
            r += 1;
        }

        let mut r = rk - 1;
        while r >= 0 {
            result |= 1_u64 << (fl + r * 8);
            if block & (1_u64 << (fl + r * 8)) > 0 {
                break;
            }
            r -= 1;
        }

        let mut f = fl + 1;
        while f <= 7 {
            result |= 1_u64 << (f + rk * 8);
            if block & (1_u64 << (f + rk * 8)) > 0 {
                break;
            }
            f += 1;
        }

        let mut f = fl - 1;
        while f >= 0 {
            result |= 1_u64 << (f + rk * 8);
            if block & (1_u64 << (f + rk * 8)) > 0 {
                break;
            }
            f -= 1;
        }
        return result;
    }

    // Likewise. At this point I'll be banned from all ICGA tournaments.
    // Such is the price of laziness.
    fn calc_bishop_attacks(sq: i32, block: u64) -> u64 {
        let mut result = 0_u64;
        let rk = sq / 8;
        let fl = sq % 8;
        let mut r = rk + 1;
        let mut f = fl + 1;
        while r <= 7 && f <= 7 {
            result |= 1_u64 << (f + r * 8);
            if block & (1_u64 << (f + r * 8)) > 0 {
                break;
            }
            r += 1;
            f += 1;
        }
        let mut r = rk + 1;
        let mut f = fl - 1;
        while r <= 7 && f >= 0 {
            result |= 1_u64 << (f + r * 8);
            if block & (1_u64 << (f + r * 8)) > 0 {
                break;
            }
            r += 1;
            f -= 1;
        }
        let mut r = rk - 1;
        let mut f = fl + 1;
        while r >= 0 && f <= 7 {
            result |= 1_u64 << (f + r * 8);
            if block & (1_u64 << (f + r * 8)) > 0 {
                break;
            }
            r -= 1;
            f += 1;
        }
        let mut r = rk - 1;
        let mut f = fl - 1;
        while r >= 0 && f >= 0 {
            result |= 1_u64 << (f + r * 8);
            if block & (1_u64 << (f + r * 8)) > 0 {
                break;
            }
            r -= 1;
            f -= 1;
        }
        return result;
    }

    // fn PrintU64(board: u64)
    // {
    //     int i;
    //     for(i = 63; i >= 0; --i)
    //     {
    //         printf("%i", (int)(1&(board>>i)));
    //         if(i%8 == 0) {printf("\n");}
    //     }
    //     printf("\n");
    //     return;
    // }


    fn new() -> Box<Magic> {
        let mut magic = Box::new(Magic {
            MagicTable: [0; 89524],
            PawnMask: [[0; 64]; 2],
            KnightMask: [0; 64],
            BishopMask: [0; 64],
            RookMask: [0; 64],
            KingMask: [0; 64],
        });

        // Pawns
        for sq in 0..64 {
            let from = 1 << sq;
            magic.PawnMask[WHITE as usize][sq] = ((from << 7) & !FileHMask) | ((from << 9) & !FileAMask);
            magic.PawnMask[BLACK as usize][sq] = ((from >> 7) & !FileAMask) | ((from >> 9) & !FileHMask);
        }

        // Knights
        for sq in 0..64 {
            let from = 1 << sq;
            magic.KnightMask[sq] = (from >> 17) & (!FileHMask); // Up 2 right 1
            magic.KnightMask[sq] |= (from >> 15) & (!FileAMask); // Up 2 left 1
            magic.KnightMask[sq] |= (from << 17) & (!FileAMask); // Down 2 left 1
            magic.KnightMask[sq] |= (from << 15) & (!FileHMask); // Down 2 right 1
            magic.KnightMask[sq] |= (from >> 10) & !(FileGMask | FileHMask); // Right 2 up 1
            magic.KnightMask[sq] |= (from << 6) & !(FileGMask | FileHMask); // Right 2 down 1
            magic.KnightMask[sq] |= (from >> 6) & !(FileAMask | FileBMask); // Left 2 up 1
            magic.KnightMask[sq] |= (from << 10) & !(FileAMask | FileBMask); // Left 2 down 1
        }

        // Bishops
        for sq in 0..64 {
            let mut b = 0;
            magic.BishopMask[sq] = Self::calc_bishop_mask(sq as i32);

            loop {
                let i = BISHOP_OFFSET[sq]
                    + (((b & magic.BishopMask[sq]).wrapping_mul(BISHOP_MAGIC[sq])) >> 55) as usize;
                magic.MagicTable[i] = Self::calc_bishop_attacks(sq as i32, b);
                b = Self::snoob(magic.BishopMask[sq], b);
                if b == 0 {
                    break;
                }
            }
        }

        // Rooks
        for sq in 0..64 {
            let mut b = 0;
            magic.RookMask[sq] = Self::calc_rook_mask(sq as i32);

            loop {
                let i =
                    RookOffset[sq] + (((b & magic.RookMask[sq]).wrapping_mul(ROOK_MAGIC[sq])) >> 52) as usize;
                magic.MagicTable[i] = Self::calc_rook_attacks(sq as i32, b);
                b = Self::snoob(magic.RookMask[sq], b);
                if b == 0 {
                    break;
                }
            }
        }

        // Kings
        for sq in 0..64 {
            let from = 1 << sq;
            magic.KingMask[sq] = from >> 8; // Up 1
            magic.KingMask[sq] |= from << 8; // Down 1
            magic.KingMask[sq] |= (from >> 1) & (!FileHMask); // Right 1
            magic.KingMask[sq] |= (from << 1) & (!FileAMask); // Left 1
            magic.KingMask[sq] |= (from >> 7) & (!FileAMask); // Up 1 Left 1
            magic.KingMask[sq] |= (from >> 9) & (!FileHMask); // Up 1 Right 1
            magic.KingMask[sq] |= (from << 7) & (!FileHMask); // Down 1 Right 1
            magic.KingMask[sq] |= (from << 9) & (!FileAMask); // Down 1 Left 1
        }
        magic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::attacks::BitboardAttacks;
    use crate::bitboard::bb_classical::ClassicalBitboard;
    use crate::bitboard::bitboard::Bitboard;
    use crate::globals::constants::*;

    #[test]
    fn test_magic_rook_attacks() {
        let mg = Magic::default();
        let occ = a1 | a2 | a7 | c3 | c6;
        let attacks = mg.rook_attacks(occ, a6.square());
        assert_eq!(attacks, (Bitboard::FILE_A - a1 - a6 - a8) | b6 | c6);
    }

    #[test]
    fn test_magic_bishop_attacks() {
        let mag = Magic::default();
        let occ = a1 | a2 | a7 | c3 | c6;
        let attacks = mag.bishop_attacks(occ, a6.square());
        assert_eq!(attacks, f1 | e2 | d3 | c4 | b5 | b7 | c8);

        let occ = b2;
        let attacks = mag.bishop_attacks(occ, c1.square());
        assert_eq!(attacks, b2 | d2 | e3 | f4 | g5 | h6);
    }

    #[test]
    fn test_magic_vs_classical_quick() {
        let cb = ClassicalBitboard::default();
        let mg = Magic::default();
        let occ = a3 | b5 | f3 | g4 | h4;
        let line_pieces = a1 | d5 | h8 | a5 | b3 | b8;
        let att1 = line_pieces.squares().map(|sq| cb.bishop_attacks(occ, sq));
        let att2 = line_pieces.squares().map(|sq| mg.bishop_attacks(occ, sq));
        assert!(att1.eq(att2));

        let att1 = line_pieces.squares().map(|sq| cb.rook_attacks(occ, sq));
        let att2 = line_pieces.squares().map(|sq| mg.rook_attacks(occ, sq));
        assert!(att1.eq(att2));

        let occ = b1 | c1 | d1 | e1 | f1 | g1;
        let att1: Vec<_> = a1.squares().map(|sq| cb.rook_attacks(occ, sq)).collect();
        let att2: Vec<_> = a1.squares().map(|sq| mg.rook_attacks(occ, sq)).collect();
        assert_eq!(att1, att2);

        for sq in Bitboard::all().squares() {
            let occ = b5 | f3 | g4 | h4;
            assert_eq!(
                mg.bishop_attacks(occ, sq),
                cb.bishop_attacks(occ, sq),
                "square {:?}",
                sq.as_bb()
            );
            assert_eq!(
                mg.rook_attacks(occ, sq),
                cb.rook_attacks(occ, sq),
                "square {:?}",
                sq.as_bb()
            );
        }
    }

    #[test]
    fn test_magic_vs_classical_slow() {
        let cb = ClassicalBitboard::default();
        let mg = Magic::default();
        for sq in Bitboard::all().squares() {
            for f in sq.file().power_set_iter() {
                for r in sq.rank().power_set_iter() {
                    let occ = f | r;
                    assert_eq!(
                        mg.rook_attacks(occ, sq),
                        cb.rook_attacks(occ, sq),
                        "square {:?} occ f:{:?} r:{:?}",
                        sq.as_bb(),
                        f,
                        r
                    );
                }
            }
            for d in sq.diag().power_set_iter() {
                for ad in sq.anti_diag().power_set_iter() {
                    let occ = d | ad;
                    assert_eq!(
                        mg.bishop_attacks(occ, sq),
                        cb.bishop_attacks(occ, sq),
                        "square {:?}",
                        sq.as_bb()
                    );
                }
            }
        }
    }
}
