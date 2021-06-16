#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]


#[macro_use]
extern crate bitflags;
// extern crate lazy_static;

// #[macro_use]
// extern crate log;

extern crate once_cell;

// #[macro_use]
// extern crate ctor;

extern crate regex;
// extern crate crossbeam;


#[macro_use]
pub mod logger;

pub use crate::logger::LogInit;



// pub mod logger;
pub mod bitboard;
pub mod board;
pub mod catalog;
pub mod clock;
pub mod comms;
pub mod config;
pub mod debug;
pub mod eval;
pub mod exam;
pub mod game;
pub mod globals;
pub mod hasher;
pub mod material;
pub mod movelist;
pub mod mv;
pub mod variation;
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
pub mod tt2;
pub mod types;
pub mod utils;
pub mod version;

pub use crate::bitboard::bitboard::Bitboard;
pub use crate::board::boardbuf::BoardBuf;
pub use crate::board::Board;
pub use crate::config::Config;
pub use crate::exam::Exam;
pub use crate::hasher::Hasher;
pub use crate::movelist::MoveList;
pub use crate::position::Position;
pub use crate::search::algo::Algo;
pub use crate::search::searchstats::SearchStats;
pub use crate::tags::Tags;
// pub use crate::logger::LogInit;
pub use crate::types::{Color, Piece};
pub use crate::version::Version;
