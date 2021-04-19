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
pub mod clock;
pub mod catalog;
pub mod comms;
pub mod eval;
pub mod game;
pub mod globals;
pub mod material;
pub mod movelist;
pub mod outcome;
pub mod parse;
pub mod perft;
pub mod pvtable;
pub mod search;
pub mod types;
pub mod task;
mod utils;
pub mod version;
pub mod config;
pub mod logger;
pub mod position;
pub mod exam;
pub mod stat;
pub mod hasher;

pub use crate::attacks::ClassicalBitboard;
pub use crate::bitboard::Bitboard;
pub use crate::hasher::Hasher;
pub use crate::position::Position;
pub use crate::exam::Exam;
pub use crate::board::boardbuf::BoardBuf;
pub use crate::board::Board;
pub use crate::movelist::{Move, MoveList};
pub use crate::search::algo::Algo;
pub use crate::search::searchstats::SearchStats;
pub use crate::types::{Color, Piece};
pub use crate::version::Version;
pub use crate::config::Config;

