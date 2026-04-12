use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use qpack::VarInt;

fn bench_var_int_encoding(c: &mut Criterion) {
    const ENCODE_TESTS: [(VarInt, u8); 3] = [
        (VarInt::new(10), 5),
        (VarInt::new(1337), 5),
        (VarInt::new(42), 8),
    ];

    let mut b = c.benchmark_group("Encode VarInt");

    for (int, prefix) in ENCODE_TESTS {
        b.bench_with_input(
            BenchmarkId::from_parameter(int),
            &(int, prefix),
            |b, &(int, prefix)| b.iter(|| black_box(int).encode(black_box(prefix))),
        );
    }
}

fn bench_var_int_decoding(c: &mut Criterion) {
    const DECODE_TESTS: [(u64, &[u8], u8); 3] = [
        (10, &[0b01010], 5),
        (1337, &[0b11111, 0b10011010, 0b00001010], 5),
        (42, &[0b101010], 8),
    ];

    let mut b = c.benchmark_group("Decode VarInt");

    for (int, bytes, prefix) in DECODE_TESTS {
        b.bench_with_input(
            BenchmarkId::from_parameter(int),
            &(bytes, prefix),
            |b, &(bytes, prefix)| b.iter(|| VarInt::decode(black_box(bytes), black_box(prefix))),
        );
    }
}

criterion_group!(benches, bench_var_int_encoding, bench_var_int_decoding);
criterion_main!(benches);
