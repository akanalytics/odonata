
use crate::bitboard::{Bitboard, Dir};
use lazy_static::lazy_static;




pub trait SlidingAttacks {
    
    // fn init();
    
    //fn bishop_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    // fn rook_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    //fn knight_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;
    //fn king_attacks(occupied: Bitboard, from_sq: u32) -> Bitboard;

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






lazy_static! {
    pub static ref ATTACKS: [[Bitboard; 8]; 64] = {
        let mut attacks = [[Bitboard::EMPTY; 8]; 64];
        for sq in 0..64 {
            for dir in 0..8 {
                let mask = SlidingAttacks::ray(Dir::ALL[dir], Bitboard::ALL[sq]);
                attacks[sq][dir] = mask;
            }
        }
        attacks
    };
}


pub struct Classical {
}

impl SlidingAttacks for Classical {


}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let c3 = Bitboard::C3;
        let north = Classical::ray(Dir::N, c3);
        println!( "\nN \n{}", north);
        assert!(north.contains(Bitboard::C8) );
        assert_eq!(north.count(), 5 );
        
        println!( "\nNE\n{}", Classical::ray( Dir::NE, c3 ) );
        println!( "\nSW\n{}", Classical::ray( Dir::SW, c3 ) );
        println!( "\nS \n{}", Classical::ray( Dir::S, c3 ) );
        println!( "\nNW\n{}", Classical::ray( Dir::NW, c3 ) );

        // assert!(a1b2.contains(Bitboard::A1));
        //assert!(a1b2 & Bitboard::C1 == Bitboard::A1 - Bitboard::A1);
    }
}