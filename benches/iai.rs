use odonata::catalog::*;
use odonata::eval::eval::SimpleScorer;
use odonata::eval::model::Model;
use odonata::eval::model::ModelScore;
use odonata::eval::switches::Switches;
use odonata::position::*;
use odonata::movelist::*;
use odonata::perft::Perft;
use odonata::infra::tracer::*;
use odonata::search::engine::Engine;
use odonata::search::node::Node;
use odonata::search::timecontrol::TimeControl;
use static_init::{dynamic};

// use criterion::measurement::Measurement;
// use criterion::black_box;

use iai::black_box;


// criterion_group!(
//     name = benches;
//     config = Criterion::default().with_measurement(PerfMeasurement::new(PerfMode::Instructions));
//     targets = benchmark_perft, benchmark_bitboard
// );

iai::main!(
    iai_trace,
    iai_read_lock,
    iai_board_clone,
    iai_model_build,
    iai_model_predict,
    iai_eval_some,
    iai_search,
    iai_legal_moves_into,
    iai_perft5,
    iai_eval_full,
    iai_eval_material,
    iai_eval_model,
    iai_build_model_and_eval_model,
);

//
// we use dynamic here so that it gets init before main and gets included in the calibration run 
// and hence subtracted from the final results displayed
// we use default engine not new (loads config.toml) as reading a file does not play 
// well with instruction counts
//

#[dynamic]
static mut ENGINE: Engine = { let mut e = Engine::default(); e.algo.eval.pawn = true; e };

#[dynamic]
static mut POS: Position = Catalog::starting_position();


#[dynamic]
static mut MODEL: Model = Model::from_board(POS.read().board(), Switches::ALL_SCORING);
// fn main() {
//     for n in 0..100000 {
//         iai_eval_model();
//     }
// }

#[dynamic]
static WARM_UP: bool = {
    let pos = POS.read();
    let mut ml = MoveList::new();
    black_box(black_box(pos.board()).legal_moves_into(&mut ml));
    true
};

// use criterion_linux_perf::{PerfMeasurement, PerfMode};

fn iai_legal_moves_into() {
    let pos = POS.read();
    let mut ml = MoveList::new();
    black_box(black_box(pos.board()).legal_moves_into(&mut ml));
}

fn iai_board_clone() {
    black_box(POS.read().board().clone());
}

fn iai_read_lock() {
    black_box(POS.read().board());
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

fn iai_eval_some() {
    black_box(ENGINE.read().algo.eval.w_eval_some(POS.read().board(), Switches::ALL_SCORING));
}

fn iai_eval_material() {
    black_box(ENGINE.read().algo.eval.w_eval_some(POS.read().board(), Switches::MATERIAL));
}

fn iai_search() {
    let mut engine = Engine::new();
    engine.algo.set_timing_method(TimeControl::Depth(5)).build();
    engine.set_position(Catalog::starting_position());
    black_box(engine.search());
}


fn iai_model_build() {
    black_box(Model::from_board(POS.read().board(), Switches::ALL_SCORING));
}

fn iai_model_predict() {
    let mut model_score = ModelScore::new(50);
    black_box(ENGINE.read().algo.eval.predict(black_box(&MODEL.read()), &mut model_score) );
}

fn iai_build_model_and_eval_model() {
    let eval = SimpleScorer::new();
    let pos = Catalog::starting_position();
    let mut model_score = ModelScore::new(50);
    for _ in 0..10000 {
        let model = black_box(Model::from_board(pos.board(), Switches::ALL_SCORING));
        black_box(eval.predict(black_box(&model), &mut model_score));
    }
}

fn iai_eval_model() {
    let eval = SimpleScorer::new();
    let pos = Catalog::starting_position();
    let model = Model::from_board(pos.board(), Switches::ALL_SCORING);
    let mut model_score = ModelScore::new(50);
    for _ in 0..10000 {
        black_box(eval.predict(black_box(&model), &mut model_score));
    }
}

fn iai_trace() {
    let nt = NullTracer;
    for _i in 0..10_000 {
        nt.trace("Hello").trace("world").trace(&42);
    }

}
