#![cfg_attr(debug_assertions, allow(dead_code))]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::correctness,
    clippy::style,
    clippy::complexity,
    clippy::cargo,
    clippy::perf
)]
extern crate test_log;

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

// #[macro_use]
// extern crate log;

#[macro_use]
extern crate tracing;

pub use crate::tracing::{debug, error, info, trace, warn};
pub use crate::tracing::log::Level;
pub use crate::tracing::event_enabled;

// pub mod logger;
pub mod bits;
pub mod board;
pub mod bound;
pub mod cache;
pub mod catalog;
pub mod clock;
pub mod comms;
pub mod domain;
pub mod eval;
pub mod exam;
pub mod globals;
pub mod infra;
pub mod movelist;
pub mod mv;
pub mod other;
pub mod parse;
pub mod perft;
pub mod phaser;
pub mod piece;
pub mod position;
pub mod repetition;
pub mod search;
pub mod tags;
pub mod trace;
pub mod tune;
pub mod types;
pub mod variation;

pub use crate::bits::bitboard::Bitboard;
pub use crate::bits::precalc::PreCalc;
pub use crate::exam::Exam;
pub use crate::movelist::MoveList;
pub use crate::position::Position;
pub use crate::search::algo::Algo;
pub use crate::tags::Tags;
// pub use crate::logger::LogInit;
pub use crate::piece::{Color, Piece};
