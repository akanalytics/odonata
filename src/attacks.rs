use crate::bitboard::{Bitboard, Dir, Color};
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

    fn pawn_pushes(&self, empty: Bitboard, pawns: Bitboard, color: &Color) -> Bitboard {
        let single = pawns.shift(&color.pawn_move) & empty; 
        single | (single.shift(&color.pawn_move) & empty & color.double_push_dest_rank)
    }
}


// @Profiler
// def by_pawns_single_and_double_push(self, pawns: BitBoard, empty: BitBoard, color: Color) -> Tuple[BitBoard, BitBoard]:
//     att = pawns
//     if color in "wW":
//         single = (att << 8) & empty
//         double_push_to_rank = regions.RANK_4.bits
//         return single, (single << 8) & empty & double_push_to_rank
//     else:
//         single = (att >> 8) & empty
//         double_push_to_rank = regions.RANK_5.bits
//         return single, (single >> 8) & empty & double_push_to_rank


// @Profiler
// def by_pawns_capture_east_and_west(self, pawns: BitBoard, opponent: BitBoard, color: Color) -> Tuple[BitBoard,BitBoard]:
//     att = pawns
//     # captures
//     if color == "w":
//             captures_e = ((att & Attacks.NOT_H) << 9)
//             captures_w = ((att & Attacks.NOT_A) << 7)
//     else:
//             captures_e = ((att & Attacks.NOT_H) >> 7)
//             captures_w = ((att & Attacks.NOT_A) >> 9)

//     return  (captures_e & opponent, captures_w & opponent)













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
    use crate::globals::constants::*;

    #[test]
    fn test_rays() {
        let north = Classical::ray(&Dir::N, c3);
        assert_eq!(north, c4|c5|c6|c7|c8);
        assert_eq!(north.count(), 5);

        assert_eq!(Classical::ray(&Dir::NE, c3), d4|e5|f6|g7|h8);
        assert_eq!(Classical::ray(&Dir::SW, c3), a1|b2);
        assert_eq!(Classical::ray(&Dir::S, c3), c1|c2);
        assert_eq!(Classical::ray(&Dir::NW, c3), a5|b4);

        let classical = Classical::init();
        let north = classical.sliding_attacks[16 + 2][Dir::N.index];
        assert!(north.contains(c8));
        assert_eq!(north.count(), 5);
    }

    #[test]
    fn test_rook_attacks() {
        let classical = Classical::init();
        let occupied = a1|a2|a7|c3|c6;
        let attacks = classical.rook_attacks(occupied, Bitboard::A6.first_square());
        assert_eq!(attacks, (Bitboard::FILE_A - a1 - a6 - a8) | b6|c6)
    }

    #[test]
    fn test_bishop_attacks() {
        let classical = Classical::init();
        let occupied = a1|a2|a7|c3|c6;
        let attacks = classical.bishop_attacks(occupied, Bitboard::A6.first_square());
        assert_eq!(attacks, f1|e2|d3|c4|b5|b7|c8)
    }

    #[test]
    fn test_king_attacks() {
        let classical = Classical::init();
        let attacks = classical.king_attacks(Bitboard::A6.first_square());
        assert_eq!(attacks, a5|b5|b6|b7|a7);

        let attacks = classical.king_attacks(Bitboard::C6.first_square());
        assert_eq!(attacks, b5|c5|d5|b6|d6|b7|c7|d7)
    }

    #[test]
    fn test_knight_attacks() {
        let classical = Classical::init();
        let attacks = classical.knight_attacks(Bitboard::A1.first_square());
        assert_eq!(attacks, b3|c2);

        let attacks = classical.knight_attacks(Bitboard::C6.first_square());
        assert_eq!(attacks, a5|a7|b4|b8|d4|d8|e5|e7)
    }


    #[test]
    fn test_pawn_pushes() {
        let classical = Classical::init();
        let pawns_w = a2 | b3 | c2 | d7 | f5 | g4 | h4 | h5;
        let pawns_b = a4 | b4 | d3 | g5;
        let occupied = pawns_w | pawns_b;
        let empty = !occupied;
        let ep_square = g6;
        let pawn_single_push = classical.pawn_pushes(occupied, pawns_w, &Color::WHITE);
        // pawn_capture_e = Region(bits=Attacks().by_pawns_capture(pawns_w.bits, occupied.bits, pawns_b.bits, "w", True))
        // pawn_capture_w = Region(bits=Attacks().by_pawns_capture(pawns_w.bits, occupied.bits, pawns_b.bits, "w", False))
        // pawn_en_passant_e = Region(bits=Attacks().by_pawns_en_passant(pawns_w.bits, occupied.bits, pawns_b.bits, "w", ep_square.bits, True))
        // pawn_en_passant_w = Region(bits=Attacks().by_pawns_en_passant(pawns_w.bits, occupied.bits, pawns_b.bits, "w", ep_square.bits, False))
        let expect_single_push = a3 | c3 | d8 | f6 | h6;
        let expect_double_push = c4;
        assert_eq!(pawn_single_push, expect_single_push | expect_double_push)
    }

}
