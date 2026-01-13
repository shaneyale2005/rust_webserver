// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! HTTP 响应处理模块
//!
//! 本模块负责构建和封装 HTTP 响应（Response）。
//! 它涵盖了从文件读取、目录列表生成、状态码响应构建、
//! 内容压缩（Gzip, Deflate, Brotli）、缓存交互以及 HTTP 报文序列化等功能。

use crate::{
    cache::FileCache,
    config::Config,
    param::*,
    request::Request,
    util::{format_file_size, handle_php, HtmlBuilder},
};

use brotli::enc::{self, backward_references::BrotliEncoderParams};
use bytes::Bytes;
use chrono::prelude::*;
use flate2::{
    write::{DeflateEncoder, GzEncoder},
    Compression,
};
use log::{debug, error, warn};

use std::{
    ffi::OsStr,
    fs::{self, metadata, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex},
};

/// 表示一个 HTTP 响应结构体。
///
/// 该结构体封装了发送给客户端的所有必要信息，包括状态行、响应头和响应体。
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP 版本（如 HTTP/1.1）
    version: HttpVersion,
    /// HTTP 状态码（如 200, 404）
    status_code: u16,
    /// 状态码对应的描述信息（如 "OK", "Not Found"）
    information: String,
    /// Content-Type 响应头
    content_type: Option<String>,
    /// Content-Length 响应头，表示内容长度
    content_length: u64,
    /// Date 响应头，表示响应生成时间
    date: DateTime<Utc>,
    /// Content-Encoding 响应头，表示使用的压缩算法
    content_encoding: Option<HttpEncoding>,
    /// Server 响应头，表示服务器名称
    server_name: String,
    /// Allow 响应头，用于 OPTIONS 请求或 405 错误
    allow: Option<Vec<HttpRequestMethod>>,
    /// 响应体内容（二进制数据）
    content: Option<Bytes>,
    /// Content-Range 响应头，用于断点续传
    content_range: Option<String>,
    /// Accept-Ranges 响应头，告知客户端服务器支持范围请求
    accept_ranges: Option<String>,
}

impl Response {
    /// 创建一个新的默认 Response 实例。
    ///
    /// 默认为 200 OK，HTTP/1.1，无内容。
    pub fn new() -> Self {
        Self {
            version: HttpVersion::V1_1,
            status_code: 200,
            information: "OK".to_string(),
            content_type: None,
            content_length: 0,
            date: Utc::now(),
            content_encoding: None,
            server_name: SERVER_NAME.to_string(),
            allow: Some(ALLOWED_METHODS.to_vec()),
            content: None,
            content_range: None,
            accept_ranges: None,
        }
    }

