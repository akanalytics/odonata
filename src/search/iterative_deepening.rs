use crate::domain::info::Info;
use crate::domain::info::InfoKind;
use crate::domain::Trail;
use crate::eval::score::Score;
use crate::infra::component::{Component, State};
use crate::infra::metric::Metrics;
use crate::piece::{Ply, MAX_PLY};
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::search::timecontrol::TimeControl;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::Event;
use crate::domain::SearchResults;

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
    // run_search -> search_iteratively -> aspirated_search -> root_search -> alpha_beta

    pub fn search_iteratively(&mut self) {
        self.ids.calc_range(&self.mte.time_control());
        let mut ply = self.ids.start_ply;
        let mut multi_pv = Vec::new();
        let mut last_good_multi_pv = Vec::new();        
        let mut score = Score::zero();
        let mut sel_depth = None;

        'outer: loop {
            Metrics::flush_thread_local();
            self.set_state(State::StartDepthIteration(ply));
            let t = Metrics::timing_start();
            // self.stats.new_iteration();
            multi_pv.resize_with(self.restrictions.multi_pv_count, Default::default);
            let mut exit = false;
            for i in 0..self.restrictions.multi_pv_count {
                let mut trail = Trail::new(self.board.clone());
                score = self
                    .aspirated_search(
                        &mut trail,
                        &mut self.board.clone(),
                        &mut Node::root(ply),
                        score,
                    )
                    .0;
                self.mte.estimate_iteration(ply + 1, &mut self.clock);
                let pv = trail.pv();

                let info = if score.is_finite() {
                    sel_depth = Some(trail.selective_depth());
                    Info {
                        kind: InfoKind::Pv,
                        nodes: Some(self.clock.cumul_nodes_all_threads()),
                        // self.nodes = Some(clock.elapsed_search().instructions);
                        nodes_thread: Some(self.clock.cumul_nodes_this_thread()),
                        nps: Some(self.clock.cumul_knps_all_threads() * 1000),
                        time_millis: Some(self.clock.elapsed_search().time.as_millis() as u64),
                        hashfull_per_mille: Some(self.tt.hashfull_per_mille()),

                        multi_pv: Some(self.restrictions.multi_pv_index() + 1),
                        // self.multi_pv_index_of = restrictions.multi_pv_count;
                        pv: Some(pv.to_inner()),
                        // self.best_pv = stats.pv().clone();
                        score: Some(score),
                        // self.best_score = stats.score();
                        depth: Some(ply),
                        seldepth: Some(trail.selective_depth()),
                        ..Info::default()
                    }
                } else {
                    Info {
                        kind: InfoKind::NodeCounts,
                        nodes: Some(self.clock.cumul_nodes_all_threads()),
                        // self.nodes = Some(clock.elapsed_search().instructions);
                        nodes_thread: Some(self.clock.cumul_nodes_this_thread()),
                        nps: Some(self.clock.cumul_knps_all_threads() * 1000),
                        time_millis: Some(self.clock.elapsed_search().time.as_millis() as u64),
                        hashfull_per_mille: Some(self.tt.hashfull_per_mille()),
                        ..Info::default()
                    }
                };

                // progress.snapshot_bests();
                self.controller.invoke_callback(&info);
                exit = self.exit_iteration(ply, score);

                multi_pv[i] = (pv.to_inner(), score);

                if let Some(mv) = multi_pv[i].0.first() {
                    let mv = self.board.augment_move(mv);
                    self.restrictions.exclude_moves.push(mv);
                }
                if trail.chess_tree.enabled() {
                    // info!(target:"tree", "chess tree\n{:#?}", trail.chess_tree);
                    trace!(target:"tree","chess tree\n{:#}", trail.chess_tree);
                }
            }
            if let Some(t) = t {
                Metrics::elapsed(ply, t.elapsed(), Event::DurationIterActual);
            }
            if exit {
                break 'outer;
            }
            last_good_multi_pv = std::mem::take(&mut multi_pv);
            ply += self.ids.step_size
        }

        let results = if self.time_up_or_cancelled(ply, false).0 {
            SearchResults::new(self, ply - self.ids.step_size, last_good_multi_pv, sel_depth)
        } else {
            SearchResults::new(self, ply, multi_pv, sel_depth)
        };

        // record final outcome of search
        // self.game
        //     .make_engine_move(results.clone(), Duration::from_millis(results.time_millis)); // *self.mte.time_control());
        self.results = results;

        let info = Info {
            kind: InfoKind::BestMove,
            pv: Some(self.results.pv()),
            ..Info::default()
        };
        self.controller.invoke_callback(&info);
        // if self.max_depth > 0
        //     && !progress.outcome.is_game_over()
        //     && progress.bm().is_null()
        // {
        //     error!("bm is null\n{}\n{:?}", self, progress);
        // }
    }

    pub fn exit_iteration(&mut self, ply: Ply, _s: Score) -> bool {
        self.time_up_or_cancelled(ply, false).0
            || self.mte.probable_timeout(ply)
            || ply >= self.ids.end_ply
            || ply >= MAX_PLY / 2
        // || (self.restrictions.exclude_moves.is_empty() && s.is_mate())
        // pv.empty = draw
    }
}
