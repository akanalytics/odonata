use crate::bitboard::attacks::BitboardAttacks;
use crate::bitboard::bitboard::{Bitboard, Dir, Square};
use crate::bitboard::bb_classical::ClassicalBitboard;
use once_cell::sync::Lazy;


static STATIC_INSTANCE: Lazy<Hyperbola> = Lazy::new(|| Hyperbola::new());



enum MaskType {
    Diag = 0,
    AntiDiag = 1,
    File = 2,
    Rank = 3,
}

#[derive(Copy, Clone, Debug, Default)]
struct HyperbolaMask {
    diag: Bitboard,
    anti_diag: Bitboard,
    file: Bitboard,
    // rank: Bitboard,
}

#[derive(Clone, Debug)]
pub struct Hyperbola {
    mask: [HyperbolaMask; 64],
    rank_attacks: [[Bitboard; 8]; 64], // for perm of 6 bit-occupancy (64) and for each rook square (8)
}

impl Hyperbola {
    fn new() -> Self {
        let mut me = Self {
            mask: [HyperbolaMask::default(); 64],
            rank_attacks: [[Bitboard::EMPTY; 8]; 64],
        };

        Bitboard::all().squares().for_each(|s| {
            me.mask[s.index()].diag = s.as_bb().diag_flood();
            me.mask[s.index()].anti_diag = s.as_bb().anti_diag_flood();
            me.mask[s.index()].file = s.as_bb().file_flood();
            // println!(
            //     "#{}\n{}\n{}\n{}\n{}",
            //     s.index(),
            //     s.as_bb(),
            //     s.as_bb().diag_flood(),
            //     s.as_bb().anti_diag_flood(),
            //     s.as_bb().file_flood()
            // );
        });
        for occupancy_bits in 0..63 {
            let occ_incl_rook = Bitboard::from_bits_truncate(occupancy_bits).shift(Dir::E);
            for rook_sq in Bitboard::RANK_1.squares() {
                let occ = occ_incl_rook - rook_sq.as_bb();

                let east_of_rook = rook_sq.as_bb().ray(Dir::E);
                let blockers = east_of_rook & occ;
                let east_att = if blockers.is_empty() {
                    east_of_rook
                } else {
                    east_of_rook - blockers.first().ray(Dir::E)
                };

                let west_of_rook = rook_sq.as_bb().ray(Dir::W);
                let blockers = west_of_rook & occ;
                let west_att = if blockers.is_empty() {
                    west_of_rook
                } else {
                    west_of_rook - blockers.last().ray(Dir::W)
                };
                // println!(
                //     "{:>2} occ:{:08b} r:{:08b} er:{:08b} wr:{:08b} ea:{:08b} wa:{:08b} ",
                //     occupancy_bits,
                //     occ,
                //     rook_sq.as_bb(),
                //     east_of_rook,
                //     west_of_rook,
                //     east_att,
                //     west_att
                // );
                me.rank_attacks[occupancy_bits as usize][rook_sq.file_index()] = east_att | west_att;
            }
        }
        me
    }

    // doesnt impl Default as too large to copy by value
    pub fn default() -> &'static Self {
        &STATIC_INSTANCE
    }



    #[inline]
    fn hyperbola(&self, occ: Bitboard, sq: Square, mask: Bitboard) -> Bitboard {
        let o = occ & mask;
        let r = o.flip_vertical();
        let forward = o.wrapping_sub(sq.as_bb());
        let reverse = r.wrapping_sub(sq.flip_vertical().as_bb()).flip_vertical();
        (forward ^ reverse) & mask
    }



    // using_reverse_bits
    //
    // fn rank_hyperbola(&self, occ: Bitboard, sq: Square, mask: Bitboard) -> Bitboard {
    //     let o = occ & mask;
    //     let r = o.reverse_bits();
    //     let forward = o - sq.as_bb();
    //     let reverse = (r - Bitboard::from_sq(63 - sq.index() as u32);
    //     (forward ^ reverse) & mask
    // }
    // https://www.chessprogramming.org/First_Rank_Attacks#Attacks_on_all_Ranks
    //
    #[inline]
    fn rank_hyperbola(&self, occ: Bitboard, sq: Square) -> Bitboard {
        const FILES_BCDEFG: Bitboard = Bitboard::all().xor(Bitboard::FILE_A.or(Bitboard::FILE_H));
        let file = sq.file_index();
        let rank = sq.rank_index() * 8;
        let occupied = (occ.bits() >> rank) & (63 << 1); // 63 << 1 is middle 6 bits
        let attacks = self.rank_attacks[occupied as usize >> 1][file]; // occupied >> 1 is now range 0-63
        Bitboard::from_bits_truncate((attacks.bits()) << rank)
    }
}

impl BitboardAttacks for Hyperbola {
    #[inline]
    fn rook_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from.index()].file) | self.rank_hyperbola(occ, from)
    }

    #[inline]
    fn bishop_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from.index()].diag)
            | self.hyperbola(occ, from, self.mask[from.index()].anti_diag)
    }

    #[inline]
    fn king_attacks(&self, from: Square) -> Bitboard {
        ClassicalBitboard::default().king_attacks(from)
    }

    #[inline]
    fn knight_attacks(&self, from: Square) -> Bitboard {
        ClassicalBitboard::default().knight_attacks(from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::constants::*;

    #[test]
    fn test_rook_attacks() {
        let hq = Hyperbola::default();
        let occ = a1 | a2 | a7 | c3 | c6;
        // rank_6_occ = c6;
        // shift to RANK1 gives c1, shift down by 1 to B1 (occ=2), file(sq=a6) = 0
        assert_eq!(hq.rank_attacks[2][0], b1 | c1);
        assert_eq!(hq.rank_hyperbola(occ, a6.square()), b6 | c6);
        let attacks = hq.rook_attacks(occ, a6.square());
        assert_eq!(attacks, (Bitboard::FILE_A - a1 - a6 - a8) | b6 | c6);
    }

    #[test]
    fn test_bishop_attacks() {
        let hq = Hyperbola::default();
        let occ = a1 | a2 | a7 | c3 | c6;
        let attacks = hq.bishop_attacks(occ, a6.square());
        assert_eq!(attacks, f1 | e2 | d3 | c4 | b5 | b7 | c8);

        let occ = b2;
        let attacks = hq.bishop_attacks(occ, c1.square());
        assert_eq!(attacks, b2 | d2 | e3 | f4 | g5 | h6);
    }

    #[test]
    fn test_vs_classical() {
        let cl = ClassicalBitboard::default();
        let hq = Hyperbola::default();
        let occ = a3 | b5 | f3 | g4 | h4;
        let line_pieces = a1 | d5| h8;
        let att1 = line_pieces.squares().map(|sq| cl.bishop_attacks(occ,sq));
        let att2 = line_pieces.squares().map(|sq| hq.bishop_attacks(occ,sq));
        assert!(att1.eq(att2));

        let att1 = line_pieces.squares().map(|sq| cl.rook_attacks(occ,sq));
        let att2 = line_pieces.squares().map(|sq| hq.rook_attacks(occ,sq));
        assert!(att1.eq(att2));

    }
}