    /// 从文件系统构建响应。
    ///
    /// 此方法是处理静态文件请求的核心逻辑，包含以下功能：
    /// 1. 获取文件元数据。
    /// 2. 处理 HTTP Range 请求（断点续传/部分内容）。
    /// 3. 判断是否使用流式传输（大文件）。
    /// 4. 协商内容压缩（Gzip/Br/Deflate）。
    /// 5. 与内存缓存（FileCache）交互，处理缓存命中与更新。
    ///
    /// # 参数
    ///
    /// * `path` - 请求的文件路径。
    /// * `request` - 原始 HTTP 请求对象。
    /// * `id` - 请求 ID，用于日志追踪。
    /// * `cache` - 全局文件缓存。
    /// * `headonly` - 是否仅处理 HEAD 请求（不返回 body）。
    /// * `mime` - 文件的 MIME 类型。
    /// * `config` - 服务器配置。
    fn from_file(
        path: &str,
        request: &Request,
        id: u128,
        cache: &Arc<Mutex<FileCache>>,
        headonly: bool,
        mime: &str,
        config: &Config,
    ) -> Self {
        let accept_encoding = request.accept_encoding().to_vec();
        let mut response = Self::new();
        // 静态文件通常不需要 Allow 头，除非特定策略
        response.allow = None;

        let file_path = Path::new(path);
        
        // 1. 获取文件元数据
        let file_metadata = match metadata(file_path) {
            Ok(meta) => meta,
            Err(e) => {
                error!("[ID{}]无法获取文件{}的元数据: {}", id, path, e);
                panic!(); // 注意：此处 panic 会导致当前线程崩溃，生产环境建议返回 500 错误
            }
        };
        let file_size = file_metadata.len();
        let file_modified_time = match file_metadata.modified() {
            Ok(time) => time,
            Err(e) => {
                error!("[ID{}]无法获取文件{}的修改时间: {}", id, path, e);
                panic!();
            }
        };

        // 告知客户端支持 Range 请求
        if config.enable_range_requests() {
            response.accept_ranges = Some("bytes".to_string());
        }

        let range_request = request.range();
        
        // 判断是否触发流式传输逻辑：
        // 1. 文件大小超过阈值
        // 2. 或者这是一个 Range 请求
        let use_streaming = file_size > config.streaming_threshold() || range_request.is_some();
        
        debug!(
            "[ID{}]文件大小: {} bytes, 流式阈值: {} bytes, 使用流式传输: {}, Range请求: {:?}",
            id, file_size, config.streaming_threshold(), use_streaming, range_request
        );

        // 获取缓存锁，如果锁中毒则恢复
        let mut cache_lock = match cache.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                warn!("[ID{}]缓存锁被污染，恢复并继续", id);
                poisoned.into_inner()
            }
        };
        
        // 2. 处理 Range 请求 (HTTP 206 Partial Content)
        if let Some((start, end)) = range_request {
            let end = end.unwrap_or(file_size - 1);
            
            // 验证 Range 有效性
            if start >= file_size || end >= file_size || start > end {
                error!("[ID{}]无效的Range请求: start={}, end={}, file_size={}", id, start, end, file_size);
                response.set_code(416); // Range Not Satisfiable
                response.content_range = Some(format!("bytes */{}", file_size));
                response.content_length = 0;
                return response;
            }
            
            let content_length = end - start + 1;
            debug!("[ID{}]处理Range请求: bytes {}-{}/{} ({}字节)", 
                   id, start, end, file_size, content_length);
            
            response.set_code(206);
            response.content_range = Some(format!("bytes {}-{}/{}", start, end, file_size));
            response.content_type = Some(mime.to_string());
            response.content_length = content_length;
            
            // 对于 HEAD 请求，不读取实际内容
            if !headonly {
                let mut file = match File::open(path) {
                    Ok(f) => f,
                    Err(e) => {
                        error!("[ID{}]无法打开文件{}: {}", id, path, e);
                        panic!();
                    }
                };
                
                // 定位并读取指定范围
                if let Err(e) = file.seek(SeekFrom::Start(start)) {
                    error!("[ID{}]无法定位到文件位置{}: {}", id, start, e);
                    panic!();
                }
                
                let mut buffer = vec![0u8; content_length as usize];
                match file.read_exact(&mut buffer) {
                    Ok(_) => {
                        response.content = Some(Bytes::from(buffer));
                        debug!("[ID{}]Range内容读取成功", id);
                    }
                    Err(e) => {
                        error!("[ID{}]读取Range内容失败: {}", id, e);
                        panic!();
                    }
                }
            }
            
            return response;
        }
        
        // 3. 处理流式传输模式（非 Range 的大文件）
        // 如果启用流式传输且不是 HEAD 请求，则不在此处加载内容到内存
        // 内容将在 HTTP 响应写入阶段分块发送
        if use_streaming && !headonly {
            debug!("[ID{}]使用流式传输模式（文件将在write时分块发送）", id);
            response.content_type = Some(mime.to_string());
            response.content_length = file_size;
            response.content = None; // content 为 None 触发流式发送逻辑

            return response;
        }
        
        // 4. 压缩协商
        let skip_compression = should_skip_compression(mime);
        debug!(
            "[ID{}]文件类型: {}, 跳过压缩: {}",
            id, mime, skip_compression
        );
        
        response.content_encoding = match headonly {
            true => None,
            false => {
                if skip_compression {
                    debug!("[ID{}]跳过压缩，不设置编码", id);
                    None
                } else {
                    let encoding = decide_encoding(&accept_encoding);
                    debug!("[ID{}]决定使用编码: {:?}", id, encoding);
                    encoding
                }
            }
        };
        
        match response.content_encoding {
            Some(HttpEncoding::Gzip) => debug!("[ID{}]使用Gzip压缩编码", id),
            Some(HttpEncoding::Br) => debug!("[ID{}]使用Brotli压缩编码", id),
            Some(HttpEncoding::Deflate) => debug!("[ID{}]使用Deflate压缩编码", id),
            None => debug!("[ID{}]不进行压缩", id),
        };
        
        // 5. 缓存查找与处理
        match cache_lock.find(path, file_modified_time) {
            Some(bytes) => {
                // --- 缓存命中 ---
                debug!("[ID{}]缓存命中，原始大小: {} bytes", id, bytes.len());
                let mut contents = bytes.to_vec();
                let original_size = contents.len();

                // 如果需要压缩，对缓存的内容进行压缩
                // 注意：这里目前的实现是对缓存的原始数据进行实时压缩，
                // 也可以优化为缓存已压缩的数据。
                if response.content_encoding.is_some() {
                    debug!(
                        "[ID{}]对缓存内容进行压缩，编码方式: {:?}",
                        id, response.content_encoding
                    );
                    contents = match compress(contents, response.content_encoding) {
                        Ok(c) => c,
                        Err(e) => {
                            error!("[ID{}]压缩缓存内容失败: {}，返回未压缩内容", id, e);
                            response.content_encoding = None;
                            bytes.to_vec()
                        }
                    };
                    debug!(
                        "[ID{}]压缩完成，原始: {} bytes -> 压缩后: {} bytes, 压缩率: {:.1}%",
                        id,
                        original_size,
                        contents.len(),
                        (1.0 - contents.len() as f64 / original_size as f64) * 100.0
                    );
                }

                response.content_length = contents.len() as u64;
                response.content = match headonly {
                    true => None,
                    false => Some(Bytes::from(contents)),
                };
                let content_type_str = mime.to_string();
                debug!("[ID{}]Content-Type: {}", id, &content_type_str);
                response.content_type = Some(content_type_str);
            }
            None => {
                // --- 缓存未命中 ---
                debug!("[ID{}]缓存未命中或文件已修改", id);
                if headonly {
                    let path = Path::new(path);
                    let metadata = metadata(path).unwrap();
                    let content_type_str = mime.to_string();
                    debug!("[ID{}]Content-Type: {}", id, &content_type_str);
                    response.content_type = Some(content_type_str);
                    response.content = None;
                    response.content_length = metadata.len();
                } else {
                    debug!("[ID{}]读取文件: {}", id, path);
                    let mut file = match File::open(path) {
                        Ok(f) => f,
                        Err(e) => {
                            error!("[ID{}]无法打开路径{}指定的文件。错误：{}", id, path, e);
                            panic!();
                        }
                    };
                    let mut contents = Vec::new();
                    match file.read_to_end(&mut contents) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[ID{}]无法读取文件{}。错误：{}", id, path, e);
                            panic!();
                        }
                    }
                    let original_size = contents.len();
                    
                    // 压缩文件内容
                    debug!(
                        "[ID{}]开始压缩文件，原始大小: {} bytes, 编码方式: {:?}",
                        id, original_size, response.content_encoding
                    );
                    contents = match compress(contents, response.content_encoding) {
                        Ok(c) => c,
                        Err(e) => {
                            error!("[ID{}]压缩文件{}失败: {}，返回未压缩内容", id, path, e);
                            response.content_encoding = None;
                            // 压缩失败回退到读取原始文件（虽然上面 contents 已被所有权转移，这里重新读）
                            let mut file = File::open(path).unwrap();
                            let mut buf = Vec::new();
                            file.read_to_end(&mut buf).unwrap();
                            buf
                        }
                    };

                    response.content_length = contents.len() as u64;
                    debug!("[ID{}]Content-Length: {}", id, response.content_length);

                    let content_type_str = mime.to_string();
                    debug!("[ID{}]Content-Type: {}", id, &content_type_str);
                    response.content_type = Some(content_type_str);

                    response.content = Some(Bytes::from(contents.clone()));
                    
                    // 为了存入缓存，需要原始的未压缩数据
                    let original_contents = match response.content_encoding {
                        Some(_) => {
                            let mut file = File::open(path).unwrap();
                            let mut buf = Vec::new();
                            file.read_to_end(&mut buf).unwrap();
                            buf
                        }
                        None => contents,
                    };
                    
                    // 判断文件大小是否适合放入缓存
                    if FileCache::should_cache(file_size, config.streaming_threshold()) {
                        cache_lock.push(path, Bytes::from(original_contents), file_modified_time);
                        debug!("[ID{}]文件已加入缓存", id);
                    } else {
                        debug!("[ID{}]文件过大({} bytes)，跳过缓存", id, file_size);
                    }
                }
            }
        }
        response
    }

    /// 根据 HTTP 状态码创建响应。
    ///
    /// 自动生成常用错误代码（404, 405, 500）的 HTML 页面，并进行压缩。
    fn from_status_code(code: u16, accept_encoding: Vec<HttpEncoding>, id: u128) -> Self {
        let mut response = Self::new();
        response.content_encoding = decide_encoding(&accept_encoding);
        
        // 204 No Content 特殊处理
        if code == 204 {
            response.content = None;
            response.content_encoding = None;
            response.content_type = None;
            response.allow = Some(ALLOWED_METHODS.to_vec());
            response.set_code(code);
            return response;
        }
        
        response.allow = None;
        match response.content_encoding {
            Some(HttpEncoding::Gzip) => debug!("[ID{}]使用Gzip压缩编码", id),
            Some(HttpEncoding::Br) => debug!("[ID{}]使用Brotli压缩编码", id),
            Some(HttpEncoding::Deflate) => debug!("[ID{}]使用Deflate压缩编码", id),
            None => debug!("[ID{}]不进行压缩", id),
        };
        
        // 构建默认的错误页面 HTML
        let content = match code {
            404 => HtmlBuilder::from_status_code(404, Some(
                r"<h2>噢！</h2><p>你指定的网页无法找到。</p>"
            )),
            405 => HtmlBuilder::from_status_code(405, Some(
                r"<h2>噢！</h2><p>你的浏览器发出了一个非GET方法的HTTP请求。本服务器目前仅支持GET方法。</p>"
            )),
            500 => HtmlBuilder::from_status_code(500, Some(
                r"<h2>噢！</h2><p>服务器出现了一个内部错误。</p>"
            )),
            _ => HtmlBuilder::from_status_code(code, None),
        }.build();
        
        let content_compressed = compress(content.into_bytes(), response.content_encoding).unwrap();
        let bytes = Bytes::from(content_compressed);
        response.content_length = bytes.len() as u64;
        response.content = Some(bytes);
        response.content_type = Some("text/html;charset=utf-8".to_string());
        response.set_code(code);
        response
    }

    /// 处理目录请求，生成目录列表（HTML 或 JSON）。
    ///
    /// # 参数
    ///
    /// * `path` - 目录路径。
    /// * `is_json` - 是否请求 JSON 格式（通过 Accept 头判断）。
    fn from_dir(
        path: &str,
        accept_encoding: Vec<HttpEncoding>,
        id: u128,
        cache: &Arc<Mutex<FileCache>>,
        headonly: bool,
        is_json: bool,
    ) -> Self {
        debug!("[ID{}]from_dir: path={}, is_json={}", id, path, is_json);
        let mut response = Self::new();
        response.allow = None;
        response.content_encoding = match headonly {
            true => None,
            false => decide_encoding(&accept_encoding),
        };
        match response.content_encoding {
            Some(HttpEncoding::Gzip) => debug!("[ID{}]使用Gzip压缩编码", id),
            Some(HttpEncoding::Br) => debug!("[ID{}]使用Brotli压缩编码", id),
            Some(HttpEncoding::Deflate) => debug!("[ID{}]使用Deflate压缩编码", id),
            None => debug!("[ID{}]不进行压缩", id),
        };

        if !headonly {
            if is_json {
                debug!("[ID{}]设置Content-Type为application/json", id);
                response.content_type = Some("application/json".to_string());
            } else {
                debug!("[ID{}]设置Content-Type为text/html", id);
                response.content_type = Some("text/html;charset=utf-8".to_string());
            }
        } else {
            response.content_type = None;
        }

        let dir_path = Path::new(path);
        let dir_modified_time = match metadata(dir_path) {
            Ok(meta) => match meta.modified() {
                Ok(time) => time,
                Err(e) => {
                    error!("[ID{}]无法获取目录{}的修改时间: {}", id, path, e);
                    panic!();
                }
            },
            Err(e) => {
                error!("[ID{}]无法获取目录{}的元数据: {}", id, path, e);
                panic!();
            }
        };

        let mut cache_lock = match cache.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                warn!("[ID{}]缓存锁被污染，恢复并继续", id);
                poisoned.into_inner()
            }
        };

        // 区分 JSON 和 HTML 的缓存 Key
        let cache_key = if is_json {
            format!("{}:json", path)
        } else {
            path.to_string()
        };

        match cache_lock.find(&cache_key, dir_modified_time) {
            Some(bytes) => {
                // --- 缓存命中 ---
                debug!("[ID{}]缓存命中，原始大小: {} bytes", id, bytes.len());
                let mut content_data = bytes.to_vec();
                let original_size = content_data.len();

                if response.content_encoding.is_some() {
                    debug!(
                        "[ID{}]对缓存的目录内容进行厊缩，编码方式: {:?}",
                        id, response.content_encoding
                    );
                    content_data = match compress(content_data, response.content_encoding) {
                        Ok(c) => c,
                        Err(e) => {
                            error!("[ID{}]厊缩缓存的目录内容失败: {}，返回未厊缩内容", id, e);
                            response.content_encoding = None;
                            bytes.to_vec()
                        }
                    };
                    debug!(
                        "[ID{}]厊缩完成，原始: {} bytes -> 厊缩后: {} bytes, 厊缩率: {:.1}%",
                        id,
                        original_size,
                        content_data.len(),
                        (1.0 - content_data.len() as f64 / original_size as f64) * 100.0
                    );
                }

                response.content = match headonly {
                    true => None,
                    false => Some(Bytes::from(content_data.clone())),
                };
                response.content_length = content_data.len() as u64;
            }
            None => {
                // --- 缓存未命中，重新生成目录列表 ---
                debug!("[ID{}]缓存未命中或目录已修改", id);
                let mut dir_vec = Vec::<PathBuf>::new();
                let entries = fs::read_dir(path).unwrap();
                for entry in entries.into_iter() {
                    dir_vec.push(entry.unwrap().path());
                }

                // 根据请求类型生成 JSON 数据或 HTML 页面
                let content_bytes = if is_json {
                    let json_struct: Vec<_> = dir_vec
                        .iter()
                        .map(|p| {
                            let meta = fs::metadata(p).ok();
                            let is_dir = p.is_dir();
                            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                            let modified = meta
                                .as_ref()
                                .and_then(|m| m.modified().ok())
                                .map(|t| DateTime::<Utc>::from(t).to_rfc3339())
                                .unwrap_or_default();

                            let size_str = format_file_size(size);
                            serde_json::json!({
                                "name": p.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                                "type": if is_dir { "dir" } else { "file" },
                                "size": if is_dir { "-" } else { &size_str },
                                "raw_size": size,
                                "date": modified
                            })
                        })
                        .collect();
                    serde_json::to_vec(&json_struct).unwrap()
                } else {
                    let content = HtmlBuilder::from_dir(path, &mut dir_vec).build();
                    content.into_bytes()
                };

                debug!(
                    "[ID{}]开始压缩目录内容，原始大小: {} bytes",
                    id,
                    content_bytes.len()
                );
                let content_compressed =
                    match compress(content_bytes.clone(), response.content_encoding) {
                        Ok(c) => c,
                        Err(e) => {
                            error!("[ID{}]压缩目录{}内容失败: {}，返回未压缩内容", id, path, e);
                            response.content_encoding = None;
                            content_bytes.clone()
                        }
                    };
                response.content_length = content_compressed.len() as u64;
                response.content = match headonly {
                    true => None,
                    false => Some(Bytes::from(content_compressed.clone())),
                };

                // 更新缓存
                cache_lock.push(
                    &cache_key,
                    Bytes::from(content_bytes),
                    dir_modified_time,
                );
            }
        }
        response
    }

    /// 从 HTML 字符串直接构建响应（主要用于 PHP 处理结果）。
    fn from_html(
        html: &str,
        accept_encoding: Vec<HttpEncoding>,
        id: u128,
        headonly: bool,
    ) -> Response {
        let mut response = Self::new();
        response.allow = None;
        if headonly {
            response.content_encoding = None;
            response.content_type = None;
            response.content = None;
            return response;
        }
        response.content_encoding = decide_encoding(&accept_encoding);
        match response.content_encoding {
            Some(HttpEncoding::Gzip) => debug!("[ID{}]使用Gzip压缩编码", id),
            Some(HttpEncoding::Br) => debug!("[ID{}]使用Brotli压缩编码", id),
            Some(HttpEncoding::Deflate) => debug!("[ID{}]使用Deflate压缩编码", id),
            None => debug!("[ID{}]不进行压缩", id),
        };
        debug!("[ID{}]开始压缩HTML，原始大小: {} bytes", id, html.len());
        let content_compressed = match compress(Vec::from(html), response.content_encoding) {
            Ok(c) => c,
            Err(e) => {
                error!("[ID{}]压缩HTML失败: {}，返回未压缩内容", id, e);
                response.content_encoding = None;
                Vec::from(html)
            }
        };
        response.content_length = content_compressed.len() as u64;
        response.content_type = Some("text/html;charset=utf-8".to_string());
        response.content = Some(Bytes::from(content_compressed));
        response
    }

    // --- 构建者模式 Setter 方法 ---

    /// 设置响应日期为当前 UTC 时间。
    fn set_date(&mut self) -> &mut Self {
        self.date = Utc::now();
        self
    }

    /// 设置 HTTP 版本（默认 HTTP/1.1）。
    fn set_version(&mut self) -> &mut Self {
        self.version = HttpVersion::V1_1;
        self
    }

    /// 设置服务器名称头。
    fn set_server_name(&mut self) -> &mut Self {
        self.server_name = SERVER_NAME.to_string();
        self
    }

    /// 设置状态码，并自动更新对应的状态描述信息。
    fn set_code(&mut self, code: u16) -> &mut Self {
        self.status_code = code;
        self.information = match STATUS_CODES.get(&code) {
            Some(&debug) => debug.to_string(),
            None => {
                error!("非法的状态码：{}。这条错误说明代码编写出现了错误。", code);
                panic!();
            }
        };
        self
    }

    /// 静态工厂方法：构建 404 Not Found 响应。
    pub fn response_404(request: &Request, id: u128) -> Self {
        let accept_encoding = request.accept_encoding().to_vec();
        Self::from_status_code(404, accept_encoding, id)
            .set_date()
            .set_code(404)
            .set_version()
            .to_owned()
    }

    /// 静态工厂方法：构建 500 Internal Server Error 响应。
    pub fn response_500(request: &Request, id: u128) -> Self {
        let accept_encoding = request.accept_encoding().to_vec();
        Self::from_status_code(500, accept_encoding, id)
            .set_date()
            .set_code(500)
            .set_version()
            .to_owned()
    }

    /// 静态工厂方法：构建 400 Bad Request 响应。
    pub fn response_400(request: &Request, id: u128) -> Self {
        let accept_encoding = request.accept_encoding().to_vec();
        Self::from_status_code(400, accept_encoding, id)
            .set_date()
            .set_code(400)
            .set_version()
            .to_owned()
    }

    /// 处理请求的主入口函数。
    ///
    /// 根据请求的方法（Method）和路径（Path）分发到具体的处理逻辑（文件、目录、PHP 等）。
    pub fn from(
        path: &str,
        request: &Request,
        id: u128,
        cache: &Arc<Mutex<FileCache>>,
        config: &Config,
    ) -> Response {
        let accept_encoding = request.accept_encoding().to_vec();
        let method = request.method();
        let metadata_result = fs::metadata(path);

        // 验证请求方法是否支持
        if method != HttpRequestMethod::Get
            && method != HttpRequestMethod::Head
            && method != HttpRequestMethod::Options
        {
            return Self::from_status_code(405, accept_encoding, id)
                .set_date()
                .set_version()
                .set_server_name()
                .to_owned();
        }

        // 处理 OPTIONS 请求
        if method == HttpRequestMethod::Options {
            debug!("[ID{}]请求方法为OPTIONS", id);
            return Self::from_status_code(204, accept_encoding, id)
                .set_date()
                .set_version()
                .set_server_name()
                .to_owned();
        }

        let headonly = match method {
            HttpRequestMethod::Head => {
                debug!("[ID{}]请求方法为HEAD", id);
                true
            }
            _ => false,
        };

        match metadata_result {
            Ok(metadata) => {
                if metadata.is_dir() {
                    debug!("[ID{}]请求的路径是目录", id);
                    let is_json = request
                        .accept()
                        .map_or(false, |a| a.contains("application/json"));
                    Self::from_dir(path, accept_encoding, id, cache, headonly, is_json)
                        .set_date()
                        .set_code(200)
                        .set_version()
                        .set_server_name()
                        .to_owned()
                } else {
                    debug!("[ID{}]请求的路径是文件", id);
                    let extention = match Path::new(path).extension() {
                        Some(e) => e,
                        None => {
                            error!("[ID{}]无法确定请求路径{}的文件扩展名", id, path);
                            return Self::response_404(request, id);
                        }
                    };
                    debug!("[ID{}]文件扩展名: {}", id, extention.to_str().unwrap());
                    
                    // 特殊处理 PHP 文件
                    if extention == "php" {
                        debug!("[ID{}]请求的文件是PHP，启用PHP处理", id);
                        let html = match handle_php(path, id) {
                            Ok(html) => html,
                            Err(e) => {
                                error!("[ID{}]解析PHP文件{}时出错：{}", id, path, e);
                                return Self::response_500(request, id);
                            }
                        };
                        return Self::from_html(&html, accept_encoding, id, headonly)
                            .set_date()
                            .set_code(200)
                            .set_version()
                            .set_server_name()
                            .to_owned();
                    }
                    
                    // 处理普通静态文件
                    let mime = get_mime(extention);
                    debug!("[ID{}]MIME类型: {}", id, mime);
                    Self::from_file(path, request, id, cache, headonly, mime, config)
                        .set_date()
                        .set_code(200)
                        .set_version()
                        .set_server_name()
                        .to_owned()
                }
            }
            Err(_) => {
                warn!("[ID{}]无法获取{}的元数据，产生500 response", id, path);
                Self::response_500(request, id)
            }
        }
    }

    /// 将 Response 对象序列化为 HTTP 响应字节流。
    ///
    /// 包含状态行、Headers 和 Body。
    pub fn as_bytes(&self) -> Vec<u8> {
        if self.content == None && self.content_type == None {
            assert_eq!(self.content_encoding, None);
        }
        let version: &str = match self.version {
            HttpVersion::V1_1 => "HTTP/1.1",
        };
        let status_code: &str = &self.status_code.to_string();
        let information: &str = &self.information;
        let content_length: &str = &self.content_length.to_string();
        let date: &str = &format_date(&self.date);
        let server: &str = &self.server_name;

        // 手动构建 HTTP 头部字符串
        let header = [
            version,
            " ",
            status_code,
            " ",
            information,
            CRLF,
            match &self.content_type {
                Some(t) => ["Content-Type: ", &t, CRLF].concat(),
                None => "".to_string(),
            }
            .as_str(),
            match self.content_encoding {
                Some(e) => [
                    "Content-encoding: ",
                    match e {
                        HttpEncoding::Gzip => "gzip",
                        HttpEncoding::Deflate => "deflate",
                        HttpEncoding::Br => "br",
                    },
                    CRLF,
                ]
                .concat()
                .to_string(),
                None => "".to_string(),
            }
            .as_str(),
            "Content-Length: ",
            content_length,
            CRLF,
            "Date: ",
            date,
            CRLF,
            "Server: ",
            server,
            CRLF,
            match &self.allow {
                Some(a) => {
                    let mut allow_str = String::new();
                    for (index, method) in a.iter().enumerate() {
                        allow_str.push_str(&format!("{}", method));
                        if index < a.len() - 1 {
                            allow_str.push_str(", ");
                        }
                    }
                    ["Allow: ", &allow_str, CRLF].concat()
                }
                None => "".to_string(),
            }
            .as_str(),
            match &self.accept_ranges {
                Some(r) => ["Accept-Ranges: ", r, CRLF].concat(),
                None => "".to_string(),
            }
            .as_str(),
            match &self.content_range {
                Some(r) => ["Content-Range: ", r, CRLF].concat(),
                None => "".to_string(),
            }
            .as_str(),
            CRLF,
        ]
        .concat();
        
        // 拼接头部和内容
        [
            header.as_bytes(),
            match &self.content {
                Some(c) => &c,
                None => b"",
            },
        ]
        .concat()
    }
}

