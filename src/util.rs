// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # HTML 构建与 PHP 处理模块
//! 
//! 该模块负责生成 Web 服务器所需的动态 HTML 内容，包括：
//! 1. 状态码对应的错误页面。
//! 2. 目录文件的索引列表页面。
//! 3. 辅助工具函数（文件大小格式化、目录排序）。
//! 4. 外部 PHP 脚本的解析与执行。

use std::{path::PathBuf, process::Command};
use chrono::{DateTime, Local};
use log::error;
use crate::{exception::Exception, param::STATUS_CODES};

/// `HtmlBuilder` 用于构建符合 HTML5 标准的页面字符串。
/// 
/// 该结构体采用建造者模式的思想，通过收集标题、样式、脚本和主体内容，
/// 最终生成完整的 HTML 源码。
pub struct HtmlBuilder {
    /// 页面 `<title>` 标签的内容
    title: String,
    /// 注入 `<style>` 标签的 CSS 样式
    css: String,
    /// 注入 `<script>` 标签的 JavaScript 脚本
    script: String,
    /// 注入 `<body>` 标签的 HTML 主体内容
    body: String,
}

impl HtmlBuilder {
    /// 根据 HTTP 状态码创建状态页面。
    /// 
    /// # 参数
    /// * `code` - HTTP 状态码（如 200, 404, 500）。
    /// * `note` - 可选的自定义描述信息。如果为 `None`，则从系统预设的 `STATUS_CODES` 中获取。
    /// 
    /// # 异常
    /// 如果传入了系统未定义且未提供 `note` 的状态码，该函数会触发 `panic`。
    pub fn from_status_code(code: u16, note: Option<&str>) -> Self {
        let title = format!("{}", code);
        let css = r"
            body {
                width: 35em;
                margin: 0 auto;
                font-family: Tahoma, Verdana, Arial, sans-serif;
            }
            "
        .to_string();
        let description = match note {
            Some(n) => n,
            None => match STATUS_CODES.get(&code) {
                Some(d) => *d,
                None => {
                    panic!("非法的状态码：{}", code);
                }
            },
        };
        let body = format!(
            r"
            <h1>{}</h1>
            <p>{}</p>
            ",
            code, description
        );
        Self {
            title,
            css,
            script: "".to_string(),
            body,
        }
    }

    /// 根据目录路径和文件列表创建目录索引页面。
    /// 
    /// # 参数
    /// * `path` - 当前访问的 URL 路径字符串。
    /// * `dir_vec` - 包含该目录下所有文件和子目录 `PathBuf` 的向量。
    /// 
    /// # 功能描述
    /// 1. 对文件列表进行排序（文件夹在前，文件在后）。
    /// 2. 生成包含文件名、大小、修改时间的表格。
    /// 3. 自动处理路径结尾的斜杠并添加“返回上级目录”的链接。
    pub fn from_dir(path: &str, dir_vec: &mut Vec<PathBuf>) -> Self {
        let mut body = String::new();
        sort_dir_entries(dir_vec);

        let mut path_mut = path;
        if path_mut.ends_with("/") {
            let len = path_mut.len();
            path_mut = &path_mut[..(len - 1)];
        }
        body.push_str(&format!("<h1>{}的文件列表</h1><hr>", path_mut));
        body.push_str("<table>");
        body.push_str(
            r#"
            <tr>
                <td>文件名</td>
                <td>大小</td>
                <td>修改时间</td>
            </tr>
            <tr>
                <td><a href="../">..</a></td>
                <td></td>
                <td></td>
            </tr>
            "#,
        );
        for entry in dir_vec {
            let metadata = entry.metadata().unwrap();
            let local_time: DateTime<Local> = metadata.modified().unwrap().into();
            let formatted_time = local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string();

            let filename = entry.file_name().unwrap().to_string_lossy();

            if entry.is_file() {
                let size = metadata.len();
                let formatted_size = format_file_size(size);
                body.push_str(&format!(
                    r#"
                    <tr>
                        <td><a href="{}">{}</a></td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>
                    "#,
                    &filename, &filename, &formatted_size, &formatted_time
                ));
            } else if entry.is_dir() {
                let filename = [&filename, "/"].concat();
                body.push_str(&format!(
                    r#"
                    <tr>
                    <td><a href="{}">{}</a></td>
                        <td>文件夹</td>
                        <td>{}</td>
                    </tr>
                    "#,
                    &filename, &filename, &formatted_time
                ));
            } else {
                panic!();
            }
        }
        body.push_str("</table>");
        let title = format!("{}的文件列表", path);
        let css = r"
            table {
                border-collapse: collapse;
                width: 100%;
            }

            td {
                padding: 8px;
                white-space: pre-wrap; /* 保留换行符和空格 */
                border: none; /* 隐藏单元格边框 */
            }

            th {
                padding: 8px;
                border: none; /* 隐藏表头边框 */
            }"
        .to_string();
        HtmlBuilder {
            title,
            css,
            script: "".to_string(),
            body,
        }
    }

    /// 组装所有组件，生成最终的 HTML 5 字符串。
    /// 
    /// # 返回
    /// 返回一个完整的 HTML 文档字符串，包含 DOCTYPE、head、style、script 和 body。
    pub fn build(&self) -> String {
        format!(
            r##"<!DOCTYPE html>
            <!-- 本文件由shaneyale的Rust Webserver自动生成 -->
            <html>
                <head>
                    <meta charset="utf-8">
                    <script>{}</script>
                    <title>{}</title>
                    <style>{}</style>
                </head>
                <body>
                {}
                </body>
            </html>"##,
            self.script, self.title, self.css, self.body
        )
    }
}

