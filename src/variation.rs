use itertools::Itertools;

use crate::board::Board;
use crate::domain::info::BareMoveVariation;
use crate::mv::Move;
use crate::piece::Ply;
use std::fmt;
use std::ops::{Deref, DerefMut};

#[derive(Clone, PartialEq, Eq)]
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

impl fmt::Debug for Variation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Variation")
            .field("moves", &self.to_uci())
            .finish()
    }
}

pub static EMPTY: Variation = Variation { moves: Vec::new() };

impl Variation {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_inner(bmv: &BareMoveVariation, b: &Board) -> Self {
        let mut var = Self::new();
        let mut b = b.clone();
        for mv in bmv.moves() {
            let mv = b.augment_move(*mv);
            var.push(mv);
            b = b.make_move(&mv);
        }
        var
    }


    #[inline]
    pub fn empty() -> &'static Self {
        &EMPTY
    }

    pub fn to_inner(&self) -> BareMoveVariation {
        BareMoveVariation(self.moves().map(Move::to_inner).collect_vec())
    }

    #[inline]
    pub fn moves(&self) -> impl Iterator<Item = &Move> {
        self.moves.iter()
    }

    pub fn to_uci(&self) -> String {
        self.moves
            .iter()
            .map(|mv| mv.to_uci())
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub fn parse_uci(s: &str, bd: &Board) -> anyhow::Result<Variation> {
        let mut variation = Variation::new();
        let mut b = bd.clone();
        for word in s.split_whitespace() {
            let mv = b.parse_uci_move(word)?;
            b = b.make_move(&mv);
            variation.push(mv)
        }
        Ok(variation)
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

    pub fn to_san(&self, b: &Board) -> String {
        let mut b2 = b.clone();
        let mut s = Vec::new();
        for mv in &self.moves {
            if !b2.is_pseudo_legal_and_legal_move(*mv) {
                panic!(
                    "Move {mv} in {} is not legal for board {}",
                    self,
                    b.to_fen()
                );
            }
            s.push(b2.to_san(mv));
            b2 = b2.make_move(mv);
        }
        s.join(" ")
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

    #[deprecated]
    pub fn apply_to(&self, b: &Board) -> Board {
        b.make_moves_old(&self)
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

#[cfg(test)]
mod tests {
    use crate::catalog::Catalog;
    use test_log::test;

    use super::*;

    #[test]
    fn test_variation() -> anyhow::Result<()> {
        let bd = Catalog::starting_board();
        assert_eq!(Variation::parse_uci("a2a3", &bd)?.to_uci(), "a2a3");
        assert_eq!(
            Variation::parse_uci("a2a3 a7a6", &bd)?.to_uci(),
            "a2a3 a7a6"
        );
        Ok(())
    }
}