impl Response {
    /// 获取 HTTP 状态码。
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// 获取状态信息文本。
    pub fn information(&self) -> &str {
        &self.information
    }
    
    /// 判断是否为流式响应。
    ///
    /// 如果内容为空，但设置了 Content-Type 且 Content-Length > 0，则假定为流式发送。
    pub fn is_streaming(&self) -> bool {
        self.content.is_none() && self.content_type.is_some() && self.content_length > 0
    }
    
    /// 获取内容长度。
    pub fn get_content_length(&self) -> u64 {
        self.content_length
    }
}

/// 格式化日期为 HTTP Date 头所需的 RFC 2822 格式。
fn format_date(date: &DateTime<Utc>) -> String {
    date.to_rfc2822()
}

/// 压缩数据。
///
/// 支持 Gzip, Deflate, Brotli 算法。
///
/// # 参数
///
/// * `data` - 待压缩的原始字节数据。
/// * `mode` - 指定的压缩编码。
fn compress(data: Vec<u8>, mode: Option<HttpEncoding>) -> io::Result<Vec<u8>> {
    let original_size = data.len();
    let result = match mode {
        Some(HttpEncoding::Gzip) => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&data)?;
            encoder.finish()
        }
        Some(HttpEncoding::Deflate) => {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&data)?;
            encoder.finish()
        }
        Some(HttpEncoding::Br) => {
            let params = BrotliEncoderParams::default();
            let mut output = Vec::new();
            enc::BrotliCompress(&mut io::Cursor::new(data), &mut output, &params)?;
            Ok(output)
        }
        None => {
            Ok(data)
        }
    };

    if let Ok(ref compressed) = result {
        let compressed_size = compressed.len();
        let ratio = if original_size > 0 {
            ((original_size as i64 - compressed_size as i64) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };
        debug!(
            "压缩完成: {:?}, 原始大小: {} bytes, 压缩后: {} bytes, 压缩率: {:.1}%",
            mode, original_size, compressed_size, ratio
        );
    }

    result
}

