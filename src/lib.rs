// 库接口，用于测试和基准测试

pub mod cache;
pub mod config;
pub mod exception;
pub mod param;
pub mod request;
pub mod response;
pub mod util;

// 重新导出常用类型
pub use cache::FileCache;
pub use exception::Exception;
pub use param::{HttpEncoding, HttpRequestMethod, HttpVersion};
pub use request::Request;
pub use response::Response;
pub use util::HtmlBuilder;
