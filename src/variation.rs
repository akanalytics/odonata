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

    #[inline]
    pub fn moves(&self) -> impl Iterator<Item = &Move> {
        self.moves.iter()
    }

    pub fn uci(&self) -> String {
        self.moves
            .iter()
            .map(|mv| mv.uci())
            .collect::<Vec<String>>()
            .join(" ")
    }

    /// variation without last move or None if empty
    pub fn stem(&self) -> Option<Variation> {
        if !self.moves.is_empty() {
            let moves = self.moves[0..self.moves.len() - 1].to_owned();
            Some(Variation { moves })
        } else {
            None
        }
    }

    pub fn append(&self, mv: Move) -> Variation {
        let mut var = self.clone();
        var.push(mv);
        var
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
            debug_assert!(
                ply > self.moves.len(),
                "Assert {} > {}",
                ply,
                self.moves.len()
            );
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
