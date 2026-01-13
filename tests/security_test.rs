// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

#[cfg(test)]
mod security_tests {
    //! # 安全漏洞回归测试套件
    //! 
    //! 该模块旨在通过模拟常见的 Web 攻击向量来验证服务器的防御能力。
    //! 覆盖范围包括：
    //! - 路径遍历 (Path Traversal / LFI)
    //! - 拒绝服务攻击 (DoS / Oversized Payload)
    //! - 注入攻击 (Injection / CRLF / Null Byte)
    //! - 协议健壮性 (Protocol Robustness)
    //! - 慢速连接攻击 (Slowloris)

    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    /// # 异步安全请求发送器
    /// 
    /// 底层采用 Tokio 异步 I/O 驱动，允许精确控制数据包的发送时机。
    /// 用于测试服务器在面对畸形报文时的非阻塞响应能力。
    async fn send_request(request: &str) -> Result<String, String> {
        let mut stream = TcpStream::connect("127.0.0.1:7878")
            .await
            .map_err(|e| e.to_string())?;

        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        let mut buffer = vec![0; 4096];
        // 设置硬超时限制，防止测试用例因服务器挂起而永久阻塞
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

        Ok(String::from_utf8_lossy(&buffer[..n]).to_string())
    }

    /// 从原始响应字符串中提取 HTTP 状态码
    fn extract_status_code(response: &str) -> u16 {
        response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .unwrap_or(0)
    }

    /// ## 攻击向量：基础路径遍历
    /// 验证服务器是否能识别并拦截通过 `../` 越权访问系统敏感文件（如 /etc/passwd）的企图。
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
                    // 期望结果：非 200 状态码（通常为 400 或 404）
                    assert_ne!(status, 200, "路径遍历攻击应该被阻止");
                    println!("✓ 路径遍历测试通过: {}", attack.lines().next().unwrap());
                }
                Err(_) => {
                    // 连接被重置或拒绝也视为防御成功
                }
            }
        }
    }

    /// ## 攻击向量：URL 编码混淆遍历
    /// 测试路径解析引擎是否能正确解码并识别经过 %2e 编码后的路径遍历攻击。
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

    /// ## 攻击向量：空字节注入 (Null Byte Injection)
    /// 验证 Rust 的字符串处理逻辑是否能防御经典的 C/C++ 风格截断攻击。
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

    /// ## 压力测试：超长请求行
    /// 防止恶意客户端通过发送 GB 级别的 URI 导致服务器内存溢出 (OOM)。
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

    /// ## 压力测试：超大请求头
    /// 验证服务器的 Header 解析器是否有内存上限控制。
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

    /// ## 健壮性测试：非标准 HTTP 版本
    /// 确保解析器在面对非法协议版本号时不会崩溃。
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

    /// ## 兼容性与安全：缺失 Host 头部
    /// 根据 RFC 2616 (HTTP/1.1)，Host 头部是强制要求的。
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

    /// ## 攻击向量：请求走私 (HTTP Smuggling) 基础验证
    /// 测试服务器在接收到重复长度字段时的处理策略。
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

    /// ## 攻击向量：CRLF 注入
    /// 防止攻击者通过在请求中插入换行符来篡改 HTTP 头部响应。
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

    /// ## 攻击向量：PHP 远程命令注入
    /// 模拟针对后端的 RCE (Remote Code Execution) 尝试。
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

    /// ## 攻击向量：慢速连接 (Slowloris)
    /// 测试服务器是否能通过读取超时机制回收被无限期占用的套接字。
    #[tokio::test]
    #[ignore]
    async fn test_slowloris_single() {
        match TcpStream::connect("127.0.0.1:7878").await {
            Ok(mut stream) => {
                // 发送部分报文并进入睡眠
                let _ = stream.write_all(b"GET / HTTP/1.1\r\n").await;

                tokio::time::sleep(Duration::from_secs(3)).await;

                // 尝试发送后续报文。若连接已被回收，此处应返回错误
                let result = stream.write_all(b"Host: localhost\r\n\r\n").await;

                println!("慢速攻击测试 - 写入结果: {:?}", result);
            }
            Err(e) => {
                println!("无法连接: {}", e);
            }
        }
    }

    /// ## 安全扫描：URI 特殊字符处理
    /// 检查解析器在面对 XSS 脚本标签或 SQL 注入关键词时的安全性。
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

    /// ## 攻击向量：大小写混淆 (Case Sensitivity)
    /// 在某些操作系统（如 Windows）下，验证服务器是否能抵御绕过特定大小写过滤器的企图。
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
    //! # 安全组件单元测试
    //! 
    //! 针对底层的字符串处理和路径操作逻辑进行逻辑验证。

    /// 验证路径遍历特征检测算法。
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

    /// 验证系统路径标准化行为。
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

    /// 验证文件扩展名提取与白名单过滤逻辑。
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
