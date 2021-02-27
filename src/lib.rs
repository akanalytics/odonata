#[macro_use]
extern crate bitflags;
// extern crate lazy_static;


pub mod bitboard;
mod attacks;
mod globals;
pub mod board;
mod movegen;

pub use crate::bitboard::{Bitboard};
pub use crate::board::{Board, Color};
pub use crate::attacks::{ClassicalBitboard};
pub use crate::movegen::{Move};
