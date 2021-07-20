use crate::bitboard::bitboard::{Bitboard};
use crate::bitboard::square::Square;
// use crate::bitboard::bb_classical::ClassicalBitboard;
// use crate::bitboard::bb_hyperbola::Hyperbola;
// use crate::bitboard::bb_magic::Magic;

pub trait SlidingPieceAttacks {
    fn new() -> Box<Self>;
    fn bishop_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard;
    fn rook_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard;
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::globals::constants::*;

    // fn init() {
    //     // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // }
}
