#![cfg_attr(debug_assertions, allow(dead_code))]
#![cfg_attr(not(debug_assertions), allow(dead_code))]


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

pub use crate::tracing::log::Level;
pub use crate::tracing::{debug, error, event_enabled, info, trace, warn};

// pub mod logger;
pub mod bits;
pub mod boards;
pub mod catalog;
pub mod clock;
pub mod domain;
pub mod eg;
pub mod epd;
pub mod infra;
pub mod movelist;
pub mod mv;
pub mod other;
pub mod piece;
pub mod prelude;
pub mod trace;
pub mod variation;

pub use crate::bits::bitboard::Bitboard;
pub use crate::bits::precalc::PreCalc;
pub use crate::epd::Epd;
pub use crate::movelist::MoveList;
pub use crate::piece::FlipVertical;
// pub use crate::logger::LogInit;
pub use crate::piece::{Color, Piece};
