#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

#[macro_use]
extern crate bitflags;
// extern crate lazy_static;

#[macro_use]
extern crate log;

extern crate once_cell;

extern crate regex;

pub mod bitboard;
pub mod board;
pub mod catalog;
pub mod clock;
pub mod comms;
pub mod config;
pub mod eval;
pub mod exam;
pub mod game;
pub mod globals;
pub mod hasher;
pub mod logger;
pub mod material;
pub mod movelist;
pub mod outcome;
pub mod parse;
pub mod perft;
pub mod phase;
pub mod position;
pub mod pvtable;
pub mod repetition;
pub mod search;
pub mod stat;
pub mod tags;
pub mod task;
pub mod tt;
pub mod types;
pub mod utils;
pub mod version;

pub use crate::bitboard::bitboard::Bitboard;
pub use crate::board::boardbuf::BoardBuf;
pub use crate::board::Board;
pub use crate::config::Config;
pub use crate::exam::Exam;
pub use crate::hasher::Hasher;
pub use crate::movelist::{Move, MoveList};
pub use crate::position::Position;
pub use crate::search::algo::Algo;
pub use crate::search::searchstats::SearchStats;
pub use crate::tags::Tags;
pub use crate::types::{Color, Piece};
pub use crate::version::Version;
