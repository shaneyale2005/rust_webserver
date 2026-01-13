// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # Exception 模块
//!
//! 该模块定义了 Web 服务器在请求处理生命周期中可能抛出的各类异常情况。
//! 
//! ## 设计意图
//! - **错误分类**：涵盖了协议解析错误、文件系统错误以及后端脚本（PHP）执行错误。
//! - **语义映射**：每个变体都对应了特定的业务逻辑，便于上层模块将其转化为对应的 HTTP 响应状态码。
//! - **用户友好**：通过实现 `std::fmt::Display`，确保错误信息可以被安全地记录到日志或返回给客户端。

use std::fmt;

/// 服务器处理请求过程中发生的异常类型。
///
/// 该枚举通常作为 `Result` 的 `Err` 部分返回，用于指示处理失败的具体原因。
#[derive(Debug, Copy, Clone)]
pub enum Exception {
    /// 客户端发送的请求字节流无法解析为合法的 UTF-8 字符串。
    /// 这通常发生在请求头或正文包含非法字符时。
    RequestIsNotUtf8,
    /// 客户端使用了服务器暂不支持的 HTTP 方法（例如：使用了非 GET/POST 方法）。
    UnSupportedRequestMethod,
    /// 客户端使用了服务器不支持的 HTTP 协议版本（例如：HTTP/0.9 或过高的版本）。
    UnsupportedHttpVersion,
    /// 在指定的资源根目录下未找到所请求的文件。在 Web 语义中对应 `404 Not Found`。
    FileNotFound,
    /// 请求的路径格式非法或包含越权尝试（如目录遍历攻击）。对应 `400 Bad Request`。
    InvalidPath,
    /// 调用 PHP 解释器执行脚本失败。通常是由于环境配置错误或二进制路径无效引起的。
    PHPExecuteFailed,
    /// PHP 脚本内部运行错误。代表脚本已启动但执行过程中崩溃，对应 `500 Internal Server Error`。
    PHPCodeError,
}

use Exception::*;

/// 为 `Exception` 实现 `Display` 特性，使其支持字符串格式化输出。
///
/// 工业实践中，这些描述信息常用于系统日志（Logging）以及发送给开发者的调试响应体中。
impl fmt::Display for Exception {
    /// 根据错误类型写入人类可读的描述文本。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestIsNotUtf8 => write!(f, "Request bytes can't be parsed in UTF-8"),
            UnSupportedRequestMethod => write!(f, "Unsupported request method"),
            UnsupportedHttpVersion => write!(f, "Unsupported HTTP version"),
            FileNotFound => write!(f, "File not found (404)"),
            InvalidPath => write!(f, "Invalid path (400)"),
            PHPExecuteFailed => write!(f, "Couldn't invoke PHP interpreter"),
            PHPCodeError => write!(f, "An error happened in php code"),
        }
    }
}