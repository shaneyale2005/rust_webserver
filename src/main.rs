// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # 异步 Web 服务器
//! 
//! 该模块实现了基于 Tokio 运行时的高性能多线程 Web 服务器。
//! 核心功能包括：
//! - 基于 LRU 或类似机制的文件缓存系统
//! - 支持多线程异步 I/O 处理
//! - 动态 PHP 解释器探测
//! - 灵活的路由系统（支持静态资源、JSON API 以及 SPA 路由）
//! - 流式大文件传输协议（Chunked Transfer 模拟）
//! - 后台管理控制台（CLI 指令交互）

#![allow(clippy::unused_io_amount)]

// --- 模块定义 ---
mod cache;      // 高效文件缓存实现
mod config;     // 配置解析与管理
mod exception;  // 自定义异常与错误处理
mod param;      // 全局常量与静态参数
mod request;    // HTTP 请求报文解析器
mod response;   // HTTP 响应报文构建器
mod util;       // 通用工具函数

use cache::FileCache;
use config::Config;
use request::Request;
use response::Response;

use log::{debug, error, info, warn};
use log4rs;
use regex::Regex;
use tokio::{
    fs::File as TokioFile,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    runtime::Builder,
};

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{exception::Exception, param::HTML_INDEX};

/// # 程序入口点
/// 
/// 初始化系统环境、加载配置、探测外部依赖并启动主事件循环。
#[tokio::main]
async fn main() {
    // 1. 初始化日志系统：采用 log4rs 异步日志架构，通过外部 YAML 灵活配置级别与输出目的地
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    // 2. 环境配置加载：从 TOML 文件读取运行参数
    let config = Config::from_toml("config/development.toml");
    info!("配置文件已载入");
    let root = config.www_root().to_string();
    info!("www root: {}", &root);

    // 3. 异步运行时定制：根据配置文件动态分配工作线程数，实现 CPU 绑定的并发优化
    let worker_threads = config.worker_threads();
    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .build()
        .unwrap();

    // 4. 共享资源初始化：
    // - 使用 Arc<Mutex<...>> 保证缓存系统在多线程环境下的线程安全
    // - 采用容量受限的缓存机制防止内存溢出
    let cache_size = config.cache_size();
    let cache = Arc::new(Mutex::new(FileCache::from_capacity(cache_size)));
    let config_arc = Arc::new(config.clone());

    // 5. 外部依赖探测：自动检查系统环境中的 PHP 解释器版本
    let php_result = Command::new("php").arg("-v").output();
    match php_result {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(&o.stdout);
                // 使用正则表达式精准提取版本号
                let re = Regex::new(r"PHP (\d+\.\d+\.\d+-\dubuntu\d+\.\d+)").unwrap();
                if let Some(capture) = re.captures(&output) {
                    if let Some(version) = capture.get(1) {
                        info!("找到PHP解释器，版本：{}", version.as_str());
                    }
                }
            } else {
                panic!("在查找PHP解释器时遇到未知错误");
            }
        }
        Err(_) => {
            warn!("无法找到PHP解释器。服务器将继续运行，但将无法处理PHP请求。");
        }
    };

    // 6. 网络层初始化：
    // 支持全地址监听 (0.0.0.0) 或本地回环监听 (127.0.0.1)
    let port: u16 = config.port();
    info!("服务端将在{}端口上监听Socket连接", port);
    let address = match config.local() {
        true => Ipv4Addr::new(127, 0, 0, 1),
        false => Ipv4Addr::new(0, 0, 0, 0),
    };
    info!("服务端将在{}地址上监听Socket连接", address);
    let socket = SocketAddrV4::new(address, port);

    // 绑定端口并启动监听器
    let listener = match TcpListener::bind(socket).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("无法绑定端口：{}，错误：{}", port, e);
            panic!("无法绑定端口：{}，错误：{}", port, e);
        }
    };
    info!("端口{}绑定完成", port);

    // 7. 服务器状态与生命周期管理
    // shutdown_flag: 用于优雅停机 (Graceful Shutdown)
    // active_connection: 原子追踪当前并发连接数
    let shutdown_flag = Arc::new(Mutex::new(false));
    let active_connection = Arc::new(Mutex::new(0u32));

    // 8. 启动交互式管理控制台任务
    // 该任务运行在后台，不阻塞监听循环，提供运维指令支持
    runtime.spawn({
        let shutdown_flag = Arc::clone(&shutdown_flag);
        let active_connection = Arc::clone(&active_connection);
        async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut input = String::new();
            loop {
                input.clear();
                if let Ok(_) = reader.read_line(&mut input).await {
                    let cmd = input.trim();
                    match cmd {
                        "stop" => {
                            let mut flag = shutdown_flag.lock().unwrap();
                            *flag = true;
                            println!("停机指令已激活，服务器将在处理完下一个请求后关闭...");
                            break;
                        }
                        "help" => {
                            println!("== Webserver Help ==");
                            println!("stop   - 发出停机信号");
                            println!("status - 查看当前服务器运行状态");
                            println!("help   - 显示此帮助信息");
                            println!("====================");
                        }
                        "status" => {
                            let active_count = *active_connection.lock().unwrap();
                            println!("== Webserver 状态 ===");
                            println!("当前活跃连接数: {}", active_count);
                            println!("====================");
                        }
                        _ => {
                            println!("无效的命令：{}", cmd);
                        }
                    }
                } else {
                    break;
                }
            }
        }
    });

    let mut id: u128 = 0;

    // 9. 主事件循环 (Accept Loop)
    // 持续接收新连接并将其分发至 Tokio 线程池进行异步处理
    loop {
        // 检查停机标志位
        if *shutdown_flag.lock().unwrap() {
            info!("主循环接收到停机指令，正在退出...");
            break;
        }

        // 等待新的 TCP 连接
        let (mut stream, addr) = listener.accept().await.unwrap();
        debug!("新的连接：{}", addr);

        // 为每个连接克隆资源句柄（Arc 引用计数增加）
        let active_connection_arc = Arc::clone(&active_connection);
        let root_clone = root.clone();
        let cache_arc = Arc::clone(&cache);
        let config_arc_clone = Arc::clone(&config_arc);
        
        debug!("[ID{}]TCP连接已建立", id);

        // 使用轻量级绿色线程处理具体请求，确保非阻塞 IO
        tokio::spawn(async move {
            {
                // 连接计数加 1
                let mut lock = active_connection_arc.lock().unwrap();
                *lock += 1;
            }
            
            // 核心业务处理
            handle_connection(&mut stream, id, &root_clone, cache_arc, config_arc_clone).await;
            
            {
                // 处理完成后连接计数减 1
                let mut lock = active_connection_arc.lock().unwrap();
                *lock -= 1;
            }
        });
        id += 1; // 增加请求唯一标识序列
    }
}

