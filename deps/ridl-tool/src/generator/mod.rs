use crate::parser::ast::{Function, IDLItem, Interface, Method, Param, Type, IDL};
use askama::Template;
use std::path::Path;

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
        let return_type = if matches!(method.return_type, Type::Void) {
            None
        } else {
            Some(rust_type_name_for_codegen(&method.return_type))
        };

        Self {
            name: method.name,
            params: method.params.into_iter().map(|p| p.into()).collect(),
            return_type,
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

fn rust_type_name_for_codegen(ty: &Type) -> String {
    match ty {
        Type::Void => "()".to_string(),
        // Keep this small and local: current demo needs `string` => `String`.
        // Other primitives already come through as Rust-like names.
        Type::String => "String".to_string(),
        _ => ty.to_string(),
    }
}

impl From<Function> for TemplateFunction {
    fn from(function: Function) -> Self {
        let return_type = if matches!(function.return_type, Type::Void) {
            None
        } else {
            Some(rust_type_name_for_codegen(&function.return_type))
        };

        Self {
            name: function.name,
            params: function.params.into_iter().map(|p| p.into()).collect(),
            return_type,
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
        let _callbacks: Vec<Function> = vec![]; // 回调作为函数处理
        let mut using = Vec::new();
        let mut imports = Vec::new();
        let mut singletons = Vec::new();
        let module = None;

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

pub fn generate_module_files(
    items: &[IDLItem],
    output_path: &Path,
    module_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut functions = Vec::new();
    let mut interfaces = Vec::new();

    for item in items {
        match item {
            crate::parser::ast::IDLItem::Function(f) => {
                functions.push(TemplateFunction::from(f.clone()))
            }
            crate::parser::ast::IDLItem::Interface(i) => {
                interfaces.push(TemplateInterface::from(i.clone()))
            }
            // 其他类型暂不处理，可根据需要添加
            _ => {}
        }
    }

    // 生成Rust胶水代码
    let rust_glue_template = RustGlueTemplate {
        module_name: module_name.to_string(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
    };
    let rust_glue_code = rust_glue_template.render()?;
    std::fs::write(
        output_path.join(format!("{}_glue.rs", module_name)),
        rust_glue_code,
    )?;

    // 生成Rust实现骨架
    let rust_impl_template = RustImplTemplate {
        module_name: module_name.to_string(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
    };
    let rust_impl_code = rust_impl_template.render()?;
    std::fs::write(
        output_path.join(format!("{}_impl.rs", module_name)),
        rust_impl_code,
    )?;

    // 注意：模块命令只生成Rust胶水代码和实现骨架，其他文件在aggregate命令中生成

    Ok(())
}

pub fn generate_module_api_file(
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let api = "// Generated module initializer API for RIDL extensions\n\
\
pub fn initialize_module() {\n\
    crate::generated::symbols::ensure_symbols();\n\
}\n";

    std::fs::write(out_dir.join("ridl_module_api.rs"), api)?;
    Ok(())
}

pub fn generate_shared_files(
    ridl_files: &[String],
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure aggregate header exists even when there are no RIDL modules.
    // mquickjs-build includes this header unconditionally.
    // IMPORTANT: JS_RIDL_EXTENSIONS must not reference any js_* symbols in this case.
    if ridl_files.is_empty() {
        std::fs::write(
            std::path::Path::new(output_dir).join("mquickjs_ridl_register.h"),
            "/* Generated by ridl-tool: no RIDL modules selected */\n#ifndef MQUICKJS_RIDL_REGISTER_H\n#define MQUICKJS_RIDL_REGISTER_H\n\n/* Hook used by mqjs_stdlib_template.c */\n#define JS_RIDL_EXTENSIONS /* empty */\n\n#endif\n",
        )?;
        return Ok(());
    }

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
                crate::parser::ast::IDLItem::Function(f) => {
                    functions.push(TemplateFunction::from(f))
                }
                crate::parser::ast::IDLItem::Interface(i) => {
                    interfaces.push(TemplateInterface::from(i))
                }
                // 其他类型暂不处理
                _ => {}
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
    std::fs::write(
        std::path::Path::new(output_dir).join("mquickjs_ridl_register.h"),
        c_code,
    )?;

    // 生成总的聚合符号文件
    let mut agg_symbols_content =
        "// Generated symbol references for RIDL extensions\n".to_string();
    agg_symbols_content.push_str("use mquickjs_rs::mquickjs_ffi::{JSContext, JSValue};\n");

    // extern 声明所有 js_* 符号（不 include glue，避免重复定义）
    for (_module_name, functions, interfaces) in &all_module_symbols {
        for function in functions {
            let func_name_lower = function.name.to_lowercase();
            agg_symbols_content.push_str(&format!("unsafe extern \"C\" {{ fn js_{func}(ctx: *mut JSContext, this_val: JSValue, argc: i32, argv: *mut JSValue) -> JSValue; }}\n", func=func_name_lower));
        }
        for interface in interfaces {
            for method in &interface.methods {
                let interface_name_lower = interface.name.to_lowercase();
                let method_name_lower = method.name.to_lowercase();
                agg_symbols_content.push_str(&format!("unsafe extern \"C\" {{ fn js_{iface}_{meth}(ctx: *mut JSContext, this_val: JSValue, argc: i32, argv: *mut JSValue) -> JSValue; }}\n",
                    iface=interface_name_lower, meth=method_name_lower));
            }
        }
    }

    // 通过 ensure_symbols 引用符号，防止裁剪
    agg_symbols_content.push_str("\npub fn ensure_symbols() {\n");
    for (_module_name, functions, interfaces) in &all_module_symbols {
        for function in functions {
            let func_name_lower = function.name.to_lowercase();
            agg_symbols_content.push_str(&format!("    let _ = js_{func_name_lower} as unsafe extern \"C\" fn(*mut JSContext, JSValue, i32, *mut JSValue) -> JSValue;\n", func_name_lower=func_name_lower));
        }
        for interface in interfaces {
            for method in &interface.methods {
                let interface_name_lower = interface.name.to_lowercase();
                let method_name_lower = method.name.to_lowercase();
                agg_symbols_content.push_str(&format!("    let _ = js_{iface}_{meth} as unsafe extern \"C\" fn(*mut JSContext, JSValue, i32, *mut JSValue) -> JSValue;\n",
                    iface=interface_name_lower, meth=method_name_lower));
            }
        }
    }
    agg_symbols_content.push_str("}\n");

    let out_dir = std::path::Path::new(output_dir);

    std::fs::write(out_dir.join("ridl_symbols.rs"), &agg_symbols_content)?;

    // Generate a small helper module that strongly references the selected module crates.
    // This ensures the rlibs that define js_* symbols are linked into the final binary.
    // NOTE: module list must be derived from the resolve plan (crate names), not ridl file stems.
    // `generate` command will assemble this file using plan.modules.

    Ok(())
}
