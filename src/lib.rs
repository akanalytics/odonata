#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

#[macro_use]
extern crate bitflags;
// extern crate lazy_static;

#[macro_use]
extern crate log;

extern crate once_cell;

extern crate regex;

mod attacks;
pub mod bitboard;
pub mod board;
pub mod catalog;
pub mod eval;
pub mod game;
pub mod globals;
pub mod material;
pub mod movelist;
pub mod outcome;
pub mod parse;
pub mod pvtable;
pub mod search;
pub mod types;
mod utils;

pub use crate::attacks::ClassicalBitboard;
pub use crate::bitboard::Bitboard;
pub use crate::board::boardbuf::BoardBuf;
pub use crate::board::Board;
pub use crate::movelist::{Move, MoveList};
pub use crate::search::Search;
pub use crate::types::{Color, Piece};
