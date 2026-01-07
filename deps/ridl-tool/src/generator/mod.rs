use askama::Template;
use std::path::Path;
use crate::parser::ast::{IDLItem, Interface, Method, Param, Function, IDL, Type};
use std::collections::HashMap;

mod filters;

#[derive(Template)]
#[template(path = "c_header.rs.j2")]
struct CHeaderTemplate {
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
}

#[derive(Template)]
#[template(path = "rust_glue.rs.j2")]
struct RustGlueTemplate {
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
}

#[derive(Template)]
#[template(path = "rust_impl.rs.j2")]
struct RustImplTemplate {
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
}

#[derive(Template)]
#[template(path = "symbols.rs.j2")]
struct SymbolsTemplate {
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
}

#[derive(Debug, Clone)]
struct TemplateInterface {
    name: String,
    methods: Vec<TemplateMethod>,
}

#[derive(Debug, Clone)]
struct TemplateMethod {
    name: String,
    params: Vec<TemplateParam>,
    return_type: Option<String>,
}

#[derive(Debug, Clone)]
struct TemplateParam {
    name: String,
    param_type: String,
}

#[derive(Debug, Clone)]
struct TemplateFunction {
    name: String,
    params: Vec<TemplateParam>,
    return_type: Option<String>,
}

impl From<Interface> for TemplateInterface {
    fn from(interface: Interface) -> Self {
        Self {
            name: interface.name,
            methods: interface.methods.into_iter().map(|m| m.into()).collect(),
        }
    }
}

impl From<Method> for TemplateMethod {
    fn from(method: Method) -> Self {
        Self {
            name: method.name,
            params: method.params.into_iter().map(|p| p.into()).collect(),
            return_type: if matches!(method.return_type, Type::Void) {
                None
            } else {
                Some(method.return_type.to_string())
            },
        }
    }
}

impl From<Param> for TemplateParam {
    fn from(param: Param) -> Self {
        Self {
            name: param.name,
            param_type: param.param_type.to_string(),
        }
    }
}

impl From<Function> for TemplateFunction {
    fn from(function: Function) -> Self {
        Self {
            name: function.name,
            params: function.params.into_iter().map(|p| p.into()).collect(),
            return_type: if matches!(function.return_type, Type::Void) {
                None
            } else {
                Some(function.return_type.to_string())
            },
        }
    }
}

pub fn collect_definitions(ridl_files: &[String]) -> Result<Vec<IDL>, Box<dyn std::error::Error>> {
    let mut all_definitions = Vec::new();
    
    for ridl_file in ridl_files {
        let content = std::fs::read_to_string(ridl_file)?;
        let items = crate::parser::parse_ridl(&content)?;
        
        // 将解析出的Vec<IDLItem>转换为单个IDL结构
        let mut functions = Vec::new();
        let mut interfaces = Vec::new();
        let mut classes = Vec::new();
        let mut enums = Vec::new();
        let mut structs = Vec::new();
        let callbacks: Vec<Function> = vec![]; // 回调作为函数处理
        let mut using = Vec::new();
        let mut imports = Vec::new();
        let mut singletons = Vec::new();
        let mut module = None;
        
        for item in items {
            match item {
                crate::parser::ast::IDLItem::Function(f) => functions.push(f),
                crate::parser::ast::IDLItem::Interface(i) => interfaces.push(i),
                crate::parser::ast::IDLItem::Class(c) => classes.push(c),
                crate::parser::ast::IDLItem::Enum(e) => enums.push(e),
                crate::parser::ast::IDLItem::Struct(s) => structs.push(s),
                crate::parser::ast::IDLItem::Using(u) => using.push(u),
                crate::parser::ast::IDLItem::Import(im) => imports.push(im),
                crate::parser::ast::IDLItem::Singleton(s) => singletons.push(s),
            }
        }
        
        let idl = IDL {
            functions,
            interfaces,
            classes,
            enums,
            structs,
            callbacks: vec![], // 回调作为函数处理
            using,
            imports,
            singletons,
            module,
        };
        
        all_definitions.push(idl);
    }
    
    Ok(all_definitions)
}

