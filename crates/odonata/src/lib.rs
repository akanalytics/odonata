#![cfg_attr(debug_assertions, allow(dead_code))]
#![cfg_attr(not(debug_assertions), allow(dead_code))]
#![warn(
    // clippy::all,
    // clippy::pedantic,
    clippy::correctness,
    clippy::style,
    clippy::complexity,
    clippy::cargo,
    clippy::perf
)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::if_not_else)]
#![allow(clippy::module_name_repetitions)]




extern crate test_log;

extern crate include_dir;

#[macro_use]
extern crate bitflags;


extern crate once_cell;

// #[macro_use]
// extern crate ctor;

extern crate regex;

#[macro_use]
extern crate tracing;

pub use crate::tracing::{debug, error, event_enabled, info, log::Level, trace, warn};

// pub mod logger;
pub mod bits;
pub mod boards;
pub mod book;
pub mod cache;
pub mod catalog;
pub mod clock;
pub mod comms;
pub mod domain;
pub mod eval;
pub mod eg;
pub mod exam;
pub mod globals;
pub mod infra;
pub mod movelist;
pub mod mv;
pub mod other;
pub mod piece;
pub mod position;
pub mod prelude;
pub mod search;
pub mod trace;
pub mod tune;
pub mod types;
pub mod variation;

pub use crate::{
    bits::{bitboard::Bitboard, precalc::PreCalc},
    exam::Exam,
    movelist::MoveList,
    position::Position,
    search::algo::Algo,
};
// pub use crate::logger::LogInit;
pub use crate::piece::{Color, Piece};
