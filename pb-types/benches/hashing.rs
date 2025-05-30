use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn bench_hashing(c: &mut Criterion) {
    let datas: Vec<Vec<u8>> = [400, 3_200, 25_600, 204_800]
        .into_iter()
        .map(|size| vec![42u8; size])
        .collect();
    let mut group = c.benchmark_group("hashing");

    for data in datas {
        group.throughput(Throughput::Bytes(data.len() as u64));

        group.bench_with_input(BenchmarkId::new("blake3", data.len()), &data, |b, w| {
            b.iter(|| {
                let _ = std::hint::black_box(blake3::hash(w));
            })
        });

        group.bench_with_input(BenchmarkId::new("md5", data.len()), &data, |b, w| {
            b.iter(|| {
                let _ = std::hint::black_box(md5::compute(w));
            })
        });

        group.bench_with_input(BenchmarkId::new("sha256", data.len()), &data, |b, w| {
            b.iter(|| {
                use sha2::Digest;
                let _ = std::hint::black_box(sha2::Sha256::digest(w));
            })
        });

        group.bench_with_input(
            BenchmarkId::new("xxhash xxh3-64", data.len()),
            &data,
            |b, w| {
                b.iter(|| {
                    let _ = std::hint::black_box(xxhash_rust::xxh3::xxh3_64(w));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("xxhash xxh3-128", data.len()),
            &data,
            |b, w| {
                b.iter(|| {
                    let _ = std::hint::black_box(xxhash_rust::xxh3::xxh3_128(w));
                })
            },
        );
    }
}

criterion_group!(benches, bench_hashing);
criterion_main!(benches);
