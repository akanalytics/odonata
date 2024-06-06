use std::fmt::{self, Debug};
use std::ops::Index;

use anyhow::Context;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::domain::score::Score;
use crate::movelist::ScoredMoveList;
use crate::mv::Move;
use crate::piece::Ply;
use crate::prelude::Board;

#[derive(Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Variation {
    moves: Vec<Move>,
}

#[derive(Clone, Default, PartialEq, Debug, Eq, Serialize, Deserialize)]
pub struct ScoredVariation {
    pub var:   Variation,
    pub score: Score,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
// #[serde(transparent)]
pub struct MultiVariation {
    vars_and_scores: Vec<ScoredVariation>,
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
        f.debug_struct("Variation").field("moves", &self.to_uci()).finish()
    }
}

pub static EMPTY: Variation = Variation { moves: Vec::new() };

/// ""            < "a4 d5 b4"
/// "a4 d5"       < "a4 d5 b4"
/// "a4 d5 b4"   == "a4 d5 b4"
/// "a4 d5 b4 e5" > "a4 d5 b4"
/// "a4 a5"  not comparable to "a4 c5"  (None)
///
/// broadly comparable-to means part-of-same super-variation
impl PartialOrd for Variation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.starts_with(other) {
            Some(std::cmp::Ordering::Greater)
        } else if other.starts_with(self) {
            Some(std::cmp::Ordering::Less)
        } else if self.eq(other) {
            return Some(std::cmp::Ordering::Equal);
        } else {
            None
        }
    }
}

impl Index<usize> for Variation {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

impl FromIterator<Move> for Variation {
    fn from_iter<T: IntoIterator<Item = Move>>(iter: T) -> Self {
        let mut var = Variation::new();
        iter.into_iter().for_each(|mv| var.push(mv));
        var
    }
}

impl IntoIterator for Variation {
    type Item = Move;
    type IntoIter = <Vec<Move> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.moves.into_iter()
    }
}

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

    pub fn from_move(mv: Move) -> Self {
        Self { moves: vec![mv] }
    }

    pub fn first(&self) -> Option<Move> {
        self.moves.first().cloned()
    }

    pub fn second(&self) -> Option<Move> {
        self.moves().nth(1)
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
    pub fn moves(&self) -> impl DoubleEndedIterator<Item = Move> + ExactSizeIterator<Item = Move> + '_ {
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

    pub fn flip_vertical(&mut self) {
        self.moves.iter_mut().for_each(Move::flip_vertical)
    }

    pub fn to_uci(&self) -> String {
        self.moves().map(|mv| mv.to_uci()).collect::<Vec<String>>().join(" ")
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

    /// "a4 d5 b4" starts with "a4 d5"
    /// "a4 d5 b4" starts with ""
    /// "a4 d5 b4" doesn't start with "a4 d5 b4 e5"
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

    #[must_use]
    pub fn chain(&self, var: &Variation) -> Variation {
        let mut me = self.clone();
        me.extend(var);
        me
    }

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
                panic!("{uci}: {mv} is not legal for board {b}", uci = self.to_uci(),);
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
                    s.push(b2.to_san(mv).to_string());
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

    pub fn pop_front(&mut self) -> Option<Move> {
        match self.moves.len() {
            0 => None,
            _ => Some(self.moves.remove(0)),
        }
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
            debug_assert!(ply > self.moves.len(), "Assert {} > {}", ply, self.moves.len());
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
        b.make_moves_old(self)
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
            for (i, vs) in self.vars_and_scores.iter().enumerate() {
                writeln!(f, "{i} {sc:>5}{var}", sc = vs.score, var = vs.var)?;
            }
        } else {
            let s = self
                .vars_and_scores
                .iter()
                .map(|vs| format!("{sc}: {var}", sc = vs.score, var = vs.var))
                .join(", ");
            write!(f, "{}", s)?;
        }
        Ok(())
    }
}

