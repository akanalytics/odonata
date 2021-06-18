use crate::catalog::*;
use crate::clock::Clock;
use crate::config::Component;
use crate::perft::Perft;
use crate::search::algo::Algo;
use crate::search::timecontrol::TimeControl;
// use env_logger;
use crate::globals::counts;
use std::time::Instant;

pub struct Bench;

impl Bench {
    pub fn perft(depth: u32) {
        let mut board = Catalog::starting_position();
        for d in 1..=depth {
            let t = Instant::now();
            let p = Perft::perft(&mut board, d);
            println!("perft({}) = {:<14} in {}", d, p, Clock::format(t.elapsed()));
        }
        println!("\nstatistics\n{}", counts::GLOBAL_COUNTS);
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
            println!(
                "{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}",
                d, total, p.captures, p.en_passant, p.castles, p.promos, time
            );
        }
        println!("\nstatistics\n{}", counts::GLOBAL_COUNTS);
    }

    pub fn search() {
        let mut search = Algo::new()
            .set_timing_method(TimeControl::move_time_secs(1))
            .build();
        let pos = &Catalog::test_position();

        for _i in 0..6 {
            search.search(pos.board());
            // println!("{}", search.results());
            search.new_game();
        }
        println!("\n\n{}\n", search);
    }
}
