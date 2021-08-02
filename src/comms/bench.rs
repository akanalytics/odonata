use crate::catalog::*;
use crate::clock::Clock;
use crate::config::Component;
use crate::perft::Perft;
use crate::search::algo::Engine;
use crate::tags::Tag;
use crate::search::timecontrol::TimeControl;
// use env_logger;
use crate::utils::Formatter;
// use crate::globals::counts;
use std::time::{Duration, Instant};

pub struct Bench;

impl Bench {
    pub fn perft(depth: u32) {
        let mut board = Catalog::starting_position();
        for d in 1..=depth {
            let t = Instant::now();
            let p = Perft::perft(&mut board, d);
            let elapsed = t.elapsed();
            println!("perft({}) = {:<14} in {}", d, p, Clock::format(elapsed));
            if d == depth {
                println!(
                    "\n{} nodes/sec",
                    Formatter::format_f64(p as f64 / elapsed.as_secs_f64())
                );
            }
        }
        // println!("\nstatistics\n{}", counts::GLOBAL_COUNTS);
    }

    pub fn perft_cat(depth: u32) {
        let mut board = Catalog::starting_position();
        println!(
            "{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}",
            "depth", "total", "captures", "en passant", "castles", "promos", "time"
        );
        for d in 1..=depth {
            let t = Instant::now();
            let mut p = Perft::default();
            let total = p.perft_cat(&mut board, d);
            let time = Clock::format(t.elapsed());
            let elapsed = t.elapsed();
            println!(
                "{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}",
                d, total, p.captures, p.en_passant, p.castles, p.promos, time
            );
            if d == depth {
                println!(
                    "\n{} nodes/sec",
                    Formatter::format_f64(total as f64 / elapsed.as_secs_f64())
                );
            }
        }
        // println!("\nstatistics\n{}", counts::GLOBAL_COUNTS);
    }

    pub fn search(tc: TimeControl, threads: u32) {
        println!("time control {}\n", tc);
        let mut engine = Engine::new();
        engine.thread_count = threads;
        engine.algo.set_timing_method(tc);
        let positions = &Catalog::bench();

        println!(
            "{:>3} {:<8} {} {:>13} {:>7} {:>5}  {:<85}",
            "#", "bm", "*", "nodes", "nps", "depth", "fen"
        );
        let mut total_time = Duration::from_millis(0);
        let mut total_nodes = 0;
        let mut total_depth = 0;
        let mut score = 0;
        for (i, pos) in positions.iter().enumerate() {
            let t = Instant::now();

            engine.new_game();
            engine.algo.board = pos.board().clone();
            if tc == TimeControl::DefaultTime {
                let suggested_depth = pos.acd().unwrap();
                engine.algo.set_timing_method(TimeControl::Depth(suggested_depth));
            }

            engine.search();
            let elapsed = t.elapsed();
            let bm = pos.board().to_san(&engine.algo.bm());
            let correct = if pos.bm().ok().unwrap().contains(&engine.algo.bm()) {
                score += 1;
                '1'
            } else {
                '-'
            };
            let depth = engine.algo.results().acd().unwrap();
            let sel_depth = engine.algo.results().get_tag(Tag::ACSD).value_uci();
            let nodes = engine.algo.results().acn().unwrap();
            let nps = Formatter::format_f64(nodes as f64 / elapsed.as_secs_f64());
            let fen = engine.algo.results().board().to_fen();
            total_time += elapsed;
            total_nodes += nodes;
            total_depth += depth;
            let nodes = Formatter::format_u128(nodes);
            println!(
                "{:>3} {:<8} {} {:>13} {:>7} {:>2}/{:<2}  {:<85}",
                i + 1,
                bm,
                correct,
                nodes,
                nps,
                depth, sel_depth,
                fen
            );
        }
        let average_depth = total_depth as f64 / positions.len() as f64;
        let nps = total_nodes as f64 / total_time.as_secs_f64();
        println!();
        println!("nodes/sec:     {}", Formatter::format_f64(nps));
        println!("average depth: {}", Formatter::format_f64(average_depth));
        println!("total nodes:   {}", Formatter::format_u128(total_nodes));
        println!("total time:    {}", Formatter::format_duration(total_time));
        println!("score:         {}", score);
    }
}