pub fn generate_module_files(items: &[IDLItem], output_path: &Path, module_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut functions = Vec::new();
    let mut interfaces = Vec::new();
    
    for item in items {
        match item {
            crate::parser::ast::IDLItem::Function(f) => functions.push(TemplateFunction::from(f.clone())),
            crate::parser::ast::IDLItem::Interface(i) => interfaces.push(TemplateInterface::from(i.clone())),
            // 其他类型暂不处理，可根据需要添加
            _ => {},
        }
    }
    
    // 生成Rust胶水代码
    let rust_glue_template = RustGlueTemplate {
        module_name: module_name.to_string(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
    };
    let rust_glue_code = rust_glue_template.render()?;
    std::fs::write(output_path.join(format!("{}_glue.rs", module_name)), rust_glue_code)?;

    // 生成Rust实现骨架
    let rust_impl_template = RustImplTemplate {
        module_name: module_name.to_string(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
    };
    let rust_impl_code = rust_impl_template.render()?;
    std::fs::write(output_path.join(format!("{}_impl.rs", module_name)), rust_impl_code)?;

    // 注意：模块命令只生成Rust胶水代码和实现骨架，其他文件在aggregate命令中生成

    Ok(())
}

pub fn generate_shared_files(ridl_files: &[String], output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 为了聚合符号，我们需要读取每个模块的名称并生成一个聚合的符号文件
    let mut all_module_symbols = Vec::new();
    
    for ridl_file in ridl_files {
        // 从文件路径提取模块名
        let module_name = std::path::Path::new(ridl_file)
            .file_stem()
            .ok_or("Invalid ridl file path")?
            .to_str()
            .ok_or("Invalid UTF-8 in file name")?
            .to_string();
        
        // 读取并解析RIDL文件
        let content = std::fs::read_to_string(ridl_file)?;
        let items = crate::parser::parse_ridl(&content)?;
        
        // 提取函数和接口
        let mut functions = Vec::new();
        let mut interfaces = Vec::new();
        
        for item in items {
            match item {
                crate::parser::ast::IDLItem::Function(f) => functions.push(TemplateFunction::from(f)),
                crate::parser::ast::IDLItem::Interface(i) => interfaces.push(TemplateInterface::from(i)),
                // 其他类型暂不处理
                _ => {},
            }
        }
        
        all_module_symbols.push((module_name, functions, interfaces));
    }
    
    // 生成聚合的C头文件
    let mut all_interfaces = Vec::new();
    let mut all_functions = Vec::new();
    
    for (_, functions, interfaces) in &all_module_symbols {
        all_functions.extend(functions.clone());
        all_interfaces.extend(interfaces.clone());
    }
    
    let c_template = CHeaderTemplate {
        module_name: "mquickjs_ridl".to_string(),
        interfaces: all_interfaces,
        functions: all_functions,
    };
    let c_code = c_template.render()?;
    std::fs::write(std::path::Path::new(output_dir).join("mquickjs_ridl_register.h"), c_code)?;

    // 生成总的聚合符号文件
    let mut agg_symbols_content = "// Generated symbol references for RIDL extensions\n".to_string();
    for (module_name, functions, interfaces) in &all_module_symbols {
        for function in functions {
            // 使用与模板中相同的转换：转换为小写
            let func_name_lower = function.name.to_lowercase();
            agg_symbols_content.push_str(&format!("use crate::{}_glue::js_{};\n", module_name, func_name_lower));
        }
        for interface in interfaces {
            for method in &interface.methods {
                // 对于接口方法，也使用小写转换
                let interface_name_lower = interface.name.to_lowercase();
                let method_name_lower = method.name.to_lowercase();
                agg_symbols_content.push_str(&format!("use crate::{}_glue::js_{}_{};\n", 
                    interface_name_lower, interface_name_lower, method_name_lower));
            }
        }
    }
    
    agg_symbols_content.push_str("\n// Use all glue functions to ensure they're linked\n");
    agg_symbols_content.push_str("pub fn ensure_symbols() {\n");
    for (module_name, functions, interfaces) in &all_module_symbols {
        for function in functions {
            let func_name_lower = function.name.to_lowercase();
            agg_symbols_content.push_str(&format!("    let _ = js_{};\n", func_name_lower));
        }
        for interface in interfaces {
            for method in &interface.methods {
                let interface_name_lower = interface.name.to_lowercase();
                let method_name_lower = method.name.to_lowercase();
                agg_symbols_content.push_str(&format!("    let _ = js_{}_{};\n", 
                    interface_name_lower, method_name_lower));
            }
        }
    }
    agg_symbols_content.push_str("}\n");
    
    std::fs::write(
        std::path::Path::new(output_dir).join("ridl_symbols.rs"), 
        agg_symbols_content
    )?;

    Ok(())
}