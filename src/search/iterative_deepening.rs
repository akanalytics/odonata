use crate::globals::counts;
use crate::infra::component::{Component, State};
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::searchstats::{NodeStats, SearchStats};
use crate::search::timecontrol::TimeControl;
use crate::types::{Ply, MAX_PLY};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IterativeDeepening {
    pub enabled: bool,
    pub part_ply: bool,
    pub step_size: Ply,

    #[serde(skip)]
    pub start_ply: Ply,

    #[serde(skip)]
    pub end_ply: Ply,

    #[serde(skip)]
    iterations: Vec<SearchStats>,
}

impl Component for IterativeDeepening {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.start_ply = 1;
        self.end_ply = MAX_PLY - 1;
        self.iterations.clear();
    }
}

impl Default for IterativeDeepening {
    fn default() -> Self {
        Self {
            enabled: true,
            part_ply: false,
            step_size: 1,

            start_ply: 1,
            end_ply: MAX_PLY - 1,
            iterations: Vec::new(),
        }
    }
}

impl fmt::Display for IterativeDeepening {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "part_ply         : {}", self.part_ply)?;
        writeln!(f, "step_size        : {}", self.step_size)?;
        writeln!(f, "start_ply        : {}", self.start_ply)?;
        writeln!(f, "end_ply          : {}", self.end_ply)?;
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
                if iter.interrupted() { "PART" } else { "FULL" }
            )?;
            iter.iteration().fmt_data(f)?;
            writeln!(
                f,
                " {:>8} {:<11}",
                iter.score().to_string(),
                iter.pv().to_string()
            )?;
        }
        if let Some(last) = self.iterations.last() {
            write!(f, "{:>3} {:>4} ", "---", "----")?;
            NodeStats::fmt_underline(f)?;
            writeln!(f, " {:>8} {:<11}", "--------", "-----------")?;
            write!(f, "{:>8} ", "cumul")?;
            last.cumulative().fmt_data(f)?;
            writeln!(f, " {:>8} {:<11}", "-", "-")?;
        }
        Ok(())
    }
}

impl IterativeDeepening {
    pub fn calc_range(&mut self, tc: &TimeControl) {
        if let TimeControl::Depth(depth) = *tc {
            if !self.enabled || depth == 0 {
                self.start_ply = depth;
                self.end_ply = depth;
            } else {
                self.start_ply = 1;
                self.end_ply = depth;
            }
        } else {
            // regardless of iterative deeping, we apply it if no explicit depth given
            self.start_ply = 1;
            self.end_ply = MAX_PLY - 1;
        };
    }
}

impl Algo {
    pub fn search_iteratively(&mut self) {
        // self.new_search();
        self.ids.calc_range(&self.mte.time_control);
        let mut depth = self.ids.start_ply;
        'outer: loop {
            self.set_state(State::StartDepthIteration(depth));
            self.stats.new_iteration();

            for _multi_pv_index in 0..self.restrictions.multi_pv_count {
                self.aspiration(&mut self.board.clone(), &mut Node::root(depth));
                // self.stats.clock.start_ply();
                self.mte.estimate_iteration(depth + 1, &self.clock);
                self.stats
                    .record_time_estimate(depth + 1, &self.mte.estimate_move_time);
                self.ids.iterations.push(self.search_stats().clone());

                if self.search_stats().interrupted() {
                    counts::SEARCH_IDS_TIMEOUTS.increment();
                } else {
                    counts::SEARCH_IDS_COMPLETES.increment();
                }

                self.results.with_pv_change(
                    &self.board,
                    &self.clock,
                    &self.stats,
                    &self.restrictions,
                    &self.tt,
                );

                // let results = &self.results;
                self.results.snapshot_bests();
                self.controller.invoke_callback(&self.results);
                let exit = self.exit_iteration(depth);
                if exit {
                    break 'outer;
                }
                self.restrictions
                    .exclude_moves
                    .push(self.results.initial_move());
            }
            depth += self.ids.step_size
        }
        self.results.with_best_move(&self.board.outcome());
        self.controller.invoke_callback(&self.results);
        // debug!("\n\n\n=====Search completed=====\n{}", self);
        if self.max_depth > 0 && !self.results.outcome.is_game_over() && self.results.bm().is_null()
        {
            error!("bm is null\n{}\n{:?}", self, self.results);
        }

        // self.results.with_best_move();
        // self.task_control.invoke_callback(&self.results);
        // debug!("\n\n\n=====Search completed=====\n{}", self);
        // if self.results.bm().is_null() {
        //     error!("bm is null\n{}\n{:?}", self, self.results);
        // }
    }

    pub fn exit_iteration(&self, depth: Ply) -> bool {
        self.search_stats().interrupted()
            || self.mte.probable_timeout(depth)
            || self.stats.depth >= self.ids.end_ply
            || self.stats.depth >= MAX_PLY / 2
            || (self.restrictions.exclude_moves.is_empty()
                && (self.search_stats().score().is_mate() || self.pv().is_empty()))
        // pv.empty = draw
    }
}
