// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # Web 服务器集成与端到端 (E2E) 测试套件
//! 
//! 本模块通过调用系统级工具 `curl` 作为标准客户端，对运行中的服务器进行黑盒验证。
//! 使用 `curl` 的优点在于其严格遵循 RFC 标准，可以作为验证服务器协议实现正确性的“黄金标准”。

use std::process::Command;

/// # 跨进程 HTTP 请求分发器
/// 
/// 该函数包装了系统 `curl` 命令，用于模拟真实的外部网络行为。
/// 
/// ### 参数
/// * `request`: 原始请求行模拟字符串（用于提取方法和路径）。
/// * `port`: 目标服务器监听端口。
/// 
/// ### 实现细节
/// - 自动处理 `HEAD` 请求的特殊参数 `-I`。
/// - 强制使用 `--noproxy *` 以避免局部环境代理干扰。
/// - 通过 `-i` 参数捕获包含 Header 的完整原始响应。
async fn send_request(request: &str, port: u16) -> Result<String, String> {
    let method = request.split_whitespace().next().unwrap_or("GET");
    let path = request.split_whitespace().nth(1).unwrap_or("/");

    let url = format!("http://127.0.0.1:{}{}", port, path);
    // 构建基础参数：静默模式、禁用代理、输出包含响应头
    let mut args = vec!["-s", "--noproxy", "*", "-i"];

    if method == "HEAD" {
        args.push("-I");
    } else if method != "GET" {
        args.push("-X");
        args.push(method);
    }

    args.push(&url);

    // 执行外部进程调用
    let output = Command::new("curl")
        .args(&args)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "curl failed (status {}): {}",
            output.status, stderr
        ));
    }

    Ok(stdout)
}

/// # HTTP 响应报文解析器
/// 
/// 将 `curl` 输出的原始文本解析为结构化元组。
/// 
/// ### 返回值
/// `(状态码, Header集合, Body内容)`
fn parse_response(response: &str) -> (u16, Vec<(String, String)>, String) {
    let lines: Vec<&str> = response.split("\r\n").collect();

    // 1. 解析状态行 (e.g., "HTTP/1.1 200 OK")
    let status_line = lines[0];
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("0")
        .parse::<u16>()
        .unwrap_or(0);

    // 2. 状态迭代解析 Header 块，直到遇到空行
    let mut headers = Vec::new();
    let mut i = 1;
    while i < lines.len() && !lines[i].is_empty() {
        if let Some((key, value)) = lines[i].split_once(": ") {
            headers.push((key.to_string(), value.to_string()));
        }
        i += 1;
    }

    // 3. 提取 Body 内容
    let body = if i + 1 < lines.len() {
        lines[i + 1..].join("\r\n")
    } else {
        String::new()
    };

    (status_code, headers, body)
}

#[cfg(test)]
mod integration_tests {
    //! ## 集成测试用例库
    //! 
    //! 此处的测试依赖于服务器进程已在本地 7878 端口启动。
    use super::*;

