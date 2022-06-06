use std::iter::{self, FromIterator};

use crate::eval::score::Score;
use crate::mv::Move;
use crate::outcome::Outcome;
use crate::tags::Tag;
use crate::types::Ply;
use crate::variation::Variation;
use crate::{Board, MoveList, Position, Algo};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchResults {
    pub board: Board,
    pub depth: Ply,
    pub seldepth: Ply,
    pub time_millis: u64,
    pub nodes: u64,
    pub nodes_thread: u64,
    pub nps: u64,
    pub tbhits: u64,
    pub branching_factor: f32,
    pub hashfull_per_mille: u32,
    pub outcome: Outcome,
    pub multi_pv: Vec<(Variation, Score)>,
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl SearchResults {
    pub fn new(algo: &Algo) -> Self {
        SearchResults {
            board: algo.board.clone(),
            outcome: Outcome::Unterminated,
            tbhits: 0,
            nodes : algo.clock.cumul_nodes_all_threads(),
            nodes_thread : algo.clock.cumul_nodes_this_thread(),
            nps : algo.clock.cumul_knps_all_threads() * 1000,
            depth : algo.stats.depth(),
            seldepth : algo.pv_table.selective_depth(),
            time_millis : algo.clock.elapsed_search().0.as_millis() as u64,
            hashfull_per_mille : algo.tt.hashfull_per_mille(),
            branching_factor : algo.stats.branching_factor(),
            multi_pv: Default::default(),
        }
    // [algo.restrictions.multi_pv_index()] = (algo.stats.pv().clone(), algo.stats.score()),
}






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