/// 判断特定的 MIME 类型是否应该跳过压缩。
///
/// 对于已经是压缩格式的文件（如 zip, jpeg, mp4），再次压缩通常效果不佳且浪费 CPU。
fn should_skip_compression(mime_type: &str) -> bool {
    let skip_types = [
        "image/jpeg",
        "image/jpg",
        "image/png",
        "image/gif",
        "image/webp",
        "image/bmp",
        "image/x-icon",
        "video/",
        "audio/",
        "application/zip",
        "application/x-rar",
        "application/x-7z-compressed",
        "application/gzip",
        "application/x-gzip",
        "font/woff",
        "font/woff2",
        "application/vnd.ms-fontobject",
    ];

    skip_types
        .iter()
        .any(|&skip_type| mime_type.starts_with(skip_type))
}

/// 协商压缩编码。
///
/// 根据客户端的 Accept-Encoding 头选择合适的压缩算法。
/// 优先级：Gzip > Deflate (当前实现忽略了 Brotli 的优先级选择，除非只有 Brotli 也不支持)。
fn decide_encoding(accept_encoding: &Vec<HttpEncoding>) -> Option<HttpEncoding> {
    if accept_encoding.contains(&HttpEncoding::Gzip) {
        Some(HttpEncoding::Gzip)
    } else if accept_encoding.contains(&HttpEncoding::Deflate) {
        Some(HttpEncoding::Deflate)
    } else {
        None
    }
}

