use crate::bits::bitboard::Bitboard;
use crate::types::Board;
use crate::cache::hasher::Hasher;
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::search::node::{Counter, Timing};
use crate::piece::{Piece, Repeats};
use crate::variation::Variation;
use anyhow::Result;

use std::cell::Cell;

// pub trait MoveDelta {
//     fn make_move(&mut self, mv: &Move);
//     fn undo_move(&mut self, mv: &Move);
// }

pub trait MoveMaker {
    fn make_move(&self, m: &Move) -> Board;
    fn make_moves(&self, m: &Variation) -> Board;
    fn undo_move(&self, m: &Move);
}

