use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use itertools::Itertools;
use odonata_base::catalog::*;
use odonata_base::domain::timecontrol::TimeControl;
use odonata_base::infra::component::{Component, State};
use odonata_base::infra::utils::Formatting;
use odonata_base::other::tags::EpdOps as _;

use crate::search::engine::ThreadedSearch;
pub struct Bench;

impl Bench {
    pub fn profile_me() {
        let _engine = ThreadedSearch::new();
    }

    pub fn search(tc: TimeControl, threads: Option<u32>, settings: HashMap<String, String>) -> Result<u64> {
        let mut engine = ThreadedSearch::with_threads(threads.unwrap_or(1));
        engine.configure(settings)?;
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
            let fen = res.to_results_epd().board().to_fen();
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
        println!("eval          : {}", engine.search.eval.eval_kind);
        println!("time control  : {}", tc);
        println!("threads       : {}", engine.thread_count);
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
    use std::hint::black_box;

    use odonata_base::infra::profiler::PerfProfiler;
    use test_log::test;

    use super::*;

    #[test]
    fn bench_bratko() {
        let mut total_nodes = 0;
        let tc = if let Ok(s) = std::env::var("RUST_BENCH_TC") {
            s.parse()
                .unwrap_or_else(|_| panic!("RUST_BENCH_TC not a valid time control: {s}"))
        } else {
            TimeControl::NodeCount(1000)
        };

        let mut prof = PerfProfiler::new("bench_bratko_approx");
        prof.bench(|| total_nodes += Bench::search(tc.clone(), None, HashMap::new()).unwrap());
        prof.set_iters(total_nodes / 1000); // total number of searches

        let mut prof_accurate = PerfProfiler::new("bench.bratko");

        let mut engine = ThreadedSearch::new();
        let mut nodes = 0;
        let tc = TimeControl::NodeCount(1_000);

        for epd in Catalog::bench().iter() {
            prof_accurate.bench(|| {
                engine.set_state(State::NewGame);
                let res = engine.search(epd.clone(), tc.clone()).unwrap();
                nodes += res.nodes;
            });
        }
        prof.set_iters(Catalog::bench().len() as u64);
    }

    #[test]
    fn bench_search() {
        let pos = Catalog::test_position();
        let mut eng = ThreadedSearch::new();
        let tc = TimeControl::Depth(10);
        let mut prof = PerfProfiler::new("bench_search");
        let _ = black_box(prof.bench(|| {
            eng.search.new_game();
            eng.search(pos.clone(), tc.clone())
        }));
        // Metrics::flush_thread_local();
        let metrics = eng.metrics("").unwrap();
        info!(target:"metrics","{metrics}");
    }
}
