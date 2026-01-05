//! jidl-tool - mquickjs IDL代码生成工具
//! 
//! 用于将IDL定义转换为Rust胶水代码和C绑定代码的工具

use pest::Parser;

pub mod parser;
pub mod generator;
pub mod validator;

pub use parser::parse_idl;
pub use generator::generate_code;

use crate::parser::ast;

/// 解析RIDL内容为AST
pub fn parse_ridl_content(content: &str, file_path: &str) -> Result<Vec<ast::IDLItem>, Vec<crate::validator::RIDLError>> {
    // 首先使用pest解析器检查语法
    let _pest_pairs = parser::IDLParser::parse(parser::Rule::idl, content)
        .map_err(|e| {
            // 将pest错误转换为RIDL错误
            let (line, col) = match e.line_col {
                pest::error::LineColLocation::Pos((line, col)) => (line, col),
                pest::error::LineColLocation::Span((line_start, col_start), _) => (line_start, col_start),
            };
            let error = crate::validator::RIDLError::new(
                e.to_string(),
                line,  // 行号
                col,   // 列号
                file_path.to_string(),
                crate::validator::RIDLErrorType::SyntaxError,
            );
            vec![error]
        })?;

    // 使用现有的parse_idl函数解析内容
    let items = parser::parse_idl(content).map_err(|e| {
        // 将解析错误转换为RIDL错误
        let error = crate::validator::RIDLError::new(
            e.to_string(),
            1,  // 默认行号
            1,  // 默认列号
            file_path.to_string(),
            crate::validator::RIDLErrorType::SyntaxError,
        );
        vec![error]
    })?;
    
    // 创建一个虚拟的IDL结构来包装items，以便验证器可以使用
    let mut idl_wrapper = ast::IDL {
        module: None,
        interfaces: vec![],
        classes: vec![],
        enums: vec![],
        structs: vec![],
        functions: vec![],
        using: vec![],
        imports: vec![],
        singletons: vec![],
        callbacks: vec![],
    };
    
    // 从items中提取各种定义到idl_wrapper中
    for item in &items {
        match item {
            ast::IDLItem::Interface(interface) => idl_wrapper.interfaces.push(interface.clone()),
            ast::IDLItem::Class(class) => idl_wrapper.classes.push(class.clone()),
            ast::IDLItem::Enum(enum_def) => idl_wrapper.enums.push(enum_def.clone()),
            ast::IDLItem::Struct(struct_def) => idl_wrapper.structs.push(struct_def.clone()),
            ast::IDLItem::Function(function) => idl_wrapper.functions.push(function.clone()),
            ast::IDLItem::Using(using) => idl_wrapper.using.push(using.clone()),
            ast::IDLItem::Import(import) => idl_wrapper.imports.push(import.clone()),
            ast::IDLItem::Singleton(singleton) => idl_wrapper.singletons.push(singleton.clone()),
        }
    }
    
    // 使用验证器验证语义
    let mut validator = crate::validator::SemanticValidator::new(file_path.to_string());
    validator.validate(&idl_wrapper).map(|_| items)
}