use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

#[path = "../src/scalar.rs"]
mod scalar;

const SEED: u64 = 5_381;
const INPUT_SIZES: [usize; 6] = [8, 16, 64, 1_024, 64 * 1_024, 1_024 * 1_024];

fn checksum_byte_at_a_time(mut hash: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        hash = hash.wrapping_mul(33).wrapping_add(u64::from(byte));
    }
    hash
}

fn make_input(size: usize) -> Vec<u8> {
    (0..size).map(|index| (index.wrapping_mul(197).wrapping_add(101) & 0xff) as u8).collect()
}

fn runtime_backend() -> &'static str {
    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            "NEON"
        } else {
            "scalar (NEON unavailable)"
        }
    }

    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            "AVX2"
        } else {
            "scalar (AVX2 unavailable)"
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        "scalar (no SIMD implementation for this architecture)"
    }
}

fn benchmark_checksums(criterion: &mut Criterion) {
    println!("Runtime-dispatched backend: {}", runtime_backend());

    let mut group = criterion.benchmark_group("djb2");
    for size in INPUT_SIZES {
        let input = make_input(size);
        let expected = checksum_byte_at_a_time(SEED, &input);
        assert_eq!(scalar::checksum(SEED, &input), expected);
        assert_eq!(rokbattles_djb2_simd::checksum(SEED, &input), expected);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("byte_at_a_time", size),
            &input,
            |bencher, input| {
                bencher
                    .iter(|| black_box(checksum_byte_at_a_time(SEED, black_box(input.as_slice()))));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("scalar_four_at_a_time", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(scalar::checksum(SEED, black_box(input.as_slice()))));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("runtime_dispatch", size),
            &input,
            |bencher, input| {
                bencher.iter(|| {
                    black_box(rokbattles_djb2_simd::checksum(SEED, black_box(input.as_slice())))
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, benchmark_checksums);
criterion_main!(benches);
