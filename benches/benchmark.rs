use std::ops::BitOr;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flounder::bitboard::Bitboard;

fn handcrafted_bit_manip(n: u64) -> bool {
    ((1 << n) | (1 << n)) & (1 << n) != 0
}

fn benchmark_handcrafted(c: &mut Criterion) {
    c.bench_function("handcrafted_bit_manip", |b| {
        b.iter(|| handcrafted_bit_manip(black_box(4)))
    });
}

fn bitboard_bit_manip(n: u64) -> bool {
    Bitboard::from_bits_truncate(n)
        .bitor(Bitboard::from_bits_truncate(n))
        .contains(Bitboard::from_bits_truncate(n))
}

fn benchmark_bitboard(c: &mut Criterion) {
    c.bench_function("bitboard_bit_manip", |b| {
        b.iter(|| bitboard_bit_manip(black_box(Bitboard::D1.bits())))
    });
}

criterion_group!(benches, benchmark_bitboard, benchmark_handcrafted);
criterion_main!(benches);
