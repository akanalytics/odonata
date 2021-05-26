use crate::catalog::*;
use crate::clock::Clock;
use crate::perft::Perft;
use crate::config::Component;
use crate::search::algo::Algo;
use crate::search::timecontrol::TimeControl;
// use env_logger;
use std::time::{Instant};
use crate::globals::counts;

pub struct Bench;

impl Bench {
    pub fn perft(depth: u32) {
        let mut board = Catalog::starting_position();
        for d in 1..=depth {
            let t = Instant::now();
            let p = Perft::perft(&mut board, d);
            println!("perft({}) = {:<12} in {}", d, p, Clock::format(t.elapsed()));
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
