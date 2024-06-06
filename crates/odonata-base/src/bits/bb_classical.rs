use crate::bits::bitboard::{Bitboard, Dir};
use crate::bits::square::Square;

// inspired by https://www.chessprogramming.org/Hiding_the_Implementation
//
pub struct ClassicalBitboard {
    rays:         [[Bitboard; 8]; 64],
    king_moves:   [Bitboard; 64],
    knight_moves: [Bitboard; 64],
}

static STATIC_INSTANCE: ClassicalBitboard = ClassicalBitboard::new();

impl ClassicalBitboard {
    // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn instance() -> &'static Self {
        &STATIC_INSTANCE
    }

    pub const fn new() -> ClassicalBitboard {
        // let mut attacks = [[Bitboard::EMPTY; 8]; 64];
        let mut classical = ClassicalBitboard {
            rays:         [[Bitboard::EMPTY; 8]; 64],
            king_moves:   [Bitboard::EMPTY; 64],
            knight_moves: [Bitboard::EMPTY; 64],
        };
        let mut sq_i = 0;
        while sq_i < 64 {
            let sq = Square::from_usize(sq_i);
            let mut d = 0;

            while d < 8 {
                let dir = Dir::ALL[d];
                let bb = Bitboard::from_sq(sq as u16);
                let mask = bb.rays(dir);
                classical.rays[sq_i][d] = mask;
                classical.king_moves[sq_i] = classical.king_moves[sq_i].or(bb.shift(dir));

                // for example a knight attack might be step N followed by step NE
                let next_dir = dir.rotate_clockwise();
                classical.knight_moves[sq_i] = classical.knight_moves[sq_i].or(bb.shift(dir).shift(next_dir));
                d += 1;
            }
            sq_i += 1;
        }
        classical
    }

    #[inline]
    const fn sliding_attacks(&self, occupied: Bitboard, from: Square, dir: Dir) -> Bitboard {
        let attacks = self.rays[from.index()][dir.index()];
        let blockers = attacks.and(occupied);

        let blocker_sq = if dir.shift() > 0 {
            blockers.find_first_square()
        } else {
            blockers.find_last_square()
        };

        // blockers is empty
        let Some(blocker_sq) = blocker_sq else {
            return attacks;
        };
        // remove attacks from blocker sq and beyond
        attacks.sub(self.rays[blocker_sq.index()][dir.index()])
    }

    #[inline]
    pub fn rook_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.sliding_attacks(occ, from, Dir::N)
            | self.sliding_attacks(occ, from, Dir::E)
            | self.sliding_attacks(occ, from, Dir::S)
            | self.sliding_attacks(occ, from, Dir::W)
    }

    #[inline]
    pub fn bishop_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.sliding_attacks(occ, from, Dir::NE)
            | self.sliding_attacks(occ, from, Dir::SE)
            | self.sliding_attacks(occ, from, Dir::SW)
            | self.sliding_attacks(occ, from, Dir::NW)
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;
    use Square::*;

    use super::*;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<ClassicalBitboard>(), 5120, "ClassicalBitboard");
    }

    #[test]
    fn test_rays() {
        let classical = ClassicalBitboard::new();
        let north = classical.rays[16 + 2][Dir::N.index()];
        assert!(north.contains(Bitboard::C8));
        assert_eq!(north.popcount(), 5);
    }

    #[test]
    fn test_rook_attacks() {
        let classical = ClassicalBitboard::new();
        let occupied = A1 | A2 | A7 | C3 | C6;
        let attacks = classical.rook_attacks(occupied, Square::A6);
        assert_eq!(attacks, (Bitboard::FILE_A - (A1 | A6 | A8)) | B6 | C6)
    }

    #[test]
    fn test_bishop_attacks() {
        let classical = ClassicalBitboard::new();
        let occupied = A1 | A2 | A7 | C3 | C6;
        let attacks = classical.bishop_attacks(occupied, Square::A6);
        assert_eq!(attacks, F1 | E2 | D3 | C4 | B5 | B7 | C8);

        let occupied = Bitboard::B2;
        let attacks = classical.bishop_attacks(occupied, Square::C1);
        assert_eq!(attacks, B2 | D2 | E3 | F4 | G5 | H6);
    }
}
