// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # Web Server 核心库
//!
//! 该库提供了一套完整的 HTTP 处理框架，涵盖了从底层请求解析到高层响应构建的核心功能。
//! 
//! ## 模块架构设计
//! 
//! 本项目采用了模块化的设计思路，各组件职责明确：
//! - **请求处理**: `request` 与 `param` 模块负责解析与验证。
//! - **响应构建**: `response` 与 `util` 模块负责生成输出。
//! - **性能优化**: `cache` 模块提供基于内存的快速文件检索。
//! - **配置与异常**: `config` 与 `exception` 模块确保系统的可配置性与健壮性。
//!
//! ## 快捷导出 (Public API)
//!
//! 为了简化调用方的使用，本项目通过 `pub use` 将核心类型重定向至根命名空间，
//! 开发者可以直接通过 `crate::Request` 或 `crate::Response` 进行调用，而无需关心内部路径。

/// 内部缓存实现模块，支持过期验证。
pub mod cache;
/// 配置管理模块，支持 TOML 解析。
pub mod config;
/// 全局异常与错误类型定义模块。
pub mod exception;
/// HTTP 协议相关的参数定义（方法、版本、编码）。
pub mod param;
/// HTTP 请求对象的定义与解析逻辑。
pub mod request;
/// HTTP 响应对象的构建与序列化。
pub mod response;
/// 通用辅助工具，包含 HTML 模板构建器等。
pub mod util;

// --- 统一对外的公共接口 (Facade Pattern) ---

/// 重定向导出 `FileCache`：提供高效的文件内存缓存。
pub use cache::FileCache;

/// 重定向导出 `Exception`：统一的错误处理枚举。
pub use exception::Exception;

/// 重定向导出基础协议参数。
pub use param::{HttpEncoding, HttpRequestMethod, HttpVersion};

/// 重定向导出 `Request`：代表一个解析后的客户端请求。
pub use request::Request;

/// 重定向导出 `Response`：用于构造发送回客户端的响应。
pub use response::Response;

/// 重定向导出 `HtmlBuilder`：支持链式调用的 HTML 生成工具。
pub use util::HtmlBuilder;