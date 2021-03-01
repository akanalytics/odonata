use crate::bitboard::{Bitboard, Dir};
use crate::board::Color;

// use lazy_static::lazy_static;

pub trait BitboardAttacks {
    // fn new() -> Self;
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

    #[inline]
    fn pawn_pushes(&self, occupied: Bitboard, pawns: Bitboard, color: &Color) -> Bitboard {
        let empty = !occupied;
        let single = pawns.shift(&color.pawn_move) & empty;
        single | (single.shift(&color.pawn_move) & empty & color.double_push_dest_rank)
    }

    #[inline]
    fn pawn_attacks(&self, pawns: Bitboard, color: &Color) -> (Bitboard, Bitboard) {
        (pawns.shift(&color.pawn_capture_east), pawns.shift(&color.pawn_capture_west))
    }

    fn pawn_en_passant_captures(
        &self,
        pawns: Bitboard,
        opponent: Bitboard,
        color: &Color,
        en_passant: Bitboard,
    ) -> (Bitboard, Bitboard) {
        assert!(!en_passant.is_empty());
        let (east, west) = self.pawn_attacks(pawns, color);

        let enemy_pawn = en_passant.shift(&color.opposite().pawn_move);

        // check enemy have occupied the square one beyond en passant square
        if (enemy_pawn & opponent).is_empty() {
            return (Bitboard::EMPTY, Bitboard::EMPTY);
        }

        return (east & en_passant, west & en_passant);
    }
}

pub struct ClassicalBitboard {
    rays: [[Bitboard; 8]; 64],
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
}

impl ClassicalBitboard {
    pub fn new() -> ClassicalBitboard {
        // let mut attacks = [[Bitboard::EMPTY; 8]; 64];
        let mut classical = ClassicalBitboard {
            rays: [[Bitboard::EMPTY; 8]; 64],
            king_moves: [Bitboard::EMPTY; 64],
            knight_moves: [Bitboard::EMPTY; 64],
        };
        for sq in 0..64_usize {
            for dir in Dir::ALL.iter() {
                let bb = Bitboard::from_sq(sq as u32);
                let mask = Self::ray(dir, bb);
                classical.rays[sq][dir.index] = mask;
                classical.king_moves[sq] |= bb.shift(dir);

                // for example a night attack might be step N followed by step NE
                let next_dir = &Dir::ALL[(dir.index + 1) % 8];
                classical.knight_moves[sq] |= bb.shift(dir).shift(next_dir);
            }
        }
        classical
    }

    fn sliding_attacks(&self, occupied: Bitboard, from_sq: usize, dir: &Dir) -> Bitboard {
        let attacks = self.rays[from_sq][dir.index];
        let blockers = attacks & occupied;

        if blockers.is_empty() {
            return attacks;
        }
        let blocker_sq = if dir.shift > 0 { blockers.first_square() } else { blockers.last_square() };
        // println!("attcks::: dir:{}, from:sq:{} blockers: {:?} blocker_sq:{} \n",  dir.index, from_sq, blockers, blocker_sq);
        // println!("blockers:\n{} \nattacks:\n{} \n",blockers, attacks);
        // println!("minus\n{}\n", self.attacks[blocker_sq][dir.index]);
        // remove attacks from blocker sq and beyond
        attacks - self.rays[blocker_sq][dir.index]
    }
}

impl BitboardAttacks for ClassicalBitboard {
    fn rook_attacks(&self, occ: Bitboard, from_sq: usize) -> Bitboard {
        self.sliding_attacks(occ, from_sq, &Dir::N)
            | self.sliding_attacks(occ, from_sq, &Dir::E)
            | self.sliding_attacks(occ, from_sq, &Dir::S)
            | self.sliding_attacks(occ, from_sq, &Dir::W)
    }

    fn bishop_attacks(&self, occ: Bitboard, from_sq: usize) -> Bitboard {
        self.sliding_attacks(occ, from_sq, &Dir::NE)
            | self.sliding_attacks(occ, from_sq, &Dir::SE)
            | self.sliding_attacks(occ, from_sq, &Dir::SW)
            | self.sliding_attacks(occ, from_sq, &Dir::NW)
    }

