use odonata::catalog::*;
use odonata::eval::eval::SimpleScorer;
use odonata::eval::model::Model;
use odonata::movelist::*;
use odonata::perft::Perft;
use odonata::search::algo::Engine;
use odonata::search::node::Node;
use odonata::search::timecontrol::TimeControl;

// use criterion::measurement::Measurement;
// use criterion::black_box;

use iai::black_box;


// criterion_group!(
//     name = benches;
//     config = Criterion::default().with_measurement(PerfMeasurement::new(PerfMode::Instructions));
//     targets = benchmark_perft, benchmark_bitboard
// );

iai::main!(
    iai_search,
    iai_legal_moves_into,
    iai_perft5,
    iai_eval_full,
    iai_eval_without_wdl,
    iai_eval_model,
    iai_build_model_and_eval_model,
);


// fn main() {
//     for n in 0..100000 {
//         iai_eval_model();
//     }
// }


// use criterion_linux_perf::{PerfMeasurement, PerfMode};

fn iai_legal_moves_into() {
    let pos = Catalog::starting_position();
    let board = pos.board();
    let mut ml = MoveList::new();
    black_box(black_box(&board).legal_moves_into(&mut ml));
}

fn iai_perft5() {
    let mut pos = Catalog::starting_position();
    black_box(Perft::perft(&mut pos.board_mut(), 5));
}

fn iai_eval_full() {
    let engine = Engine::new();
    let pos = Catalog::starting_position();
    black_box(engine.algo.eval.w_evaluate(pos.board(), &Node::root(0)));
}

fn iai_eval_without_wdl() {
    let engine = Engine::new();
    let pos = Catalog::starting_position();
    black_box(engine.algo.eval.w_eval_without_wdl(pos.board(), &Node::root(0)));
}

fn iai_search() {
    let mut engine = Engine::new();
    engine.algo.set_timing_method(TimeControl::Depth(5)).build();
    engine.set_position(Catalog::starting_position());
    black_box(engine.search());
}

fn iai_build_model_and_eval_model() {
    let eval = SimpleScorer::new();
    let pos = Catalog::starting_position();
    for _ in 0..10000 {
        let model = black_box(Model::from_board(pos.board()));
        black_box(eval.predict(black_box(&model)));
    }
}

fn iai_eval_model() {
    let eval = SimpleScorer::new();
    let pos = Catalog::starting_position();
    let model = Model::from_board(pos.board());
    for _ in 0..10000 {
        black_box(eval.predict(black_box(&model)));
    }
}

