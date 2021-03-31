use crate::movelist::{Move, MoveList};
use crate::search::stats::Stats;
use crate::eval::Score;

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
    pub fn from_stats(stats: &Stats) -> Self {
        SearchProgress {
            nodes: Some(stats.total_nodes()),
            nps: Some(stats.knps() * 1000),
            depth: Some(stats.depth),
            seldepth: Some(stats.seldepth),
            time_millis: Some(stats.clock.elapsed().as_millis() as u64),
            ..Default::default()
        }
    }
}
