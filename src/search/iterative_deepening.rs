use crate::config::{Config, Component};
use crate::globals::counts;
use crate::{debug, logger::LogInit};
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::searchprogress::SearchProgress;
use crate::search::searchstats::{NodeStats, SearchStats};
use crate::search::timecontrol::TimeControl;
use crate::position::Position;
use crate::tags::Tag;
use crate::movelist::MoveList;
use crate::types::{Ply, MAX_PLY};
use std::fmt;
use std::iter::FromIterator;
use std::iter;

#[derive(Clone, Debug)]
pub struct IterativeDeepening {
    pub enabled: bool,
    pub part_ply: bool,
    pub step_size: Ply,
    pub start_ply: Ply,
    pub end_ply: Ply,
    iterations: Vec<SearchStats>,
}

impl Component for IterativeDeepening {
    
    fn settings(&self, c: &mut Config) {
        c.set("ids.enabled", "type check default true");
        c.set("ids.part_ply", "type check default true");
    }

    fn configure(&mut self, c: &Config) {
        debug!("ids.configure");
        self.enabled = c.bool("ids.enabled").unwrap_or(self.enabled);
        self.part_ply = c.bool("ids.part_ply").unwrap_or(self.part_ply);
    }

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.iterations.clear();
        // self.tt.next_generation();
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
                if iter.completed() { "FULL" } else { "PART" }
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
                self.end_ply = depth + 1;
            } else {
                self.start_ply = 1;
                self.end_ply = depth + 1;
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

        for depth in (self.ids.start_ply..self.ids.end_ply).step_by(self.ids.step_size as usize) {
            //let mut root_node = Node::new_root(&mut self.board.clone());
            self.max_depth = depth;
            self.search_stats.depth = depth;
            // self.eval.depth = depth;

            self.run_alphabeta(&mut self.board.clone(), &mut Node::root(depth));
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

            let mut sp = SearchProgress::from_stats(&res, self.board.color_us());
            let pv = res.pv.clone();
            // pv.truncate(depth as usize);
            sp.pv = Some(pv);
            if !self.board.is_legal_variation(&res.pv) {             
                debug_assert!(false, "PV  {} is invalid on board {}\n{:?}\n{}", res.pv, self.board, res.pv, self);
                res.pv.truncate(1);
                let pv = res.pv.clone();
                sp.pv = Some(pv);
            }
    

            sp.score = Some(res.score.to_root_score(res.depth));
            self.task_control.invoke_callback(&sp);
            counts::SEARCH_IDS_COMPLETES.increment();
            if res.score.is_mate() {
                break;
            }
        }

        let i = self.ids.iterations.iter().rposition(|r| r.completed());
        // let last = self.ids.iterations.last().unwrap();
        if i.is_some() {
            let i = i.unwrap();
            let res = &self.ids.iterations[i];
            self.search_stats.pv = res.pv.clone();
            self.search_stats.score = res.score;
        }

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
        let sp = SearchProgress::from_best_move(Some(self.bm()), self.board.color_us(), &self.search_stats);
        self.task_control.invoke_callback(&sp);

        self.results = Position::from_board(self.board.clone());
        self.results.set(Tag::SuppliedMove(self.bm()));
        self.results.set(Tag::BestMove(MoveList::from_iter(iter::once(self.bm()))));
        self.results.set(Tag::Pv(self.pv().clone()));
        self.results.set(Tag::CentipawnEvaluation(self.score().as_i16() as i32));
        self.results.set(Tag::AnalysisCountDepth(self.search_stats().depth()));
        self.results.set(Tag::AnalysisCountSelDepth(self.search_stats().selective_depth()));
        self.results.set(Tag::AnalysisCountNodes(self.search_stats().cumulative().nodes() as u128));
        self.results.set(Tag::BranchingFactorPercent((100.0 * self.search_stats().branching_factor()) as u32 ));

    }
}
