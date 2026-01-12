use crate::parser::ast::{Function, IDLItem, Interface, Method, Param, Type, IDL};
use askama::Template;
use std::path::Path;

// NOTE: kept for potential future use in codegen templates.
#[allow(dead_code)]
fn to_rust_type_ident_simple(name: &str) -> String {
    // Minimal PascalCase conversion for RIDL identifiers.
    let mut out = String::new();
    let mut upper = true;
    for ch in name.chars() {
        if ch == '_' || ch == '-' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        "Singleton".to_string()
    } else {
        out
    }
}

mod code_writer;
mod context_init;
mod filters;

pub use context_init::generate_ridl_context_init;

// singleton aggregation (Option A: erased slots)
pub mod singleton_aggregate;

#[derive(Template)]
#[template(path = "c_header.rs.j2")]
struct CHeaderTemplate {
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
    singletons: Vec<crate::parser::ast::Singleton>,
}

#[derive(Template)]
#[template(path = "rust_glue.rs.j2")]
struct RustGlueTemplate {
    #[allow(dead_code)]
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
    singletons: Vec<TemplateInterface>,
}

#[derive(Template)]
#[template(path = "rust_api.rs.j2")]
#[allow(dead_code)]
struct RustApiTemplate {
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
    singletons: Vec<TemplateInterface>,
}

#[derive(Template)]
#[template(path = "symbols.rs.j2")]
#[allow(dead_code)]
struct SymbolsTemplate {
    module_name: String,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
}

#[derive(Template)]
#[template(path = "aggregated_symbols.rs.j2")]
#[allow(dead_code)]
struct AggSymbolsTemplate {
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
}

#[derive(Template)]
#[template(path = "ridl_context_init.rs.j2")]
struct RidlContextInitTemplate {
    header_struct_name: String,
    singletons: Vec<context_init::TemplateSingletonVTable>,
}

#[derive(Debug, Clone)]
struct TemplateInterface {
    name: String,
    #[allow(dead_code)]
    slot_index: u32,
    methods: Vec<TemplateMethod>,
    properties: Vec<crate::parser::ast::Property>,
}

#[derive(Debug, Clone)]
struct TemplateMethod {
    name: String,
    params: Vec<TemplateParam>,
    return_type: Type,
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateParam {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) variadic: bool,
}

#[derive(Debug, Clone)]
struct TemplateFunction {
    name: String,
    params: Vec<TemplateParam>,
    return_type: Type,
}

impl TemplateInterface {
    fn from_with_mode(interface: Interface, file_mode: crate::parser::FileMode) -> Self {
        Self {
            name: interface.name,
            slot_index: 0,
            methods: interface
                .methods
                .into_iter()
                .map(|m| TemplateMethod::from_with_mode(m, file_mode))
                .collect(),
            properties: interface.properties,
        }
    }
}

impl TemplateMethod {
    fn from_with_mode(method: Method, file_mode: crate::parser::FileMode) -> Self {
        let params: Vec<TemplateParam> = method
            .params
            .into_iter()
            .map(|p| TemplateParam::from_with_mode(p, file_mode))
            .collect();

        Self {
            name: method.name,
            params,
            return_type: method.return_type,
        }
    }
}

impl TemplateParam {
    fn from_with_mode(param: Param, _file_mode: crate::parser::FileMode) -> Self {
        Self {
            name: param.name,
            ty: param.param_type,
            variadic: param.variadic,
        }
    }
}

impl TemplateFunction {
    fn from_with_mode(function: Function, file_mode: crate::parser::FileMode) -> Self {
        let params: Vec<TemplateParam> = function
            .params
            .into_iter()
            .map(|p| TemplateParam::from_with_mode(p, file_mode))
            .collect();

        Self {
            name: function.name,
            params,
            return_type: function.return_type,
        }
    }
}

