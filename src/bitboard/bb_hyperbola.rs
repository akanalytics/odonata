use crate::bitboard::bitboard::{Bitboard, Square};
use crate::bitboard::attacks::BitboardAttacks;

enum MaskType { Diag= 0, AntiDiag = 1, File = 2, Rank = 3 }

#[derive(Copy,Clone, Debug, Default)]
struct HyperbolaMask {
    diag: Bitboard,
    anti_diag: Bitboard,
    file: Bitboard,
    rank: Bitboard,
} 


#[derive(Clone, Debug)]
struct Hyperbola {
    mask: [HyperbolaMask; 64],
    rank_attacks: [u8; 64*8],

}



impl Hyperbola {

    fn new() -> Self {
        let mut me  = Self {
            mask: [HyperbolaMask::default(); 64],
            rank_attacks: [0; 64*8],
        };
        
        // for sq in 0..64_usize {
        //     for &dir in Dir::ALL.iter() {
        //         let bb = Bitboard::from_sq(sq as u32);
        //         let mask = bb.ray(dir);
        //         classical.rays[sq][dir.index] = mask;
        //         classical.king_moves[sq] |= bb.shift(dir);

        //         // for example a night attack might be step N followed by step NE
        //         let next_dir = Dir::ALL[(dir.index + 1) % 8];
        //         classical.knight_moves[sq] |= bb.shift(dir).shift(next_dir);
        //     }
        // }
        me
    }


    fn hyperbola(&self, occ: Bitboard, sq: Square, mask: Bitboard) -> Bitboard {
        let o = occ & mask;
        let r = o.swap_bytes();
        let forward = o - sq.as_bb();
        let reverse = (r - Bitboard::from_sq(sq.index() as u32 ^ 56u32)).swap_bytes();
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
    fn rank_hyperbola(&self, occ: Bitboard, sq: Square, mask: Bitboard) -> Bitboard {
        let file = sq.file_index();
        let rank = sq.rank_index() * 8;
        let occ_byte = (occ.bits() >> rank) & 2*63;
        let attacks = self.rank_attacks[4_usize * occ_byte as usize + file as usize];
        Bitboard::from_bits_truncate((attacks as u64) << rank)
    }
}


impl BitboardAttacks for Hyperbola {


    fn rook_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from.index()].file) | self.rank_hyperbola(occ, from, self.mask[from.index()].rank)
    }

    fn bishop_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.hyperbola(occ, from, self.mask[from.index()].diag) | self.hyperbola(occ, from, self.mask[from.index()].anti_diag)
    }

    fn king_attacks(&self, from: Square) -> Bitboard {
        unimplemented!();
    }

    fn knight_attacks(&self, from: Square) -> Bitboard {
        unimplemented!();
    }
}


 

