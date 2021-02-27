use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flouder::bitboard::*;
use flouder::board::*;

fn bitwise_handcrafted(c: &mut Criterion) {
    c.bench_function("bitwise_handcrafted", |b| {
        b.iter(|| {
            let n1 = black_box(16);
            let n2 = black_box(32);
            let n3 = black_box(64);
            let a = (1 << n1) | (1 << n2);
            let b = a & (1 << n3) ;
            black_box(b);
        });
    });
}

fn bitwise_bitflags(c: &mut Criterion) {
    c.bench_function("bibitwise_bitflags", |b| {
        b.iter(|| {
            let n1 = black_box(Bitboard::D1.bits());
            let n2 = black_box(Bitboard::E1.bits());
            let n3 = black_box(Bitboard::F1.bits());
            let a = Bitboard::from_bits_truncate(n1) | Bitboard::from_bits_truncate(n2);
            let b = a & Bitboard::from_bits_truncate(n3);
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

criterion_group!(benches, bitwise_handcrafted, bitwise_bitflags, piece_to_upper_char, piece_to_char);
criterion_main!(benches);
