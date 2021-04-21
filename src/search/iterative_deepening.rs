use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::log_debug;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::searchprogress::SearchProgress;
use crate::search::searchstats::{NodeStats, SearchStats};
use crate::search::timecontrol::TimeControl;
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
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "iterations       : {}", self.iterations.len())?;
        write!(f, "{:>4} ", "stat")?;
        NodeStats::fmt_header(f)?;
        writeln!(f, " {:>8} {:>8} {:>8} {:>11}", "alpha", "beta", "score", "pv")?;

        write!(f, "{:>4} ", "----")?;
        NodeStats::fmt_underline(f)?;
        writeln!(f, " {:>8} {:>8} {:>8} {:>11}", "--------", "--------", "--------", "-----------")?;
        for iter in self.iterations.iter() {
            write!(f, "{:>4} ", if iter.completed() { "OK" } else { "FAIL" })?;
            iter.total().fmt_data(f)?;
            writeln!(
                f,
                " {:>8} {:>8} {:>8} {:>11}",
                iter.alpha.to_string(),
                iter.beta.to_string(),
                iter.score.to_string(),
                iter.pv().to_string()
            )?;
        }
        Ok(())
    }
}

impl IterativeDeepening {
    pub fn calc_range(&self, tc: &TimeControl) -> Range<u32> {
        let range = if let TimeControl::Depth(depth) = *tc {
            if self.enabled {
                1..depth + 1
            } else {
                depth..depth + 1
            }
        } else {
            // regardless of iterative deeping, we apply it if no explicit depth given
            1..(MAX_PLY as u32)
        };
        range
    }

    pub fn reset(&mut self) {
        self.iterations.clear();
    }
}

impl Algo {
    pub fn search(&mut self, board: &Board) {
        self.search_stats = SearchStats::new();
        self.board = board.clone();
        self.task_control.set_running();
        self.ids.reset();
        self.range = self.ids.calc_range(&self.mte.time_control);
        for depth in self.range.clone() {
            //let mut root_node = Node::new_root(&mut self.board.clone());
            self.max_depth = depth;

            self.alphabeta(&mut Node::new_root(&mut self.board.clone()));
            let res = self.search_stats().clone();

            self.mte.calc_estimates_for_ply(depth + 1, &res);
            self.search_stats.record_time_estimate(depth + 1, &self.mte.time_estimate);
            if !res.completed() || res.score.is_mate() || self.mte.probable_timeout(&res) {
                self.ids.iterations.push(res);
                break;
            }
            // println!("{}", self);
            let mut sp = SearchProgress::from_search_stats(&res);
            sp.pv = Some(res.pv.clone());
            sp.score = Some(res.score);
            self.ids.iterations.push(res);
            self.task_control.invoke_callback(&sp);
        }

        let i = self.ids.iterations.iter().rposition(|r| r.completed());
        if i.is_none() {
            println!("rpos!!!\n\n{}", self);
        }
        let i = i.unwrap();
        let res = &self.ids.iterations[i];
        
        self.search_stats.pv = res.pv.clone();
        self.search_stats.score = res.score;
        self.search_stats.alpha = res.alpha;
        self.search_stats.beta = res.beta;
        // callback
        let sp = SearchProgress::from_best_move(Some(self.bm()));
        self.task_control.invoke_callback(&sp);
        // self.pv = res.pv().clone();
    }
}
