use std::iter::{self, FromIterator};

use super::node::{Event, Node};
use super::restrictions::Restrictions;
use crate::board::Board;
use crate::cache::tt2::TranspositionTable2;
use crate::clock::Clock;
use crate::eval::score::Score;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::other::outcome::Outcome;
use crate::other::pvtable::PvTable;
use crate::tags::Tag;
use crate::types::Ply;
use crate::variation::Variation;
use crate::{Algo, MoveList, Position};
use serde::{Deserialize, Serialize};
use std::fmt;

/// essentially all the data needed for UCI info status updates or for a decent progress bar
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SearchProgressMode {
    BestMove,
    Refutation,
    PvChange,
    NodeCounts,
}

impl Default for SearchProgressMode {
    fn default() -> Self {
        Self::PvChange
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[rustfmt::skip]
pub struct SearchProgress {
    #[serde(skip)] pub take_move_from_part_ply: bool,

    
    #[serde(skip)] pub board: Board,
    #[serde(skip)] pub mode: SearchProgressMode,
    #[serde(skip)] pub depth: Ply,
    #[serde(skip)] pub seldepth: Ply,
    #[serde(skip)] pub time_millis: Option<u64>,
    #[serde(skip)] pub multi_pv_index: usize,
    #[serde(skip)] pub multi_pv_index_of: usize,
    #[serde(skip)] pub pv: Variation,
    #[serde(skip)] pub nodes: Option<u64>,
    #[serde(skip)] pub nodes_thread: Option<u64>,
    #[serde(skip)] pub score: Score,
    #[serde(skip)] pub currmove: Option<Move>,
    #[serde(skip)] pub currmovenumber_from_1: Option<u32>,
    #[serde(skip)] pub hashfull_per_mille: Option<u32>,
    #[serde(skip)] pub nps: Option<u64>,
    #[serde(skip)] pub tbhits: Option<u64>,
    #[serde(skip)] pub cpuload_per_mille: Option<u32>,
    // #[serde(skip)] pub branching_factor: Option<f32>,
    #[serde(skip)] pub event: Option<Event>,
    
    #[serde(skip)] pub best_score: Score,
    #[serde(skip)] pub best_pv: Variation,
    #[serde(skip)] pub outcome: Outcome,
    // pub refutation: Option<Move>,
    // pub currline: Option<MoveList>,
}

impl Component for SearchProgress {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_iter(&mut self) {
        // seldepth can be lower at a higher depth
        self.depth = 0;
        self.seldepth = 0;
    }

    fn new_position(&mut self) {
        *self = Self::default();
    }
}

impl fmt::Display for SearchProgress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl SearchProgress {
    pub fn with_report_progress(algo: &Algo) -> Self {
        SearchProgress {
            mode: SearchProgressMode::NodeCounts,
            board: algo.board.clone(),
            nodes: Some(algo.clock.cumul_nodes_all_threads()),
            nodes_thread: Some(algo.clock.cumul_nodes_this_thread()),
            nps: Some(algo.clock.cumul_knps_all_threads() * 1000),
            hashfull_per_mille: Some(algo.tt.hashfull_per_mille()),
            time_millis: Some(algo.clock.elapsed_search().0.as_millis() as u64),
            ..Default::default()
        }
    }

    // pub fn with_best_move(sr: &SearchResults) -> Self {
    //     SearchResults {
    //         mode: SearchResultsMode::BestMove,
    //         ..sr.clone()
    //     }
    // }

    pub fn bm(&self) -> Move {
        if self.best_pv.len() >= 1 {
            return self.best_pv[0];
        }
        Move::NULL_MOVE
    }

    // for multi pv the pv and the best_pv are very different
    pub fn initial_move(&self) -> Move {
        if self.pv.len() >= 1 {
            return self.pv[0];
        }
        Move::NULL_MOVE
    }

    pub fn set_pv(&mut self, event: Event, pv: &Variation) {
        self.event = Some(event);
        self.pv = pv.clone();
    }

    pub fn set_seldepth(&mut self, n: &Node) {
        // if !n.is_qs() && n.ply > self.depth {
        //     self.depth = n.ply;
        // }
        if n.ply > self.seldepth {
            self.seldepth = n.ply;
        }
    }

    pub fn with_best_move(&mut self, outcome: &Outcome) {
        self.mode = SearchProgressMode::BestMove;
        self.outcome = *outcome;
    }

    pub fn old_with_best_move(sr: &SearchProgress) -> Self {
        SearchProgress {
            mode: SearchProgressMode::BestMove,
            best_score: sr.score,
            best_pv: sr.pv.clone(),
            ..sr.clone()
        }
    }


    pub fn snapshot_bests(&mut self) {
        if self.score.is_finite() {
            // succesfully completed iter
            if self.multi_pv_index == 0 && self.pv.len() == 0 {
                info!(
                    "Would copying score {} pv {} over best pv {} for iter {} event {:?}",
                    self.score, self.pv, self.best_pv, self.depth, self.event
                );
            }
            if self.multi_pv_index == 0 {
                // && self.pv.len() > 0 {
                self.best_score = self.score;
                self.best_pv = self.pv.clone();
            }
        } else if self.multi_pv_index == 0 && self.take_move_from_part_ply {
            self.best_score = self.score;
            self.best_pv = self.pv.clone();
        }
    }

    pub fn with_pv_change(
        &mut self,
        board: &Board,
        clock: &Clock,
        restrictions: &Restrictions,
        tt: &TranspositionTable2,
        pv_table: &PvTable,
        ply: Ply,
        score: Score,
    ) {
        self.board = board.clone();
        self.multi_pv_index = restrictions.multi_pv_index();
        self.multi_pv_index_of = restrictions.multi_pv_count;
        self.pv = pv_table.extract_pv();
        // self.best_pv = stats.pv().clone();
        self.score = score;
        // self.best_score = stats.score();
        self.nodes = Some(clock.cumul_nodes_all_threads());
        self.nodes_thread = Some(clock.cumul_nodes_this_thread());
        self.nps = Some(clock.cumul_knps_all_threads() * 1000);
        self.depth = ply;
        self.seldepth = pv_table.selective_depth();
        self.time_millis = Some(clock.elapsed_search().0.as_millis() as u64);
        self.hashfull_per_mille = Some(tt.hashfull_per_mille());
        // self.branching_factor = Some(stats.branching_factor());
        if self.score.is_finite() {
            self.mode = SearchProgressMode::PvChange;
        } else {
            self.mode = SearchProgressMode::NodeCounts;
        }

        // check PV for validity
        if !board.is_legal_variation(&self.pv) {
            debug_assert!(
                false,
                "PV  {} is invalid on board {}\n{:?}",
                self.pv,
                board,
                self.pv,
            );
            self.pv.truncate(1);
        }
    }

    pub fn update_with_pv_change(
        &mut self,
        clock: &Clock,
        depth: Ply,
        score: Score,
        event: Event,
    ) {
        if event != Event::UserCancelled && event != Event::SearchTimeUp {
            self.mode = SearchProgressMode::PvChange;
        } else {
            self.mode = SearchProgressMode::NodeCounts;
        }

        self.event = Some(event);
        self.score = score;
        self.depth = depth;
        self.nodes = Some(clock.cumul_nodes_all_threads());
        self.nodes_thread = Some(clock.cumul_nodes_this_thread());
        self.nps = Some(clock.cumul_knps_all_threads() * 1000);
        self.time_millis = Some(clock.elapsed_search().0.as_millis() as u64);
        // self.branching_factor = Some(counts.cumul(Event::PercentBranchingFactor) as f32 / 100.0);

        // check PV for validity
        if !self.board.is_legal_variation(&self.pv) {
            error!(
                "PV  {} is invalid on board {}\n{:?}\n",
                self.pv, self.board, self.pv
            );
            self.pv.truncate(1);
        }
    }

    pub fn to_pos(&self) -> Position {
        let mut pos = Position::from_board(self.board.clone());
        pos.set(Tag::Pv(self.best_pv.clone()));
        if self.best_pv.len() > 0 {
            pos.set(Tag::SuppliedMove(self.best_pv[0]));
            pos.set(Tag::BestMoves(MoveList::from_iter(iter::once(
                self.best_pv[0],
            ))));
        }
        pos.set(Tag::CentipawnEvaluation(self.best_score.as_i16() as i32));
        pos.set(Tag::AnalysisCountDepth(self.depth));
        pos.set(Tag::AnalysisCountSelDepth(self.seldepth));

        if let Some(nodes) = self.nodes_thread {
            pos.set(Tag::AnalysisCountNodes(nodes as u128));
        }
        // if let Some(bf) = self.branching_factor {
        //     pos.set(Tag::BranchingFactorPercent((100.0 * bf) as u32));
        // }
        pos
    }
}
