use std::{path::PathBuf, process::Command};

use chrono::{DateTime, Local};
use log::error;

use crate::{exception::Exception, param::STATUS_CODES};

pub struct HtmlBuilder {
    title: String,
    css: String,
    script: String,
    body: String,
}

impl HtmlBuilder {
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

pub fn handle_php(path: &str, id: u128) -> Result<String, Exception> {
    let result = Command::new("php")
        .arg(path) // PHP文件路径
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

    #[test]
    fn test_sort_dir_entries() {
        let mut entries = vec![PathBuf::from("file1.txt"), PathBuf::from("file2.txt")];

        sort_dir_entries(&mut entries);

        assert_eq!(entries[0].file_name().unwrap(), "file1.txt");
        assert_eq!(entries[1].file_name().unwrap(), "file2.txt");
    }

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

    #[test]
    fn test_format_file_size_edge_cases() {
        assert_eq!(format_file_size(1024 - 1), "1023.0 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1024 * 1024 - 1), "1024.0 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
    }
}
