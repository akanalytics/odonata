use criterion::{black_box, criterion_group, criterion_main, Criterion};
use odonata::bitboard::*;
use odonata::board::makemove::*;
use odonata::board::movegen::*;
use odonata::catalog::*;
use odonata::eval::*;
use odonata::material::*;
use odonata::movelist::*;
use odonata::pvtable::*;
use odonata::perft::Perft;
use odonata::search::algo::Algo;
use odonata::search::timecontrol::TimeControl;
use odonata::types::*;

/*
Bitboard 2.7ns (a|b)&c
Chooser 1.7ns
PieceToUpperChar 622ns
Perft(5) 334ms
Makemove 26.25ns
legal_moves 966ns
pseudo_legal_moves 300ns
Score: 5 > 4  1ns

search/minmax Depth 5  | search/alphabeta  Depth 5
initial         = 482ms    5.7ms
game end logic  = 6.8s     48ms
& early fin - no effect
collect PV no effect

12s (6.5s without game end test) - forgot to short circuit legal moves on leaf node


search/alphabeta Depth 5  =
85 ms (48 ms without game end tests)

Array (int = 6.3, enum = 6.7)

Material.is_insufficient 9.8ns


Jons_problem
180s (9ply)
1 hr (10ply)
478s (looks at end of game)

*/

fn bitwise_handcrafted(c: &mut Criterion) {
    let n1 = 1u64 << 3;
    let n2 = 1u64 << 4;
    let n3 = 1u64 << 5;
    c.bench_function("bitwise_handcrafted", |b| {
        b.iter(|| {
            let a = black_box(n1) | black_box(n2);
            let b = a & black_box(n3);
            black_box(b);
        });
    });
}

fn bitwise_bitflags(c: &mut Criterion) {
    let n1 = Bitboard::D1;
    let n2 = Bitboard::E1;
    let n3 = Bitboard::F1;
    c.bench_function("bibitwise_bitflags", |b| {
        b.iter(|| {
            let a = black_box(n1) | black_box(n2);
            let b = a & black_box(n3);
            black_box(b);
        });
    });
}

fn piece_to_upper_char(c: &mut Criterion) {
    c.bench_function("piece_to_upper_char", |b| {
        b.iter(|| {
            black_box(Piece::Pawn.to_upper_char());
            black_box(Piece::Knight.to_upper_char());
        });
    });
}

fn piece_to_char(c: &mut Criterion) {
    c.bench_function("piece_to_char", |b| {
        b.iter(|| {
            black_box(Piece::Pawn.to_char(Some(Color::White)));
            black_box(Piece::Knight.to_char(Some(Color::Black)));
        });
    });
}

fn benchmark_perft5(c: &mut Criterion) {
    let board = Catalog::starting_position();
    let mut group = c.benchmark_group("sample size 10");
    group.sample_size(10);
    group.bench_function("perft5", |b| {
        b.iter(|| {
            black_box(Perft::perft(&board, black_box(5)));
        });
    });
    group.finish();
}

fn benchmark_score(c: &mut Criterion) {
    let mut group = c.benchmark_group("score");
    group.bench_function("handcrafted", |b| {
        b.iter(|| {
            black_box(black_box(5) > black_box(4));
        });
    });
    group.bench_function("score enum", |b| {
        b.iter(|| {
            black_box(Score::Millipawns(black_box(5)) > Score::Millipawns(black_box(4)));
        });
    });
    group.finish();
}

fn make_move(c: &mut Criterion) {
    let board = Catalog::starting_position();
    let mv1 = board.parse_uci_move("e2e4").unwrap();
    let mv2 = board.parse_uci_move("b1c3").unwrap();
    c.bench_function("makemove", |b| {
        b.iter(|| {
            black_box(board.make_move(black_box(&mv1)));
            black_box(board.make_move(black_box(&mv2)));
        });
    });
}

fn pseudo_legal_moves(c: &mut Criterion) {
    let board = Catalog::starting_position();
    c.bench_function("pseudo_legal_moves", |b| {
        b.iter(|| {
            black_box(black_box(&board).pseudo_legal_moves());
        });
    });
}

fn legal_moves(c: &mut Criterion) {
    let board = Catalog::starting_position();
    c.bench_function("legal_moves", |b| {
        b.iter(|| {
            black_box(black_box(&board).legal_moves());
        });
    });
}

fn bench_chooser_array(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    c.bench_function("chooser_arr", |b| {
        b.iter(|| {
            black_box(chooser_array(black_box(white), &Bitboard::RANK_4, &Bitboard::RANK_5));
            black_box(chooser_array(black_box(black), &Bitboard::RANK_4, &Bitboard::RANK_5));
        });
    });
}

fn bench_chooser_wb(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    c.bench_function("chooser_wb_ref", |b| {
        b.iter(|| {
            black_box(chooser_wb(black_box(white), &Bitboard::RANK_4, &Bitboard::RANK_5));
            black_box(chooser_wb(black_box(black), &Bitboard::RANK_4, &Bitboard::RANK_5));
        });
    });
}

fn bench_chooser_struct(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    const CHOICE: Chooser<&Bitboard> = Chooser { white: &Bitboard::RANK_4, black: &Bitboard::RANK_5 };
    c.bench_function("chooser_struct", |b| {
        b.iter(|| {
            black_box(chooser_struct(black_box(white), &CHOICE));
            black_box(chooser_struct(black_box(black), &CHOICE));
        });
    });
}

