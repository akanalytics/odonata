use odonata::catalog::*;
use odonata::movelist::*;
use odonata::perft::Perft;
use odonata::search::algo::Engine;
use odonata::search::node::Node;
use odonata::search::timecontrol::TimeControl;

// use criterion::measurement::Measurement;
// use criterion::*;


use iai::black_box;


// criterion_group!(
//     name = benches;
//     config = Criterion::default().with_measurement(PerfMeasurement::new(PerfMode::Instructions));
//     targets = benchmark_perft, benchmark_bitboard
// );

iai::main!(iai_search, iai_legal_moves_into, iai_perft5, iai_eval);
// use criterion_linux_perf::{PerfMeasurement, PerfMode};

fn iai_legal_moves_into() {
    let pos = Catalog::starting_position();
    let board = pos.board();
    let mut ml = MoveList::new();
    black_box(black_box(&board).legal_moves_into(&mut ml));
}

fn iai_perft5() {
    let mut pos = Catalog::starting_position();
    black_box(Perft::perft(&mut pos.board_mut(), black_box(5)));
}

fn iai_eval() {
    let engine = Engine::new();
    let pos = Catalog::starting_position();
    black_box(engine.algo.eval.w_evaluate(pos.board(), &Node::root(0)));
}

fn iai_search() {
    let mut engine = Engine::new();
    engine.algo.set_timing_method(TimeControl::Depth(5)).build();
    engine.set_position(Catalog::starting_position());
    black_box(engine.search());
}