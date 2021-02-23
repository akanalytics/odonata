#[macro_use]
extern crate bitflags;
// extern crate lazy_static;


pub mod bitboard;
mod attacks;
mod globals;

pub use crate::bitboard::{Bitboard, Color};

pub use crate::attacks::{ClassicalBitboard};