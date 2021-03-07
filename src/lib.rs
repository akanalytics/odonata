#[macro_use]
extern crate bitflags;
// extern crate lazy_static;

#[macro_use]
extern crate log;

extern crate once_cell;


pub mod bitboard;
mod attacks;
mod globals;
mod utils;
pub mod board;

pub use crate::bitboard::{Bitboard};
pub use crate::board::{Board, Move, Color};
pub use crate::board::boardbuf::BoardBuf;
pub use crate::attacks::{ClassicalBitboard};
