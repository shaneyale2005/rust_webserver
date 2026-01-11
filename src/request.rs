use crate::{exception::Exception, param::*};

use log::error;

#[derive(Debug, Clone)]
pub struct Request {
    method: HttpRequestMethod,
    path: String,
    version: HttpVersion,
    user_agent: String,
    accept_encoding: Vec<HttpEncoding>,
    accept: Option<String>,
}

impl Request {
    pub fn try_from(buffer: &Vec<u8>, id: u128) -> Result<Self, Exception> {
        let request_string = match String::from_utf8(buffer.to_vec()) {
            Ok(string) => string,
            Err(_) => {
                error!("[ID{}]无法解析HTTP请求", id);
                return Err(Exception::RequestIsNotUtf8);
            }
        };

        let request_lines: Vec<&str> = request_string.split(CRLF).collect();

        let first_line: Vec<&str> = request_lines[0].split(" ").collect();
        let method_str = first_line[0].to_uppercase();
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
        let path = first_line[1].to_string();
        let version_str = first_line[2].to_uppercase();
        let version = match version_str.as_str() {
            r"HTTP/1.1" => HttpVersion::V1_1,
            _ => {
                error!("[ID{}]不支持的HTTP协议版本：{}", id, &version_str);
                return Err(Exception::UnsupportedHttpVersion);
            }
        };

        let mut user_agent = "".to_string();
        let mut accept_encoding = vec![];
        let mut accept = None;
        for line in &request_lines {
            if line.to_lowercase().starts_with("user-agent") {
                if let Some(val) = line.split(": ").nth(1) {
                    user_agent = val.to_string();
                }
            } else if line.to_lowercase().starts_with("accept:") {
                if let Some(val) = line.split(": ").nth(1) {
                    accept = Some(val.to_string());
                }
            }
        }

        for line in &request_lines {
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
        })
    }
}

impl Request {
    /// 返回请求的HTTP协议版本
    pub fn version(&self) -> &HttpVersion {
        &self.version
    }

    /// 返回当前Request的请求路径
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回请求的方法
    pub fn method(&self) -> HttpRequestMethod {
        self.method
    }

    /// 返回当前Request的User-Agent
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// 返回当前浏览器接受的压缩编码
    pub fn accept_encoding(&self) -> &Vec<HttpEncoding> {
        &self.accept_encoding
    }

    /// 返回 Accept 头
    pub fn accept(&self) -> Option<&String> {
        self.accept.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_parse_head_request() {
        let request_str =
            "HEAD /index.html HTTP/1.1\r\nHost: localhost:7878\r\nUser-Agent: Test-Agent\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Head);
        assert_eq!(request.path(), "/index.html");
    }

    #[test]
    fn test_parse_options_request() {
        let request_str = "OPTIONS * HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Options);
        assert_eq!(request.path(), "*");
    }

    #[test]
    fn test_parse_post_request() {
        let request_str =
            "POST /submit HTTP/1.1\r\nHost: localhost:7878\r\nContent-Length: 10\r\n\r\ntest=value";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Post);
        assert_eq!(request.path(), "/submit");
    }

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

    #[test]
    fn test_case_insensitive_headers() {
        let request_str = "GET / HTTP/1.1\r\nhost: localhost:7878\r\nuser-agent: Test\r\naccept-encoding: gzip\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.user_agent(), "Test");
        assert!(request.accept_encoding().contains(&HttpEncoding::Gzip));
    }

    #[test]
    fn test_no_encoding_header() {
        let request_str = "GET / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert!(request.accept_encoding().is_empty());
    }

    #[test]
    fn test_partial_encoding() {
        let request_str = "GET / HTTP/1.1\r\nHost: localhost:7878\r\nAccept-Encoding: gzip\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert!(request.accept_encoding().contains(&HttpEncoding::Gzip));
        assert!(!request.accept_encoding().contains(&HttpEncoding::Br));
        assert!(!request.accept_encoding().contains(&HttpEncoding::Deflate));
    }

    #[test]
    fn test_path_with_query_string() {
        let request_str = "GET /page?id=123&name=test HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.path(), "/page?id=123&name=test");
    }

    #[test]
    fn test_lowercase_method() {
        let request_str = "get / HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();

        let request = Request::try_from(&buffer, 0).unwrap();

        assert_eq!(request.method(), HttpRequestMethod::Get);
    }
}
