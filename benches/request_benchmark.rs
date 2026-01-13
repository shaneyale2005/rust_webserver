// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # HTTP 请求解析性能基准测试
//! 
//! 本模块专注于评估 `Request::try_from` 解析器的性能边界。
//! 核心评估指标：
//! - **吞吐量 (Throughput)**: 每秒处理的请求报文数量。
//! - **内存开销 (Memory Overhead)**: 解析过程中产生的临时字符串分配频率。
//! - **协议复杂性响应**: 随着 Header 数量和 URI 长度增加，解析时间的线性增长情况（期望为 O(N)）。

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use webserver::request::Request;

/// ## 场景 1：极简请求解析 (Baseline)
/// 
/// 测量解析器在处理最基础的 HTTP/1.1 GET 请求时的基础耗时。
/// 该指标用于建立性能基准（Floor），排除了复杂 Header 带来的干扰。
fn simple_request_parse_benchmark(c: &mut Criterion) {
    let request = b"GET / HTTP/1.1\r\nHost: localhost:7878\r\nUser-Agent: Test\r\n\r\n";

    c.bench_function("simple_request_parse", |b| {
        b.iter(|| {
            // black_box 防止编译器优化掉整个解析过程
            let buffer = black_box(request.to_vec());
            let _ = Request::try_from(&buffer, 0).unwrap();
        });
    });
}

/// ## 场景 2：复杂请求解析 (Real-world Simulation)
/// 
/// 模拟现代浏览器发送的真实请求报文，包含长 URI、复杂 Query String 以及大量标准 Header。
/// 旨在观察解析器在处理多个 Header 映射及字符串切片时的性能退化情况。
fn complex_request_parse_benchmark(c: &mut Criterion) {
    let request = b"GET /path/to/resource?id=123&name=test HTTP/1.1\r\n\
                    Host: localhost:7878\r\n\
                    User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64)\r\n\
                    Accept: text/html,application/xhtml+xml\r\n\
                    Accept-Language: en-US,en;q=0.9\r\n\
                    Accept-Encoding: gzip, deflate, br\r\n\
                    Connection: keep-alive\r\n\
                    Upgrade-Insecure-Requests: 1\r\n\
                    \r\n";

    c.bench_function("complex_request_parse", |b| {
        b.iter(|| {
            let buffer = black_box(request.to_vec());
            let _ = Request::try_from(&buffer, 0).unwrap();
        });
    });
}

/// ## 场景 3：内容编码协商解析开销
/// 
/// 专注于 `Accept-Encoding` 等列表型 Header 的解析性能。
/// 验证解析器在处理逗号分隔的列表值时是否存在不必要的正则表达式调用或堆分配。
fn request_parse_with_encoding_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_parse_encoding");

    let requests = [
        (
            "no_encoding",
            b"GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: Test\r\n\r\n".as_slice(),
        ),
        (
            "gzip_only",
            b"GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: Test\r\nAccept-Encoding: gzip\r\n\r\n".as_slice(),
        ),
        (
            "all_encodings",
            b"GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: Test\r\nAccept-Encoding: gzip, deflate, br\r\n\r\n".as_slice(),
        ),
    ];

    for (name, request) in requests.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(name), request, |b, request| {
            b.iter(|| {
                let buffer = black_box(request.to_vec());
                let _ = Request::try_from(&buffer, 0).unwrap();
            });
        });
    }

    group.finish();
}

/// ## 场景 4：不同 HTTP 方法的路由匹配开销
/// 
/// 验证状态行（Status Line）解析器对不同动词长度（GET=3, OPTIONS=7）的敏感度。
/// 检查方法识别逻辑是否由于字符串匹配导致的性能抖动。
fn request_parse_different_methods_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_parse_methods");

    let requests = [
        (
            "GET",
            b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n".as_slice(),
        ),
        (
            "HEAD",
            b"HEAD / HTTP/1.1\r\nHost: localhost\r\n\r\n".as_slice(),
        ),
        (
            "POST",
            b"POST / HTTP/1.1\r\nHost: localhost\r\n\r\n".as_slice(),
        ),
        (
            "OPTIONS",
            b"OPTIONS * HTTP/1.1\r\nHost: localhost\r\n\r\n".as_slice(),
        ),
    ];

    for (method, request) in requests.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(method),
            request,
            |b, request| {
                b.iter(|| {
                    let buffer = black_box(request.to_vec());
                    let _ = Request::try_from(&buffer, 0).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// ## 场景 5：路径长度与复杂度压力测试
/// 
/// 评估 URI 路径深度及 Query 参数解析的性能曲线。
/// 长 URI 往往伴随着大量的内存拷贝，该测试可用于识别是否需要引入 `Cow` (Copy-on-Write) 优化。
fn request_parse_different_path_lengths_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_parse_path_length");

    let paths = [
        ("short", "/"),
        ("medium", "/path/to/resource"),
        ("long", "/very/long/path/to/some/resource/with/many/segments/and/a/query?param1=value1&param2=value2&param3=value3"),
    ];

    for (name, path) in paths.iter() {
        let request = format!("GET {} HTTP/1.1\r\nHost: localhost\r\n\r\n", path);
        group.bench_with_input(BenchmarkId::from_parameter(name), &request, |b, request| {
            b.iter(|| {
                let buffer = black_box(request.as_bytes().to_vec());
                let _ = Request::try_from(&buffer, 0).unwrap();
            });
        });
    }

    group.finish();
}

/// ## 场景 6：批处理解析吞吐量
/// 
/// 模拟高并发环境下的持续负载。
/// 用于观察 CPU L1/L2 缓存对解析器指令的热度影响，以及持续分配对 GC/内存管理器的压力。
fn request_parse_batch_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_parse_batch");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let request = b"GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: Test\r\nAccept-Encoding: gzip\r\n\r\n";

            b.iter(|| {
                for _ in 0..count {
                    let buffer = black_box(request.to_vec());
                    let _ = Request::try_from(&buffer, 0).unwrap();
                }
            });
        });
    }

    group.finish();
}

/// ## 场景 7：Header 大小写敏感性处理成本
/// 
/// HTTP 协议规定 Header Key 大小写不敏感。
/// 本测试旨在评估解析器在进行大小写规范化（Normalization）时付出的额外 CPU 周期。
/// 频繁的 `to_lowercase()` 调用通常是解析器的主要性能瓶颈。
fn request_case_insensitive_headers_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_case_insensitive");

    let requests = [
        ("lowercase", b"GET / HTTP/1.1\r\nhost: localhost\r\nuser-agent: Test\r\naccept-encoding: gzip\r\n\r\n".as_slice()),
        ("uppercase", b"GET / HTTP/1.1\r\nHOST: localhost\r\nUSER-AGENT: Test\r\nACCEPT-ENCODING: gzip\r\n\r\n".as_slice()),
        ("mixed", b"GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: Test\r\nAccept-Encoding: gzip\r\n\r\n".as_slice()),
    ];

    for (name, request) in requests.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(name), request, |b, request| {
            b.iter(|| {
                let buffer = black_box(request.to_vec());
                let _ = Request::try_from(&buffer, 0).unwrap();
            });
        });
    }

    group.finish();
}

// 注册请求解析相关的基准测试任务
criterion_group!(
    benches,
    simple_request_parse_benchmark,
    complex_request_parse_benchmark,
    request_parse_with_encoding_benchmark,
    request_parse_different_methods_benchmark,
    request_parse_different_path_lengths_benchmark,
    request_parse_batch_benchmark,
    request_case_insensitive_headers_benchmark
);

// 执行基准测试程序入口
criterion_main!(benches);