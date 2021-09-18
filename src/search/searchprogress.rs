use crate::eval::score::Score;
use crate::mv::Move;
use crate::search::searchstats::SearchStats;
use crate::types::Ply;
use crate::variation::Variation;

/// essentially all the data needed for UCI info status updates or for a decent progress bar
#[derive(Clone, Default, Debug)]
pub struct SearchProgress {
    pub refutation: bool,
    pub depth: Option<Ply>,
    pub seldepth: Option<Ply>,
    pub time_millis: Option<u64>,
    pub pv: Option<Variation>,
    pub nodes: Option<u64>,
    pub multipv: Option<String>,
    pub score: Option<Score>,
    pub currmove: Option<Move>,
    pub currmovenumber_from_1: Option<u32>,
    pub hashfull_per_mille: Option<u32>,
    pub nps: Option<u64>,
    pub tbhits: Option<u64>,
    pub cpuload_per_mille: Option<u32>,
    pub bestmove: Option<Move>,
    // pub refutation: Option<Move>,
    // pub currline: Option<MoveList>,
    pub debug_info: Option<String>,
}

impl SearchProgress {
    // pub fn from_best_move(best: Option<Move>, pov: Color, stats: &SearchStats) -> Self {
    //     SearchProgress {
    //         bestmove: best,
    //         pov,
    //         pv: Some(stats.pv().clone()),   
    //         score: Some(stats.score),  
    //         nodes: Some(stats.total().all_nodes()),
    //         nps: Some(stats.total_knps() * 1000),
    //         depth: Some(stats.depth()),
    //         seldepth: Some(stats.selective_depth()),
    //         time_millis: Some(stats.elapsed(false).as_millis() as u64),
    //         ..Default::default()
    //     }
    // }
    pub fn report_progress(stats: &SearchStats) -> Self {
        SearchProgress {
            nodes: Some(stats.all_threads_cumulative_total_nodes()),
            nps: Some(stats.all_threads_cumulative_knps() * 1000),
            ..Default::default()
        }
    }

    pub fn pv_change(best: Option<Move>, stats: &SearchStats) -> Self {
        SearchProgress {
            bestmove: best,
            pv: Some(stats.pv().clone()),   
            score: if stats.score > -Score::INFINITY {Some(stats.score)} else {None},  
            nodes: Some(stats.all_threads_cumulative_total_nodes()),
            nps: Some(stats.all_threads_cumulative_knps() * 1000),
            depth: Some(stats.depth()),
            seldepth: Some(stats.selective_depth()),
            time_millis: Some(stats.cumulative_time_as_millis() as u64),
            ..Default::default()
        }
    }
}