    fn king_attacks(&self, from_sq: usize) -> Bitboard {
        self.king_moves[from_sq]
    }

    fn knight_attacks(&self, from_sq: usize) -> Bitboard {
        self.knight_moves[from_sq]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::constants::*;

    #[test]
    fn test_rays() {
        let north = ClassicalBitboard::ray(&Dir::N, c3);
        assert_eq!(north, c4 | c5 | c6 | c7 | c8);
        assert_eq!(north.len(), 5);

        assert_eq!(ClassicalBitboard::ray(&Dir::NE, c3), d4 | e5 | f6 | g7 | h8);
        assert_eq!(ClassicalBitboard::ray(&Dir::SW, c3), a1 | b2);
        assert_eq!(ClassicalBitboard::ray(&Dir::S, c3), c1 | c2);
        assert_eq!(ClassicalBitboard::ray(&Dir::NW, c3), a5 | b4);

        let classical = ClassicalBitboard::new();
        let north = classical.rays[16 + 2][Dir::N.index];
        assert!(north.contains(c8));
        assert_eq!(north.len(), 5);
    }

    #[test]
    fn test_rook_attacks() {
        let classical = ClassicalBitboard::new();
        let occupied = a1 | a2 | a7 | c3 | c6;
        let attacks = classical.rook_attacks(occupied, a6.first_square());
        assert_eq!(attacks, (Bitboard::FILE_A - a1 - a6 - a8) | b6 | c6)
    }

    #[test]
    fn test_bishop_attacks() {
        let classical = ClassicalBitboard::new();
        let occupied = a1 | a2 | a7 | c3 | c6;
        let attacks = classical.bishop_attacks(occupied, a6.first_square());
        assert_eq!(attacks, f1 | e2 | d3 | c4 | b5 | b7 | c8);

        let occupied = b2;
        let attacks = classical.bishop_attacks(occupied, c1.first_square());
        assert_eq!(attacks, b2 | d2 | e3 | f4 | g5 | h6);
    }

    #[test]
    fn test_king_attacks() {
        let classical = ClassicalBitboard::new();
        let attacks = classical.king_attacks(a6.first_square());
        assert_eq!(attacks, a5 | b5 | b6 | b7 | a7);

        let attacks = classical.king_attacks(c6.first_square());
        assert_eq!(attacks, b5 | c5 | d5 | b6 | d6 | b7 | c7 | d7)
    }

    #[test]
    fn test_knight_attacks() {
        let classical = ClassicalBitboard::new();
        let attacks = classical.knight_attacks(a1.first_square());
        assert_eq!(attacks, b3 | c2);

        let attacks = classical.knight_attacks(c6.first_square());
        assert_eq!(attacks, a5 | a7 | b4 | b8 | d4 | d8 | e5 | e7)
    }

    #[test]
    fn test_pawns() {
        let classical = ClassicalBitboard::new();
        let pawns_w = a2 | b3 | c2 | d7 | f5 | g4 | h4 | h5;
        let opponent = a4 | b4 | d3 | g5;
        let occupied = pawns_w | opponent;

        let pawn_single_push = classical.pawn_pushes(occupied, pawns_w, &Color::WHITE);
        let single = a3 | c3 | d8 | f6 | h6;
        let double = c4;
        assert_eq!(pawn_single_push, single | double);

        let (pawn_capture_e, pawn_capture_w) = classical.pawn_attacks(pawns_w, &Color::WHITE);
        assert_eq!(pawn_capture_e & opponent, d3);

        assert_eq!(pawn_capture_w & opponent, a4 | g5);

        let ep_square = g6;
        let (east, west) = classical.pawn_en_passant_captures(pawns_w, opponent, &Color::WHITE, ep_square);
        assert_eq!(east, g6);
        assert_eq!(west, g6);
    }
}
