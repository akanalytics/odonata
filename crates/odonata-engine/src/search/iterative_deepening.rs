use odonata_base::{
    boards::Position,
    domain::{
        info::{Info, InfoKind},
        node::Node,
    },
    infra::{
        component::{Component, State},
        metric::Metrics,
        utils::calculate_branching_factor_by_nodes_and_depth,
    },
    other::outcome::Outcome,
    piece::MAX_PLY,
    prelude::*,
    variation::MultiVariation,
    Epd,
};
use serde::{Deserialize, Serialize};
use std::fmt;

use odonata_base::domain::node::{Counter, Event};

use super::{algo::Search, search_results::SearchResults, trail::Trail};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IterativeDeepening {
    pub enabled:   bool,
    pub part_ply:  bool,
    pub step_size: Ply,
    pub start_ply: Ply,
    pub end_ply:   Ply,
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
            enabled:   true,
            part_ply:  false,
            step_size: 1,

            start_ply: 1,
            end_ply:   MAX_PLY - 1,
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
        match tc.depth() {
            Some(0) => (self.start_ply, self.end_ply) = (0, 0),
            Some(d) if !self.enabled => (self.start_ply, self.end_ply) = (d, d),
            Some(d) => (self.start_ply, self.end_ply) = (1, d),
            None => (self.start_ply, self.end_ply) = (1, MAX_PLY - 1),
        }
    }
}

impl Search {
    // run_search -> search_iteratively -> aspirated_search -> root_search -> alpha_beta

    pub fn search_results_from_multi_pv(
        search: &Search,
        depth: Ply,
        multi_pv: MultiVariation,
        // seldepth: Option<Ply>,
        trail: &mut Trail,
        infos: &[Info],
    ) -> SearchResults {
        let nodes_thread_cumul = search.clock.cumul_nodes_this_thread();
        let bf = calculate_branching_factor_by_nodes_and_depth(nodes_thread_cumul, depth)
            .unwrap_or_default();
        SearchResults {
            supplied_move: multi_pv.best_move().unwrap_or_default(),
            // .get(0)
            // .map(|var| var.0.first().unwrap_or_default())
            // .unwrap_or_default(),
            outcome: Outcome::Unterminated,
            tbhits: 0,
            nodes: search.clock.cumul_nodes_all_threads(),
            nodes_thread: search.clock.cumul_nodes_this_thread(),
            nps: search.clock.cumul_knps_all_threads() * 1000,
            depth,
            seldepth: trail.selective_depth(),
            time_millis: search.clock.elapsed_search().time.as_millis() as u64,
            hashfull_per_mille: search.tt.hashfull_per_mille(),
            bf,
            multi_pv,
            infos: infos.to_vec(),
            emt: search.clock.elapsed_search().time,
            pos: Some(Epd::from_board(search.position.board().clone())),
            tc: Some(search.mte.time_control().clone()),
            // tree: Some(trail.take_tree()),
            // metrics: None,
        }
    }

