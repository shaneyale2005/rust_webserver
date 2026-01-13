// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # HTTP 请求处理模块
//! 
//! 该模块是 Web 服务器的核心组件之一，负责将 TCP 流中读取的原始字节码
//! 解析为强类型的 `Request` 结构体。它涵盖了：
//! 1. 请求行（Request-Line）的解析（方法、路径、版本）。
//! 2. 常用 HTTP 标头（Headers）的提取。
//! 3. 范围请求（Range Requests）的解析。
//! 4. 内容协商（Content Negotiation）相关的编码解析。

use crate::{exception::Exception, param::*};
use log::error;

/// 表示一个完整的 HTTP 请求元数据。
/// 
/// 该结构体不包含请求体（Body）的大数据部分，主要用于路由分发和权限校验。
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP 请求方法（GET, POST 等）
    method: HttpRequestMethod,
    /// 请求的资源路径（包含查询字符串）
    path: String,
    /// HTTP 协议版本
    version: HttpVersion,
    /// 客户端标识字符串
    user_agent: String,
    /// 客户端支持的压缩编码列表（按解析顺序排列）
    accept_encoding: Vec<HttpEncoding>,
    /// 客户端接受的内容类型（MIME）
    accept: Option<String>,
    /// 范围请求参数：(起始字节, 结束字节)
    /// 其中结束字节为 `None` 表示请求从起始位置到文件末尾的所有数据。
    range: Option<(u64, Option<u64>)>,
}

impl Request {
    /// 从原始字节缓冲区尝试构建 `Request` 实例。
    /// 
    /// # 逻辑步骤
    /// 1. 验证编码：确保请求数据是合法的 UTF-8 字符串。
    /// 2. 解析请求行：提取方法、路径和协议版本。
    /// 3. 迭代解析标头：识别并解析 `User-Agent`, `Accept`, `Range` 等字段。
    /// 4. 解析编码：专门处理 `Accept-Encoding` 以支持后续的压缩传输。
    /// 
    /// # 参数
    /// * `buffer` - 从网络 Socket 读取的原始数据。
    /// * `id` - 全局请求 ID，用于在多线程环境下追踪日志。
    /// 
    /// # 错误处理
    /// 如果请求格式不符合 HTTP 规范或使用了不支持的方法/版本，将返回相应的 `Exception`。
    pub fn try_from(buffer: &Vec<u8>, id: u128) -> Result<Self, Exception> {
        // 1. 将字节流转换为字符串，失败则判定为非法的 HTTP 请求
        let request_string = match String::from_utf8(buffer.to_vec()) {
            Ok(string) => string,
            Err(_) => {
                error!("[ID{}]无法解析HTTP请求", id);
                return Err(Exception::RequestIsNotUtf8);
            }
        };

        let request_lines: Vec<&str> = request_string.split(CRLF).collect();

        // 2. 解析请求行 (e.g., "GET /index.html HTTP/1.1")
        let first_line_parts: Vec<&str> = request_lines[0].split(" ").collect();

        if first_line_parts.len() < 3 {
            error!("[ID{}]HTTP请求行格式不正确：{}", id, request_lines[0]);
            return Err(Exception::UnSupportedRequestMethod);
        }

        // 解析方法名
        let method_str = first_line_parts[0].to_uppercase();
        let method = match method_str.as_str() {
            "GET" => HttpRequestMethod::Get,
            "HEAD" => HttpRequestMethod::Head,
            "OPTIONS" => HttpRequestMethod::Options,
            "POST" => HttpRequestMethod::Post,
            _ => {
                error!("[ID{}]不支持的HTTP请求方法：{}", id, &method_str);
                return Err(Exception::UnSupportedRequestMethod);
            }
        };

        // 解析协议版本
        let version_str = first_line_parts.last().unwrap().to_uppercase();
        let version = match version_str.as_str() {
            "HTTP/1.1" => HttpVersion::V1_1,
            _ => {
                error!("[ID{}]不支持的HTTP协议版本：{}", id, &version_str);
                return Err(Exception::UnsupportedHttpVersion);
            }
        };

        // 解析路径（考虑到路径中可能包含空格的情况，虽然不规范但通过 join 尝试恢复）
        let path = if first_line_parts.len() == 3 {
            first_line_parts[1].to_string()
        } else {
            first_line_parts[1..first_line_parts.len() - 1].join(" ")
        };

        // 3. 迭代各行解析 Headers
        let mut user_agent = "".to_string();
        let mut accept_encoding = vec![];
        let mut accept = None;
        let mut range = None;
        for line in &request_lines {
            let line_lower = line.to_lowercase();
            // 处理 User-Agent
            if line_lower.starts_with("user-agent") {
                if let Some(val) = line.split(": ").nth(1) {
                    user_agent = val.to_string();
                }
            } 
            // 处理 Accept
            else if line_lower.starts_with("accept:") {
                if let Some(val) = line.split(": ").nth(1) {
                    accept = Some(val.to_string());
                }
            } 
            // 处理 Range 请求 (RFC 7233)
            // 格式示例: Range: bytes=0-1023
            else if line_lower.starts_with("range:") {
                if let Some(val) = line.split(": ").nth(1) {
                    if let Some(bytes_part) = val.strip_prefix("bytes=") {
                        let parts: Vec<&str> = bytes_part.split('-').collect();
                        if parts.len() == 2 {
                            let start = parts[0].parse::<u64>().ok();
                            let end = if parts[1].is_empty() {
                                None
                            } else {
                                parts[1].parse::<u64>().ok()
                            };
                            if let Some(s) = start {
                                range = Some((s, end));
                            }
                        }
                    }
                }
            }
        }

        // 4. 解析 Accept-Encoding 标头
        // 这里的逻辑比较简单，只要包含关键词即视为支持
        for line in &request_lines {
            if line.starts_with("accept-encoding") || line.starts_with("Accept-Encoding") {
                let parts: Vec<&str> = line.split(": ").collect();
                if parts.len() > 1 {
                    let encoding = parts[1];
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
                break;
            }
        }

        Ok(Self {
            method,
            path,
            version,
            user_agent,
            accept_encoding,
            accept,
            range,
        })
    }
}

// --- Getter 访向器实现 ---

impl Request {
    /// 获取 HTTP 协议版本
    pub fn version(&self) -> &HttpVersion {
        &self.version
    }