fn bench_insufficient_material(c: &mut Criterion) {
    let mut group = c.benchmark_group("insufficient");
    let bd = &Catalog::starting_position();
    let m = Material::from_board(bd);
    group.bench_function("insufficient_material", |b| {
        b.iter(|| {
            black_box(black_box(m).is_insufficient());
        });
    });
    group.bench_function("insufficient_material2", |b| {
        b.iter(|| {
            black_box(Material::is_insufficient2(black_box(bd)));
        });
    });
    group.finish();
}

fn benchmark_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    group.sample_size(10);
    group.bench_function("alphabeta(5)", |b| {
        b.iter(|| {
            let board = Catalog::starting_position();
            let eval = SimpleScorer::new().set_position(false);
            let mut search = Algo::new().set_timing_method(TimeControl::Depth(5)).set_minmax(false).set_eval(eval);
            black_box(search.search(board));
        });
    });
    group.bench_function("minmax(5)", |b| {
        b.iter(|| {
            let board = Catalog::starting_position();
            let eval = SimpleScorer::new().set_position(false);
            let mut search = Algo::new().set_timing_method(TimeControl::Depth(5)).set_minmax(true).set_eval(eval);
            black_box(search.search(board));
        });
    });
    group.finish();
}

fn benchmark_mate_in_2(c: &mut Criterion) {
    let mut group = c.benchmark_group("mate2");
    group.sample_size(20);
    group.bench_function("mate_in_2_ab", |b| {
        b.iter(|| {
            let board = Catalog::mate_in_2()[0].board().clone();
            let eval = SimpleScorer::new().set_position(false);
            let mut search = Algo::new().set_timing_method(TimeControl::Depth(3)).set_minmax(false).set_eval(eval).set_iterative_deepening(false);
            black_box(search.search(black_box(board)));
            assert_eq!(search.pv.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        });
    });
    group.bench_function("mate_in_2_ab_iid", |b| {
        b.iter(|| {
            let board = Catalog::mate_in_2()[0].board().clone();
            let eval = SimpleScorer::new().set_position(false);
            let mut search = Algo::new().set_timing_method(TimeControl::Depth(3)).set_minmax(false).set_eval(eval).set_iterative_deepening(true);
            black_box(search.search(black_box(board)));
            assert_eq!(search.pv.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        });
    });
    group.finish();
}

fn benchmark_array(c: &mut Criterion) {
    let mut group = c.benchmark_group("array");
    let array = [1, 2];
    group.bench_function("int_index", |b| {
        b.iter(|| {
            black_box(black_box(array)[black_box(0)]);
            black_box(black_box(array)[black_box(1)]);
            black_box(black_box(array)[black_box(0)]);
            black_box(black_box(array)[black_box(1)]);
        });
    });
    group.bench_function("enum_index", |b| {
        b.iter(|| {
            black_box(black_box(array)[black_box(Color::White)]);
            black_box(black_box(array)[black_box(Color::Black)]);
            black_box(black_box(array)[black_box(Color::White)]);
            black_box(black_box(array)[black_box(Color::Black)]);
        });
    });
    group.finish();
}

fn benchmark_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval");
    let ef = &SimpleScorer::new();
    let ef_no_pos = &SimpleScorer::new().set_position(false);
 
    let bd = Catalog::white_starting_position();
    group.bench_function("material", |b| {
        b.iter(|| {
            black_box(bd.eval_material(black_box(ef)));
            black_box(bd.eval_material(black_box(ef)));
            black_box(bd.eval_material(black_box(ef)));
            black_box(bd.eval_material(black_box(ef)));
        });
    });
    group.bench_function("position", |b| {
        b.iter(|| {
            black_box(bd.eval_position(black_box(ef)));
            black_box(bd.eval_position(black_box(ef)));
            black_box(bd.eval_position(black_box(ef)));
            black_box(bd.eval_position(black_box(ef)));
        });
    });
    group.bench_function("all", |b| {
        b.iter(|| {
            black_box(bd.eval(black_box(ef)));
            black_box(bd.eval(black_box(ef)));
            black_box(bd.eval(black_box(ef)));
            black_box(bd.eval(black_box(ef)));
        });
    });
    group.bench_function("all_less_pos", |b| {
        b.iter(|| {
            black_box(bd.eval(black_box(ef_no_pos)));
            black_box(bd.eval(black_box(ef_no_pos)));
            black_box(bd.eval(black_box(ef_no_pos)));
            black_box(bd.eval(black_box(ef_no_pos)));
        });
    });
    group.finish();
}

fn bench_chooser_pvtable(c: &mut Criterion) {
    let mut pv_table = PvTable::new(MAX_PLY);
    c.bench_function("pv_table", |b| {
        b.iter(|| {
            for i in 1..7 {
                pv_table.set(i, black_box(&Move::new_null()));
                pv_table.propagate_from(i);

            }
        });
    });
}





criterion_group!(
    benches,
    bitwise_handcrafted,
    bitwise_bitflags,
    piece_to_upper_char,
    piece_to_char,
    benchmark_perft5,
    make_move,
    legal_moves,
    pseudo_legal_moves,
    bench_chooser_struct,
    bench_chooser_wb,
    bench_chooser_array,
    benchmark_score,
    benchmark_search,
    benchmark_array,
    bench_insufficient_material,
    benchmark_mate_in_2,
    benchmark_eval,
    bench_chooser_pvtable
);
criterion_main!(benches);
