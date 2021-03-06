use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flouder::bitboard::*;
use flouder::board::*;
use flouder::board::movegen::*;
use flouder::board::makemove::*;
use flouder::board::catalog::*;

fn bitwise_handcrafted(c: &mut Criterion) {
    let n1 = 1u64 << 3;
    let n2 = 1u64 << 4;
    let n3 = 1u64 << 5;
    c.bench_function("bitwise_handcrafted", |b| {
        b.iter(|| {
            let a = black_box(n1) | black_box(n2);
            let b = a & black_box(n3) ;
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
            black_box(Piece::Pawn.to_char(Some(Color::WHITE)));
            black_box(Piece::Knight.to_char(Some(Color::BLACK)));
        });
    });
}

fn benchmark_perft(c: &mut Criterion) {
    let board = Catalog::starting_position();
    c.bench_function("perft", |b| {
        b.iter(|| {
            black_box(perft(&board, black_box(5)));
        });
    });
}

fn make_move(c: &mut Criterion) {
    let board = Catalog::starting_position();
    let mv1 = board.validate_uci_move("e2e4").unwrap();
    let mv2 = board.validate_uci_move("b1c3").unwrap();
    c.bench_function("makemove", |b| {
        b.iter(|| {
            black_box(board.make_move( black_box(&mv1)));
            black_box(board.make_move( black_box(&mv2)));
        });
    });
}



criterion_group!(benches, bitwise_handcrafted, bitwise_bitflags, piece_to_upper_char, piece_to_char,benchmark_perft,make_move);
criterion_main!(benches);
