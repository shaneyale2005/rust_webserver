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
    range: Option<(u64, Option<u64>)>,
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

        let first_line_parts: Vec<&str> = request_lines[0].split(" ").collect();

        if first_line_parts.len() < 3 {
            error!("[ID{}]HTTP请求行格式不正确：{}", id, request_lines[0]);
            return Err(Exception::UnSupportedRequestMethod);
        }

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

        let version_str = first_line_parts.last().unwrap().to_uppercase();
        let version = match version_str.as_str() {
            "HTTP/1.1" => HttpVersion::V1_1,
            _ => {
                error!("[ID{}]不支持的HTTP协议版本：{}", id, &version_str);
                return Err(Exception::UnsupportedHttpVersion);
            }
        };

        let path = if first_line_parts.len() == 3 {
            first_line_parts[1].to_string()
        } else {
            first_line_parts[1..first_line_parts.len() - 1].join(" ")
        };

        let mut user_agent = "".to_string();
        let mut accept_encoding = vec![];
        let mut accept = None;
        let mut range = None;
        for line in &request_lines {
            if line.to_lowercase().starts_with("user-agent") {
                if let Some(val) = line.split(": ").nth(1) {
                    user_agent = val.to_string();
                }
            } else if line.to_lowercase().starts_with("accept:") {
                if let Some(val) = line.split(": ").nth(1) {
                    accept = Some(val.to_string());
                }
            } else if line.to_lowercase().starts_with("range:") {
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
            range,
        })
    }
}

impl Request {
    pub fn version(&self) -> &HttpVersion {
        &self.version
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn method(&self) -> HttpRequestMethod {
        self.method
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn accept_encoding(&self) -> &Vec<HttpEncoding> {
        &self.accept_encoding
    }

    pub fn accept(&self) -> Option<&String> {
        self.accept.as_ref()
    }

    pub fn range(&self) -> Option<(u64, Option<u64>)> {
        self.range
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