/// # 连接处理器
/// 
/// 负责单个 TCP 流的生命周期，包括读取解析请求、执行路由逻辑、以及构建并发送响应。
async fn handle_connection(
    stream: &mut TcpStream,
    id: u128,
    root: &str,
    cache: Arc<Mutex<FileCache>>,
    config: Arc<Config>,
) {
    let mut buffer = vec![0; 1024];

    // 等待流进入可读状态
    stream.readable().await.unwrap();

    // 尝试非阻塞读取 HTTP 报文
    match stream.try_read(&mut buffer) {
        Ok(0) => return, // 客户端主动关闭连接
        Err(e) => {
            error!("[ID{}]读取TCPStream时遇到错误: {}", id, e);
            return;
        }
        _ => {}
    }
    debug!("[ID{}]HTTP请求接收完毕", id);

    let start_time = Instant::now();

    // 1. 协议解析阶段：将字节流转换为结构化的 Request 对象
    let request = match Request::try_from(&buffer, id) {
        Ok(req) => req,
        Err(e) => {
            error!("[ID{}]解析HTTP请求失败: {:?}", id, e);
            let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 11\r\n\r\nBad Request";
            let _ = stream.write_all(response.as_bytes()).await;
            return;
        }
    };
    debug!("[ID{}]成功解析HTTP请求", id);

    // 2. 意图分析：根据 Accept 头部判断是否为 JSON 数据交互
    let is_json = request
        .accept()
        .map_or(false, |a| a.contains("application/json"));

    // 3. 路由匹配阶段：确定资源在文件系统中的物理路径
    let result = route(&request.path(), id, root, is_json).await;
    debug!("[ID{}]HTTP路由解析完毕", id);

    // 4. 响应构建阶段：根据路由结果和缓存状态生成 Response 对象
    let response = match result {
        Ok(path) => {
            let path_str = match path.to_str() {
                Some(s) => s,
                None => {
                    error!("[ID{}]无法将路径转换为str", id);
                    return;
                }
            };
            // 自动处理缓存命中与过期逻辑
            Response::from(path_str, &request, id, &cache, &config)
        }
        Err(Exception::FileNotFound) => {
            warn!("[ID{}]请求的路径：{} 不存在，返回404", id, &request.path());
            Response::response_404(&request, id)
        }
        Err(Exception::InvalidPath) => {
            warn!("[ID{}]请求的路径：{} 包含非法字符，返回400", id, &request.path());
            Response::response_400(&request, id)
        }
        Err(Exception::UnsupportedHttpVersion) => {
            warn!("[ID{}]不支持的协议版本，返回400", id);
            Response::response_400(&request, id)
        }
        Err(e) => {
            error!("[ID{}]处理请求时发生未知异常: {}", id, e);
            return;
        }
    };

    debug!(
        "[ID{}]HTTP响应构建完成，服务端用时{}ms。",
        id,
        start_time.elapsed().as_millis()
    );

    // 5. 结构化日志记录：便于后期审计与性能监控
    info!(
        "[ID{}] {}, {}, {}, {}, {}, {}, ",
        id,
        request.version(),
        request.path(),
        request.method(),
        response.status_code(),
        response.information(),
        request.user_agent(),
    );

    // 6. 数据发送阶段
    if response.is_streaming() {
        // --- 模式 A: 流式传输 (适用于大文件，避免内存暴涨) ---
        debug!("[ID{}]使用流式传输模式发送大文件", id);
        
        let response_bytes = response.as_bytes(); // 发送响应头
        if let Err(e) = stream.write_all(&response_bytes).await {
            error!("[ID{}]发送响应头失败: {}", id, e);
            return;
        }
        
        // 重新获取物理路径以打开文件
        if let Ok(path) = route(&request.path(), id, root, false).await {
            if let Some(path_str) = path.to_str() {
                match TokioFile::open(path_str).await {
                    Ok(mut file) => {
                        let chunk_size = config.chunk_size();
                        let mut buffer = vec![0u8; chunk_size];
                        let mut total_sent = 0u64;
                        let content_length = response.get_content_length();
                        
                        debug!("[ID{}]开始流式传输，文件大小: {} bytes", id, content_length);
                        
                        loop {
                            match file.read(&mut buffer).await {
                                Ok(0) => break, // 文件读取完毕
                                Ok(n) => {
                                    // 持续将缓冲区内容写入 Socket
                                    if let Err(e) = stream.write_all(&buffer[..n]).await {
                                        error!("[ID{}]流式写入失败: {}", id, e);
                                        return;
                                    }
                                    total_sent += n as u64;
                                }
                                Err(e) => {
                                    error!("[ID{}]读取文件失败: {}", id, e);
                                    return;
                                }
                            }
                        }
                        let _ = stream.flush().await;
                        debug!("[ID{}]流式传输完成，共发送 {} 字节", id, total_sent);
                    }
                    Err(e) => {
                        error!("[ID{}]无法打开流文件: {}", id, e);
                    }
                }
            }
        }
    } else {
        // --- 模式 B: 一次性传输 (适用于小文件或 API 响应) ---
        let response_bytes = response.as_bytes();
        debug!("[ID{}]发送全量响应，长度: {}", id, response_bytes.len());
        let _ = stream.write_all(&response_bytes).await;
        let _ = stream.flush().await;
    }
}

