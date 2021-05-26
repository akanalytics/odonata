use crate::config::{Config, Component};
use crate::globals::counts;
use crate::log_debug;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::searchprogress::SearchProgress;
use crate::search::searchstats::{NodeStats, SearchStats};
use crate::search::timecontrol::TimeControl;
use crate::types::{Ply, MAX_PLY};
use std::fmt;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct IterativeDeepening {
    pub enabled: bool,
    pub part_ply: bool,
    iterations: Vec<SearchStats>,
}

impl Component for IterativeDeepening {
    fn settings(&self, c: &mut Config) {
        c.set("ids.enabled", "type check default true");
        c.set("ids.part_ply", "type check default true");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("qsearch.configure with {}", c);
        self.enabled = c.bool("ids.enabled").unwrap_or(self.enabled);
        self.part_ply = c.bool("ids.part_ply").unwrap_or(self.part_ply);
    }
    fn new_game(&mut self) {
        self.new_search();
    }

    fn new_search(&mut self) {
        self.iterations.clear();
        // self.tt.next_generation();
    }
}

impl Default for IterativeDeepening {
    fn default() -> Self {
        Self {
            enabled: true,
            part_ply: true,
            iterations: Vec::new(),
        }
    }
}

impl fmt::Display for IterativeDeepening {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "part_ply         : {}", self.part_ply)?;
        writeln!(f, "iterations       : {}", self.iterations.len())?;
        write!(f, "{:>3} {:>4} ", "dep", "stat")?;
        NodeStats::fmt_header(f)?;
        writeln!(f, " {:>8} {:<11}", "score", "pv")?;

        write!(f, "{:>3} {:>4} ", "---", "----")?;
        NodeStats::fmt_underline(f)?;
        writeln!(f, " {:>8} {:<11}", "--------", "-----------")?;
        for iter in self.iterations.iter() {
            write!(
                f,
                "D{:<2} {:>4} ",
                iter.depth,
                if iter.completed() { "OK" } else { "FAIL" }
            )?;
            iter.total().fmt_data(f)?;
            writeln!(f, " {:>8} {:<11}", iter.score.to_string(), iter.pv().to_string())?;
        }
        Ok(())
    }
}

impl IterativeDeepening {
    pub fn calc_range(&self, tc: &TimeControl) -> Range<Ply> {
        let range = if let TimeControl::Depth(depth) = *tc {
            if self.enabled {
                1..depth + 1
            } else {
                depth..depth + 1
            }
        } else {
            // regardless of iterative deeping, we apply it if no explicit depth given
            1..(MAX_PLY - 1)
        };
        range
    }


}

impl Algo {
    #[inline]
    pub fn set_iteration_depth(&mut self, max_depth: Ply) {
        self.max_depth = max_depth;
    }

    pub fn search_iteratively(&mut self) {
        self.new_search();

        //self.ids.reset();
        // self.tt.next_generation();
        // self.eval.cache.next_generation();
        self.range = self.ids.calc_range(&self.mte.time_control);
        for depth in self.range.clone() {
            //let mut root_node = Node::new_root(&mut self.board.clone());
            self.max_depth = depth;
            self.search_stats.depth = depth;

            self.alphabeta(&mut Node::new_root(&mut self.board.clone()));
            let mut res = self.search_stats().clone();

            self.mte.estimate_ply(depth + 1, &res);
            self.search_stats
                .record_time_estimate(depth + 1, &self.mte.time_estimate);
            self.ids.iterations.push(res.clone());
            if !res.completed() {
                counts::SEARCH_IDS_TIMEOUTS.increment();
                break;
            }
            if self.mte.probable_timeout(&res)  {
                break;
            }

            let mut sp = SearchProgress::from_search_stats(&res);
            sp.pv = Some(res.pv.clone());
            if !self.board.is_legal_variation(&res.pv) {             
                debug_assert!(false, "Unable to fetch valid pv {} on board {}\n{}", res.pv.clone(), self.board, self);
                res.pv.truncate(1);
                sp.pv = Some(res.pv.clone());
            }
    

            sp.score = Some(res.score);
            self.task_control.invoke_callback(&sp);
            counts::SEARCH_IDS_COMPLETES.increment();
            if res.score.is_mate() {
                break;
            }
        }

        let i = self.ids.iterations.iter().rposition(|r| r.completed());
        let last = self.ids.iterations.last().unwrap();
        if i.is_some() {
            let i = i.unwrap();
            let res = &self.ids.iterations[i];
            self.search_stats.pv = res.pv.clone();
            self.search_stats.score = res.score;
        }

        // && last.score > res.score
        if self.ids.part_ply  {
            self.search_stats.pv = last.pv.clone();
            self.search_stats.score = last.score;
        }
        self.search_stats.pv.truncate(self.max_depth as usize);
        let sp = SearchProgress::from_best_move(Some(self.bm()));
        self.task_control.invoke_callback(&sp);
    }
}
