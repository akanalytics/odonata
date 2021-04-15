use crate::search::algo::Algo;
use crate::board::Board;
use crate::search::timecontrol::TimeControl;
use crate::config::{Config, Configurable};
use crate::search::node::Node;
use crate::search::searchstats::SearchStats;
use crate::search::searchprogress::SearchProgress;
use crate::log_debug;
use crate::types::MAX_PLY;
use std::fmt;
use std::ops::Range;



#[derive(Clone, Debug)]
pub struct IterativeDeepening {
    enabled: bool,
    iterations: Vec<SearchStats>,
}



impl Configurable for IterativeDeepening {
    fn settings(&self, c: &mut Config) {
        c.set("ids.enabled", "type check default true");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("quiescence.configure with {}", c);
        self.enabled = c.bool("ids.enabled").unwrap_or(self.enabled);
    }
}

impl Default for IterativeDeepening {
    fn default() -> Self {
        Self { enabled: true, iterations: Vec::new() }
    }
}

impl fmt::Display for IterativeDeepening {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)
    }
}


impl IterativeDeepening {

    pub fn calculate_range(&self, tc: &TimeControl) -> Range<u32> {
        let range = if let TimeControl::Depth(depth) = *tc {
                if self.enabled {
                    1..depth+1
                } else {
                    depth..depth+1
                }
            } else {
                // regardless of iterative deeping, we apply it if no explicit depth given
                1..(MAX_PLY as u32) 
            };
            range
    }
}



impl Algo {

    pub fn search2(&mut self, board: &Board) {
        for depth in self.range.clone() {
            let mut root_node = Node::new_root(&board);
            
            self.alphabeta(&mut root_node);
            let results = self.search_stats().clone();

            self.move_time_estimator.calculate_etimates_for_ply(depth+1, &results);
            self.search_stats.record_time_estimate(depth+1, &self.move_time_estimator.time_estimate);
            if !results.complete() || results.score.is_mate() || self.move_time_estimator.probable_timeout(&results) {
                self.ids.iterations.push(results);
                break;
            }
            self.ids.iterations.push(results);
        }
        let i = self.ids.iterations.iter().rposition(|r| r.complete()).unwrap();
        let results = &self.ids.iterations[i];

        // callback
        let mut sp = SearchProgress::from_search_stats(&results);
        sp.pv = Some(results.pv.clone());
        sp.score = Some(results.score);
        self.task_control.invoke_callback(&sp);
    }

}
// pub fn search(&mut self, mut board: Board) -> Algo {
//     self.search_stats = SearchStats::new();
//     self.current_best = None;
//     self.overall_best_move = Move::NULL_MOVE;
//     self.score = Score::default();
//     self.clock_checks = 0;
//     self.task_control.set_running();

//     for depth in self.range.clone() {
//         self.set_iteration_depth(depth);
//         let mut root_node = Node::new_root(&mut board);
//         let stats = &mut self.search_stats;
//         let mut sp = SearchProgress::from_search_stats(stats);
//         self.move_time_estimator.calculate_etimates_for_ply(depth, stats);
//         stats.record_time_estimate(depth, &self.move_time_estimator.time_estimate);
        
//         if self.score.is_mate() || self.move_time_estimator.probable_timeout(stats) {
//             break;
//         }
//         self.score = Score::default();
//         self.pv_table = PvTable::new(MAX_PLY);
//         self.search_stats.clear_node_stats();
//         let clock = Clock::new();
//         // println!("Iterative deepening... ply {}", depth);

//     XXX self.alphabeta(&mut root_node);
        
//         self.search_stats.record_time_actual(depth, &clock.elapsed());
//         if !self.task_control.is_cancelled() {
//             self.score = root_node.score;
//             self.pv = self.pv_table.extract_pv();
//             self.pv_table = self.pv_table.clone();
//             self.current_best = Some(self.pv[0]);
//             sp = SearchProgress::from_search_stats(&self.search_stats());
//             sp.pv = Some(self.pv.clone());
//             sp.score = Some(self.score);
//             self.task_control.invoke_callback(&sp);
//         } else {
//             self.task_control.invoke_callback(&sp);
//             break;
//         }
//     }

//     self.overall_best_move = self.pv()[0];
//     let sp = SearchProgress::from_best_move(Some(self.overall_best_move()));
//     self.task_control.invoke_callback(&sp);
//     self.clone()
// }

