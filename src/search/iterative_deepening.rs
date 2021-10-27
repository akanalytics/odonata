use crate::eval::score::Score;
use crate::globals::counts;
use crate::infra::parsed_config::Component;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::search_results::SearchResults;
use crate::search::searchstats::{NodeStats, SearchStats};
use crate::search::timecontrol::TimeControl;
use crate::types::{Ply, MAX_PLY};
use serde::{Deserialize, Serialize};
use std::fmt;


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct IterativeDeepening {
    pub enabled: bool,
    pub part_ply: bool,
    pub step_size: Ply,

    #[serde(skip)]
    pub start_ply: Ply,
    pub end_ply: Ply,


    #[serde(skip)]
    iterations: Vec<SearchStats>,
}

impl Component for IterativeDeepening {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.iterations.clear();
    }
}

impl Default for IterativeDeepening {
    fn default() -> Self {
        Self {
            enabled: true,
            part_ply: true,
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
            iter.total().fmt_data(f)?;
            writeln!(f, " {:>8} {:<11}", iter.score.to_string(), iter.pv().to_string())?;
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
            self.restrictions.exclude_moves.clear();
            for multi_pv_index in 0..self.restrictions.multi_pv_count {

                self.run_alphabeta(&mut self.board.clone(), &mut Node::root(depth));
                self.stats.clock.start_ply();
                self.mte.estimate_iteration(depth + 1, &self.stats);
                self.stats
                    .record_time_estimate(depth + 1, &self.mte.time_estimate);
                self.ids.iterations.push(self.search_stats().clone());

                if self.search_stats().interrupted()  {
                    counts::SEARCH_IDS_TIMEOUTS.increment();
                } else {
                    counts::SEARCH_IDS_COMPLETES.increment();
                }

                let results = SearchResults::with_pv_change(&self);
                let bm = results.bm();
                // if we were interrupted and no score was set, use the score/move/pv given
                if let Some(score) = results.score {
                    if score != -Score::INFINITY && score != Score::INFINITY {
                        self.task_control.invoke_callback(&results);
                        // we take snapshot the pv/bm only if has completed
                        if multi_pv_index == 0 { // && !self.search_stats().interrupted() {
                            self.results = results;
                        }
                    }
                }

                let exit = self.exit_iteration();
                if exit {
                    break 'outer;
                }
                self.restrictions.exclude_moves.push(bm);
            }
            depth += self.ids.step_size
        }
        let bm_results = SearchResults::with_best_move(&self.results);
        self.task_control.invoke_callback(&bm_results);
    }

