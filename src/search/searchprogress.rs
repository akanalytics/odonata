use crate::movelist::{Move, MoveList};
use crate::search::searchstats::SearchStats;
use crate::eval::score::Score;

/// essentially all the data needed for UCI info status updates or for a decent progress bar
#[derive(Clone, Default, Debug)]
pub struct SearchProgress {
    pub depth: Option<u32>,
    pub seldepth: Option<u32>,
    pub time_millis: Option<u64>,
    pub pv: Option<MoveList>,
    pub nodes: Option<u64>,
    pub multipv: Option<String>,
    pub score: Option<Score>,
    pub currmove: Option<Move>,
    pub currmovenumber_from_1: Option<u32>,
    pub hashfull_per_mille: Option<u32>,
    pub nps: Option<u128>,
    pub tbhits: Option<u64>,
    pub cpuload_per_mille: Option<u32>,
    pub bestmove: Option<Move>,
    // pub refutation: Option<Move>,
    // pub currline: Option<MoveList>,
    pub debug_info: Option<String>,
}

impl SearchProgress {
    pub fn from_best_move(best: Option<Move>) -> Self {
        SearchProgress { bestmove: best, ..Default::default() }
    }
    pub fn from_search_stats(search_stats: &SearchStats) -> Self {
        SearchProgress {
            nodes: Some(search_stats.total().nodes()),
            nps: Some(search_stats.total_knps() * 1000),
            depth: Some(search_stats.depth()),
            seldepth: Some(search_stats.selective_depth()),
            time_millis: Some(search_stats.elapsed(false).as_millis() as u64),
            ..Default::default()
        }
    }
}
