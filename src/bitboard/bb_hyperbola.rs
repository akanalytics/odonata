use crate::bitboard::attacks::BitboardAttacks;
use crate::bitboard::bb_classical::ClassicalBitboard;
use crate::bitboard::bitboard::{Bitboard, Dir};
use crate::bitboard::square::Square;
use once_cell::sync::Lazy;

static STATIC_INSTANCE: Lazy<Hyperbola> = Lazy::new(|| Hyperbola::new());

// enum MaskType {
//     Diag = 0,
//     AntiDiag = 1,
//     File = 2,
//     Rank = 3,
// }

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
    between: [[Bitboard; 64]; 64],
    line: [[Bitboard; 64]; 64],
}

impl Hyperbola {
    fn new() -> Self {
        let mut me = Self {
            mask: [HyperbolaMask::default(); 64],
            rank_attacks: [[Bitboard::EMPTY; 8]; 64],
            between: [[Bitboard::EMPTY; 64]; 64],
            line: [[Bitboard::EMPTY; 64]; 64],
        };

        Self::pop_mask(&mut me.mask);
        Self::pop_rank_attacks(&mut me.rank_attacks);
        Self::pop_between(&mut me.between);
        Self::pop_line(&mut me.line);
        me
    }


    fn pop_between(between: &mut [[Bitboard; 64]; 64]) {
        for s1 in Bitboard::all().squares() {
            for s2 in Bitboard::all().squares() {
                between[s1][s2] = Square::calc_line_through(s1, s2) & Square::bounding_rectangle(s1, s2);

            }
        }
    }

    fn pop_line(line: &mut [[Bitboard; 64]; 64]) {
        for s1 in Bitboard::all().squares() {
            for s2 in Bitboard::all().squares() {
                line[s1][s2] = Square::calc_line_through(s1, s2);

            }
        }
    }

    fn pop_rank_attacks(rank_attacks: &mut [[Bitboard; 8]; 64]) {
        for occupancy_bits in 0..64 {
            let occ_incl_rook = Bitboard::from_u64(occupancy_bits).shift(Dir::E);
            for rook_sq in Bitboard::RANK_1.squares() {
                let occ = occ_incl_rook.exclude(rook_sq);

                let east_of_rook = rook_sq.as_bb().rays(Dir::E);
                let blockers = east_of_rook & occ;
                let east_att = if blockers.is_empty() {
                    east_of_rook
                } else {
                    east_of_rook - blockers.first().rays(Dir::E)
                };

                let west_of_rook = rook_sq.as_bb().rays(Dir::W);
                let blockers = west_of_rook & occ;
                let west_att = if blockers.is_empty() {
                    west_of_rook
                } else {
                    west_of_rook - blockers.last().rays(Dir::W)
                };
                rank_attacks[occupancy_bits as usize][rook_sq.file_index()] = east_att | west_att;
            }
        }
    }

    fn pop_mask(mask: &mut [HyperbolaMask; 64]) {
        Bitboard::all().squares().for_each(|s| {
            mask[s.index()].diag = s.as_bb().diag_flood().exclude(s);
            mask[s.index()].anti_diag = s.as_bb().anti_diag_flood().exclude(s);
            mask[s.index()].file = s.as_bb().file_flood().exclude(s);
        });
    }


    // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn default() -> &'static Self {
        &STATIC_INSTANCE
    }

    #[inline]
    fn hyperbola(&self, occ: Bitboard, sq: Square, mask: Bitboard) -> Bitboard {
        let o = occ & mask;
        let r = o.flip_vertical();
        let forward = o.wrapping_sub(sq.as_bb());
        // let reverse = r.wrapping_sub(sq.as_bb().flip_vertical()).flip_vertical();
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
        // const FILES_BCDEFG: Bitboard = Bitboard::all().xor(Bitboard::FILE_A.or(Bitboard::FILE_H));
        let file = sq.file_index();
        let rank = sq.rank_index() * 8;
        let occupied = (occ.bits() >> rank) & (63 << 1); // 63 << 1 is middle 6 bits
        let attacks = self.rank_attacks[occupied as usize >> 1][file]; // occupied >> 1 is now range 0-63
        Bitboard::from_u64((attacks.bits()) << rank)    
    }
}

impl BitboardAttacks for Hyperbola {

    // inclusive of end points
    #[inline]
    fn between(&self, s1: Square, s2: Square) -> Bitboard {
        self.between[s1][s2]
    }

    #[inline]
    fn line_through(&self, s1: Square, s2: Square) -> Bitboard {
        self.line[s1][s2]
    }

    #[inline]
    fn rook_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from].file) | self.rank_hyperbola(occ, from)
    }

    #[inline]
    fn bishop_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from].diag)
            | self.hyperbola(occ, from, self.mask[from].anti_diag)
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
    fn test_vs_classical_quick() {
        let cb = ClassicalBitboard::default();
        let hq = Hyperbola::default();
        let occ = a3 | b5 | f3 | g4 | h4;
        let line_pieces = a1 | d5 | h8 | a5 | b3 | b8;
        let att1 = line_pieces.squares().map(|sq| cb.bishop_attacks(occ, sq));
        let att2 = line_pieces.squares().map(|sq| hq.bishop_attacks(occ, sq));
        assert!(att1.eq(att2));

        let att1 = line_pieces.squares().map(|sq| cb.rook_attacks(occ, sq));
        let att2 = line_pieces.squares().map(|sq| hq.rook_attacks(occ, sq));
        assert!(att1.eq(att2));


        let occ = b1 | c1 | d1 | e1 | f1 | g1;
        let att1: Vec<_> = a1.squares().map(|sq| cb.rook_attacks(occ, sq)).collect();
        let att2: Vec<_> = a1.squares().map(|sq| hq.rook_attacks(occ, sq)).collect();
        assert_eq!(att1, att2);

        for sq in Bitboard::all().squares() {
            let occ = b5 | f3 | g4 | h4;
            assert_eq!(
                hq.bishop_attacks(occ, sq),
                cb.bishop_attacks(occ, sq),
                "square {:?}",
                sq.as_bb()
            );
            assert_eq!(
                hq.rook_attacks(occ, sq),
                cb.rook_attacks(occ, sq),
                "square {:?}",
                sq.as_bb()
            );
        }
    }

    #[test]
    fn test_vs_classical_slow() {
        let cb = ClassicalBitboard::default();
        let hq = Hyperbola::default();
        for sq in Bitboard::all().squares() {
            for f in sq.file().power_set_iter() {
                for r in sq.rank().power_set_iter() {
                    let occ = f | r;
                    assert_eq!(
                        hq.rook_attacks(occ, sq),
                        cb.rook_attacks(occ, sq),
                        "square {:?} occ f:{:?} r:{:?}",
                        sq.as_bb(), f, r
                    );
                }
            }
            for d in sq.diag().power_set_iter() {
                for ad in sq.anti_diag().power_set_iter() {
                    let occ = d | ad;
                    assert_eq!(
                        hq.bishop_attacks(occ, sq),
                        cb.bishop_attacks(occ, sq),
                        "square {:?}",
                        sq.as_bb()
                    );
                }
            }
        }
    }
}