    pub fn exit_iteration(&self) -> bool {
        self.search_stats().interrupted()
            || self.mte.probable_timeout(&self.search_stats())
            || self.stats.depth >= self.ids.end_ply
            || self.stats.depth >= MAX_PLY / 2
            || ( self.restrictions.exclude_moves.len() == 0 && (self.search_stats().score.is_mate()
              || self.pv().is_empty()))  // pv.empty = draw 
    }
}

    // self.task_control.invoke_callback(&self.results);
    //let i = self.ids.iterations.iter().rposition(|r| r.completed());
    // let last = self.ids.iterations.last().unwrap();
    // if i.is_some() {
    //     let i = i.unwrap();
    //     let res = &self.ids.iterations[i];
    //     self.search_stats.pv = res.pv.clone();
    //     self.search_stats.score = res.score;
    // }

    // TODO!
    // in theory the root node in the tt wont have been written by a partial completed ply (single threaded mode), but
    // the pv and the score might have changed. Really we should take the score and bm from a partially completed ply if set, as
    // the previous best will have been searched first, and a change means its no longer best.
    // if self.ids.part_ply  {
    //     self.search_stats.pv = last.pv.clone();
    //     if self.search_stats.pv.len() > 0 {
    //         self.search_stats.score = self.tt.extract_pv_and_score(&self.board).1.unwrap_or_default();  // default wrong as its -inf
    //     }
    // }
    // self.search_stats.pv.truncate(self.max_depth as usize);
    // self.results = sp.to_pos();

    // pub fn search_iteratively3(&mut self) {
    //     // self.new_search();
    //     self.ids.calc_range(&self.mte.time_control);

    //     for depth in (self.ids.start_ply..self.ids.end_ply).step_by(self.ids.step_size as usize) {
    //         //let mut root_node = Node::new_root(&mut self.board.clone());
    //         self.max_depth = depth;
    //         self.stats.depth = depth;
    //         // self.eval.depth = depth;

    //         self.run_alphabeta(&mut self.board.clone(), &mut Node::root(depth));
    //         let mut res = self.search_stats().clone();

    //         self.stats.clock.start_ply();
    //         self.mte.estimate_iteration(depth + 1, &res);
    //         self.stats
    //             .record_time_estimate(depth + 1, &self.mte.time_estimate);
    //         self.ids.iterations.push(res.clone());
    //         if res.interrupted() {
    //             counts::SEARCH_IDS_TIMEOUTS.increment();
    //             break;
    //         }
    //         if self.mte.probable_timeout(&res)  {
    //             break;
    //         }

    //         let mut sp = SearchResults::with_pv_change(&self);
    //         let pv = res.pv.clone();
    //         // pv.truncate(depth as usize);
    //         sp.pv = Some(pv);
    //         if !self.board.is_legal_variation(&res.pv) {             
    //             debug_assert!(false, "PV  {} is invalid on board {}\n{:?}\n{}", res.pv, self.board, res.pv, self);
    //             res.pv.truncate(1);
    //             let pv = res.pv.clone();
    //             sp.pv = Some(pv);
    //         }
    

    //         sp.score = Some(res.score);
    //         self.task_control.invoke_callback(&sp);
    //         counts::SEARCH_IDS_COMPLETES.increment();
    //         if res.score.is_mate() {
    //             break;
    //         }
    //     }

    //     let i = self.ids.iterations.iter().rposition(|r| !r.interrupted());
    //     // let last = self.ids.iterations.last().unwrap();
    //     if i.is_some() {
    //         let i = i.unwrap();
    //         let res = &self.ids.iterations[i];
    //         self.stats.pv = res.pv.clone();
    //         self.stats.score = res.score;
    //     }

    //     // TODO!
    //     // in theory the root node in the tt wont have been written by a partial completed ply (single threaded mode), but
    //     // the pv and the score might have changed. Really we should take the score and bm from a partially completed ply if set, as 
    //     // the previous best will have been searched first, and a change means its no longer best.  
    //     // if self.ids.part_ply  {
    //     //     self.stats.pv = last.pv.clone();
    //     //     if self.stats.pv.len() > 0 {
    //     //         self.search_stats.score = self.tt.extract_pv_and_score(&self.board).1.unwrap_or_default();  // default wrong as its -inf
    //     //     }
    //     // }
    //     // self.search_stats.pv.truncate(self.max_depth as usize);
    //     self.results.pv = Some(self.stats.pv.clone());
    //     let sp = SearchResults::pv_change(Some(self.bm()), &self);
    //     self.task_control.invoke_callback(&sp);

    //     // self.results = Position::from_board(self.board.clone());
    //     // self.results.set(Tag::SuppliedMove(self.bm()));
    //     // self.results.set(Tag::BestMove(MoveList::from_iter(iter::once(self.bm()))));
    //     // self.results.set(Tag::Pv(self.pv().clone()));
    //     // self.results.set(Tag::CentipawnEvaluation(self.score().as_i16() as i32));
    //     // self.results.set(Tag::AnalysisCountDepth(self.search_stats().depth()));
    //     // self.results.set(Tag::AnalysisCountSelDepth(self.search_stats().selective_depth()));
    //     // self.results.set(Tag::AnalysisCountNodes(self.search_stats().cumulative().all_nodes() as u128));
    //     // self.results.set(Tag::BranchingFactorPercent((100.0 * self.search_stats().branching_factor()) as u32 ));

    // }


