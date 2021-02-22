use crate::bitboard::{Bitboard, Dir};
// use lazy_static::lazy_static;

pub trait SlidingAttacks {
    // fn init() -> Self;
    fn bishop_attacks(&self, occupied: Bitboard, from_sq: usize) -> Bitboard;
    fn rook_attacks(&self, occupied: Bitboard, from_sq: usize) -> Bitboard;
    fn knight_attacks(&self, from_sq: usize) -> Bitboard;
    fn king_attacks(&self, from_sq: usize) -> Bitboard;

    // fn step_is_on_board(dir: Direction, sq: i32) -> bool {
    //     false
    // }

    // excludes the src square itself, but includes edge squares
    fn ray(dir: &Dir, src: Bitboard) -> Bitboard {
        let mut sq = src;
        let mut bb = Bitboard::EMPTY;
        while !sq.is_empty() {
            sq = sq.shift(dir);
            bb |= sq;
        }
        bb
    }
}

pub struct Classical {
    sliding_attacks: [[Bitboard; 8]; 64],
    king_attacks: [Bitboard; 64],
    knight_attacks: [Bitboard; 64],
}

impl Classical {
    fn init() -> Classical {
        // let mut attacks = [[Bitboard::EMPTY; 8]; 64];
        let mut classical = Classical {
            sliding_attacks: [[Bitboard::EMPTY; 8]; 64],
            king_attacks: [Bitboard::EMPTY; 64],
            knight_attacks: [Bitboard::EMPTY; 64],
        };
        for sq in 0..64_usize {
            for dir in Dir::ALL.iter() {
                let bb = Bitboard::from_sq(sq as u32);
                let mask = Self::ray(dir, bb);
                classical.sliding_attacks[sq][dir.index] = mask;
                classical.king_attacks[sq] |= bb.shift(dir);

                // for example a night attack might be step N followed by step NE
                let next_dir = &Dir::ALL[(dir.index+1) % 8];
                classical.knight_attacks[sq] |= bb.shift(dir).shift(next_dir);
            }
        }
        classical
    }

    fn attacks(&self, occupied: Bitboard, from_sq: usize, dir: &Dir) -> Bitboard {
        let attacks = self.sliding_attacks[from_sq][dir.index];
        let blockers = attacks & occupied;

        if blockers.is_empty() {
            return attacks
        }
        let blocker_sq = if dir.shift > 0 {
            blockers.first_square()
        } else {
            blockers.last_square()
        };
        // println!("attcks::: dir:{}, from:sq:{} blockers: {:?} blocker_sq:{} \n",  dir.index, from_sq, blockers, blocker_sq);
        // println!("blockers:\n{} \nattacks:\n{} \n",blockers, attacks);
        // println!("minus\n{}\n", self.attacks[blocker_sq][dir.index]);
        // remove attacks from blocker sq and beyond
        attacks - self.sliding_attacks[blocker_sq][dir.index]
    }
}

impl SlidingAttacks for Classical {
    fn rook_attacks(&self, occ: Bitboard, from_sq: usize) -> Bitboard {
        self.attacks(occ, from_sq, &Dir::N)
            | self.attacks(occ, from_sq, &Dir::E)
            | self.attacks(occ, from_sq, &Dir::S)
            | self.attacks(occ, from_sq, &Dir::W)
    }

    fn bishop_attacks(&self, occ: Bitboard, from_sq: usize) -> Bitboard {
        self.attacks(occ, from_sq, &Dir::NE)
            | self.attacks(occ, from_sq, &Dir::SE)
            | self.attacks(occ, from_sq, &Dir::SW)
            | self.attacks(occ, from_sq, &Dir::NW)
    }

    fn king_attacks(&self, from_sq: usize) -> Bitboard {
        self.king_attacks[from_sq]
    }

    fn knight_attacks(&self, from_sq: usize) -> Bitboard {
        self.knight_attacks[from_sq]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let c3 = Bitboard::C3;
        let north = Classical::ray(&Dir::N, c3);
        assert!(north.contains(Bitboard::C8));
        assert_eq!(north.count(), 5);

        assert_eq!(Classical::ray(&Dir::NE, c3), Bitboard::D4 | Bitboard::E5 | Bitboard::F6 | Bitboard::G7 | Bitboard::H8);
        assert_eq!(Classical::ray(&Dir::SW, c3), Bitboard::A1 | Bitboard::B2);
        assert_eq!(Classical::ray(&Dir::S, c3), Bitboard::C1 | Bitboard::C2);
        assert_eq!(Classical::ray(&Dir::NW, c3), Bitboard::A5 | Bitboard::B4);

        let classical = Classical::init();
        let north = classical.sliding_attacks[16 + 2][Dir::N.index];
        assert!(north.contains(Bitboard::C8));
        assert_eq!(north.count(), 5);
    }

    #[test]
    fn test_rook_attacks() {
        let classical = Classical::init();
        let occupied = Bitboard::A1 | Bitboard::A2 | Bitboard::A7 | Bitboard::C3 | Bitboard::C6;
        let attacks = classical.rook_attacks(occupied, Bitboard::A6.first_square());
        assert_eq!(attacks, (Bitboard::FILE_A - Bitboard::A1 - Bitboard::A6 - Bitboard::A8) | Bitboard::B6 | Bitboard::C6)
    }

    #[test]
    fn test_bishop_attacks() {
        let classical = Classical::init();
        let occupied = Bitboard::A1 | Bitboard::A2 | Bitboard::A7 | Bitboard::C3 | Bitboard::C6;
        let attacks = classical.bishop_attacks(occupied, Bitboard::A6.first_square());
        assert_eq!(attacks, Bitboard::F1 | Bitboard::E2 | Bitboard::D3 | Bitboard::C4 | Bitboard::B5 | Bitboard::B7 | Bitboard::C8)
    }

    #[test]
    fn test_king_attacks() {
        let classical = Classical::init();
        let attacks = classical.king_attacks(Bitboard::A6.first_square());
        assert_eq!(attacks, Bitboard::A5 | Bitboard::B5 | Bitboard::B6 | Bitboard::B7 | Bitboard::A7);

        let attacks = classical.king_attacks(Bitboard::C6.first_square());
        assert_eq!(attacks, Bitboard::B5 | Bitboard::C5 | Bitboard::D5 | Bitboard::B6 | Bitboard::D6 | Bitboard::B7 | Bitboard::C7 | Bitboard::D7)
    }

    #[test]
    fn test_knight_attacks() {
        let classical = Classical::init();
        let attacks = classical.knight_attacks(Bitboard::A1.first_square());
        assert_eq!(attacks, Bitboard::B3 | Bitboard::C2);

        let attacks = classical.knight_attacks(Bitboard::C6.first_square());
        assert_eq!(attacks, Bitboard::A5 | Bitboard::A7 | Bitboard::B4 | Bitboard::B8 | Bitboard::D4 | Bitboard::D8 | Bitboard::E5 | Bitboard::E7)
    }
}
