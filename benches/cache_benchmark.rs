use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::SystemTime;

use webserver::cache::FileCache;

fn cache_push_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_push");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut cache = FileCache::from_capacity(size);
                let time = SystemTime::now();
                let content = Bytes::from("test content");

                for i in 0..size {
                    let filename = format!("file{}.txt", i);
                    cache.push(
                        black_box(&filename),
                        black_box(content.clone()),
                        black_box(time),
                    );
                }
            });
        });
    }

    group.finish();
}

fn cache_find_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_find");

    for size in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {

            let mut cache = FileCache::from_capacity(size);
            let time = SystemTime::now();
            let content = Bytes::from("test content");

            for i in 0..size {
                let filename = format!("file{}.txt", i);
                cache.push(&filename, content.clone(), time);
            }

            b.iter(|| {
                for i in 0..size {
                    let filename = format!("file{}.txt", i);
                    let _ = cache.find(black_box(&filename), black_box(time));
                }
            });
        });
    }

    group.finish();
}

fn cache_find_miss_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_find_miss");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut cache = FileCache::from_capacity(size);
            let time = SystemTime::now();
            let content = Bytes::from("test content");

            for i in 0..size {
                let filename = format!("file{}.txt", i);
                cache.push(&filename, content.clone(), time);
            }

            b.iter(|| {
                let _ = cache.find(black_box("nonexistent.txt"), black_box(time));
            });
        });
    }

    group.finish();
}

fn cache_eviction_benchmark(c: &mut Criterion) {
    c.bench_function("cache_eviction", |b| {
        b.iter(|| {
            let mut cache = FileCache::from_capacity(100);
            let time = SystemTime::now();
            let content = Bytes::from("test content");

            for i in 0..200 {
                let filename = format!("file{}.txt", i);
                cache.push(
                    black_box(&filename),
                    black_box(content.clone()),
                    black_box(time),
                );
            }
        });
    });
}

fn cache_time_invalidation_benchmark(c: &mut Criterion) {
    c.bench_function("cache_time_invalidation", |b| {
        let mut cache = FileCache::from_capacity(100);
        let time1 = SystemTime::now();
        let time2 = time1 + std::time::Duration::from_secs(1);
        let content = Bytes::from("test content");

        for i in 0..100 {
            let filename = format!("file{}.txt", i);
            cache.push(&filename, content.clone(), time1);
        }

        b.iter(|| {
            for i in 0..100 {
                let filename = format!("file{}.txt", i);
                let _ = cache.find(black_box(&filename), black_box(time2));
            }
        });
    });
}

fn cache_large_content_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_large_content");

    for content_size in [1024, 10240, 102400].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(content_size),
            content_size,
            |b, &content_size| {
                b.iter(|| {
                    let mut cache = FileCache::from_capacity(10);
                    let time = SystemTime::now();
                    let content = Bytes::from(vec![0u8; content_size]);

                    for i in 0..10 {
                        let filename = format!("file{}.txt", i);
                        cache.push(
                            black_box(&filename),
                            black_box(content.clone()),
                            black_box(time),
                        );
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    cache_push_benchmark,
    cache_find_benchmark,
    cache_find_miss_benchmark,
    cache_eviction_benchmark,
    cache_time_invalidation_benchmark,
    cache_large_content_benchmark
);
criterion_main!(benches);
