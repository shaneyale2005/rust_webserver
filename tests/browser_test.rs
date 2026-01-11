#[cfg(test)]
mod browser_tests {
    use std::io::Write;
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    const CONNECTION_TIMEOUT: Duration = Duration::from_secs(2);
    const READ_WRITE_TIMEOUT: Duration = Duration::from_secs(3);

    fn send_request(path: &str, accept_json: bool) -> Result<String, String> {
        thread::sleep(Duration::from_millis(50));

        let mut stream =
            TcpStream::connect_timeout(&"127.0.0.1:7878".parse().unwrap(), CONNECTION_TIMEOUT)
                .map_err(|e| format!("连接失败: {} - 请确保服务器在7878端口运行", e))?;

        stream
            .set_read_timeout(Some(READ_WRITE_TIMEOUT))
            .map_err(|e| format!("设置读取超时失败: {}", e))?;
        stream
            .set_write_timeout(Some(READ_WRITE_TIMEOUT))
            .map_err(|e| format!("设置写入超时失败: {}", e))?;

        let request = if accept_json {
            format!(
                "GET {} HTTP/1.1\r\nHost: localhost:7878\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
                path
            )
        } else {
            format!(
                "GET {} HTTP/1.1\r\nHost: localhost:7878\r\nConnection: close\r\n\r\n",
                path
            )
        };

        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("写入请求失败: {}", e))?;

        let mut response = String::new();
        use std::io::Read;
        stream
            .read_to_string(&mut response)
            .map_err(|e| format!("读取响应失败: {}", e))?;

        Ok(response)
    }

    fn is_json_response(response: &str) -> bool {
        response.contains("Content-Type: application/json")
            || response.contains("content-type: application/json")
    }

    fn is_html_response(response: &str) -> bool {
        response.contains("Content-Type: text/html")
            || response.contains("content-type: text/html")
            || response.contains("<html")
            || response.contains("<!DOCTYPE")
    }

    #[test]
    #[ignore] // 需要服务器运行才能测试
    fn test_browser_first_visit_root_returns_json() {
        println!("测试1：第一次访问根目录应返回JSON");
        let response = send_request("/", true).expect("请求失败");

        println!(
            "响应头部分：\n{}",
            response.lines().take(20).collect::<Vec<_>>().join("\n")
        );

        assert!(
            is_json_response(&response),
            "第一次访问根目录应该返回JSON响应"
        );
        assert!(
            !is_html_response(&response),
            "第一次访问根目录不应该返回HTML响应"
        );
    }

    #[test]
    #[ignore]
    fn test_browser_second_visit_subdirectory_returns_json() {
        println!("测试2：访问子目录（如/assets）应返回JSON");

        let _response1 = send_request("/", true).expect("第一次请求失败");
        println!("第一次访问根目录完成");

        thread::sleep(Duration::from_millis(100));
        let response2 = send_request("/assets", true).expect("第二次请求失败");

        println!(
            "第二次访问子目录响应头部分：\n{}",
            response2.lines().take(20).collect::<Vec<_>>().join("\n")
        );

        assert!(
            is_json_response(&response2),
            "第二次访问子目录应该返回JSON响应，而不是HTML。这是当前的bug！"
        );
        assert!(
            !is_html_response(&response2),
            "第二次访问子目录不应该返回HTML响应"
        );
    }

    #[test]
    #[ignore]
    fn test_browser_multiple_subdirectory_visits() {
        println!("测试3：多次访问不同子目录都应返回JSON");

        let paths = vec!["/", "/assets", "/browser", "/demo"];

        for path in paths {
            thread::sleep(Duration::from_millis(100));
            let response =
                send_request(path, true).unwrap_or_else(|e| panic!("访问 {} 失败: {}", path, e));

            println!("访问 {} 的响应：", path);
            println!(
                "{}",
                response.lines().take(15).collect::<Vec<_>>().join("\n")
            );

            assert!(
                is_json_response(&response),
                "访问路径 {} 应该返回JSON响应",
                path
            );
            assert!(
                !is_html_response(&response),
                "访问路径 {} 不应该返回HTML响应",
                path
            );
        }
    }

    #[test]
    #[ignore]
    fn test_browser_without_json_accept_header_returns_html() {
        println!("测试4：没有Accept: application/json头的请求应返回HTML");
        let response = send_request("/", false).expect("请求失败");

        println!(
            "响应头部分：\n{}",
            response.lines().take(20).collect::<Vec<_>>().join("\n")
        );

        assert!(
            is_html_response(&response),
            "没有Accept: application/json的请求应该返回HTML目录列表"
        );
    }
}
