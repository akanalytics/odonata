use crate::eval::score::Score;
use crate::infra::component::{Component, State};
use crate::infra::metric::Metric;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::timecontrol::TimeControl;
use crate::types::{Ply, MAX_PLY};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::search_results::SearchResults;

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

    // #[serde(skip)]
    // iterations: Vec<SearchStats>,
}

impl Component for IterativeDeepening {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.start_ply = 1;
        self.end_ply = MAX_PLY - 1;
        // self.iterations.clear();
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
            // iterations: Vec::new(),
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
        // writeln!(f, "iterations       : {}", self.iterations.len())?;
        // write!(f, "{:>3} {:>4} ", "dep", "stat")?;
        // NodeStats::fmt_header(f)?;
        // writeln!(f, " {:>8} {:<11}", "score", "pv")?;

        // write!(f, "{:>3} {:>4} ", "---", "----")?;
        // NodeStats::fmt_underline(f)?;
        // writeln!(f, " {:>8} {:<11}", "--------", "-----------")?;
        // for iter in self.iterations.iter() {
        //     write!(
        //         f,
        //         "D{:<2} {:>4} ",
        //         iter.depth,
        //         if iter.interrupted() { "PART" } else { "FULL" }
        //     )?;
        //     iter.iteration().fmt_data(f)?;
        //     writeln!(
        //         f,
        //         " {:>8} {:<11}",
        //         iter.score().to_string(),
        //         "".to_string() // iter.pv().to_string()
        //     )?;
        // }
        // if let Some(last) = self.iterations.last() {
        //     write!(f, "{:>3} {:>4} ", "---", "----")?;
        //     NodeStats::fmt_underline(f)?;
        //     writeln!(f, " {:>8} {:<11}", "--------", "-----------")?;
        //     write!(f, "{:>8} ", "cumul")?;
        //     last.cumulative().fmt_data(f)?;
        //     writeln!(f, " {:>8} {:<11}", "-", "-")?;
        // }
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
        self.ids.calc_range(&self.mte.time_control);
        let mut ply = self.ids.start_ply;
        let mut multi_pv = Vec::new();
        let mut last_good_multi_pv = Vec::new();

        'outer: loop {
            self.set_state(State::StartDepthIteration(ply));
            let t = Metric::timing_start();
            self.stats.new_iteration();
            multi_pv.resize_with(self.restrictions.multi_pv_count, Default::default);
            for i in 0..self.restrictions.multi_pv_count {
                let score = self
                    .aspirated_search(&mut self.board.clone(), &mut Node::root(ply))
                    .0;
                self.mte.estimate_iteration(ply + 1, &self.clock);
                // self.stats
                //     .record_time_estimate(ply + 1, &self.mte.estimate_move_time);
                // self.ids.iterations.push(self.search_stats().clone());
                let pv = self.pv_table.extract_pv();
                self.progress.with_pv_change(
                    &self.board,
                    &self.clock,
                    &self.restrictions,
                    &self.tt,
                    &self.pv_table,
                    ply,
                    score,
                );

                self.progress.snapshot_bests();
                self.controller.invoke_callback(&self.progress);
                let exit = self.exit_iteration(ply, score);

                multi_pv[i] = (pv.clone(), score);

                if exit {
                    break 'outer;
                }
                if let Some(&mv) = multi_pv[i].0.first() {
                    self.restrictions.exclude_moves.push(mv);
                }
            }
            Metric::IterActual(ply, t).record();
            last_good_multi_pv = std::mem::take(&mut multi_pv);
            ply += self.ids.step_size
        }

        self.results = SearchResults::new(self);
        if self.time_up_or_cancelled(ply, false).0 {
            self.results.multi_pv = last_good_multi_pv;
        } else {
            self.results.multi_pv = multi_pv;
        }

        self.progress.with_best_move(&self.board.outcome());
        self.controller.invoke_callback(&self.progress);
        if self.max_depth > 0
            && !self.progress.outcome.is_game_over()
            && self.progress.bm().is_null()
        {
            error!("bm is null\n{}\n{:?}", self, self.progress);
        }
    }

    pub fn exit_iteration(&mut self, ply: Ply, s: Score) -> bool {
        self.time_up_or_cancelled(ply, false).0
            || self.mte.probable_timeout(ply)
            || ply >= self.ids.end_ply
            || ply >= MAX_PLY / 2
            || (self.restrictions.exclude_moves.is_empty() && s.is_mate())
        // pv.empty = draw
    }
}
