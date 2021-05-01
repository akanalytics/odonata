use crate::board::Board;
use crate::config::{Config, Configurable};
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
        Self {
            enabled: true,
            iterations: Vec::new(),
        }
    }
}

impl fmt::Display for IterativeDeepening {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
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

    pub fn reset(&mut self) {
        self.iterations.clear();
    }
}

impl Algo {
    #[inline]
    pub fn set_iteration_depth(&mut self, max_depth: Ply) {
        self.max_depth = max_depth;
    }

    pub fn search(&mut self, board: &Board) {
        self.search_stats = SearchStats::new();
        self.board = board.clone();
        self.task_control.set_running();
        self.ids.reset();
        self.range = self.ids.calc_range(&self.mte.time_control);
        for depth in self.range.clone() {
            //let mut root_node = Node::new_root(&mut self.board.clone());
            self.max_depth = depth;
            self.search_stats.depth = depth;

            self.alphabeta(&mut Node::new_root(&mut self.board.clone()));
            let res = self.search_stats().clone();

            self.mte.estimate_ply(depth + 1, &res);
            self.search_stats
                .record_time_estimate(depth + 1, &self.mte.time_estimate);
            self.ids.iterations.push(res.clone());
            if !res.completed() {
                counts::SEARCH_IDS_TIMEOUTS.increment();
                break;
            }
            if self.mte.probable_timeout(&res) || res.score.is_mate() {
                break;
            }

            let mut sp = SearchProgress::from_search_stats(&res);
            sp.pv = Some(res.pv.clone());
            sp.score = Some(res.score);
            self.task_control.invoke_callback(&sp);
            counts::SEARCH_IDS_COMPLETES.increment();
        }

        let i = self.ids.iterations.iter().rposition(|r| r.completed());
        if i.is_none() {
            println!("rpos!!!\n\n{}", self);
        }
        let i = i.unwrap();
        let res = &self.ids.iterations[i];
        self.search_stats.pv = res.pv.clone();
        self.search_stats.score = res.score;
        // callback
        let sp = SearchProgress::from_best_move(Some(self.bm()));
        self.task_control.invoke_callback(&sp);
        info!("{}", self);
        // self.pv = res.pv().clone();
    }
}
