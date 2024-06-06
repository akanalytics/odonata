use static_init::dynamic;

use crate::bits::bitboard::{Bitboard, Dir};
use crate::bits::square::Square;
use crate::piece::FlipVertical;

// #[ctor]
// fn init_module() {
//     Hyperbola::init();
// }

// static mut STATIC_INSTANCE: *const Hyperbola = std::ptr::null();
// impl Hyperbola {
//         pub fn init() {
//         let me = Self::new();
//         unsafe {
//             // leak the value, so it will never be dropped or freed
//             STATIC_INSTANCE = Box::leak(me) as *const Self;
//         }
//     }

//     // doesnt impl Default as too large to copy by value
//     #[inline]
//     pub fn default() -> &'static Self {
//         unsafe {
//             &*STATIC_INSTANCE
//         }
//     }

// }

#[dynamic]
static STATIC_INSTANCE: Box<Hyperbola> = Hyperbola::new_box();

impl Hyperbola {
    // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn instance() -> &'static Self {
        &STATIC_INSTANCE
    }
}

// static STATIC_INSTANCE: Lazy<Box<Hyperbola>> = Lazy::new(|| Hyperbola::new());
// impl Hyperbola {
//     // doesnt impl Default as too large to copy by value
//     #[inline]
//     pub fn default() -> &'static Self {
//         &STATIC_INSTANCE
//     }
// }

#[derive(Copy, Clone, Debug, Default)]
struct HyperbolaMask {
    diag:      Bitboard,
    anti_diag: Bitboard,
    file:      Bitboard,
    // rank: Bitboard,
}

impl HyperbolaMask {
    pub(super) const fn empty() -> Self {
        HyperbolaMask {
            diag:      Bitboard::empty(),
            anti_diag: Bitboard::empty(),
            file:      Bitboard::empty(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Hyperbola {
    mask:         [HyperbolaMask; 64],
    rank_attacks: [[Bitboard; 8]; 64], // for perm of 6 bit-occupancy (64) and for each rook square (8)
}

impl Hyperbola {
    pub(super) const fn new() -> Self {
        Self {
            mask:         Self::pop_mask(),
            rank_attacks: Self::pop_rank_attacks(),
        }
    }

    const fn pop_rank_attacks() -> [[Bitboard; 8]; 64] {
        let mut rank_attacks = [[Bitboard::empty(); 8]; 64];
        let mut occupancy_bits = 0;
        while occupancy_bits < 64 {
            let occ_incl_rook = Bitboard::from_u64(occupancy_bits).shift(Dir::E);
            let mut rook_sq_i = 0;
            while rook_sq_i < 8 {
                let rook_sq = Square::from_usize(rook_sq_i);
                let occ = occ_incl_rook.exclude(rook_sq);

                let east_of_rook = rook_sq.as_bb().rays(Dir::E);
                let blockers = east_of_rook.and(occ);
                let east_att = if let Some(sq) = blockers.find_first_square() {
                    east_of_rook.sub(sq.as_bb().rays(Dir::E))
                } else {
                    east_of_rook
                };

                let west_of_rook = rook_sq.as_bb().rays(Dir::W);
                let blockers = west_of_rook.and(occ);
                let west_att = if let Some(sq) = blockers.find_last_square() {
                    west_of_rook.sub(sq.as_bb().rays(Dir::W))
                } else {
                    west_of_rook
                };
                rank_attacks[occupancy_bits as usize][rook_sq.file_index()] = east_att.or(west_att);
                rook_sq_i += 1;
            }
            occupancy_bits += 1;
        }
        rank_attacks
    }

    const fn pop_mask() -> [HyperbolaMask; 64] {
        let mut mask = [HyperbolaMask::empty(); 64];
        let mut sq = 0;
        while sq < 64 {
            let s = Square::from_usize(sq);
            mask[s.index()].diag = s.as_bb().diag_flood().exclude(s);
            mask[s.index()].anti_diag = s.as_bb().anti_diag_flood().exclude(s);
            mask[s.index()].file = s.as_bb().file_flood().exclude(s);
            sq += 1;
        }
        mask
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

impl Hyperbola {
    fn new_box() -> Box<Self> {
        let mut me = Box::new(Self {
            mask:         [HyperbolaMask {
                diag:      Bitboard::EMPTY,
                anti_diag: Bitboard::EMPTY,
                file:      Bitboard::EMPTY,
            }; 64],
            rank_attacks: [[Bitboard::EMPTY; 8]; 64],
        });

        me.mask = Self::pop_mask();
        me.rank_attacks = Self::pop_rank_attacks();
        me
    }

    #[inline]
    pub fn rook_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from].file) | self.rank_hyperbola(occ, from)
    }

    #[inline]
    pub fn bishop_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from].diag) | self.hyperbola(occ, from, self.mask[from].anti_diag)
    }
}

#[cfg(test)]
mod tests {
    use Square::*;

    use super::*;
    use crate::bits::bb_classical::ClassicalBitboard;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<Hyperbola>(), 5632, "Hyperbola");
    }

    #[test]
    fn test_rook_attacks() {
        let hq = Hyperbola::instance();
        let occ = A1 | A2 | A7 | C3 | C6;
        // rank_6_occ = c6;
        // shift to RANK1 gives c1, shift down by 1 to B1 (occ=2), file(sq=a6) = 0
        assert_eq!(hq.rank_attacks[2][0], B1 | C1);
        assert_eq!(hq.rank_hyperbola(occ, A6), B6 | C6);
        let attacks = hq.rook_attacks(occ, A6);
        assert_eq!(attacks, (Bitboard::FILE_A - (A1 | A6 | A8)) | B6 | C6);
    }

    #[test]
    fn test_bishop_attacks() {
        let hq = Hyperbola::instance();
        let occ = A1 | A2 | A7 | C3 | C6;
        let attacks = hq.bishop_attacks(occ, Square::A6);
        assert_eq!(attacks, F1 | E2 | D3 | C4 | B5 | B7 | C8);

        let occ = Bitboard::B2;
        let attacks = hq.bishop_attacks(occ, Square::C1);
        assert_eq!(attacks, B2 | D2 | E3 | F4 | G5 | H6);
    }

    #[test]
    fn test_vs_classical_quick() {
        let cb = ClassicalBitboard::instance();
        let hq = Hyperbola::instance();
        let occ = A3 | B5 | F3 | G4 | H4;
        let line_pieces = A1 | D5 | H8 | A5 | B3 | B8;
        let att1 = line_pieces.squares().map(|sq| cb.bishop_attacks(occ, sq));
        let att2 = line_pieces.squares().map(|sq| hq.bishop_attacks(occ, sq));
        assert!(att1.eq(att2));

        let att1 = line_pieces.squares().map(|sq| cb.rook_attacks(occ, sq));
        let att2 = line_pieces.squares().map(|sq| hq.rook_attacks(occ, sq));
        assert!(att1.eq(att2));

        let occ = B1 | C1 | D1 | E1 | F1 | G1;
        let att1 = [A1].map(|sq| cb.rook_attacks(occ, sq));
        let att2 = [A1].map(|sq| hq.rook_attacks(occ, sq));
        assert_eq!(att1, att2);

        for sq in Bitboard::all().squares() {
            let occ = B5 | F3 | G4 | H4;
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
        let cb = ClassicalBitboard::instance();
        let hq = Hyperbola::instance();
        for sq in Bitboard::all().squares() {
            for f in sq.file_bitboard().power_set_iter() {
                for r in sq.rank_bitboard().power_set_iter() {
                    let occ = f | r;
                    assert_eq!(
                        hq.rook_attacks(occ, sq),
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