    pub fn search_iteratively(&mut self, pos: &mut Position, trail: &mut Trail) {
        self.ids.calc_range(self.mte.time_control());
        let mut ply = self.ids.start_ply;
        // let mut last_good_multi_pv = Vec::new();
        let mut score = Score::zero();
        // let mut sel_depth = None;
        let mut last_results = SearchResults::new();
        let mut book_move = false;
        let mut infos = vec![];

        'outer: loop {
            // Metrics::flush_thread_local();
            self.set_state(State::StartDepthIteration(ply));
            let t = Metrics::timing_start();
            // self.stats.new_iteration();
            let mut multi_pv = MultiVariation::new();
            self.restrictions.excluded_moves.clear();

            // multi_pv.resize_with(self.controller.multi_pv, Default::default);
            // let mut exit = false;
            for _i in 0..self.controller.multi_pv {
                let pv = if let Some(mv) = (ply == self.ids.start_ply)
                    .then(|| self.opening_book.lookup(&self.board, &self.restrictions))
                    .flatten()
                {
                    score = Score::zero();
                    book_move = true;
                    Variation::new().append(mv)
                } else {
                    score = match self.aspirated_search(
                        trail,
                        &mut pos.clone(),
                        &mut Node::root(ply),
                        score,
                    ) {
                        Ok((score, _event)) => score,
                        Err(_evt) => Score::INFINITY,
                    };
                    self.mte.estimate_iteration(ply + 1, &mut self.clock);
                    book_move = false;
                    trail.root_pv().clone()
                };

                self.tt.rewrite_pv(&self.board, &pv);

                let info = if score.is_finite() {
                    // let sel_depth = Some(trail.selective_depth());
                    #[allow(clippy::cast_possible_truncation)]
                    Info {
                        kind: InfoKind::Pv,
                        nodes: Some(self.clock.cumul_nodes_all_threads()),
                        nodes_thread: Some(self.clock.cumul_nodes_this_thread()),
                        nps: Some(self.clock.cumul_knps_all_threads() * 1000),
                        time_millis: Some(self.clock.elapsed_search().time.as_millis() as u64),
                        hashfull_per_mille: Some(self.tt.hashfull_per_mille()),
                        multi_pv: Some(self.restrictions.excluded_moves.len() + 1),
                        pv: Some(pv.clone()),
                        score: Some(score),
                        depth: Some(ply),
                        seldepth: Some(trail.selective_depth()),
                        ..Info::default()
                    }
                } else {
                    #[allow(clippy::cast_possible_truncation)]
                    Info {
                        kind: InfoKind::NodeCounts,
                        nodes: Some(self.clock.cumul_nodes_all_threads()),
                        nodes_thread: Some(self.clock.cumul_nodes_this_thread()),
                        nps: Some(self.clock.cumul_knps_all_threads() * 1000),
                        time_millis: Some(self.clock.elapsed_search().time.as_millis() as u64),
                        hashfull_per_mille: Some(self.tt.hashfull_per_mille()),
                        ..Info::default()
                    }
                };

                // progress.snapshot_bests();
                self.controller.invoke_callback(&info);
                infos.push(info);
                // exit = self.exit_iteration(ply, score);

                multi_pv.push(pv.clone(), score);

                if let Some(mv) = pv.first() {
                    self.restrictions.excluded_moves.push(mv);
                }
                debug!(target:"tree","trail\n{trail:#}");
            }
            if let Some(t) = t {
                Metrics::elapsed(ply, t.elapsed(), Event::DurationIterActual);
            }

            // some stuff is captured even if we exit part way through an iteration
            let sr = Search::search_results_from_multi_pv(self, ply, multi_pv, trail, &infos);
            last_results.nodes = sr.nodes;
            last_results.nodes_thread = sr.nodes_thread;
            last_results.nps = sr.nps;
            last_results.time_millis = sr.time_millis;
            last_results.infos = sr.infos.clone();

            if self.time_up_or_cancelled(ply, false).0 {
                break 'outer;
            }
            last_results = sr;
            if book_move
                || self.mte.probable_timeout(ply)
                || ply >= self.ids.end_ply
                || ply >= MAX_PLY / 2
            {
                break 'outer;
            }
            ply += self.ids.step_size;
        }

        // record final outcome of search
        // self.game
        //     .make_engine_move(results.clone(), Duration::from_millis(results.time_millis)); // *self.mte.time_control());

        self.results = last_results;

        // capture the piece that is the best move
        if Metrics::metrics_enabled() {
            let mv = self.results.pv().first();
            if let Some(mv) = mv {
                let mover_piece = mv.mover_piece(&self.board);
                let counter = match mover_piece {
                    Piece::King => Counter::MoveBestPieceKing,
                    Piece::Queen => Counter::MoveBestPieceQueen,
                    Piece::Rook => Counter::MoveBestPieceRook,
                    Piece::Bishop => Counter::MoveBestPieceBishop,
                    Piece::Knight => Counter::MoveBestPieceKnight,
                    Piece::Pawn => Counter::MoveBestPiecePawn,
                };
                Metrics::incr(counter);
            }
        }

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

    // pub fn exit_iteration(&mut self, ply: Ply, _s: Score) -> bool {
    //     self.time_up_or_cancelled(ply, false).0
    //         || self.mte.probable_timeout(ply)
    //         || ply >= self.ids.end_ply
    //         || ply >= MAX_PLY / 2
    //     // || (self.restrictions.exclude_moves.is_empty() && s.is_mate())
    //     // pv.empty = draw
    // }
}
