#[macro_use]
extern crate bitflags;
// extern crate lazy_static;


pub mod bitboard;
mod attacks;
mod globals;
pub mod board;

pub use crate::bitboard::{Bitboard};
pub use crate::board::{Board, Move, Color};
pub use crate::board::boardbuf::BoardBuf;
pub use crate::attacks::{ClassicalBitboard};
