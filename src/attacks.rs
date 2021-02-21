use crate::bitboard::{Bitboard, Dir};
// use lazy_static::lazy_static;

pub trait SlidingAttacks {
    // fn init() -> Self;
    //fn bishop_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    fn rook_attacks(&self, occupied: Bitboard, from_sq: usize) -> Bitboard;
    //fn knight_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    //fn king_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;

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
    attacks: [[Bitboard; 8]; 64],
}

impl Classical {
    fn init() -> Classical {
        // let mut attacks = [[Bitboard::EMPTY; 8]; 64];
        let mut classical = Classical {
            attacks: [[Bitboard::A1; 8]; 64],
        };
        for sq in 0..64 {
            for dir in Dir::ALL.iter() {
                let bb = Bitboard::from_sq(sq);
                let mask = Self::ray(dir, bb);
                classical.attacks[sq as usize][dir.index] = mask;
            }
        }
        classical
    }

    fn attacks(&self, occupied: Bitboard, from_sq: usize, dir: &Dir) -> Bitboard {
        let attacks = self.attacks[from_sq][dir.index];
        let blockers = attacks & (occupied | dir.mask); // add in the edge square so there is at least one blocker

        let blocker_sq = if dir.index > 0 {
            blockers.first_square()
        } else {
            blockers.last_square()
        };
        // remove attacks from blocker sq and beyond
        println!("attcks::: dir:{}, from:sq:{} blocker_sq:{} \n",  dir.index, from_sq, blocker_sq);
        println!("blockers:\n{} \nattacks:\n{} \n",blockers, attacks);
        println!("minus\n{}\n", self.attacks[blocker_sq][dir.index]);
        attacks - self.attacks[blocker_sq][dir.index]
    }
}

impl SlidingAttacks for Classical {
    fn rook_attacks(&self, occ: Bitboard, from_sq: usize) -> Bitboard {
        self.attacks(occ, from_sq, &Dir::N)
            | self.attacks(occ, from_sq, &Dir::E)
            | self.attacks(occ, from_sq, &Dir::S)
            | self.attacks(occ, from_sq, &Dir::W)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let c3 = Bitboard::C3;
        let north = Classical::ray(&Dir::N, c3);
        println!("\nN \n{}", north);
        assert!(north.contains(Bitboard::C8));
        assert_eq!(north.count(), 5);

        println!("\nNE\n{}", Classical::ray(&Dir::NE, c3));
        println!("\nSW\n{}", Classical::ray(&Dir::SW, c3));
        println!("\nS \n{}", Classical::ray(&Dir::S, c3));
        println!("\nNW\n{}", Classical::ray(&Dir::NW, c3));

        let classical = Classical::init();
        let north = classical.attacks[16 + 2][Dir::N.index];
        println!("\nNORTH\n{}", north);
        assert!(north.contains(Bitboard::C8));
        assert_eq!(north.count(), 5);
    }

    #[test]
    fn rook_attacks() {
        let classical = Classical::init();
        let occupied = Bitboard::A1 | Bitboard::A2 | Bitboard::A7 | Bitboard::C3 | Bitboard::C6;
        let attacks = classical.rook_attacks(occupied, Bitboard::A6.first_square());
        println!("\nocc\n{} \nrook\n{} \nattacks\n{}", occupied, Bitboard::A6, attacks);
    
    }

        // assert!(a1b2.contains(Bitboard::A1));
        //assert!(a1b2 & Bitboard::C1 == Bitboard::A1 - Bitboard::A1);
}
