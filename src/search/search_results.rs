use std::iter::{self, FromIterator};

use crate::board::Board;
use crate::eval::score::Score;
use crate::mv::Move;
use crate::tags::Tag;
use crate::types::Ply;
use crate::variation::Variation;
use crate::{Algo, MoveList, Position};

/// essentially all the data needed for UCI info status updates or for a decent progress bar
#[derive(Clone, Debug)]
pub enum SearchResultsMode {
    BestMove,
    Refutation,
    PvChange,
}

impl Default for SearchResultsMode {
    fn default() -> Self {
        Self::PvChange
    }
}

#[derive(Clone, Default, Debug)]
pub struct SearchResults {
    pub board: Board,
    pub mode: SearchResultsMode,
    pub depth: Option<Ply>,
    pub seldepth: Option<Ply>,
    pub time_millis: Option<u64>,
    pub multi_pv_index: Option<u32>,
    pub pv: Option<Variation>,
    pub nodes: Option<u64>,
    pub nodes_thread: Option<u64>,
    pub score: Option<Score>,
    pub currmove: Option<Move>,
    pub currmovenumber_from_1: Option<u32>,
    pub hashfull_per_mille: Option<u32>,
    pub nps: Option<u64>,
    pub tbhits: Option<u64>,
    pub cpuload_per_mille: Option<u32>,
    pub branching_factor: Option<f32>,
    // pub refutation: Option<Move>,
    // pub currline: Option<MoveList>,
}

impl SearchResults {
    pub fn with_report_progress(algo: &Algo) -> Self {
        SearchResults {
            board: algo.board.clone(),
            nodes: Some(algo.search_stats().all_threads_cumulative_total_nodes()),
            nodes_thread: Some(algo.search_stats().cumulative_nodes()),
            nps: Some(algo.search_stats().all_threads_cumulative_knps() * 1000),
            hashfull_per_mille: Some(algo.tt.hashfull_per_mille()),
            time_millis: Some(algo.search_stats().cumulative_time_as_millis() as u64),
            ..Default::default()
        }
    }

    pub fn with_best_move(sr: &SearchResults) -> Self {
        SearchResults {
            mode: SearchResultsMode::BestMove,
            ..sr.clone()
        }
    }

    pub fn bm(&self) -> Move {
        if let Some(ref pv) = self.pv {
            if pv.len() >= 1 {
                return pv[0];
            }
        }
        Move::NULL_MOVE
    }


    

    // pub fn pv_change(best: Option<Move>, algo: &Algo) -> Self {
    //     let stats = algo.search_stats();
    //     SearchResults {
    //         mode: SearchResultsMode::BestMove,
    //         bestmove: best,
    //         pv: Some(stats.pv().clone()),   
    //         score: if stats.score > -Score::INFINITY {Some(stats.score)} else {None},  
    //         nodes: Some(stats.all_threads_cumulative_total_nodes()),
    //         nps: Some(stats.all_threads_cumulative_knps() * 1000),
    //         depth: Some(stats.depth()),
    //         seldepth: Some(stats.selective_depth()),
    //         time_millis: Some(stats.cumulative_time_as_millis() as u64),
    //         hashfull_per_mille: Some(algo.tt.hashfull_per_mille()),
    //         ..Default::default()
    //     }
    // }



    pub fn with_pv_change(algo: &Algo) -> Self {
        let stats = algo.search_stats();
        let mut sr = SearchResults {
            mode: SearchResultsMode::PvChange,
            board: algo.board.clone(),
            multi_pv_index: if algo.restrictions.multi_pv_count > 1 {
                Some(algo.restrictions.multi_pv_index())
            } else {
                None
            },
            pv: Some(stats.pv().clone()),
            score: if stats.score() > -Score::INFINITY {
                Some(stats.score())
            } else {
                None
            },
            nodes: Some(stats.all_threads_cumulative_total_nodes()),
            nodes_thread: Some(stats.cumulative_nodes()),
            nps: Some(stats.all_threads_cumulative_knps() * 1000),
            depth: Some(stats.depth()),
            seldepth: Some(stats.selective_depth()),
            time_millis: Some(stats.cumulative_time_as_millis() as u64),
            hashfull_per_mille: Some(algo.tt.hashfull_per_mille()),
            branching_factor: Some(stats.branching_factor()),
            ..Default::default()
        };

        // check PV for validity
        if !sr.board.is_legal_variation(stats.pv()) {
            debug_assert!(
                false,
                "PV  {} is invalid on board {}\n{:?}\n{}",
                stats.pv(),
                sr.board,
                stats.pv(),
                algo
            );
            if let Some(ref mut pv) = sr.pv {
                pv.truncate(1);
            }
        }
        sr
    }

    pub fn to_pos(&self) -> Position {
        let mut pos = Position::from_board(self.board.clone());
        if let Some(ref pv) = self.pv {
            pos.set(Tag::Pv(pv.clone()));
            if pv.len() > 0 {
                pos.set(Tag::SuppliedMove(pv[0]));
                pos.set(Tag::BestMoves(MoveList::from_iter(iter::once(pv[0]))));
            }
        }
        if let Some(ce) = self.score {
            pos.set(Tag::CentipawnEvaluation(ce.as_i16() as i32));
        }
        if let Some(depth) = self.depth {
            pos.set(Tag::AnalysisCountDepth(depth));
        }
        if let Some(seldepth) = self.seldepth {
            pos.set(Tag::AnalysisCountSelDepth(seldepth));
        }
        if let Some(nodes) = self.nodes_thread {
            pos.set(Tag::AnalysisCountNodes(nodes as u128));
        }
        if let Some(bf) = self.branching_factor {
            pos.set(Tag::BranchingFactorPercent((100.0 * bf) as u32));
        }
        pos
    }
}
