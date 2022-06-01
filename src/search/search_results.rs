use std::iter::{self, FromIterator};

use crate::eval::score::Score;
use crate::mv::Move;
use crate::outcome::Outcome;
use crate::tags::Tag;
use crate::types::Ply;
use crate::variation::Variation;
use crate::{Board, MoveList, Position};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchResults {
    pub board: Board,
    pub depth: Ply,
    pub seldepth: Ply,
    pub time_millis: u64,
    // pub pv: Variation,
    pub multi_pv: Vec<(Variation, Score)>,
    pub nodes: u64,
    pub nodes_thread: u64,
    // pub score: Score,
    pub nps: u64,
    pub tbhits: u64,
    pub branching_factor: f32,
    pub hashfull_per_mille: u32,
    pub outcome: Outcome,
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl SearchResults {
    /// outcome could be abandoned or win/draw reason
    pub fn best_move(&self) -> Result<Move, Outcome> {
        if self.pv().len() > 0 {
            Ok(self.pv()[0])
        } else {
            Err(self.outcome)
        }
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    pub fn pv(&self) -> &Variation {
        if self.multi_pv.len() > 0 {
            &self.multi_pv[0].0
        } else {
            Variation::empty()
        }
    }

    pub fn score(&self) -> Score {
        if self.multi_pv.len() > 0 {
            self.multi_pv[0].1
        } else {
            Score::default()
        }
    }

    pub fn multi_pv(&self) -> &[(Variation, Score)] {
        &self.multi_pv
    }

    pub fn to_position(&self) -> Position {
        let mut pos = Position::from_board(self.board.clone());
        pos.set(Tag::Pv(self.pv().clone()));
        if self.pv().len() > 0 {
            pos.set(Tag::SuppliedMove(self.pv()[0]));
            pos.set(Tag::BestMoves(MoveList::from_iter(iter::once(self.pv()[0]))));
        }
        pos.set(Tag::CentipawnEvaluation(self.score().as_i16() as i32));
        pos.set(Tag::AnalysisCountDepth(self.depth));
        pos.set(Tag::AnalysisCountSelDepth(self.seldepth));
        pos.set(Tag::AnalysisCountNodes(self.nodes as u128));
        pos.set(Tag::BranchingFactorPercent(
            (100.0 * self.branching_factor) as u32,
        ));
        pos
    }
}
