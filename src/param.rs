// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # Web 服务器协议参数与常量模块
//!
//! 该模块定义了 `shaneyale-webserver` 遵循的 HTTP 协议相关常量和数据结构，包括：
//! - 常见的 HTTP 状态码及其原因短语（Reason Phrase）。
//! - 详尽的 MIME 类型映射表。
//! - HTTP 方法、版本及编码格式的强类型枚举。

use std::collections::HashMap;
use lazy_static::lazy_static;

/// 默认的首页 HTML 文件路径
pub const HTML_INDEX: &str = r"static/index.html";

/// 服务器名称标识，用于 HTTP 响应头的 `Server` 字段
pub const SERVER_NAME: &str = "shaneyale-webserver";

/// HTTP 协议规定的换行符（Carriage Return Line Feed）
pub const CRLF: &str = "\r\n";

lazy_static! {
    /// 服务器当前允许处理的 HTTP 方法列表。
    ///
    /// 用于在收到请求时进行初步过滤，不在该列表中的方法将触发 405 Method Not Allowed。
    pub static ref ALLOWED_METHODS: Vec<HttpRequestMethod> = {
        vec![
            HttpRequestMethod::Get,
            HttpRequestMethod::Head,
            HttpRequestMethod::Options,
        ]
    };
}

lazy_static! {
    /// HTTP 状态码与其对应的标准原因短语映射表。
    ///
    /// 参考标准：[RFC 9110: HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html)。
    pub static ref STATUS_CODES: HashMap<u16, &'static str> = {
        let mut map = HashMap::new();
        // 1xx: 信息响应 (Informational)
        map.insert(100, "Continue");
        map.insert(101, "Switching Protocols");
        
        // 2xx: 成功响应 (Successful)
        map.insert(200, "OK");
        map.insert(201, "Created");
        map.insert(202, "Accepted");
        map.insert(203, "Non-Authoritative Information");
        map.insert(204, "No Content");
        map.insert(205, "Reset Content");
        map.insert(206, "Partial Content");
        
        // 3xx: 重定向 (Redirection)
        map.insert(300, "Multiple Choices");
        map.insert(301, "Moved Permanently");
        map.insert(302, "Found");
        map.insert(303, "See Other");
        map.insert(304, "Not Modified");
        map.insert(305, "Use Proxy");
        // 306 已弃用 (Reserved)
        map.insert(307, "Temporary Redirect");
        map.insert(308, "Permanent Redirect");
        
        // 4xx: 客户端错误 (Client Error)
        map.insert(400, "Bad Request");
        map.insert(401, "Unauthorized");
        map.insert(402, "Payment Required");
        map.insert(403, "Forbidden");
        map.insert(404, "Not Found");
        map.insert(405, "Method Not Allowed");
        map.insert(406, "Not Acceptable");
        map.insert(407, "Proxy Authentication Required");
        map.insert(408, "Request Timeout");
        map.insert(409, "Conflict");
        map.insert(410, "Gone");
        map.insert(411, "Length Required");
        map.insert(412, "Precondition Failed");
        map.insert(413, "Content Too Large");
        map.insert(414, "URI Too Long");
        map.insert(415, "Unsupported Media Type");
        map.insert(416, "Range Not Satisfiable");
        map.insert(417, "Expectation Failed");
        map.insert(418, "I'm a teapot");
        map.insert(421, "Misdirected Request");
        map.insert(422, "Unprocessable Content");
        map.insert(426, "Upgrade Required");
        
        // 5xx: 服务端错误 (Server Error)
        map.insert(500, "Internal Server Error");
        map.insert(501, "Not Implemented");
        map.insert(502, "Bad Gateway");
        map.insert(503, "Service Unavailable");
        map.insert(504, "Gateway Timeout");
        map.insert(505, "HTTP Version Not Supported");
        map
    };
}