/// 根据文件扩展名获取 MIME 类型。
fn get_mime(extension: &OsStr) -> &str {
    let extension = match extension.to_str() {
        Some(e) => e,
        None => {
            error!("无法将&OsStr转换为&str类型");
            return "application/octet-stream";
        }
    };
    match MIME_TYPES.get(extension) {
        Some(v) => v,
        None => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_date() {
        let date = Utc::now();
        let formatted = format_date(&date);

        assert!(formatted.contains("+0000") || formatted.contains("GMT"));
    }

    #[test]
    fn test_compress_none() {
        let data = b"Hello, World!".to_vec();
        let result = compress(data.clone(), None).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_compress_gzip() {
        let data = b"Hello, World! This is a test string for compression.".to_vec();
        let result = compress(data.clone(), Some(HttpEncoding::Gzip)).unwrap();

        assert_ne!(result, data);
        assert_eq!(&result[0..2], &[0x1f, 0x8b]);
    }

    #[test]
    fn test_compress_deflate() {
        let data = b"Hello, World! This is a test string for compression.".to_vec();
        let result = compress(data.clone(), Some(HttpEncoding::Deflate)).unwrap();

        assert_ne!(result, data);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_compress_brotli() {
        let data = b"Hello, World! This is a test string for compression.".to_vec();
        let result = compress(data.clone(), Some(HttpEncoding::Br)).unwrap();

        assert_ne!(result, data);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_decide_encoding_gzip() {
        let encodings = vec![HttpEncoding::Gzip, HttpEncoding::Deflate];
        let result = decide_encoding(&encodings);
        assert_eq!(result, Some(HttpEncoding::Gzip));
    }

    #[test]
    fn test_decide_encoding_deflate_only() {
        let encodings = vec![HttpEncoding::Deflate];
        let result = decide_encoding(&encodings);
        assert_eq!(result, Some(HttpEncoding::Deflate));
    }

    #[test]
    fn test_decide_encoding_none() {
        let encodings = vec![];
        let result = decide_encoding(&encodings);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decide_encoding_br_ignored() {
        let encodings = vec![HttpEncoding::Br, HttpEncoding::Gzip];
        let result = decide_encoding(&encodings);
        assert_eq!(result, Some(HttpEncoding::Gzip));
    }

    #[test]
    fn test_get_mime_html() {
        let ext = OsStr::new("html");
        assert_eq!(get_mime(ext), "text/html;charset=utf-8");
    }

    #[test]
    fn test_get_mime_css() {
        let ext = OsStr::new("css");
        assert_eq!(get_mime(ext), "text/css;charset=utf-8");
    }

    #[test]
    fn test_get_mime_js() {
        let ext = OsStr::new("js");
        assert_eq!(get_mime(ext), "text/javascript;charset=utf-8");
    }

    #[test]
    fn test_get_mime_json() {
        let ext = OsStr::new("json");
        assert_eq!(get_mime(ext), "application/json");
    }

    #[test]
    fn test_get_mime_png() {
        let ext = OsStr::new("png");
        assert_eq!(get_mime(ext), "image/png");
    }

    #[test]
    fn test_get_mime_jpg() {
        let ext = OsStr::new("jpg");
        assert_eq!(get_mime(ext), "image/jpeg");
    }

    #[test]
    fn test_get_mime_unknown() {
        let ext = OsStr::new("unknown_extension");
        assert_eq!(get_mime(ext), "application/octet-stream");
    }

    #[test]
    fn test_get_mime_pdf() {
        let ext = OsStr::new("pdf");
        assert_eq!(get_mime(ext), "application/pdf");
    }

    #[test]
    fn test_response_new() {
        let response = Response::new();

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.information(), "OK");
        assert!(response.allow.is_some());
    }

    #[test]
    fn test_response_as_bytes_basic() {
        let response = Response::new();
        let bytes = response.as_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.starts_with("HTTP/1.1 200 OK"));
        assert!(response_str.contains("Content-Length: 0"));
        assert!(response_str.contains("Server: shaneyale-webserver"));
        assert!(response_str.contains("\r\n\r\n"));
    }

    #[test]
    fn test_response_as_bytes_with_content() {
        let mut response = Response::new();
        response.content = Some(Bytes::from("Hello"));
        response.content_length = 5;
        response.content_type = Some("text/plain".to_string());

        let bytes = response.as_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.contains("Content-Type: text/plain"));
        assert!(response_str.contains("Content-Length: 5"));
        assert!(response_str.ends_with("Hello"));
    }

    #[test]
    fn test_response_status_code_setter() {
        let mut response = Response::new();
        response.set_code(404);

        assert_eq!(response.status_code(), 404);
        assert_eq!(response.information(), "Not Found");
    }

    #[test]
    fn test_response_status_code_various() {
        for (code, expected_info) in [
            (200, "OK"),
            (201, "Created"),
            (204, "No Content"),
            (301, "Moved Permanently"),
            (400, "Bad Request"),
            (401, "Unauthorized"),
            (403, "Forbidden"),
            (404, "Not Found"),
            (500, "Internal Server Error"),
        ] {
            let mut response = Response::new();
            response.set_code(code);
            assert_eq!(response.status_code(), code);
            assert_eq!(response.information(), expected_info);
        }
    }

    #[test]
    fn test_response_with_gzip_encoding() {
        let mut response = Response::new();
        response.content_encoding = Some(HttpEncoding::Gzip);
        response.content = Some(Bytes::from("test"));
        response.content_length = 4;
        response.content_type = Some("text/plain".to_string());

        let bytes = response.as_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.contains("Content-encoding: gzip"));
    }

    #[test]
    fn test_response_with_allow_header() {
        let response = Response::new();
        let bytes = response.as_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.contains("Allow: GET, HEAD, OPTIONS"));
    }

    #[test]
    fn test_compress_empty_data() {
        let data = vec![];
        let result = compress(data.clone(), None).unwrap();
        assert_eq!(result, data);

        let result_gzip = compress(data, Some(HttpEncoding::Gzip)).unwrap();
        assert!(!result_gzip.is_empty());
    }

    #[test]
    fn test_compress_large_data() {
        let data = vec![b'A'; 10000];
        let result_gzip = compress(data.clone(), Some(HttpEncoding::Gzip)).unwrap();
        let result_deflate = compress(data.clone(), Some(HttpEncoding::Deflate)).unwrap();
        let result_br = compress(data.clone(), Some(HttpEncoding::Br)).unwrap();

        assert!(result_gzip.len() < data.len());
        assert!(result_deflate.len() < data.len());
        assert!(result_br.len() < data.len());
    }

    #[test]
    fn test_response_date_format() {
        let response = Response::new();
        let bytes = response.as_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.contains("Date: "));
    }

    #[test]
    fn test_head_request_response() {
        use crate::cache::FileCache;
        use crate::config::Config;
        use std::sync::{Arc, Mutex};

        let request_str = "HEAD /index.html HTTP/1.1\r\nHost: localhost:7878\r\n\r\n";
        let buffer = request_str.as_bytes().to_vec();
        let request = Request::try_from(&buffer, 1).unwrap();

        let cache = Arc::new(Mutex::new(FileCache::from_capacity(10)));
        let config = Config::new();

        let response = Response::from("static/index.html", &request, 1, &cache, &config);
        let bytes = response.as_bytes();

        let response_str = String::from_utf8_lossy(&bytes);
        assert!(response_str.starts_with("HTTP/1.1 200 OK"));
        assert!(response_str.contains("Content-Length:"));
        assert!(response_str.contains("Server: shaneyale-webserver"));

        assert!(!response_str.contains("<!DOCTYPE html>"));
    }
}
