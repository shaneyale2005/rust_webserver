#![allow(clippy::unused_io_amount)]

mod cache;
mod config;
mod exception;
mod param;
mod request;
mod response;
mod util;

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

#[tokio::main]
async fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    let config = Config::from_toml("config/development.toml");
    info!("配置文件已载入");
    let root = config.www_root().to_string();
    info!("www root: {}", &root);

    let worker_threads = config.worker_threads();
    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .build()
        .unwrap();

    let cache_size = config.cache_size();
    let cache = Arc::new(Mutex::new(FileCache::from_capacity(cache_size)));
    let config_arc = Arc::new(config.clone());

    let php_result = Command::new("php").arg("-v").output();
    match php_result {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(&o.stdout);
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

    let port: u16 = config.port();
    info!("服务端将在{}端口上监听Socket连接", port);
    let address = match config.local() {
        true => Ipv4Addr::new(127, 0, 0, 1),
        false => Ipv4Addr::new(0, 0, 0, 0),
    };
    info!("服务端将在{}地址上监听Socket连接", address);
    let socket = SocketAddrV4::new(address, port);

    let listener = match TcpListener::bind(socket).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("无法绑定端口：{}，错误：{}", port, e);
            panic!("无法绑定端口：{}，错误：{}", port, e);
        }
    };
    info!("端口{}绑定完成", port);

    let shutdown_flag = Arc::new(Mutex::new(false));
    let active_connection = Arc::new(Mutex::new(0u32));

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
                            break;
                        }
                        "help" => {
                            println!("== Webserver Help ==");
                            println!("输入stop并再发出一次连接请求以停机");
                            println!("输入status以查看当前服务器状态");
                            println!("====================");
                        }
                        "status" => {
                            let active_count = *active_connection.lock().unwrap();
                            println!("== Webserver 状态 ===");
                            println!("当前连接数: {}", active_count);
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

    loop {
        if *shutdown_flag.lock().unwrap() {
            break;
        }
        let (mut stream, addr) = listener.accept().await.unwrap();
        debug!("新的连接：{}", addr);

        let active_connection_arc = Arc::clone(&active_connection);
        let root_clone = root.clone();
        let cache_arc = Arc::clone(&cache);
        let config_arc_clone = Arc::clone(&config_arc);
        debug!("[ID{}]TCP连接已建立", id);
        tokio::spawn(async move {
            {
                let mut lock = active_connection_arc.lock().unwrap();
                *lock += 1;
            }
            handle_connection(&mut stream, id, &root_clone, cache_arc, config_arc_clone).await;
            {
                let mut lock = active_connection_arc.lock().unwrap();
                *lock -= 1;
            }
        });
        id += 1;
    }
}