lazy_static! {
    /// 文件后缀名到 MIME 类型（Media Type）的映射表。
    ///
    /// 用于设置响应头中的 `Content-Type` 字段，确保浏览器能正确解析返回的文件流。
    pub static ref MIME_TYPES: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("aac", "audio/aac");
        map.insert("abw", "application/x-abiword");
        map.insert("apk", "application/vnd.android.package-archive");
        map.insert("arc", "application/x-freearc");
        map.insert("avi", "video/x-msvideo");
        map.insert("avif", "image/avif");
        map.insert("azw", "application/vnd.amazon.ebook");
        map.insert("bin", "application/octet-stream");
        map.insert("bmp", "image/bmp");
        map.insert("bz", "application/x-bzip");
        map.insert("bz2", "application/x-bzip2");
        map.insert("cab", "application/vnd.ms-cab-compressed");
        map.insert("cda", "application/x-cdf");
        map.insert("csh", "application/x-csh");
        map.insert("css", "text/css;charset=utf-8");
        map.insert("csv", "text/csv");
        map.insert("crx", "application/x-chrome-extension");
        map.insert("deb", "application/x-deb");
        map.insert("doc", "application/msword");
        map.insert(
            "docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        );
        map.insert("eot", "application/vnd.ms-fontobject");
        map.insert("epub", "application/epub+zip");
        map.insert("exe", "application/x-msdownload");
        map.insert("gif", "image/gif");
        map.insert("gz", "application/gzip");
        map.insert("htm", "text/html;charset=utf-8");
        map.insert("html", "text/html;charset=utf-8");
        map.insert("img", "application/x-iso9660-image");
        map.insert("ico", "image/x-icon");
        map.insert("ics", "text/calendar");
        map.insert("iso", "application/x-iso9660-image");
        map.insert("jar", "application/java-archive");
        map.insert("js", "text/javascript;charset=utf-8");
        map.insert("json", "application/json");
        map.insert("jsonld", "application/ld+json");
        map.insert("jpg", "image/jpeg");
        map.insert("jpeg", "image/jpeg");
        map.insert("mid", "audio/x-midi");
        map.insert("midi", "audio/x-midi");
        map.insert("mjs", "text/javascript");
        map.insert("mkv", "video/x-matroska");
        map.insert("mp3", "audio/mpeg");
        map.insert("mp4", "video/mp4");
        map.insert("mpeg", "video/mpeg");
        map.insert("mpkg", "application/vnd.apple.installer+xml");
        map.insert("msi", "application/x-msdownload");
        map.insert("odp", "application/vnd.oasis.opendocument.presentation");
        map.insert("ods", "application/vnd.oasis.opendocument.spreadsheet");
        map.insert("odt", "application/vnd.oasis.opendocument.text");
        map.insert("oga", "audio/ogg");
        map.insert("ogv", "video/ogg");
        map.insert("ogx", "application/ogg");
        map.insert("opus", "audio/opus");
        map.insert("otf", "font/otf");
        map.insert("pdf", "application/pdf");
        map.insert("png", "image/png");
        map.insert("php", "application/x-httpd-php");
        map.insert("ppt", "application/vnd.ms-powerpoint");
        map.insert(
            "pptx",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        );
        map.insert("rar", "application/x-rar-compressed");
        map.insert("rtf", "application/rtf");
        map.insert("rpm", "application/x-rpm");
        map.insert("sh", "application/x-sh");
        map.insert("svg", "image/svg+xml");
        map.insert("swf", "application/x-shockwave-flash");
        map.insert("tar", "application/x-tar");
        map.insert("tif", "image/tiff");
        map.insert("tiff", "image/tiff");
        map.insert("ts", "video/mp2t");
        map.insert("txt", "text/plain");
        map.insert("ttf", "font/ttf");
        map.insert("vsd", "application/vnd.visio");
        map.insert("wav", "audio/wav");
        map.insert("wasm", "application/wasm");
        map.insert("weba", "audio/webm");
        map.insert("webm", "video/webm");
        map.insert("webp", "image/webp");
        map.insert("woff", "font/woff");
        map.insert("woff2", "font/woff2");
        map.insert("xhtml", "application/xhtml+xml");
        map.insert("xls", "application/vnd.ms-excel");
        map.insert(
            "xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        );
        map.insert("xml", "text/xml");
        map.insert("xpi", "application/x-xpinstall");
        map.insert("xul", "application/vnd.mozilla.xul+xml");
        map.insert("zip", "application/zip");
        map.insert("7z", "application/x-7z-compressed");
        // 兜底类型（通常用于无法识别后缀的二进制流）
        map.insert("_", "application/octet-stream");
        map
    };
}

/// 支持的 HTTP 协议版本
#[derive(Debug, Clone, Copy)]
pub enum HttpVersion {
    /// HTTP/1.1 版本
    V1_1,
}

/// 标准 HTTP 请求方法
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpRequestMethod {
    /// 获取资源
    Get,
    /// 获取资源的元数据（不包含响应体）
    Head,
    /// 查询服务器支持的选项
    Options,
    /// 提交数据或执行操作
    Post,
}

/// 支持的内容编码（压缩）格式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpEncoding {
    /// GNU zip 压缩
    Gzip,
    /// zlib 压缩
    Deflate,
    /// Brotli 压缩
    Br,
}

use std::fmt;

impl fmt::Display for HttpVersion {
    /// 将枚举格式化为 HTTP 报文中的版本字符串
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            HttpVersion::V1_1 => write!(f, "1.1"),
        }
    }
}

impl fmt::Display for HttpRequestMethod {
    /// 将枚举格式化为 HTTP 标准大写方法名
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            HttpRequestMethod::Get => write!(f, "GET"),
            HttpRequestMethod::Head => write!(f, "HEAD"),
            HttpRequestMethod::Options => write!(f, "OPTIONS"),
            HttpRequestMethod::Post => write!(f, "POST"),
        }
    }
}

impl fmt::Display for HttpEncoding {
    /// 将枚举格式化为 `Content-Encoding` 头所使用的标识符
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            HttpEncoding::Gzip => write!(f, "gzip"),
            HttpEncoding::Deflate => write!(f, "deflate"),
            HttpEncoding::Br => write!(f, "br"),
        }
    }
}