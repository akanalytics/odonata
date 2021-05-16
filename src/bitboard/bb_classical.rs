use crate::bitboard::bitboard::{Bitboard, Square, Dir};
use crate::bitboard::attacks::BitboardAttacks;
use once_cell::sync::Lazy;




// inspired by https://www.chessprogramming.org/Hiding_the_Implementation
//
pub struct ClassicalBitboard {
    rays: [[Bitboard; 8]; 64],
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
}

static STATIC_INSTANCE: Lazy<ClassicalBitboard> = Lazy::new(|| ClassicalBitboard::new());




impl ClassicalBitboard {
    pub fn new() -> ClassicalBitboard {
        // let mut attacks = [[Bitboard::EMPTY; 8]; 64];
        let mut classical = ClassicalBitboard {
            rays: [[Bitboard::EMPTY; 8]; 64],
            king_moves: [Bitboard::EMPTY; 64],
            knight_moves: [Bitboard::EMPTY; 64],
        };
        for sq in 0..64_usize {
            for &dir in Dir::ALL.iter() {
                let bb = Bitboard::from_sq(sq as u32);
                let mask = bb.ray(dir);
                classical.rays[sq][dir.index] = mask;
                classical.king_moves[sq] |= bb.shift(dir);

                // for example a night attack might be step N followed by step NE
                let next_dir = Dir::ALL[(dir.index + 1) % 8];
                classical.knight_moves[sq] |= bb.shift(dir).shift(next_dir);
            }
        }
        classical
    }

    // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn default() -> &'static Self {
        &STATIC_INSTANCE
    }



    fn sliding_attacks(&self, occupied: Bitboard, from: Square, dir: &Dir) -> Bitboard {
        let attacks = self.rays[from.index()][dir.index];
        let blockers = attacks & occupied;

        if blockers.is_empty() {
            return attacks;
        }
        let blocker_sq = if dir.shift > 0 { blockers.first_square() } else { blockers.last_square() };
        // println!("attcks::: dir:{}, from:sq:{} blockers: {:?} blocker_sq:{} \n",  dir.index, from_sq, blockers, blocker_sq);
        // println!("blockers:\n{} \nattacks:\n{} \n",blockers, attacks);
        // println!("minus\n{}\n", self.attacks[blocker_sq][dir.index]);
        // remove attacks from blocker sq and beyond
        attacks - self.rays[blocker_sq.index()][dir.index]
    }
}

impl BitboardAttacks for ClassicalBitboard {

    fn between(&self, s1:Square, s2: Square) -> Bitboard {
        Square::line_through(s1, s2) & Square::bounding_rectangle(s1, s2)
    }

    #[inline]
    fn rook_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.sliding_attacks(occ, from, &Dir::N)
            | self.sliding_attacks(occ, from, &Dir::E)
            | self.sliding_attacks(occ, from, &Dir::S)
            | self.sliding_attacks(occ, from, &Dir::W)
    }

    #[inline]
    fn bishop_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.sliding_attacks(occ, from, &Dir::NE)
            | self.sliding_attacks(occ, from, &Dir::SE)
            | self.sliding_attacks(occ, from, &Dir::SW)
            | self.sliding_attacks(occ, from, &Dir::NW)
    }

    #[inline]
    fn king_attacks(&self, from: Square) -> Bitboard {
        self.king_moves[from.index()]
    }

    #[inline]
    fn knight_attacks(&self, from: Square) -> Bitboard {
        self.knight_moves[from.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::constants::*;

    #[test]
    fn test_rays() {
        let classical = ClassicalBitboard::new();
        let north = classical.rays[16 + 2][Dir::N.index];
        assert!(north.contains(c8));
        assert_eq!(north.popcount(), 5);
    }

    fn init() {
        // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[test]
    fn test_rook_attacks() {
        let classical = ClassicalBitboard::new();
        let occupied = a1 | a2 | a7 | c3 | c6;
        let attacks = classical.rook_attacks(occupied, a6.square());
        assert_eq!(attacks, (Bitboard::FILE_A - a1 - a6 - a8) | b6 | c6)
    }

    #[test]
    fn test_bishop_attacks() {
        let classical = ClassicalBitboard::new();
        let occupied = a1 | a2 | a7 | c3 | c6;
        let attacks = classical.bishop_attacks(occupied, a6.square());
        assert_eq!(attacks, f1 | e2 | d3 | c4 | b5 | b7 | c8);

        let occupied = b2;
        let attacks = classical.bishop_attacks(occupied, c1.square());
        assert_eq!(attacks, b2 | d2 | e3 | f4 | g5 | h6);
    }

    #[test]
    fn test_king_attacks() {
        init();
        let classical = ClassicalBitboard::new();
        let attacks = classical.king_attacks(a6.square());
        assert_eq!(attacks, a5 | b5 | b6 | b7 | a7);

        let attacks = classical.king_attacks(c6.square());
        assert_eq!(attacks, b5 | c5 | d5 | b6 | d6 | b7 | c7 | d7)
    }

    #[test]
    fn test_knight_attacks() {
        let classical = ClassicalBitboard::new();
        let attacks = classical.knight_attacks(a1.square());
        assert_eq!(attacks, b3 | c2);

        let attacks = classical.knight_attacks(c6.square());
        assert_eq!(attacks, a5 | a7 | b4 | b8 | d4 | d8 | e5 | e7)
    }
}