async fn handle_connection(
    stream: &mut TcpStream,
    id: u128,
    root: &str,
    cache: Arc<Mutex<FileCache>>,
    config: Arc<Config>,
) {
    let mut buffer = vec![0; 1024];

    stream.readable().await.unwrap();

    match stream.try_read(&mut buffer) {
        Ok(0) => return,
        Err(e) => {
            error!("[ID{}]读取TCPStream时遇到错误: {}", id, e);
            panic!();
        }
        _ => {}
    }
    debug!("[ID{}]HTTP请求接收完毕", id);

    let start_time = Instant::now();

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

    let is_json = request
        .accept()
        .map_or(false, |a| a.contains("application/json"));
    let result = route(&request.path(), id, root, is_json).await;
    debug!("[ID{}]HTTP路由解析完毕", id);

    let response = match result {
        Ok(path) => {
            let path_str = match path.to_str() {
                Some(s) => s,
                None => {
                    let path_str = path.to_str().unwrap();
                    error!("[ID{}]无法将路径{}转换为str", id, path_str);
                    return;
                }
            };
            Response::from(path_str, &request, id, &cache, &config)
        }
        Err(Exception::FileNotFound) => {
            warn!(
                "[ID{}]请求的路径：{} 不存在，返回404响应",
                id,
                &request.path()
            );
            Response::response_404(&request, id)
        }
        Err(Exception::InvalidPath) => {
            warn!(
                "[ID{}]请求的路径：{} 包含非法字符，返回400响应",
                id,
                &request.path()
            );
            Response::response_400(&request, id)
        }
        Err(Exception::UnsupportedHttpVersion) => {
            warn!("[ID{}]请求的HTTP协议版本不支持，返回400响应", id);
            Response::response_400(&request, id)
        }
        Err(e) => {
            panic!("非法的错误类型：{}", e);
        }
    };

    debug!(
        "[ID{}]HTTP响应构建完成，服务端用时{}ms。",
        id,
        start_time.elapsed().as_millis()
    );

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

    if response.is_streaming() {
        debug!("[ID{}]使用流式传输模式发送大文件", id);
        
        let response_bytes = response.as_bytes();
        if let Err(e) = stream.write_all(&response_bytes).await {
            error!("[ID{}]发送响应头失败: {}", id, e);
            return;
        }
        
        let result = route(&request.path(), id, root, false).await;
        if let Ok(path) = result {
            if let Some(path_str) = path.to_str() {
                match TokioFile::open(path_str).await {
                    Ok(mut file) => {
                        let chunk_size = config.chunk_size();
                        let mut buffer = vec![0u8; chunk_size];
                        let mut total_sent = 0u64;
                        let content_length = response.get_content_length();
                        
                        debug!("[ID{}]开始流式传输，文件大小: {} bytes，块大小: {} bytes", 
                               id, content_length, chunk_size);
                        
                        loop {
                            match file.read(&mut buffer).await {
                                Ok(0) => {
                                    debug!("[ID{}]文件读取完成，总共发送: {} bytes", id, total_sent);
                                    break;
                                }
                                Ok(n) => {
                                    if let Err(e) = stream.write_all(&buffer[..n]).await {
                                        error!("[ID{}]流式传输写入失败: {}", id, e);
                                        return;
                                    }
                                    total_sent += n as u64;
                                    if total_sent % (chunk_size as u64 * 10) == 0 {
                                        debug!("[ID{}]已发送: {} / {} bytes ({:.1}%)", 
                                               id, total_sent, content_length, 
                                               (total_sent as f64 / content_length as f64) * 100.0);
                                    }
                                }
                                Err(e) => {
                                    error!("[ID{}]读取文件失败: {}", id, e);
                                    return;
                                }
                            }
                        }
                        
                        if let Err(e) = stream.flush().await {
                            error!("[ID{}]flush失败: {}", id, e);
                            return;
                        }
                        debug!("[ID{}]流式传输完成", id);
                    }
                    Err(e) => {
                        error!("[ID{}]打开文件{}进行流式传输失败: {}", id, path_str, e);
                        return;
                    }
                }
            }
        }
    } else {
        let response_bytes = response.as_bytes();
        debug!("[ID{}]响应字节长度: {}", id, response_bytes.len());
        let write_result = stream.write(&response_bytes).await;
        debug!("[ID{}]write result: {:?}", id, write_result);
        let flush_result = stream.flush().await;
        debug!("[ID{}]flush result: {:?}", id, flush_result);
    }
}

async fn route(path: &str, id: u128, root: &str, is_json: bool) -> Result<PathBuf, Exception> {
    debug!("[ID{}]route: path='{}', is_json={}", id, path, is_json);
    if path == "/" {
        debug!("[ID{}]请求路径为根目录", id);
        if is_json {
            debug!("[ID{}]JSON请求，返回根目录生成文件列表", id);
            let root_path = PathBuf::from(root);
            return Ok(root_path);
        }
        let index_path = PathBuf::from(HTML_INDEX);
        if index_path.exists() {
            debug!("[ID{}]index.html存在，返回index", id);
            return Ok(index_path);
        } else {
            debug!("[ID{}]index.html不存在，返回根目录", id);
            let root_path = PathBuf::from(root);
            return Ok(root_path);
        }
    } else if path == "/browser/" || path == "/browser" {
        if is_json {
            debug!("[ID{}]JSON请求browser目录，返回目录列表", id);
            let browser_path = PathBuf::from("static/browser");
            if browser_path.exists() && browser_path.is_dir() {
                return Ok(browser_path);
            }
        }
        debug!("[ID{}]请求Vue文件管理器HTML页面", id);
        let browser_index = PathBuf::from("static/browser/index.html");
        if browser_index.exists() {
            return Ok(browser_index);
        } else {
            error!("[ID{}]Vue文件管理器的index.html不存在", id);
            return Err(Exception::FileNotFound);
        }
    } else if path == "*" {
        debug!("[ID{}]请求路径为*", id);
        let path = PathBuf::from("*");
        return Ok(path);
    }
    let mut path_str = path.to_string();
    path_str.remove(0);
    let path_without_slash = Path::new(&path_str);
    let root = Path::new(root);
    let full_path = root.join(path_without_slash);

    let path_str_ref = match full_path.to_str() {
        Some(s) => s,
        None => {
            error!(
                "[ID{}]无法将路径{}转换为有效的UTF-8字符串",
                id,
                full_path.display()
            );
            return Err(Exception::InvalidPath);
        }
    };
    debug!("[ID{}]请求文件路径：{}", id, path_str_ref);
    match full_path.exists() {
        true => Ok(full_path),
        false => {
            if path.starts_with("/browser/") || path.starts_with("/browser") {
                debug!(
                    "[ID{}]文件不存在但路径在/browser下，尝试作为SPA路由处理",
                    id
                );
                let browser_index = PathBuf::from("static/browser/index.html");
                if browser_index.exists() {
                    debug!("[ID{}]返回Vue应用的index.html以支持客户端路由", id);
                    return Ok(browser_index);
                }
            }
            Err(Exception::FileNotFound)
        }
    }
}
