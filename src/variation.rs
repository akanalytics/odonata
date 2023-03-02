use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{boards::Board, mv::Move, piece::Ply};
use std::fmt::{self, Debug};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Variation {
    moves: Vec<Move>,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MultiVariation {
    vars: Vec<Variation>,
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

    // pub fn from_inner(bmv: &BareMoveVariation, b: &Board) -> Self {
    //     let mut var = Self::new();
    //     let mut b = b.clone();
    //     for mv in bmv.moves() {
    //         let mv = b.augment_move(*mv);
    //         var.push(mv);
    //         b = b.make_move(&mv);
    //     }
    //     var
    // }

    #[inline]
    pub fn empty() -> &'static Self {
        &EMPTY
    }

    pub fn first(&self) -> Option<Move> {
        self.moves.first().cloned()
    }

    pub fn second(&self) -> Option<Move> {
        self.moves().skip(1).next()
    }

    pub fn last(&self) -> Option<Move> {
        self.moves.last().cloned()
    }

    pub fn clear(&mut self) {
        self.moves.clear();
    }

    // pub fn to_inner(&self) -> BareMoveVariation {
    //     BareMoveVariation(self.moves().map(Move::to_inner).collect_vec())
    // }

    #[inline]
    pub fn moves(&self) -> impl DoubleEndedIterator<Item = Move> + '_ {
        self.moves.iter().cloned()
    }

    pub fn len(&self) -> usize {
        self.moves.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn validate(&self, bd: &Board) -> anyhow::Result<()> {
        bd.validate_moves(&self.moves().collect_vec())
    }

    // truncate the variation to length ply
    // so the result does not include the ply-th move in the variation
    // if len < ply just return all of the variation
    pub fn take(&self, ply: usize) -> Self {
        Variation {
            moves: self.moves().take(ply).collect_vec(),
        }
    }

    pub fn to_uci(&self) -> String {
        self.moves()
            .map(|mv| mv.to_uci())
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub fn parse_uci(s: &str, bd: &Board) -> anyhow::Result<Variation> {
        let mut variation = Variation::new();
        let mut b = bd.clone();
        for word in s.split_whitespace() {
            let mv = b.parse_uci_move(word)?;
            b = b.make_move(mv);
            variation.push(mv)
        }
        Ok(variation)
    }

    pub fn parse_san(s: &str, bd: &Board) -> anyhow::Result<Variation> {
        bd.parse_san_variation(s)
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

    pub fn starts_with(&self, var: &Variation) -> bool {
        self.len() >= var.len() && &self.take(var.len()) == var
    }

    /// variation tail
    pub fn skip(&self, ply: usize) -> Variation {
        Variation {
            moves: self.moves[ply..].to_vec(),
        }
    }

    // /// variation head
    // pub fn take(&self, ply: usize) -> Variation {
    //     let len = self.len();
    //     debug_assert!(ply <= len, "failed: ply={ply} <= len({self})={len}");
    //     Variation {
    //         moves: self.moves[..ply].to_vec(),
    //     }
    // }

    pub fn extend(&mut self, var: &Variation) {
        self.moves.extend(var.moves.iter())
    }

    pub fn extend_from_slice(&mut self, moves: &[Move]) {
        self.moves.extend(moves.iter())
    }

    /// can panic
    pub fn to_san(&self, b: &Board) -> String {
        let mut b2 = b.clone();
        let mut s = Vec::new();
        for mv in self.moves() {
            if !mv.is_null() && mv.to_inner().validate(&b2).is_err() {
                panic!(
                    "{uci}: {mv} is not legal for board {b}",
                    uci = self.to_uci(),
                );
            }
            s.push(b2.to_san(mv));
            b2 = b2.make_move(mv);
        }
        s.join(" ")
    }

    /// wont panic
    pub fn display_san(&self, b: &Board) -> String {
        let mut b2 = b.clone();
        let mut s = vec![];
        let mut errors = false;
        for mv in self.moves() {
            if !mv.is_null() && mv.to_inner().validate(&b2).is_err() {
                errors = true;
            }
            match errors {
                false => {
                    s.push(format!("{}", b2.to_san(mv)));
                    b2 = b2.make_move(mv);
                }
                true => s.push(format!("[{}]", mv.to_uci())),
            }
        }
        s.join(" ")
    }

    pub fn append(&self, mv: Move) -> Variation {
        let mut var = self.clone();
        var.push(mv);
        var
    }

    pub fn push(&mut self, mv: Move) {
        self.moves.push(mv);
    }

    pub fn pop(&mut self) -> Option<Move> {
        self.moves.pop()
    }

    pub fn push_front(&mut self, mv: Move) {
        self.moves.insert(0, mv);
    }

    #[inline]
    pub fn set_last_move(&mut self, ply: Ply, mv: Move) {
        let ply = ply as usize;
        // root node is ply 0, so len==ply, so ply 1 gets stored in 0th element
        if self.moves.len() == ply && ply > 0 {
            self.moves[ply - 1] = mv;
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
                self.moves.push(mv);
            }
            // self.moves.resize_with(ply, || *mv);
        }
    }

    /// use board.make_moves()
    #[deprecated]
    pub fn apply_to(&self, b: &Board) -> Board {
        b.make_moves_old(&self)
    }
}

// impl Deref for Variation {
//     type Target = Vec<Move>;

//     #[inline]
//     fn deref(&self) -> &Self::Target {
//         &self.moves
//     }
// }

// impl DerefMut for Variation {
//     #[inline]
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.moves
//     }
// }

impl fmt::Display for Variation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            for mv in self.moves() {
                writeln!(f, "{:#}", mv)?;
            }
        } else {
            let strings = self.moves().map(|m| m.to_string()).collect_vec();
            f.write_str(&strings.join("."))?
        }
        Ok(())
    }
}