    /// 获取请求路径（含查询参数）
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 获取请求方法
    pub fn method(&self) -> HttpRequestMethod {
        self.method
    }

    /// 获取用户代理字符串
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// 获取客户端支持的压缩算法列表
    pub fn accept_encoding(&self) -> &Vec<HttpEncoding> {
        &self.accept_encoding
    }

    /// 获取客户端接受的文件 MIME 类型
    pub fn accept(&self) -> Option<&String> {
        self.accept.as_ref()
    }

    /// 获取 Range 请求的分片范围
    pub fn range(&self) -> Option<(u64, Option<u64>)> {
        self.range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证常规 GET 请求的解析，包括 Path 和 Headers
    #[test]
    fn test_parse_get_request() {
        let request_str = "GET / HTTP/1.1\r\nHost: localhost:7878\r\nUser-Agent: Test-Browser\r\nAccept-Encoding: gzip, deflate, br\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Get);
        assert_eq!(request.path(), "/");
        assert_eq!(request.user_agent(), "Test-Browser");
        assert!(request.accept_encoding().contains(&HttpEncoding::Gzip));
        assert!(request.accept_encoding().contains(&HttpEncoding::Deflate));
        assert!(request.accept_encoding().contains(&HttpEncoding::Br));
    }

    /// 验证 HEAD 请求的解析
    #[test]
    fn test_parse_head_request() {
        let request_str =
            "HEAD /index.html HTTP/1.1\r\nHost: localhost:7878\r\nUser-Agent: Test-Agent\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Head);
        assert_eq!(request.path(), "/index.html");
    }

    /// 验证 OPTIONS 请求（常用于 CORS 预检）
    #[test]
    fn test_parse_options_request() {
        let request_str = "OPTIONS * HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Options);
        assert_eq!(request.path(), "*");
    }

    /// 验证 POST 请求的基本行解析（目前暂不处理 Body 负载）
    #[test]
    fn test_parse_post_request() {
        let request_str =
            "POST /submit HTTP/1.1\r\nHost: localhost:7878\r\nContent-Length: 10\r\n\r\ntest=value";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Post);
        assert_eq!(request.path(), "/submit");
    }

    /// 确保不支持的 HTTP 方法（如 DELETE）会返回错误
    #[test]
    fn test_unsupported_method() {
        let request_str = "DELETE /resource HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let result = Request::try_from(&buffer, 0);

        assert!(result.is_err());
        match result.unwrap_err() {
            Exception::UnSupportedRequestMethod => {}
            _ => panic!("Expected UnSupportedRequestMethod error"),
        }
    }

    /// 确保不支持的版本（如 HTTP/2.0）被正确拒绝
    #[test]
    fn test_unsupported_http_version() {
        let request_str = "GET / HTTP/2.0\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let result = Request::try_from(&buffer, 0);

        assert!(result.is_err());
        match result.unwrap_err() {
            Exception::UnsupportedHttpVersion => {}
            _ => panic!("Expected UnsupportedHttpVersion error"),
        }
    }

    /// 验证 UTF-8 编码检查
    #[test]
    fn test_invalid_utf8() {
        let buffer = vec![0xFF, 0xFE, 0xFD];

        let result = Request::try_from(&buffer, 0);

        assert!(result.is_err());
        match result.unwrap_err() {
            Exception::RequestIsNotUtf8 => {}
            _ => panic!("Expected RequestIsNotUtf8 error"),
        }
    }

    /// 验证 Header 字段名是否大小写不敏感
    #[test]
    fn test_case_insensitive_headers() {
        let request_str = "GET / HTTP/1.1\r\nhost: localhost:7878\r\nuser-agent: Test\r\naccept-encoding: gzip\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.user_agent(), "Test");
        assert!(request.accept_encoding().contains(&HttpEncoding::Gzip));
    }

    /// 测试缺失编码标头时，解析列表应为空
    #[test]
    fn test_no_encoding_header() {
        let request_str = "GET / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert!(request.accept_encoding().is_empty());
    }

    /// 验证多编码协商的解析
    #[test]
    fn test_partial_encoding() {
        let request_str = "GET / HTTP/1.1\r\nHost: localhost:7878\r\nAccept-Encoding: gzip\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert!(request.accept_encoding().contains(&HttpEncoding::Gzip));
        assert!(!request.accept_encoding().contains(&HttpEncoding::Br));
        assert!(!request.accept_encoding().contains(&HttpEncoding::Deflate));
    }

    /// 确保带查询参数的路径能完整提取
    #[test]
    fn test_path_with_query_string() {
        let request_str = "GET /page?id=123&name=test HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.path(), "/page?id=123&name=test");
    }

    /// 验证请求方法的小写兼容性处理
    #[test]
    fn test_lowercase_method() {
        let request_str = "get / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Get);
    }
}
