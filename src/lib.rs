#![cfg_attr(debug_assertions, allow(dead_code))]

extern crate test_env_log;

extern crate include_dir;


#[macro_use]
extern crate bitflags;


// #[macro_use]
// extern crate enumflags2;

// extern crate lazy_static;

// #[macro_use]
// extern crate log;

extern crate once_cell;

// #[macro_use]
// extern crate ctor;

extern crate regex;
// extern crate crossbeam;




// pub use crate::logger::LogInit;

#[macro_use]
extern crate log;




// pub mod logger;
pub mod bitboard;
pub mod board;
pub mod domain;
pub mod catalog;
pub mod bound;
pub mod clock3;
pub mod clock;
pub mod comms;
pub mod infra;
pub mod trace;
pub mod eval;
pub mod exam;
pub mod game;
pub mod globals;
pub mod movelist;
pub mod mv;
pub mod variation;
pub mod outcome;
pub mod parse;
pub mod perft;
pub mod phaser;
pub mod position;
pub mod pvtable;
pub mod repetition;
pub mod search;
pub mod tags;
pub mod tuning;
pub mod cache;
pub mod types;
pub mod utils;

pub use crate::bitboard::bitboard::Bitboard;
pub use crate::board::boardbuf::BoardBuf;
pub use crate::board::Board;
pub use crate::bitboard::precalc::PreCalc;
pub use crate::infra::parsed_config::ParsedConfig;
pub use crate::exam::Exam;
pub use crate::movelist::MoveList;
pub use crate::position::Position;
pub use crate::search::algo::Algo;
pub use crate::search::searchstats::SearchStats;
pub use crate::tags::Tags;
// pub use crate::logger::LogInit;
pub use crate::types::{Color, Piece};
