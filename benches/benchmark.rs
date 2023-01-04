// use odonata::bits::bb_classical::ClassicalBitboard;
// use odonata::bits::bb_hyperbola::Hyperbola;
// use odonata::bits::bb_magic::*;
// use odonata::bits::bb_sliders::*;
// use odonata::bits::bitboard::*;
// use odonata::bits::precalc::*;
// use odonata::bits::square::*;
// use odonata::board::analysis::Analysis;
// use odonata::board::rules::Rules;
// use odonata::boards::Board;
// use odonata::board::BoardCalcs;
// use odonata::bound::NodeType;
// use odonata::cache::hasher::*;
// use odonata::cache::tt2::*;
// use odonata::catalog::*;
// use odonata::domain::material::*;
// use odonata::eval::eval::*;
// use odonata::eval::score::*;
// use odonata::eval::see::See;
// use odonata::globals::constants::*;
// use odonata::infra::component::*;
// use odonata::infra::utils::*;
// use odonata::movelist::*;
// use odonata::mv::*;
// use odonata::perft::Perft;
// use odonata::phaser::Phaser;
// use odonata::piece::*;
// use odonata::search::algo::Algo;
// use odonata::search::move_orderer::*;
// use odonata::search::node::Node;
// use odonata::search::timecontrol::TimeControl;
// use odonata::variation::*;
// use std::sync::atomic::{AtomicU64, Ordering};
// use std::thread;
// use std::time::Instant;
// use perf_event::{events::Hardware, Builder, Counter, Group};

// use criterion::measurement::Measurement;
// use criterion::*;
// use log::*;

// use odonata::infra::black_box;
// use odonata::infra::profiler::PerfProfiler;



// // impl Drop for PerfProfiler {
// //     fn drop(&mut self) {
// //         // if log::log_enabled!(log::Level::Trace) {
// //         let _ = self.write(std::io::stdout());
// //         // }
// //     }
// // }

// fn benchmark_king_attacks() {
//     let b = Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
//         .unwrap();
//     let analysis = Analysis::of(&b);
//     let bbd = PreCalc::default();
//     let mut prof = PerfProfiler::new("king_attacks".into());
//     for _ in 0..100 {
//         prof.benchmark(|| analysis.king_attacks(b.kings() & b.white()));
//         // assert_eq!(
//         //     ka,
//         //     bbd.within_chebyshev_distance_inclusive(Square::E1, 1) - Bitboard::E1
//         // );
//     }
//     assert_eq!(
//         analysis.king_attacks(b.kings() & b.black()),
//         bbd.within_chebyshev_distance_inclusive(Square::E8, 1) - Bitboard::E8
//     );
//     prof.write(std::io::stdout());
// }


fn main() -> anyhow::Result<()> {
    eprintln!("Hello");
    // benchmark_king_attacks();
    Ok(())
}

