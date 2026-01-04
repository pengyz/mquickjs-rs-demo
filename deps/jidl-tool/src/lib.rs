//! jidl-tool - mquickjs IDL代码生成工具
//! 
//! 用于将IDL定义转换为Rust胶水代码和C绑定代码的工具

pub mod parser;
pub mod generator;

pub use parser::parse_idl;
pub use generator::generate_code;

/// 解析IDL内容并生成对应的Rust和C代码
pub fn process_idl(idl_content: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 解析IDL内容
    let ast = parse_idl(idl_content)?;
    
    // 生成代码
    generate_code(&ast, output_dir)?;
    
    Ok(())
}