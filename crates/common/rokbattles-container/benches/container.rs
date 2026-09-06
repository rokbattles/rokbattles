use std::{hint::black_box, time::Duration};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rokbattles_container::{Reader, schema, write_envelope};

#[cfg_attr(
    test,
    expect(unused_imports, reason = "Criterion does not run the included module’s unit tests")
)]
#[path = "../src/mask.rs"]
mod mask;

fn input(length: usize) -> Vec<u8> {
    (0..length).map(|index| (index.wrapping_mul(197).wrapping_add(101) & 255) as u8).collect()
}

fn benchmark_file(criterion: &mut Criterion, name: &str, file: &[u8]) {
    let reader = Reader::default();
    reader.decode(&mut file.to_vec()).expect("valid benchmark file");
    let mut group = criterion.benchmark_group(name);
    group.throughput(Throughput::Bytes(file.len() as u64));
    // Restoring the masked input is setup work; the decode benchmark includes its copy.
    group.bench_function("read_envelope", |bencher| {
        bencher.iter_batched_ref(
            || file.to_vec(),
            |bytes| {
                let envelope = reader.read_envelope(black_box(bytes)).expect("envelope");
                black_box(envelope.payload);
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("decode", |bencher| {
        bencher.iter(|| {
            let mut bytes = black_box(file).to_vec();
            black_box(reader.decode(&mut bytes).expect("decode"));
        });
    });
    group.finish();
}

fn benchmarks(criterion: &mut Criterion) {
    for size in [0, 31, 32, 33, 1_024, 64 * 1_024, 1_024 * 1_024] {
        let payload = input(size);
        let file = write_envelope(schema::BYTES, &payload, 42).expect("write bytes");
        benchmark_file(criterion, &format!("bytes/{size}"), &file);
        let mut group = criterion.benchmark_group("write_bytes");
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &payload, |bencher, bytes| {
            bencher.iter(|| {
                black_box(write_envelope(schema::BYTES, black_box(bytes), 42).expect("write"))
            });
        });
        group.finish();
    }

    let payload = input(64 * 1_024);
    let mut masked = payload.clone();
    mask::encode(&mut masked, 42);
    let mut group = criterion.benchmark_group("stages/65536");
    group.throughput(Throughput::Bytes(payload.len() as u64));
    group.bench_function("unmask", |bencher| {
        bencher.iter_batched_ref(
            || masked.clone(),
            |bytes| mask::decode(black_box(bytes), 42),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("crc32", |bencher| {
        bencher.iter(|| black_box(crc32fast::hash(black_box(&payload))));
    });
    group.finish();

    let text = "ROK Battles — éàø\0".repeat(2_048);
    benchmark_file(criterion, "text", &rokbattles_container::write_text(&text, 42).expect("text"));
    let json = serde_json::json!({"name": "ROK Battles", "coordinates": vec![[123, -456]; 2_048]});
    benchmark_file(criterion, "json", &rokbattles_container::write_json(&json, 42).expect("json"));

    #[cfg(feature = "schemas")]
    for (name, schema_id, payload) in territory_payloads() {
        let file = write_envelope(schema_id, &payload, 42).expect("territory file");
        benchmark_file(criterion, name, &file);
    }
    if let Some(path) = std::env::var_os("CONTAINER_BENCH_DIR") {
        let mut files = Vec::new();
        collect_files(std::path::Path::new(&path), &mut files);
        assert!(!files.is_empty(), "CONTAINER_BENCH_DIR must contain BIN files");
        let reader = Reader::default();
        for file in &files {
            reader.decode(&mut file.clone()).expect("valid directory fixture");
        }
        let mut group = criterion.benchmark_group("directory");
        group.throughput(Throughput::Bytes(files.iter().map(|file| file.len() as u64).sum()));
        group.bench_function("decode", |bencher| {
            bencher.iter(|| {
                for file in black_box(&files) {
                    black_box(reader.decode(&mut file.clone()).expect("decode"));
                }
            });
        });
        group.finish();
    }
    if let Some(path) = std::env::var_os("CONTAINER_BENCH_FILE") {
        let bytes = std::fs::read(path).expect("read CONTAINER_BENCH_FILE");
        benchmark_file(criterion, "file", &bytes);
    }
}

fn collect_files(path: &std::path::Path, files: &mut Vec<Vec<u8>>) {
    let mut entries: Vec<_> = std::fs::read_dir(path)
        .expect("read benchmark directory")
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "bin") {
            files.push(std::fs::read(path).expect("read benchmark file"));
        }
    }
}

#[cfg(feature = "schemas")]
fn uint(bytes: &mut Vec<u8>, mut value: u32) {
    while value >= 128 {
        bytes.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    bytes.push(value as u8);
}

#[cfg(feature = "schemas")]
fn territory_payloads() -> [(&'static str, u16, Vec<u8>); 3] {
    use rokbattles_container::schemas::territory::{
        MESH_DEFINITIONS, PROVINCE_GRID, SPATIAL_CHUNK,
    };

    let mut mesh = vec![10, 1, 1, 0];
    uint(&mut mesh, 8_192);
    for _ in 0..8_192 {
        mesh.extend_from_slice(&[2, 1]);
    }
    uint(&mut mesh, 8_190);
    for index in 0..8_190 {
        uint(&mut mesh, index);
    }

    let mut chunk = vec![10, 0, 0, 0];
    uint(&mut chunk, 1_024);
    for index in 0..1_024 {
        uint(&mut chunk, index);
        chunk.push(1);
        uint(&mut chunk, index * 20);
        uint(&mut chunk, index * 10);
    }
    chunk.extend_from_slice(&[0, 0]);

    let mut province = vec![1, 1, 10];
    uint(&mut province, 512);
    uint(&mut province, 512);
    province.extend_from_slice(&[1, 1, 1, 2]);
    uint(&mut province, 512);
    for row in 0_u32..512 {
        uint(&mut province, 512);
        province.push((row % 4) as u8);
    }
    [
        ("mesh", MESH_DEFINITIONS, mesh),
        ("chunk", SPATIAL_CHUNK, chunk),
        ("province", PROVINCE_GRID, province),
    ]
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30).warm_up_time(Duration::from_millis(500)).measurement_time(Duration::from_secs(1));
    targets = benchmarks
}
criterion_main!(benches);