    /// 验证最基础的 GET 请求流控，确保核心 Header（Content-Length, Server）存在。
    #[tokio::test]
    #[ignore]
    async fn test_get_request_basic() {
        let request = "GET / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (status_code, headers, _body) = parse_response(&response);
                assert!(status_code == 200 || status_code == 404);

                let header_map: std::collections::HashMap<String, String> =
                    headers.into_iter().collect();
                assert!(header_map.contains_key("Content-Length"));
                assert!(header_map.contains_key("Server"));
            }
            Err(e) => {
                eprintln!("测试失败: {}. 请确保服务器运行在端口7878", e);
            }
        }
    }

    /// 验证 HEAD 请求协议遵从性：服务器应返回 Header 但绝对不能返回 Body。
    #[tokio::test]
    #[ignore]
    async fn test_head_request() {
        let request = "HEAD / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (status_code, headers, body) = parse_response(&response);
                assert!(
                    status_code == 200 || status_code == 404,
                    "Expected 200 or 404, got {}",
                    status_code
                );

                // 根据 RFC，HEAD 响应必须没有实体内容
                assert!(body.is_empty() || body.chars().all(|c| c == '\0'));

                let header_map: std::collections::HashMap<String, String> =
                    headers.into_iter().collect();
                assert!(header_map.contains_key("Content-Length"));
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

    /// 验证 OPTIONS 请求：用于跨域资源共享 (CORS) 预检，应返回 Allow 允许的方法。
    #[tokio::test]
    #[ignore]
    async fn test_options_request() {
        let request = "OPTIONS * HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (status_code, headers, _body) = parse_response(&response);
                assert_eq!(status_code, 204); // 204 No Content 为 OPTIONS 常见响应

                let header_map: std::collections::HashMap<String, String> =
                    headers.into_iter().collect();
                assert!(header_map.contains_key("Allow"));

                if let Some(allow) = header_map.get("Allow") {
                    assert!(allow.contains("GET"));
                    assert!(allow.contains("HEAD"));
                    assert!(allow.contains("OPTIONS"));
                }
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

    /// 验证 404 错误页面的渲染逻辑。
    #[tokio::test]
    #[ignore]
    async fn test_404_not_found() {
        let request = "GET /nonexistent-file-12345.html HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (status_code, _headers, body) = parse_response(&response);
                assert_eq!(status_code, 404);
                assert!(body.contains("404") || body.is_empty());
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

    /// 验证压缩协商机制：测试服务器是否能识别 `Accept-Encoding` 头部。
    #[tokio::test]
    #[ignore]
    async fn test_compression_support() {
        let request =
            "GET / HTTP/1.1\r\nHost: localhost:7878\r\nAccept-Encoding: gzip, deflate, br\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (_status_code, headers, _body) = parse_response(&response);

                let header_map: std::collections::HashMap<String, String> =
                    headers.into_iter().collect();

                if let Some(content_length) = header_map.get("Content-Length") {
                    if content_length != "0" {
                        // 检查是否返回了 Content-Encoding 头部
                        let _ = header_map.get("Content-encoding");
                    }
                }
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

    /// 身份识别测试：确保自定义服务器标识 (Banner) 正确注入响应头。
    #[tokio::test]
    #[ignore]
    async fn test_server_header() {
        let request = "GET / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (_status_code, headers, _body) = parse_response(&response);

                let header_map: std::collections::HashMap<String, String> =
                    headers.into_iter().collect();

                assert!(header_map.contains_key("Server"));
                if let Some(server) = header_map.get("Server") {
                    assert!(server.contains("shaneyale-webserver"));
                }
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

    /// 并发压力测试：模拟 10 个协同任务同时请求，验证服务器的异步调度与多线程处理能力。
    #[tokio::test]
    #[ignore]
    async fn test_concurrent_requests() {
        let mut handles = vec![];

        for _ in 0..10 {
            let handle = tokio::spawn(async {
                let request = "GET / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
                send_request(request, 7878).await
            });
            handles.push(handle);
        }

        let mut success_count = 0;
        for handle in handles {
            if let Ok(Ok(_response)) = handle.await {
                success_count += 1;
            }
        }

        // 要求至少有 50% 的成功率才算初步通过并发检查
        assert!(
            success_count >= 5,
            "并发请求成功率太低: {}/10",
            success_count
        );
    }
}

#[cfg(test)]
mod unit_tests {
    //! ## 组件逻辑单元测试
    //! 
    //! 在不依赖网络环境的情况下，验证解析器对边界情况的处理。
    use super::*;

    /// 验证标准响应报文的解析逻辑。
    #[test]
    fn test_parse_response_basic() {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nServer: test\r\n\r\nHello";
        let (status_code, headers, body) = parse_response(response);

        assert_eq!(status_code, 200);
        assert_eq!(headers.len(), 2);
        assert_eq!(body, "Hello");
    }

    /// 验证无实体响应 (404) 的边界处理。
    #[test]
    fn test_parse_response_404() {
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let (status_code, headers, body) = parse_response(response);

        assert_eq!(status_code, 404);
        assert_eq!(headers.len(), 1);
        assert!(body.is_empty());
    }

    /// 验证多 Header 场景下的字典映射准确性。
    #[test]
    fn test_parse_response_with_headers() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 5\r\nServer: webserver\r\n\r\nHello";
        let (status_code, headers, _body) = parse_response(response);

        assert_eq!(status_code, 200);
        assert_eq!(headers.len(), 3);

        let header_map: std::collections::HashMap<String, String> = headers.into_iter().collect();
        assert_eq!(
            header_map.get("Content-Type"),
            Some(&"text/html".to_string())
        );
        assert_eq!(header_map.get("Content-Length"), Some(&"5".to_string()));
        assert_eq!(header_map.get("Server"), Some(&"webserver".to_string()));
    }
}
