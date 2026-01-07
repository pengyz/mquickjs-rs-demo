//! jidl-tool - mquickjs IDL代码生成工具
//! 
//! 用于将IDL定义转换为Rust胶水代码和C绑定代码的工具

pub mod parser;
pub mod validator;
pub mod generator;

// 重新导出主要函数
pub use parser::parse_ridl;
pub use validator::validate;