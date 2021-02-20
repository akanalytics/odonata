
use crate::bitboard::{Bitboard, Dir};





pub trait SlidingAttacks {
    fn bishop_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    fn rook_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    fn knight_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    fn king_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;

    // fn step_is_on_board(dir: Direction, sq: i32) -> bool {
    //     false
    // }

    // excludes the src square itself, but includes edge squares 
    fn ray(dir: Dir, src: Bitboard) -> Bitboard {
    
        let mut sq = src;
        let mut bb = Bitboard::EMPTY;
        while !sq.is_empty() {
            sq = sq.shift(&dir); 
            bb |= sq;
        }
        bb
    }
    


}


pub struct Classical {}

// impl SlidingAttacks for Classical {

// }