#[allow(dead_code)]
pub fn collect_definitions(ridl_files: &[String]) -> Result<Vec<IDL>, Box<dyn std::error::Error>> {
    let mut all_definitions = Vec::new();

    for ridl_file in ridl_files {
        let content = std::fs::read_to_string(ridl_file)?;
        let parsed = crate::parser::parse_ridl_file(&content)?;
        let items = parsed.items;

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
    file_mode: crate::parser::FileMode,
    output_path: &Path,
    module_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut functions = Vec::new();
    let mut interfaces = Vec::new();

    for item in items {
        match item {
            crate::parser::ast::IDLItem::Function(f) => {
                functions.push(TemplateFunction::from_with_mode(f.clone(), file_mode))
            }
            crate::parser::ast::IDLItem::Interface(i) => {
                interfaces.push(TemplateInterface::from_with_mode(i.clone(), file_mode))
            }
            // 其他类型暂不处理，可根据需要添加
            _ => {}
        }
    }

    // 生成Rust胶水代码
    // NOTE: singletons are modelled as interface-like shapes for method glue generation.
    // Properties are handled separately.
    let mut singletons = Vec::new();
    for item in items {
        if let crate::parser::ast::IDLItem::Singleton(s) = item {
            singletons.push(TemplateInterface::from_with_mode(
                crate::parser::ast::Interface {
                    name: s.name.clone(),
                    methods: s.methods.clone(),
                    properties: s.properties.clone(),
                    module: None,
                },
                file_mode,
            ));
        }
    }

    let rust_glue_template = RustGlueTemplate {
        module_name: module_name.to_string(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
        singletons,
    };
    let rust_glue_code = rust_glue_template.render()?;
    std::fs::write(
        output_path.join(format!("{}_glue.rs", module_name)),
        rust_glue_code,
    )?;

    // 生成 Rust API（trait/类型声明），供用户 impl 层与 glue 层共享引用。
    // 注意：这里不生成任何 `todo!()` 实现骨架，避免误导用户编辑 OUT_DIR 生成物。
    let rust_api_template = RustApiTemplate {
        module_name: module_name.to_string(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
        singletons: rust_glue_template.singletons.clone(),
    };
    let rust_api_code = rust_api_template.render()?;
    std::fs::write(
        output_path.join(format!("{}_api.rs", module_name)),
        rust_api_code,
    )?;

    // 注意：模块命令只生成 Rust glue 与 API，其他文件在 aggregate 命令中生成

    Ok(())
}

#[allow(dead_code)]
pub fn generate_module_api_file_default(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let api = "// Generated module initializer API for RIDL extensions\n\
\n\
/// Ensure QuickJS C-side symbols for this module are registered.\n\
///\n\
/// NOTE: This is *not* the per-context singleton initialization.\n\
pub fn initialize_module() {\n\
    crate::generated::symbols::ensure_symbols();\n\
}\n\
\n\
/// Fill per-context RIDL extension slots for this module.\n\
/// Called by the app-level aggregated ridl_context_init.\n\
///\n\
/// This API must not reference any app crate types (e.g. app-owned `CtxExt`).\n\
pub fn ridl_module_context_init(w: &mut dyn mquickjs_rs::ridl_runtime::RidlSlotWriter) {\n\
    // If this module declares singletons, their constructors must be implemented\n\
    // in `crate::impls` (not a generated `todo!()` stub).\n\
    //\n\
    // Default behavior: do nothing.\n\
    let _ = w;\n\
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
        let parsed = crate::parser::parse_ridl_file(&content)?;
        let items = parsed.items;

        // 提取函数/接口/单例（stdlib 注入依赖 singleton）
        let mut functions = Vec::new();
        let mut interfaces = Vec::new();
        let mut singletons = Vec::new();

        for item in items {
            match item {
                crate::parser::ast::IDLItem::Function(f) => {
                    functions.push(TemplateFunction::from_with_mode(f, parsed.mode))
                }
                crate::parser::ast::IDLItem::Interface(i) => interfaces.push(
                    TemplateInterface::from_with_mode(i, crate::parser::FileMode::Default),
                ),
                crate::parser::ast::IDLItem::Singleton(s) => singletons.push(s),
                // 其他类型暂不处理
                _ => {}
            }
        }

        all_module_symbols.push((module_name, functions, interfaces, singletons));
    }

    // 生成聚合的C头文件
    let mut all_interfaces = Vec::new();
    let mut all_functions = Vec::new();

    let mut all_singletons = Vec::new();

    for (_, functions, interfaces, singletons) in &all_module_symbols {
        all_functions.extend(functions.clone());
        all_interfaces.extend(interfaces.clone());
        all_singletons.extend(singletons.clone());
    }

    let c_template = CHeaderTemplate {
        module_name: "mquickjs_ridl".to_string(),
        interfaces: all_interfaces.clone(),
        functions: all_functions.clone(),
        singletons: all_singletons,
    };
    let c_code = c_template.render()?;
    std::fs::write(
        std::path::Path::new(output_dir).join("mquickjs_ridl_register.h"),
        c_code,
    )?;

    // 生成总的聚合符号文件（extern 声明 + ensure_symbols 引用，避免 include glue 导致重复定义）
    let agg_symbols = AggSymbolsTemplate {
        interfaces: all_interfaces.clone(),
        functions: all_functions.clone(),
    };
    let agg_symbols_content = agg_symbols.render()?;

    let out_dir = std::path::Path::new(output_dir);

    std::fs::write(out_dir.join("ridl_symbols.rs"), &agg_symbols_content)?;

    // Generate a small helper module that strongly references the selected module crates.
    // This ensures the rlibs that define js_* symbols are linked into the final binary.
    // NOTE: module list must be derived from the resolve plan (crate names), not ridl file stems.
    // `generate` command will assemble this file using plan.modules.

    Ok(())
}
