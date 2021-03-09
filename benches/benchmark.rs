use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flouder::bitboard::*;
use flouder::catalog::*;
use flouder::board::makemove::*;
use flouder::board::movegen::*;
use flouder::board::*;
use flouder::types::*;

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
            black_box(perft(&board, black_box(5)));
        });
    });
    group.finish();
}


fn make_move(c: &mut Criterion) {
    let board = Catalog::starting_position();
    let mv1 = board.validate_uci_move("e2e4").unwrap();
    let mv2 = board.validate_uci_move("b1c3").unwrap();
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
    c.bench_function("chooser_array", |b| {
        b.iter(|| {
            let bb = black_box(chooser_array(black_box(white), &Bitboard::RANK_4, &Bitboard::RANK_5 ));
            let bb = black_box(chooser_array(black_box(black), &Bitboard::RANK_4, &Bitboard::RANK_5 ));
        });
    });
}

fn bench_chooser_wb(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    c.bench_function("chooser_wb_ref", |b| {
        b.iter(|| {
            let bb = black_box(chooser_wb(black_box(white), &Bitboard::RANK_4, &Bitboard::RANK_5 ));
            let bb = black_box(chooser_wb(black_box(black), &Bitboard::RANK_4, &Bitboard::RANK_5 ));
        });
    });
}

fn bench_chooser_struct(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    const choice: Chooser<&Bitboard> = Chooser { 
        white: &Bitboard::RANK_4, 
        black: &Bitboard::RANK_5,
    };
    c.bench_function("chooser_struct", |b| {
        b.iter(|| {
            let bb = black_box(chooser_struct(black_box(white), &choice));
            let bb = black_box(chooser_struct(black_box(black), &choice));
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
);
criterion_main!(benches);
