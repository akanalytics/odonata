use crate::catalog::*;
use crate::infra::component::{Component, State};
use crate::mv::Move;
use crate::perft::Perft;
use crate::search::engine::ThreadedSearch;
use crate::search::timecontrol::TimeControl;
use crate::tags::Tag;
// use env_logger;
use crate::infra::utils::Formatting;
// use crate::globals::counts;
use itertools::Itertools;
use std::time::{Duration, Instant};

pub struct Bench;

impl Bench {
    pub fn profile_me() {
        let _engine = ThreadedSearch::new();

        // let eval = SimpleScorer::new();
        // let pos = Catalog::starting_position();
        // let model = Model::from_board(pos.board(), Switches::ALL_SCORING);
        // let mut scorer = ModelScore::new(pos.board().phase(&eval.phaser));
        // for _ in 0..100_000 {
        //     eval.predict(&model, &mut scorer);
        // }
    }

    pub fn perft(depth: u32) {
        let mut board = Catalog::starting_board();
        for d in 1..=depth {
            let t = Instant::now();
            let p = Perft::perft(&mut board, d);
            let elapsed = t.elapsed();
            println!(
                "perft({}) = {:<14} in {}",
                d,
                p,
                Formatting::duration(elapsed)
            );
            if d == depth {
                println!(
                    "\n{} nodes/sec",
                    Formatting::f64(p as f64 / elapsed.as_secs_f64())
                );
            }
        }
        // println!("\nstatistics\n{}", counts::GLOBAL_COUNTS);
    }

    pub fn perft_cat(depth: u32) {
        let mut board = Catalog::starting_board();
        println!(
            "{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}",
            "depth", "total", "captures", "en passant", "castles", "promos", "time"
        );
        for d in 1..=depth {
            let t = Instant::now();
            let mut p = Perft::default();
            let total = p.perft_cat(&mut board, d);
            let time = Formatting::duration(t.elapsed());
            let elapsed = t.elapsed();
            println!(
                "{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}",
                d, total, p.captures, p.en_passant, p.castles, p.promos, time
            );
            if d == depth {
                println!(
                    "\n{} nodes/sec",
                    Formatting::f64(total as f64 / elapsed.as_secs_f64())
                );
            }
        }
        // println!("\nstatistics\n{}", counts::GLOBAL_COUNTS);
    }

    pub fn search(tc: TimeControl, threads: Option<u32>) -> u128 {
        let mut engine = ThreadedSearch::new();
        if let Some(threads) = threads {
            engine.thread_count = threads;
        }
        let positions = &Catalog::bench();

        println!(
            "{:>3} {:<6} {:>8} {:>2} {:>13} {:>7} {:>5} {:>5}  {:<85}",
            "#", "bm", "ce", "?", "nodes", "nps", "depth", "bf", "fen"
        );
        let mut total_time = Duration::from_millis(0);
        let mut total_nodes = 0;
        let mut total_depth = 0;
        let mut total_bf = 0.0;
        let mut score = 0;
        for (i, pos) in positions.iter().enumerate() {
            let t = Instant::now();

            engine.set_state(State::NewGame);
            engine.set_position(pos.clone());
            if tc == TimeControl::DefaultTime {
                let suggested_depth = pos.acd().unwrap();
                engine
                    .algo
                    .set_timing_method(TimeControl::Depth(suggested_depth));
            } else {
                engine.algo.set_timing_method(tc);
            }

            engine.search_sync();
            let elapsed = t.elapsed();
            let bm = &engine.algo.results.best_move().unwrap_or_default();
            let correct = if pos
                .bm()
                .ok()
                .unwrap()
                .iter()
                .map(Move::to_inner)
                .contains(bm)
            {
                score += 1;
                '1'
            } else {
                '-'
            };
            let depth = engine.algo.results_as_position().acd().unwrap();
            let sel_depth = engine.algo.results_as_position().tag(Tag::ACSD).value_uci();
            let nodes = engine.algo.results_as_position().acn().unwrap();
            let cp = engine.algo.results.score().unwrap_or_default();
            let nps = Formatting::f64(nodes as f64 / elapsed.as_secs_f64());
            let bf = engine.algo.results_as_position().branching_factor();
            let bf_string = Formatting::decimal(2, bf);
            let fen = engine.algo.results_as_position().board().to_fen();
            total_bf += bf;
            total_time += elapsed;
            total_nodes += nodes;
            total_depth += depth;
            let nodes = Formatting::u128(nodes);
            println!(
                "{:>3} {:<6} {:>8} {:>2} {:>13} {:>7} {:>2}/{:<2} {:>5}  {:<85}",
                i + 1,
                bm,
                cp.to_string(),
                correct,
                nodes,
                nps,
                depth,
                sel_depth,
                bf_string,
                fen
            );
        }
        let average_depth = total_depth as f64 / positions.len() as f64;
        let average_bf = total_bf / positions.len() as f64;
        let nps = total_nodes as f64 / total_time.as_secs_f64();
        println!();
        println!("time control  : {}", tc);
        println!("nodes/sec     : {}", Formatting::f64(nps));
        println!("average depth : {}", Formatting::decimal(2, average_depth));
        println!("average bf    : {}", Formatting::decimal(2, average_bf));
        println!("total nodes   : {}", Formatting::u128(total_nodes));
        println!("total time    : {}", Formatting::duration(total_time));
        println!("score         : {}", score);
        total_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::engine::Engine,
        infra::{black_box, profiler::PerfProfiler},
    };
    use test_log::test;

    #[test]
    fn bench_bk() {
        let mut total_nodes = 0;
        let mut prof = PerfProfiler::new("bench_bratko".into());
        prof.benchmark(|| total_nodes += Bench::search(TimeControl::NodeCount(1000), None));
        prof.set_iters((total_nodes / 1000) as u64);
    }

    #[test]
    fn bench_search() {
        let pos = Catalog::test_position();
        let eng = ThreadedSearch::new();
        let mut eng = eng.algo;
        let tc = TimeControl::Depth(5);
        let mut prof = PerfProfiler::new("bench_search".into());
        let _ = black_box(prof.benchmark(|| eng.search(pos, tc)));
    }
}
