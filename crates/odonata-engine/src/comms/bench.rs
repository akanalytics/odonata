use crate::{engine::Engine, search::engine::ThreadedSearch};
use anyhow::Result;
use itertools::Itertools;
use odonata_base::{
    catalog::*,
    domain::timecontrol::TimeControl,
    infra::{
        component::{Component, State},
        utils::Formatting,
    },
};
use std::time::{Duration, Instant};
pub struct Bench;

impl Bench {
    pub fn profile_me() {
        let _engine = ThreadedSearch::new();
    }

    pub fn search(tc: TimeControl, threads: Option<u32>) -> Result<u64> {
        let mut engine = ThreadedSearch::new();
        engine.search.thread_count = threads.unwrap_or(1);
        let epds = &Catalog::bench();

        println!(
            "{:>3} {:<6} {:>8} {:>2} {:>13} {:>7} {:>6} {:>5}  {:<85}",
            "#", "bm", "ce", "?", "nodes", "nps", "depth", "bf", "fen"
        );
        let mut total_time = Duration::from_millis(0);
        let mut total_nodes = 0;
        let mut total_depth = 0;
        let mut total_bf = 0.0;
        let mut score = 0;
        for (i, epd) in epds.iter().enumerate() {
            let t = Instant::now();

            engine.set_state(State::NewGame);
            let res = engine.search(epd.clone(), tc.clone()).unwrap();
            let elapsed = t.elapsed();
            let sm = res.supplied_move().unwrap();
            let correct = if epd.moves("bm").unwrap().iter().contains(&sm) {
                score += 1;
                "✔ " // '✅'
            } else {
                " x" //'❌'
            };
            let depth = res.depth;
            let sel_depth = res.seldepth;
            let cp = res.score().unwrap_or_default();
            let nps = Formatting::f64(res.nodes as f64 / elapsed.as_secs_f64());
            let bf = res.bf;
            let bf_string = Formatting::decimal(2, bf);
            let fen = res.to_epd().board().to_fen();
            total_bf += bf;
            total_time += elapsed;
            total_nodes += res.nodes;
            total_depth += depth;
            let nodes = Formatting::u64(res.nodes);
            println!(
                "{:>3} {:<6} {:>8} {:>2} {:>13} {:>7} {:>3}/{:<2} {:>5}  {:<85}",
                i + 1,
                sm.to_string(),
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
        let average_depth = total_depth as f64 / epds.len() as f64;
        let average_bf = total_bf / epds.len() as f64;
        let nps = total_nodes as f64 / total_time.as_secs_f64();
        println!();
        println!("time control  : {}", tc);
        println!("threads       : {}", engine.search.thread_count);
        println!("nodes/sec     : {}", Formatting::f64(nps));
        println!("average depth : {}", Formatting::decimal(2, average_depth));
        println!("average bf    : {}", Formatting::decimal(2, average_bf));
        println!("total nodes   : {}", Formatting::u64(total_nodes));
        println!("total time    : {}", Formatting::duration(total_time));
        println!("score         : {}", score);
        Ok(total_nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use odonata_base::infra::profiler::PerfProfiler;
    use std::hint::black_box;
    use test_log::test;

    #[test]
    fn bench_bratko() {
        let mut total_nodes = 0;
        let tc = if let Ok(s) = std::env::var("RUST_BENCH_TC") {
            s.parse()
                .unwrap_or_else(|_| panic!("RUST_BENCH_TC not a valid time control: {s}"))
        } else {
            TimeControl::NodeCount(1000)
        };

        let mut prof = PerfProfiler::new("bench_bratko");
        prof.bench(|| total_nodes += Bench::search(tc.clone(), None).unwrap());
        prof.set_iters(total_nodes / 1000);
    }

    #[test]
    fn bench_search() {
        let pos = Catalog::test_position();
        let eng = ThreadedSearch::new();
        let mut eng = eng.search;
        let tc = TimeControl::Depth(8);
        let mut prof = PerfProfiler::new("bench_search");
        let _ = black_box(prof.bench(|| eng.search(pos.clone(), tc.clone())));
        // Metrics::flush_thread_local();
        let metrics = eng.metrics("").unwrap();
        info!(target:"metrics","{metrics}");
    }
}