/// # 路由引擎
/// 
/// 将抽象的 URI 映射到服务器本地的文件系统路径。
/// 
/// ## 路由规则：
/// 1. `/` -> 优先返回 `index.html`，若为 JSON 请求则返回根目录列表。
/// 2. `/browser` -> 专门处理前端 Vue 应用，支持 SPA (Single Page Application) 的 History 模式。
/// 3. `*` -> 特殊通配符匹配。
/// 4. 静态文件映射 -> 将 URI 拼接到 `www_root` 下进行查找。
async fn route(path: &str, id: u128, root: &str, is_json: bool) -> Result<PathBuf, Exception> {
    debug!("[ID{}]路由匹配开始: path='{}', json_mode={}", id, path, is_json);
    
    // 根目录特殊处理
    if path == "/" {
        if is_json {
            return Ok(PathBuf::from(root));
        }
        let index_path = PathBuf::from(HTML_INDEX);
        if index_path.exists() {
            return Ok(index_path);
        } else {
            return Ok(PathBuf::from(root));
        }
    } 
    // 文件管理器路由（支持 SPA 静态资源）
    else if path == "/browser/" || path == "/browser" {
        if is_json {
            let browser_path = PathBuf::from("static/browser");
            if browser_path.exists() && browser_path.is_dir() {
                return Ok(browser_path);
            }
        }
        let browser_index = PathBuf::from("static/browser/index.html");
        if browser_index.exists() {
            return Ok(browser_index);
        } else {
            return Err(Exception::FileNotFound);
        }
    } 
    // 通配符处理
    else if path == "*" {
        return Ok(PathBuf::from("*"));
    }

    // 标准静态资源路径转换逻辑
    // 去除领先的 '/' 以便进行路径拼接
    let mut path_str = path.to_string();
    path_str.remove(0);
    let path_without_slash = Path::new(&path_str);
    let root_path = Path::new(root);
    let full_path = root_path.join(path_without_slash);

    // 安全检查与路径存在性校验
    let path_str_ref = match full_path.to_str() {
        Some(s) => s,
        None => return Err(Exception::InvalidPath),
    };
    
    debug!("[ID{}]映射物理路径：{}", id, path_str_ref);
    
    match full_path.exists() {
        true => Ok(full_path),
        false => {
            // SPA (Single Page Application) 回退机制：
            // 如果在 /browser/ 路径下找不到文件，则返回 index.html，交由前端路由处理
            if path.starts_with("/browser/") || path.starts_with("/browser") {
                let browser_index = PathBuf::from("static/browser/index.html");
                if browser_index.exists() {
                    debug!("[ID{}]SPA 路由触发：返回 Vue index.html", id);
                    return Ok(browser_index);
                }
            }
            Err(Exception::FileNotFound)
        }
    }
}
