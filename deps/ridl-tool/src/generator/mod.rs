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
#[template(path = "rust_impl.rs.j2")]
struct RustImplTemplate {
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
    return_type: Option<String>,

    glue_param_extract: String,
    glue_call_args: String,
    return_kind: String,
}

#[derive(Debug, Clone)]
struct TemplateParam {
    name: String,
    param_type: String,

    glue_extract: String,
    glue_arg: String,
}

#[derive(Debug, Clone)]
struct TemplateFunction {
    name: String,
    params: Vec<TemplateParam>,
    return_type: Option<String>,

    glue_param_extract: String,
    glue_call_args: String,
    return_kind: String,
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
        let return_type = if matches!(method.return_type, Type::Void) {
            None
        } else {
            Some(rust_type_name_for_codegen(&method.return_type))
        };

        let params: Vec<TemplateParam> = method
            .params
            .into_iter()
            .map(|p| TemplateParam::from_with_mode(p, file_mode))
            .collect();
        let glue_param_extract = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                p.glue_extract
                    .replace("{IDX}", &(i + 1).to_string())
                    .replace("{IDX0}", &i.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n");
        let glue_call_args = params
            .iter()
            .map(|p| p.glue_arg.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let return_kind = glue_return_kind(&method.return_type);

        Self {
            name: method.name,
            params,
            return_type,
            glue_param_extract,
            glue_call_args,
            return_kind,
        }
    }
}

impl TemplateParam {
    fn from_with_mode(param: Param, file_mode: crate::parser::FileMode) -> Self {
        let name = param.name;
        let ty = param.param_type.clone();
        let (glue_extract, glue_arg) = glue_param_snippet(&name, &ty, param.variadic, file_mode);

        Self {
            name,
            param_type: rust_type_name_for_codegen(&ty),
            glue_extract,
            glue_arg,
        }
    }
}

fn rust_type_name_for_codegen(ty: &Type) -> String {
    match ty {
        Type::Void => "()".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Int => "i32".to_string(),
        Type::Float | Type::Double => "f64".to_string(),
        Type::String => "*const std::os::raw::c_char".to_string(),
        Type::Any => "mquickjs_rs::mquickjs_ffi::JSValue".to_string(),
        _ => ty.to_string(),
    }
}

fn glue_param_snippet(
    name: &str,
    ty: &Type,
    variadic: bool,
    _file_mode: crate::parser::FileMode,
) -> (String, String) {
    let idx = "{IDX}";
    // NOTE: avoid nested format! strings that contain `{name}` because the outer Rust format! will
    // treat them as placeholders and emit warnings.
    let vararg_err_name = format!("{}[{}]", name, "{}");

    match ty {
        Type::String => {
            if variadic {
                return (
                    format!(
                        "let mut {name}: Vec<*const c_char> = Vec::new();\nfor i in {idx0}..(argc as usize) {{\n    let v = unsafe {{ *argv.add(i) }};\n    let rel = i - {idx0};\n    if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsString(ctx, v) }} == 0 {{ return js_throw_type_error(ctx, &format!(\"invalid string argument: {vararg_err_name}\", rel)); }}\n    let mut {name}_buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf {{ buf: [0u8; 5] }};\n    let ptr = unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, v, &mut {name}_buf as *mut _) }};\n    if ptr.is_null() {{ return js_throw_type_error(ctx, &format!(\"invalid string argument: {vararg_err_name}\", rel)); }}\n    {name}.push(ptr);\n}}",
                        name = name,
                        idx0 = "{IDX0}",
                        vararg_err_name = vararg_err_name
                    ),
                    name.to_string(),
                );
            }

            let check = format!(
                "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsString(ctx, unsafe {{ *argv.add({idx0}) }}) }} == 0 {{ return js_throw_type_error(ctx, \"invalid string argument: {name}\"); }}\n",
                idx0 = "{IDX0}",
                name = name
            );

            (
                format!(
                    "if argc < {idx} {{ return js_throw_type_error(ctx, \"missing argument: {name}\"); }}\n{check}let mut {name}_buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf {{ buf: [0u8; 5] }};\nlet {name}_ptr = unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, unsafe {{ *argv.add({idx0}) }}, &mut {name}_buf as *mut _) }};\nif {name}_ptr.is_null() {{ return js_throw_type_error(ctx, \"invalid string argument: {name}\"); }}\nlet {name}: *const c_char = {name}_ptr;",
                    idx = idx,
                    idx0 = "{IDX0}",
                    name = name,
                    check = check
                ),
                name.to_string(),
            )
        }
        Type::Int => {
            if variadic {
                return (
                    format!(
                        "let mut {name}: Vec<i32> = Vec::new();\nfor i in {idx0}..(argc as usize) {{\n    let v = unsafe {{ *argv.add(i) }};\n    let rel = i - {idx0};\n    if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsNumber(ctx, v) }} == 0 {{ return js_throw_type_error(ctx, &format!(\"invalid int argument: {vararg_err_name}\", rel)); }}\n    let mut out: i32 = 0;\n    if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToInt32(ctx, &mut out as *mut _, v) }} < 0 {{ return js_throw_type_error(ctx, &format!(\"invalid int argument: {vararg_err_name}\", rel)); }}\n    {name}.push(out);\n}}",
                        name = name,
                        idx0 = "{IDX0}",
                        vararg_err_name = vararg_err_name
                    ),
                    name.to_string(),
                );
            }

            (
                format!(
                    "if argc < {idx} {{ return js_throw_type_error(ctx, \"missing argument: {name}\"); }}\nlet mut {name}: i32 = 0;\n{check}if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToInt32(ctx, &mut {name} as *mut _, unsafe {{ *argv.add({idx0}) }}) }} < 0 {{ return js_throw_type_error(ctx, \"invalid int argument: {name}\"); }}",
                    idx = idx,
                    idx0 = "{IDX0}",
                    name = name,
                    check = format!(
                        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsNumber(ctx, unsafe {{ *argv.add({idx0}) }}) }} == 0 {{ return js_throw_type_error(ctx, \"invalid int argument: {name}\"); }}\n",
                        idx0 = "{IDX0}",
                        name = name
                    )
                ),
                name.to_string(),
            )
        }
        Type::Bool => {
            if variadic {
                return (
                    format!(
                        "let mut {name}: Vec<bool> = Vec::new();\nfor i in {idx0}..(argc as usize) {{\n    let rel = i - {idx0};\n    let v: u32 = unsafe {{ *argv.add(i) }} as u32;\n    if (v & ((1 << mquickjs_rs::mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1)) != 3 {{ return js_throw_type_error(ctx, &format!(\"invalid bool argument: {vararg_err_name}\", rel)); }}\n    {name}.push(v != 3);\n}}",
                        name = name,
                        idx0 = "{IDX0}",
                        vararg_err_name = vararg_err_name
                    ),
                    name.to_string(),
                );
            }

            (
                format!(
                    "if argc < {idx} {{ return js_throw_type_error(ctx, \"missing argument: {name}\"); }}\n\
// Only accept actual booleans in v1.
// bindings doesn't expose JS_IsBool; detect via NaN-boxed tag (JS_TAG_BOOL = 3).
let {name}_v: u32 = unsafe {{ *argv.add({idx0}) }} as u32;\n\
if ({name}_v & ((1 << mquickjs_rs::mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1)) != 3 {{ return js_throw_type_error(ctx, \"invalid bool argument: {name}\"); }}\n\
let {name}: bool = {name}_v != 3;",
                    idx = idx,
                    idx0 = "{IDX0}",
                    name = name
                ),
                name.to_string(),
            )
        }
        Type::Double | Type::Float => {
            if variadic {
                return (
                    format!(
                        "let mut {name}: Vec<f64> = Vec::new();\nfor i in {idx0}..(argc as usize) {{\n    let v = unsafe {{ *argv.add(i) }};\n    let rel = i - {idx0};\n    if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsNumber(ctx, v) }} == 0 {{ return js_throw_type_error(ctx, &format!(\"invalid double argument: {vararg_err_name}\", rel)); }}\n    let mut out: f64 = 0.0;\n    if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut out as *mut _, v) }} < 0 {{ return js_throw_type_error(ctx, &format!(\"invalid double argument: {vararg_err_name}\", rel)); }}\n    {name}.push(out);\n}}",
                        name = name,
                        idx0 = "{IDX0}",
                        vararg_err_name = vararg_err_name
                    ),
                    name.to_string(),
                );
            }

            (
                format!(
                    "if argc < {idx} {{ return js_throw_type_error(ctx, \"missing argument: {name}\"); }}\nlet mut {name}: f64 = 0.0;\n{check}if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut {name} as *mut _, unsafe {{ *argv.add({idx0}) }}) }} < 0 {{ return js_throw_type_error(ctx, \"invalid double argument: {name}\"); }}",
                    idx = idx,
                    idx0 = "{IDX0}",
                    name = name,
                    check = format!(
                        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsNumber(ctx, unsafe {{ *argv.add({idx0}) }}) }} == 0 {{ return js_throw_type_error(ctx, \"invalid double argument: {name}\"); }}\n",
                        idx0 = "{IDX0}",
                        name = name
                    )
                ),
                name.to_string(),
            )
        }
        Type::Any => {
            if variadic {
                return (
                    format!(
                        "let mut {name}: Vec<JSValue> = Vec::new();\nfor i in {idx0}..(argc as usize) {{\n    {name}.push(unsafe {{ *argv.add(i) }});\n}}",
                        name = name,
                        idx0 = "{IDX0}"
                    ),
                    name.to_string(),
                );
            }

            (
                format!(
                    "if argc < {idx} {{ return js_throw_type_error(ctx, \"missing argument: {name}\"); }}\n\
let {name}: JSValue = unsafe {{ *argv.add({idx0}) }};",
                    idx = idx,
                    idx0 = "{IDX0}",
                    name = name
                ),
                name.to_string(),
            )
        }
        _ => (
            format!(
                "compile_error!(\"v1 glue: unsupported param type '{ty}' for {name}\");",
                ty = ty,
                name = name
            ),
            name.to_string(),
        ),
    }
}

fn glue_return_kind(ty: &Type) -> String {
    match ty {
        Type::Void => "void".to_string(),
        Type::String => "string".to_string(),
        Type::Int => "int".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Double | Type::Float => "double".to_string(),
        Type::Any => "any".to_string(),
        _ => "unsupported".to_string(),
    }
}

impl TemplateFunction {
    fn from_with_mode(function: Function, file_mode: crate::parser::FileMode) -> Self {
        let return_type = if matches!(function.return_type, Type::Void) {
            None
        } else {
            Some(rust_type_name_for_codegen(&function.return_type))
        };

        let params: Vec<TemplateParam> = function
            .params
            .into_iter()
            .map(|p| TemplateParam::from_with_mode(p, file_mode))
            .collect();
        let glue_param_extract = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                p.glue_extract
                    .replace("{IDX}", &(i + 1).to_string())
                    .replace("{IDX0}", &i.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n");
        let glue_call_args = params
            .iter()
            .map(|p| p.glue_arg.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let return_kind = glue_return_kind(&function.return_type);

        Self {
            name: function.name,
            params,
            return_type,
            glue_param_extract,
            glue_call_args,
            return_kind,
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

    // 生成Rust实现骨架
    let rust_impl_template = RustImplTemplate {
        module_name: module_name.to_string(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
        singletons: rust_glue_template.singletons.clone(),
    };
    let rust_impl_code = rust_impl_template.render()?;
    std::fs::write(
        output_path.join(format!("{}_impl.rs", module_name)),
        rust_impl_code,
    )?;

    // 注意：模块命令只生成Rust胶水代码和实现骨架，其他文件在aggregate命令中生成

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