/// 将以字节为单位的文件大小转换为易读的格式（B, KB, MB, GB, TB）。
/// 
/// # 参数
/// * `size` - 文件大小（字节）。
/// 
/// # 示例
/// ```
/// use webserver::util::format_file_size;
/// let human_size = format_file_size(1024);
/// assert_eq!(human_size, "1.0 KB");
/// ```
pub fn format_file_size(size: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, units[unit_index])
}

/// 对文件路径向量进行排序。
/// 
/// 排序规则：
/// 1. 优先排列目录（Directory）。
/// 2. 同类型（同为目录或同为文件）按照路径名称升序排列。
fn sort_dir_entries(vec: &mut Vec<PathBuf>) {
    vec.sort_by(|a, b| {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();

        if a_is_dir && !b_is_dir {
            std::cmp::Ordering::Less
        } else if !a_is_dir && b_is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });
}

/// 调用系统环境中的 PHP 解释器执行指定的 PHP 文件。
/// 
/// # 参数
/// * `path` - PHP 文件的本地绝对路径或相对路径。
/// * `id` - 当前请求的唯一 ID，用于日志记录。
/// 
/// # 返回值
/// * `Ok(String)` - PHP 脚本标准输出的内容。
/// * `Err(Exception)` - 如果无法调用 PHP 解释器（`PHPExecuteFailed`）或脚本执行报错（`PHPCodeError`）。
/// 
/// # 注意
/// 运行环境必须在系统 PATH 中安装有 `php` 命令。
pub fn handle_php(path: &str, id: u128) -> Result<String, Exception> {
    let result = Command::new("php")
        .arg(path)
        .output();
    let output = match result {
        Ok(o) => o,
        Err(_) => return Err(Exception::PHPExecuteFailed),
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(String::from(stdout))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("[ID{}]PHP解释器出错：{}", id, stderr);
        Err(Exception::PHPCodeError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 测试文件大小格式化逻辑是否正确处理各数量级转换
    #[test]
    fn test_file_size() {
        let a = 9926;
        let b = 51800;
        assert_eq!(format_file_size(a), "9.7 KB".to_string());
        assert_eq!(format_file_size(b), "50.6 KB".to_string());
    }

    #[test]
    fn test_file_size_bytes() {
        assert_eq!(format_file_size(0), "0.0 B");
        assert_eq!(format_file_size(512), "512.0 B");
        assert_eq!(format_file_size(1023), "1023.0 B");
    }

    #[test]
    fn test_file_size_kb() {
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(2048), "2.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
    }

    #[test]
    fn test_file_size_mb() {
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(5242880), "5.0 MB");
    }

    #[test]
    fn test_file_size_gb() {
        assert_eq!(format_file_size(1073741824), "1.0 GB");
        assert_eq!(format_file_size(3221225472), "3.0 GB");
    }

    #[test]
    fn test_file_size_tb() {
        assert_eq!(format_file_size(1099511627776), "1.0 TB");
    }

    /// 测试状态码页面生成是否包含核心 HTML 标签和预期的描述
    #[test]
    fn test_html_builder_from_status_code() {
        let html = HtmlBuilder::from_status_code(404, Some("测试404")).build();
        assert!(html.contains("404"));
        assert!(html.contains("测试404"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_html_builder_from_status_code_no_note() {
        let html = HtmlBuilder::from_status_code(200, None).build();
        assert!(html.contains("200"));
        assert!(html.contains("OK"));
    }

    /// 验证非法状态码是否会引起 Panic
    #[test]
    #[should_panic(expected = "非法的状态码")]
    fn test_html_builder_invalid_status_code() {
        HtmlBuilder::from_status_code(999, None);
    }

    #[test]
    fn test_html_builder_various_codes() {
        for code in [200, 201, 204, 400, 401, 403, 404, 500, 502, 503] {
            let html = HtmlBuilder::from_status_code(code, None).build();
            assert!(html.contains(&code.to_string()));
            assert!(html.contains("<!DOCTYPE html>"));
        }
    }

    /// 测试目录排序功能：文件夹是否排在文件之前
    #[test]
    fn test_sort_dir_entries() {
        let mut entries = vec![PathBuf::from("file1.txt"), PathBuf::from("file2.txt")];

        sort_dir_entries(&mut entries);

        assert_eq!(entries[0].file_name().unwrap(), "file1.txt");
        assert_eq!(entries[1].file_name().unwrap(), "file2.txt");
    }

    /// 验证生成的页面结构是否符合 HTML5 标准格式
    #[test]
    fn test_html_builder_structure() {
        let html = HtmlBuilder::from_status_code(404, Some("测试")).build();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<head>"));
        assert!(html.contains("</head>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("</body>"));
        assert!(html.contains("<title>"));
        assert!(html.contains("</title>"));
        assert!(html.contains("<style>"));
        assert!(html.contains("</style>"));
        assert!(html.contains("charset=\"utf-8\""));
    }

    /// 边界值测试：测试文件大小在临界点（如 1023B 转换到 1KB）的切换是否正确
    #[test]
    fn test_format_file_size_edge_cases() {
        assert_eq!(format_file_size(1024 - 1), "1023.0 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1024 * 1024 - 1), "1024.0 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
    }
}
