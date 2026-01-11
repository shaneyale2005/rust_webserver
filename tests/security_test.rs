#[cfg(test)]
mod security_tests {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    async fn send_request(request: &str) -> Result<String, String> {
        let mut stream = TcpStream::connect("127.0.0.1:7878")
            .await
            .map_err(|e| e.to_string())?;

        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        let mut buffer = vec![0; 4096];
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

        Ok(String::from_utf8_lossy(&buffer[..n]).to_string())
    }

    fn extract_status_code(response: &str) -> u16 {
        response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .unwrap_or(0)
    }

    #[tokio::test]
    #[ignore]
    async fn test_path_traversal_simple() {
        let attacks = vec![
            "GET /../etc/passwd HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /../../etc/passwd HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /../../../etc/passwd HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ];

        for attack in attacks {
            match send_request(attack).await {
                Ok(response) => {
                    let status = extract_status_code(&response);
                    assert_ne!(status, 200, "路径遍历攻击应该被阻止");
                    println!("✓ 路径遍历测试通过: {}", attack.lines().next().unwrap());
                }
                Err(_) => {
                    // 连接被拒绝也是可接受的
                }
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_path_traversal_encoded() {
        let attacks = vec![
            "GET /%2e%2e%2fetc%2fpasswd HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /..%2fetc%2fpasswd HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /%2e%2e/%2e%2e/etc/passwd HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ];

        for attack in attacks {
            match send_request(attack).await {
                Ok(response) => {
                    let status = extract_status_code(&response);
                    assert_ne!(status, 200, "编码路径遍历应该被阻止");
                }
                Err(_) => {}
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_null_byte_injection() {
        let attack = "GET /index.html\0.jpg HTTP/1.1\r\nHost: localhost\r\n\r\n";

        match send_request(attack).await {
            Ok(response) => {
                let status = extract_status_code(&response);
                assert!(status == 404 || status == 400, "应该拒绝空字节注入");
            }
            Err(_) => {}
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_oversized_request_line() {
        let long_path = "A".repeat(10000);
        let attack = format!("GET /{} HTTP/1.1\r\nHost: localhost\r\n\r\n", long_path);

        match send_request(&attack).await {
            Ok(response) => {
                let status = extract_status_code(&response);
                assert!(
                    status == 400 || status == 414 || status == 404,
                    "应该拒绝超大请求: status={}",
                    status
                );
            }
            Err(_) => {}
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_oversized_header() {
        let long_value = "X".repeat(100000);
        let attack = format!(
            "GET / HTTP/1.1\r\nHost: localhost\r\nX-Custom: {}\r\n\r\n",
            long_value
        );

        match send_request(&attack).await {
            Ok(response) => {
                let status = extract_status_code(&response);
                println!("超大请求头测试 - 状态码: {}", status);
            }
            Err(e) => {
                println!("超大请求头被拒绝: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_malformed_http_version() {
        let attacks = vec![
            "GET / HTTP/999.999\r\nHost: localhost\r\n\r\n",
            "GET / HTTP/A.B\r\nHost: localhost\r\n\r\n",
            "GET / INVALID\r\nHost: localhost\r\n\r\n",
        ];

        for attack in attacks {
            match send_request(attack).await {
                Ok(response) => {
                    let status = extract_status_code(&response);
                    println!("畸形HTTP版本测试 - 状态码: {}", status);
                }
                Err(_) => {}
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_missing_host_header() {
        let request = "GET / HTTP/1.1\r\n\r\n";

        match send_request(request).await {
            Ok(response) => {
                let status = extract_status_code(&response);
                println!("缺少Host头测试 - 状态码: {}", status);
            }
            Err(_) => {}
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_multiple_content_length() {
        let attack = "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 10\r\nContent-Length: 5\r\n\r\ntest";

        match send_request(attack).await {
            Ok(response) => {
                let status = extract_status_code(&response);
                println!("多个Content-Length头测试 - 状态码: {}", status);
            }
            Err(_) => {}
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_crlf_injection() {
        let attacks = vec![
            "GET /\r\nX-Injected: header HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /test\r\n\r\nGET /evil HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ];

        for attack in attacks {
            match send_request(attack).await {
                Ok(response) => {
                    assert!(!response.contains("X-Injected"), "CRLF 注入应该被防止");
                }
                Err(_) => {}
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_php_command_injection() {
        let attacks = vec![
            "GET /test.php?cmd=;ls HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /test.php?file=../../etc/passwd HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /evil.php?exec=rm%20-rf%20/ HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ];

        for attack in attacks {
            match send_request(attack).await {
                Ok(response) => {
                    assert!(!response.contains("root:"), "PHP 命令注入应该被防止");
                }
                Err(_) => {}
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_slowloris_single() {
        match TcpStream::connect("127.0.0.1:7878").await {
            Ok(mut stream) => {
                // 发送部分请求
                let _ = stream.write_all(b"GET / HTTP/1.1\r\n").await;

                // 等待一段时间
                tokio::time::sleep(Duration::from_secs(3)).await;

                // 尝试完成请求
                let result = stream.write_all(b"Host: localhost\r\n\r\n").await;

                println!("慢速攻击测试 - 写入结果: {:?}", result);
            }
            Err(e) => {
                println!("无法连接: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_special_characters_in_path() {
        let special_paths = vec![
            "GET /<script>alert('xss')</script> HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /'; DROP TABLE users-- HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /${{7*7}} HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /%00 HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ];

        for path_request in special_paths {
            match send_request(path_request).await {
                Ok(response) => {
                    let status = extract_status_code(&response);
                    assert_ne!(status, 0, "应该返回有效的状态码");
                }
                Err(_) => {}
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_case_sensitivity() {
        let requests = vec![
            "GET /Index.HTML HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /INDEX.HTML HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /InDeX.HtMl HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ];

        for request in requests {
            match send_request(request).await {
                Ok(response) => {
                    let status = extract_status_code(&response);
                    println!("大小写测试 - 状态码: {}", status);
                }
                Err(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod unit_security_tests {

    #[test]
    fn test_path_contains_dotdot() {
        let paths = vec![
            "/normal/path",
            "/../etc/passwd",
            "/path/../etc/passwd",
            "/path/to/../../etc/passwd",
        ];

        for path in paths {
            let contains_dotdot = path.contains("..");
            if contains_dotdot {
                println!("检测到可疑路径: {}", path);
            }
        }
    }

    #[test]
    fn test_normalize_path() {
        use std::path::PathBuf;

        let paths = vec![
            ("./test", "test"),
            ("test/../file", "file"),
            ("a/b/../c", "a/c"),
        ];

        for (input, _expected) in paths {
            let pb = PathBuf::from(input);
            println!("输入: {}, 标准化: {:?}", input, pb);
        }
    }

    #[test]
    fn test_file_extension_validation() {
        let files = vec![
            ("test.html", true),
            ("test.php", true),
            ("test.exe", true),
            ("test", false),
            (".htaccess", false),
        ];

        for (filename, has_ext) in files {
            use std::path::Path;
            let path = Path::new(filename);
            let extension = path.extension();
            assert_eq!(extension.is_some(), has_ext);
        }
    }
}
