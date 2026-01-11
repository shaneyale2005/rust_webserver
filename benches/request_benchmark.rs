use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

#[derive(Debug, Clone, Copy, PartialEq)]
enum HttpRequestMethod {
    Get,
    Head,
    Options,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HttpEncoding {
    Gzip,
    Deflate,
    Br,
}

#[derive(Debug, Clone)]
struct Request {
    #[allow(dead_code)]
    method: HttpRequestMethod,
    #[allow(dead_code)]
    path: String,
    #[allow(dead_code)]
    user_agent: String,
    #[allow(dead_code)]
    accept_encoding: Vec<HttpEncoding>,
}

impl Request {
    fn try_from(buffer: &[u8]) -> Result<Self, String> {
        let request_string = String::from_utf8(buffer.to_vec()).map_err(|_| "Invalid UTF-8")?;

        let request_lines: Vec<&str> = request_string.split("\r\n").collect();
        let first_line: Vec<&str> = request_lines[0].split(" ").collect();

        let method_str = first_line[0].to_uppercase();
        let method = match method_str.as_str() {
            "GET" => HttpRequestMethod::Get,
            "HEAD" => HttpRequestMethod::Head,
            "OPTIONS" => HttpRequestMethod::Options,
            "POST" => HttpRequestMethod::Post,
            _ => return Err("Unsupported method".to_string()),
        };

        let path = first_line[1].to_string();

        let mut user_agent = String::new();
        let mut accept_encoding = Vec::new();

        for line in &request_lines {
            if line.starts_with("user-agent") || line.starts_with("User-Agent") {
                user_agent = line.split(": ").collect::<Vec<&str>>()[1].to_string();
            }
            if line.starts_with("accept-encoding") || line.starts_with("Accept-Encoding") {
                let encoding = line.split(": ").collect::<Vec<&str>>()[1];
                if encoding.contains("gzip") {
                    accept_encoding.push(HttpEncoding::Gzip);
                }
                if encoding.contains("deflate") {
                    accept_encoding.push(HttpEncoding::Deflate);
                }
                if encoding.contains("br") {
                    accept_encoding.push(HttpEncoding::Br);
                }
            }
        }

        Ok(Self {
            method,
            path,
            user_agent,
            accept_encoding,
        })
    }
}

fn simple_request_parse_benchmark(c: &mut Criterion) {
    let request = b"GET / HTTP/1.1\r\nHost: localhost:7878\r\nUser-Agent: Test\r\n\r\n";

    c.bench_function("simple_request_parse", |b| {
        b.iter(|| {
            let _ = Request::try_from(black_box(request.as_slice()));
        });
    });
}

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
            let _ = Request::try_from(black_box(request.as_slice()));
        });
    });
}

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
                let _ = Request::try_from(black_box(request));
            });
        });
    }

    group.finish();
}

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
                    let _ = Request::try_from(black_box(request));
                });
            },
        );
    }

    group.finish();
}

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
                let _ = Request::try_from(black_box(request.as_bytes()));
            });
        });
    }

    group.finish();
}

fn request_parse_batch_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_parse_batch");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let request = b"GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: Test\r\nAccept-Encoding: gzip\r\n\r\n";

            b.iter(|| {
                for _ in 0..count {
                    let _ = Request::try_from(black_box(request.as_slice()));
                }
            });
        });
    }

    group.finish();
}

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
                let _ = Request::try_from(black_box(request));
            });
        });
    }

    group.finish();
}

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
criterion_main!(benches);