impl fmt::Display for MultiVariation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            for (i, var) in self.vars.iter().enumerate() {
                writeln!(f, "{i} {var}")?;
            }
        } else {
            write!(f, "{}", self.vars.iter().map(|v| v.to_string()).join(", "))?;
        }
        Ok(())
    }
}

impl FromIterator<Variation> for MultiVariation {
    fn from_iter<T: IntoIterator<Item = Variation>>(iter: T) -> Self {
        Self {
            vars: iter.into_iter().collect(),
        }
    }
}

// impl IntoIterator for MultiVariation {
//     type Item = Variation;
//     type IntoIter = IntoIter<Variation>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.vars.into_iter()
//     }
// }

impl MultiVariation {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.vars.len()
    }

    pub fn to_uci(&self) -> String {
        self.vars.iter().map(|var| var.to_uci()).join(", ")
    }

    pub fn to_san(&self, bd: &Board) -> String {
        self.vars.iter().map(|var| var.to_san(bd)).join(", ")
    }

    pub fn parse_uci(s: &str, bd: &Board) -> anyhow::Result<Self> {
        let vars = s
            .split(",")
            .map(|s| Variation::parse_uci(s.trim(), bd))
            .collect::<anyhow::Result<Vec<Variation>>>()?;
        Ok(Self { vars })
    }

    pub fn parse_san(s: &str, bd: &Board) -> anyhow::Result<Self> {
        let vars = s
            .split(",")
            .map(|s| Variation::parse_san(s.trim(), bd))
            .collect::<anyhow::Result<Vec<Variation>>>()?;
        Ok(Self { vars })
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

    #[test]
    fn test_multi_variation() {
        let bd = Board::starting_pos();
        assert_eq!(
            MultiVariation::parse_uci("a2a3, a2a4", &bd)
                .unwrap()
                .to_uci(),
            "a2a3, a2a4"
        );
        assert_eq!(
            MultiVariation::parse_uci("a2a3, a2a4", &bd)
                .unwrap()
                .to_san(&bd),
            "a3, a4"
        );
        let mvar = MultiVariation::parse_uci("a2a3 a7a6, a2a4,", &bd).unwrap();
        assert_eq!(mvar.len(), 3);
        assert_eq!(mvar.to_uci(), "a2a3 a7a6, a2a4, ");
        assert_eq!(
            MultiVariation::parse_san("a3 a6, a4", &bd)
                .unwrap()
                .to_uci(),
            "a2a3 a7a6, a2a4"
        );
    }
}