impl FromIterator<ScoredVariation> for MultiVariation {
    fn from_iter<T: IntoIterator<Item = ScoredVariation>>(iter: T) -> Self {
        Self {
            vars_and_scores: iter.into_iter().collect(),
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

impl ScoredVariation {
    pub fn parse_uci(s: &str, bd: &Board) -> anyhow::Result<Self> {
        if let Some((sc, var)) = s.split_once(':') {
            Ok(Self {
                score: Score::parse_pgn_pawn_value(sc)?,
                var:   Variation::parse_uci(var.trim(), bd)?,
            })
        } else {
            anyhow::bail!("unable to split '{s}' into score and variation")
        }
    }

    pub fn parse_san(s: &str, bd: &Board) -> anyhow::Result<Self> {
        if let Some((sc, var)) = s.split_once(':') {
            Ok(Self {
                score: Score::parse_pgn_pawn_value(sc)?,
                var:   Variation::parse_san(var.trim(), bd)?,
            })
        } else {
            anyhow::bail!("unable to split '{s}' into score and variation")
        }
    }
}

impl MultiVariation {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.vars_and_scores.len()
    }

    pub fn from_scored_variation(var: Variation, score: Score) -> Self {
        Self {
            vars_and_scores: vec![ScoredVariation { var, score }],
        }
    }

    pub fn push(&mut self, var: Variation, score: Score) {
        self.vars_and_scores.push(ScoredVariation { var, score });
    }

    pub fn iter(&self) -> impl Iterator<Item = &ScoredVariation> {
        self.vars_and_scores.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ScoredVariation> {
        self.vars_and_scores.iter_mut()
    }

    pub fn vars(&self) -> impl Iterator<Item = &Variation> {
        self.vars_and_scores.iter().map(|sv| &sv.var)
    }

    pub fn find_by_move(&self, mv: Move) -> Option<&ScoredVariation> {
        self.vars_and_scores
            .iter()
            .find_map(|v| if v.var.first() == Some(mv) { Some(v) } else { None })
    }

    pub fn score_for(&self, var: &Variation) -> Option<Score> {
        self.vars_and_scores
            .iter()
            .find_map(|v| if v.var == *var { Some(v.score) } else { None })
    }

    pub fn best_move(&self) -> Option<Move> {
        ScoredMoveList::from(self.clone()).best_move()
    }

    pub fn first(&self) -> Option<ScoredVariation> {
        self.vars_and_scores.first().cloned()
    }

    pub fn find_first_starting_with(&self, var: &Variation) -> Option<&ScoredVariation> {
        self.vars_and_scores
            .iter()
            .find_map(|v| if v.var.starts_with(var) { Some(v) } else { None })
    }

    pub fn to_uci(&self) -> String {
        self.vars_and_scores
            .iter()
            .map(|vs| format!("{sc}:{uci}", sc = vs.score.to_pgn(), uci = vs.var.to_uci()))
            .join(", ")
    }

    pub fn to_san(&self, bd: &Board) -> String {
        self.vars_and_scores
            .iter()
            .map(|vs| format!("{sc}:{s}", sc = vs.score.to_pgn(), s = vs.var.to_san(bd)))
            .join(", ")
    }

    pub fn validate(&self, bd: &Board) -> anyhow::Result<()> {
        self.vars_and_scores
            .iter()
            .try_for_each(|vs| vs.var.validate(bd))
            .with_context(|| self.to_uci())?;
        Ok(())
    }

    pub fn parse_uci(s: &str, bd: &Board) -> anyhow::Result<Self> {
        let vars_and_scores = s
            .split(',')
            .map(|s| ScoredVariation::parse_uci(s.trim(), bd))
            .collect::<anyhow::Result<Vec<ScoredVariation>>>()?;
        Ok(Self { vars_and_scores })
    }

    pub fn parse_san(s: &str, bd: &Board) -> anyhow::Result<Self> {
        let vars_and_scores = s
            .split(',')
            .map(|s| ScoredVariation::parse_san(s.trim(), bd))
            .collect::<anyhow::Result<Vec<ScoredVariation>>>()?;
        Ok(Self { vars_and_scores })
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;
    use crate::catalog::Catalog;
    use crate::prelude::testing::*;
    use crate::prelude::*;

    #[test]
    fn test_variation() -> anyhow::Result<()> {
        let b = &Catalog::starting_board();
        assert_eq!(Variation::parse_uci("a2a3", b)?.to_uci(), "a2a3");
        assert_eq!(Variation::parse_uci("a2a3 a7a6", b)?.to_uci(), "a2a3 a7a6");
        let a3 = "a3".mv(b);
        let a6 = "a6".mv(&b.make_move(a3));
        let var = "a3 a6".var(b);
        assert_eq!(var[0], a3);
        assert_eq!(var[1], a6);
        assert_eq!(b.make_moves(&var.take(0)), *b);
        assert_eq!(b.make_moves(&var.take(2)), b.make_move(a3).make_move(a6));
        Ok(())
    }

    #[test]
    fn test_variation_ordering() {
        let b = &Board::starting_pos();
        assert!("".var(b) < "a4 d5 b4".var(b));
        assert!("a4 d5".var(b) == "a4 d5".var(b));
        assert!("a4 d5 b4 e5".var(b) > "a4 d5 b4".var(b));
        assert!("a4 a5".var(b).partial_cmp(&"a4 c5".var(b)).is_none());
    }

    #[test]
    fn test_multi_variation() {
        let bd = Board::starting_pos();
        assert_eq!(
            MultiVariation::parse_uci("+34.00:a2a3 a7a6, -20.00:a2a4", &bd)
                .unwrap()
                .to_uci(),
            "+34.00:a2a3 a7a6, -20.00:a2a4"
        );
        assert_eq!(
            MultiVariation::parse_uci("+1.00:a2a3, +M2:a2a4", &bd)
                .unwrap()
                .to_san(&bd),
            "+1.00:a3, +M2:a4"
        );

        // test empty variation
        let mvar = MultiVariation::parse_uci("+100.00:a2a3 a7a6, -50.00:a2a4,-40.00:", &bd).unwrap();
        assert_eq!(mvar.len(), 3);
        assert_eq!(mvar.to_uci(), "+100.00:a2a3 a7a6, -50.00:a2a4, -40.00:");
        assert_eq!(
            MultiVariation::parse_san("+M2:a3 a6, -M2:a4", &bd).unwrap().to_uci(),
            "+M2:a2a3 a7a6, -M2:a2a4"
        );
    }
}
