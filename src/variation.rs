use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::mv::Move;
use crate::types::Ply;
use std::fmt;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variation {
    moves: Vec<Move>,
}

impl Default for Variation {
    #[inline]
    fn default() -> Self {
        Self {
            moves: Vec::with_capacity(60),
        }
    }
}


pub static EMPTY: Variation = Variation { moves: Vec::new() };


impl Variation {

    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn empty() -> &'static Self {
        &EMPTY
    }

    pub fn uci(&self) -> String {
        self.moves
            .iter()
            .map(|mv| mv.uci())
            .collect::<Vec<String>>()
            .join(" ")
    }

    #[inline]
    pub fn contains_null_move(&self) -> bool {
        self.moves.iter().any(|mv| mv.is_null())
    }

    #[inline]
    pub fn set_last_move(&mut self, ply: Ply, mv: &Move) {
        let ply = ply as usize;
        // root node is ply 0, so len==ply, so ply 1 gets stored in 0th element
        if self.moves.len() == ply && ply > 0 {
            self.moves[ply - 1] = *mv;
        } else if ply < self.moves.len() {
            self.moves.truncate(ply);
        } else {
            debug_assert!(ply > self.moves.len(), "Assert {} > {}", ply, self.moves.len());
            let len = ply - self.moves.len();
            for _ in 0..len {
                self.moves.push(*mv);
            }
            //self.moves.resize_with(ply, || *mv);
        }
    }

    pub fn apply_to(&self, b: &Board) -> Board {
        let mut board = b.clone();
        for mv in self.iter() {
            board = board.make_move(mv);
        }
        board
    }
}

impl Deref for Variation {
    type Target = Vec<Move>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.moves
    }
}

impl DerefMut for Variation {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.moves
    }
}

impl fmt::Display for Variation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            for mv in self.iter() {
                writeln!(f, "{:#}", mv)?;
            }
        } else {
            let strings: Vec<String> = self.moves.iter().map(Move::to_string).collect();
            f.write_str(&strings.join(", "))?
        }
        Ok(())
    }
}
