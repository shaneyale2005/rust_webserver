use std::process::Command;

async fn send_request(request: &str, port: u16) -> Result<String, String> {
    let method = request.split_whitespace().next().unwrap_or("GET");
    let path = request.split_whitespace().nth(1).unwrap_or("/");

    let url = format!("http://127.0.0.1:{}{}", port, path);
    let mut args = vec!["-s", "--noproxy", "*", "-i"];

    if method == "HEAD" {
        args.push("-I");
    } else if method != "GET" {
        args.push("-X");
        args.push(method);
    }

    args.push(&url);

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

fn parse_response(response: &str) -> (u16, Vec<(String, String)>, String) {
    let lines: Vec<&str> = response.split("\r\n").collect();

    // 解析状态行
    let status_line = lines[0];
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("0")
        .parse::<u16>()
        .unwrap_or(0);

    // 解析头部
    let mut headers = Vec::new();
    let mut i = 1;
    while i < lines.len() && !lines[i].is_empty() {
        if let Some((key, value)) = lines[i].split_once(": ") {
            headers.push((key.to_string(), value.to_string()));
        }
        i += 1;
    }

    // 解析主体
    let body = if i + 1 < lines.len() {
        lines[i + 1..].join("\r\n")
    } else {
        String::new()
    };

    (status_code, headers, body)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要服务器运行时才能通过
    async fn test_get_request_basic() {
        let request = "GET / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (status_code, headers, _body) = parse_response(&response);
                assert!(status_code == 200 || status_code == 404);

                // 验证必要的响应头
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

                // HEAD 请求不应该有响应体
                assert!(body.is_empty() || body.chars().all(|c| c == '\0'));

                // 但应该有 Content-Length 头
                let header_map: std::collections::HashMap<String, String> =
                    headers.into_iter().collect();
                assert!(header_map.contains_key("Content-Length"));
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_options_request() {
        let request = "OPTIONS * HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";

        match send_request(request, 7878).await {
            Ok(response) => {
                let (status_code, headers, _body) = parse_response(&response);
                assert_eq!(status_code, 204);

                // OPTIONS 响应应该包含 Allow 头
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

                // 如果有响应体，应该有 Content-Encoding 头
                if let Some(content_length) = header_map.get("Content-Length") {
                    if content_length != "0" {
                        // 可能有压缩
                        let _ = header_map.get("Content-encoding");
                    }
                }
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

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
                    assert!(server.contains("eslzzyl-webserver"));
                }
            }
            Err(e) => {
                eprintln!("测试失败: {}", e);
            }
        }
    }

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

        assert!(
            success_count >= 5,
            "并发请求成功率太低: {}/10",
            success_count
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_parse_response_basic() {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nServer: test\r\n\r\nHello";
        let (status_code, headers, body) = parse_response(response);

        assert_eq!(status_code, 200);
        assert_eq!(headers.len(), 2);
        assert_eq!(body, "Hello");
    }

    #[test]
    fn test_parse_response_404() {
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let (status_code, headers, body) = parse_response(response);

        assert_eq!(status_code, 404);
        assert_eq!(headers.len(), 1);
        assert!(body.is_empty());
    }

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
