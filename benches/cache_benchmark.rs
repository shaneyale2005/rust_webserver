// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # 文件缓存系统基准测试套件
//! 
//! 该模块利用 `criterion` 库对 `FileCache` 进行多维度的性能分析。
//! 核心评估指标包括：
//! - 吞吐量 (Throughput)：单位时间内处理的插入或查询请求数。
//! - 延迟 (Latency)：完成单次缓存操作所需的时间。
//! - 伸缩性 (Scalability)：随着数据规模增长，性能下降的曲线是否符合预期（如 O(1)）。
//! - 淘汰策略开销 (Eviction Overhead)：当触发缓存满额时的处理成本。

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::SystemTime;

use webserver::cache::FileCache;

/// ## 维度 1：缓存插入性能测试
/// 
/// 评估在不同预设容量下，连续推入新文件记录的开销。
/// 这里关注的是内存分配与哈希映射表的初始构建速度。
fn cache_push_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_push");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                // 每次迭代创建一个新缓存，以排除旧数据干扰
                let mut cache = FileCache::from_capacity(size);
                let time = SystemTime::now();
                let content = Bytes::from("test content");

                for i in 0..size {
                    let filename = format!("file{}.txt", i);
                    // 使用 black_box 确保编译器不会因为 filename 或 content 未被读取而优化掉 push 操作
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

/// ## 维度 2：热数据查询性能测试
/// 
/// 在缓存完全命中的理想情况下，验证查询操作的响应速度。
/// 理想结果应表现为稳定的 O(1) 复杂度。
fn cache_find_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_find");

    for size in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {

            // 环境初始化：预填充缓存数据
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
                    // 测试核心：衡量 find 逻辑及哈希检索耗时
                    let _ = cache.find(black_box(&filename), black_box(time));
                }
            });
        });
    }

    group.finish();
}

/// ## 维度 3：缓存失效（Miss）性能测试
/// 
/// 模拟冷数据请求，验证当 Key 不存在时，系统的快速失败能力。
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
                // 针对一个确定不存在的 Key 进行检索
                let _ = cache.find(black_box("nonexistent.txt"), black_box(time));
            });
        });
    }

    group.finish();
}

/// ## 维度 4：缓存淘汰策略压力测试
/// 
/// 模拟缓存溢出场景。容量为 100，写入 200 个条目。
/// 旨在观察 LRU 或 FIFO 淘汰算法在处理旧记录释放时的 CPU 密集程度。
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

/// ## 维度 5：时间一致性校验开销
/// 
/// 测试当文件系统修改时间 (mtime) 发生变化时，缓存自动失效逻辑的性能。
fn cache_time_invalidation_benchmark(c: &mut Criterion) {
    c.bench_function("cache_time_invalidation", |b| {
        let mut cache = FileCache::from_capacity(100);
        let time1 = SystemTime::now();
        // 模拟一秒后的新时间戳
        let time2 = time1 + std::time::Duration::from_secs(1);
        let content = Bytes::from("test content");

        for i in 0..100 {
            let filename = format!("file{}.txt", i);
            cache.push(&filename, content.clone(), time1);
        }

        b.iter(|| {
            for i in 0..100 {
                let filename = format!("file{}.txt", i);
                // 传入更新后的时间戳，触发缓存项的 Stale 校验逻辑
                let _ = cache.find(black_box(&filename), black_box(time2));
            }
        });
    });
}

/// ## 维度 6：大规模数据块影响分析
/// 
/// 评估 `Bytes` 克隆（ARC 引用计数增加）在面对大字节流时的表现。
/// 验证内容大小是否会对哈希表操作产生间接的内存缓存压力。
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
                    // 分配指定大小的零填充数据块
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

// 注册所有基准测试组
criterion_group!(
    benches,
    cache_push_benchmark,
    cache_find_benchmark,
    cache_find_miss_benchmark,
    cache_eviction_benchmark,
    cache_time_invalidation_benchmark,
    cache_large_content_benchmark
);

// 基准测试执行入口
criterion_main!(benches);