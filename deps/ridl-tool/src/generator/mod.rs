use crate::parser::ast::{Class, DecoratorArg, Function, IDLItem, Interface, Method, Param, Type, IDL};

use union_types::collect_union_types;

fn apply_union_rust_ty_overrides(
    union_types: &[TemplateUnionType],
    tpl: &mut impl RustGlueLikeTemplate,
) {
    for itf in tpl.interfaces_mut().iter_mut() {
        for m in &mut itf.methods {
            let name = m.name.clone();
            apply_union_rust_ty_overrides_method(union_types, &name, m);
        }
    }

    for f in tpl.functions_mut().iter_mut() {
        let name = f.name.clone();
        apply_union_rust_ty_overrides_function(union_types, &name, f);
    }

    for s in tpl.singletons_mut().iter_mut() {
        for m in &mut s.methods {
            let name = m.name.clone();
            apply_union_rust_ty_overrides_method(union_types, &name, m);
        }
    }

    for c in tpl.classes_mut().iter_mut() {
        if let Some(ctor) = &mut c.constructor {
            let name = ctor.name.clone();
            apply_union_rust_ty_overrides_function(union_types, &name, ctor);
        }
        for m in &mut c.methods {
            let name = m.name.clone();
            apply_union_rust_ty_overrides_method(union_types, &name, m);
        }
    }
}

fn apply_union_rust_ty_overrides_function(
    union_types: &[TemplateUnionType],
    fn_name: &str,
    f: &mut TemplateFunction,
) {
    for p in &mut f.params {
        apply_union_rust_ty_overrides_param(union_types, fn_name, &p.name, &p.ty, &mut p.rust_ty);
    }

    // Return type: optionality comes from the type shape; we don't use the "Optional" label.
    apply_union_rust_ty_overrides_ty(
        union_types,
        fn_name,
        "",
        &f.return_type,
        &mut f.return_rust_ty,
    );
}

fn apply_union_rust_ty_overrides_method(
    union_types: &[TemplateUnionType],
    fn_name: &str,
    m: &mut TemplateMethod,
) {
    for p in &mut m.params {
        apply_union_rust_ty_overrides_param(union_types, fn_name, &p.name, &p.ty, &mut p.rust_ty);
    }

    // Return type label should not affect optionality; only the type shape should.
    apply_union_rust_ty_overrides_ty(
        union_types,
        fn_name,
        "",
        &m.return_type,
        &mut m.return_rust_ty,
    );
}

fn apply_union_rust_ty_overrides_param(
    union_types: &[TemplateUnionType],
    fn_name: &str,
    param_name: &str,
    ty: &Type,
    out_rust_ty: &mut String,
) {
    apply_union_rust_ty_overrides_ty(union_types, fn_name, param_name, ty, out_rust_ty);
}

fn apply_union_rust_ty_overrides_ty(
    union_types: &[TemplateUnionType],
    fn_name: &str,
    label: &str,
    ty: &Type,
    out_rust_ty: &mut String,
) {
    if !contains_union(ty) {
        return;
    }

    if let Some((enum_path, optional)) = union_enum_path_for_ty(union_types, fn_name, label, ty) {
        *out_rust_ty = if optional {
            format!("Option<{}>", enum_path)
        } else {
            enum_path
        };
        return;
    }

    // Fallback: union exists but we don't support generating a stable enum for this shape yet.
    // Keep previous rust_ty (likely derived from rust_type_from_idl) so callers can surface a
    // deterministic compile_error later.
}

fn contains_union(ty: &Type) -> bool {
    match ty {
        Type::Union(_) => true,
        Type::Optional(inner) => contains_union(inner),
        Type::Group(inner) => contains_union(inner),
        Type::Traced(inner) => contains_union(inner),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Phase D: named-type override pass
//
// 与 union 覆写同一范式：渲染前对方法/函数签名中的命名类型（同模块
// struct/enum）改写 rust_ty 为生成类型路径，并把解析出的类型形状挂到
// 模板节点上供转换 emitter 使用。
//
// 执行顺序契约：先 union 覆写、后 named-type 覆写（见 generate_module_files）。
// 两个 pass 的作用域天然不相交（union 覆写只改写含 Union 的类型；
// named-type 覆写只改写裸 Custom），顺序测试钉死在
// tests/generator_named_types_override_test.rs。
// ---------------------------------------------------------------------------

/// Classify where a named-type reference occurs inside `ty`.
enum NamedTypeRef<'a> {
    /// Bare `Custom(name)` resolving to a same-module struct/enum.
    Direct(&'a NamedTypeInfo),
    /// A resolvable named type appears in a nested position (array/optional/map/
    /// traced/union/group) — unsupported in v1, diagnosable error.
    Nested(&'a NamedTypeInfo, String),
    /// No resolvable named type inside `ty`.
    None,
}

fn classify_named_type_ref<'a>(ty: &Type, named_types: &'a [NamedTypeInfo]) -> NamedTypeRef<'a> {
    match ty {
        Type::Custom(name) => {
            // `(A | B)` shaped Custom strings are the union-in-Optional encoding
            // handled by the union pass; never treat them as named types.
            if name.starts_with('(') {
                return NamedTypeRef::None;
            }
            match named_types.iter().find(|n| n.ridl_name == *name) {
                Some(info) => NamedTypeRef::Direct(info),
                None => NamedTypeRef::None,
            }
        }
        Type::Optional(inner) => classify_nested(inner, named_types, "optional"),
        Type::Group(inner) => classify_nested(inner, named_types, "group"),
        Type::Traced(inner) => classify_nested(inner, named_types, "Traced"),
        Type::Array(inner) => classify_nested(inner, named_types, "array"),
        Type::Map(k, v) => {
            for (ty, pos) in [(k.as_ref(), "map key"), (v.as_ref(), "map value")] {
                if let NamedTypeRef::Direct(info) = classify_named_type_ref(ty, named_types) {
                    return NamedTypeRef::Nested(info, pos.to_string());
                }
            }
            NamedTypeRef::None
        }
        Type::Union(types) => {
            for t in types {
                if let NamedTypeRef::Direct(info) = classify_named_type_ref(t, named_types) {
                    return NamedTypeRef::Nested(info, "union member".to_string());
                }
            }
            NamedTypeRef::None
        }
        _ => NamedTypeRef::None,
    }
}

fn classify_nested<'a>(
    ty: &Type,
    named_types: &'a [NamedTypeInfo],
    position: &str,
) -> NamedTypeRef<'a> {
    match classify_named_type_ref(ty, named_types) {
        NamedTypeRef::Direct(info) => NamedTypeRef::Nested(info, position.to_string()),
        other => other,
    }
}

fn apply_named_type_rust_ty_overrides(
    named_types: &[NamedTypeInfo],
    tpl: &mut impl RustGlueLikeTemplate,
) -> Result<(), String> {
    for itf in tpl.interfaces_mut().iter_mut() {
        for m in &mut itf.methods {
            let name = m.name.clone();
            apply_named_type_overrides_method(named_types, &name, m)?;
        }
    }

    for f in tpl.functions_mut().iter_mut() {
        let name = f.name.clone();
        apply_named_type_overrides_function(named_types, &name, f)?;
    }

    for s in tpl.singletons_mut().iter_mut() {
        for m in &mut s.methods {
            let name = m.name.clone();
            apply_named_type_overrides_method(named_types, &name, m)?;
        }
    }

    for c in tpl.classes_mut().iter_mut() {
        if let Some(ctor) = &mut c.constructor {
            let name = ctor.name.clone();
            apply_named_type_overrides_function(named_types, &name, ctor)?;
        }
        for m in &mut c.methods {
            let name = m.name.clone();
            apply_named_type_overrides_method(named_types, &name, m)?;
        }
    }

    Ok(())
}

fn apply_named_type_overrides_function(
    named_types: &[NamedTypeInfo],
    fn_name: &str,
    f: &mut TemplateFunction,
) -> Result<(), String> {
    for p in &mut f.params {
        apply_named_type_overrides_param(named_types, fn_name, p)?;
    }
    apply_named_type_overrides_return(named_types, fn_name, &f.return_type, &mut f.return_rust_ty, &mut f.return_named)
}

fn apply_named_type_overrides_method(
    named_types: &[NamedTypeInfo],
    fn_name: &str,
    m: &mut TemplateMethod,
) -> Result<(), String> {
    for p in &mut m.params {
        apply_named_type_overrides_param(named_types, fn_name, p)?;
    }
    apply_named_type_overrides_return(named_types, fn_name, &m.return_type, &mut m.return_rust_ty, &mut m.return_named)
}

fn apply_named_type_overrides_param(
    named_types: &[NamedTypeInfo],
    fn_name: &str,
    p: &mut TemplateParam,
) -> Result<(), String> {
    // 方案外缺口（按方案精神补齐的可诊断拒绝）：`...rest: Address` 形式的
    // 命名类型变参不在 v1 范围（变参收集循环与命名类型提取不兼容）。
    if p.variadic {
        if let NamedTypeRef::Direct(info) = classify_named_type_ref(&p.ty, named_types) {
            return Err(format!(
                "named type '{}' (struct/enum) in fn '{fn_name}' variadic param '{}': varargs of named types are not supported in v1",
                info.ridl_name, p.name
            ));
        }
    }
    match classify_named_type_ref(&p.ty, named_types) {
        NamedTypeRef::Direct(info) => {
            p.rust_ty = info.rust_path.clone();
            p.named = Some(info.clone());
            Ok(())
        }
        NamedTypeRef::Nested(info, position) => Err(format!(
            "named type '{}' (struct/enum) in fn '{fn_name}' param '{}': named types are not supported inside {position} in v1 (use the named type directly as a parameter/return type)",
            info.ridl_name, p.name
        )),
        NamedTypeRef::None => Ok(()),
    }
}

fn apply_named_type_overrides_return(
    named_types: &[NamedTypeInfo],
    fn_name: &str,
    ty: &Type,
    out_rust_ty: &mut String,
    out_named: &mut Option<NamedTypeInfo>,
) -> Result<(), String> {
    match classify_named_type_ref(ty, named_types) {
        NamedTypeRef::Direct(info) => {
            *out_rust_ty = info.rust_path.clone();
            *out_named = Some(info.clone());
            Ok(())
        }
        NamedTypeRef::Nested(info, position) => Err(format!(
            "named type '{}' (struct/enum) in fn '{fn_name}' return type: named types are not supported inside {position} in v1 (use the named type directly as a parameter/return type)",
            info.ridl_name
        )),
        NamedTypeRef::None => Ok(()),
    }
}

fn union_enum_path_for_ty(
    union_types: &[TemplateUnionType],
    fn_name: &str,
    label: &str,
    ty: &Type,
) -> Option<(String, bool)> {
    match ty {
        Type::Optional(inner) => {
            // Optional(T) means nullable at the outer layer.
            // For unions we normalize nullability into Option<UnionEnum>.
            let (base, base_opt) = union_enum_path_for_ty(union_types, fn_name, label, inner)?;

            // Always optional due to outer Optional; if inner already implies optional, avoid double Option.
            Some((base, !base_opt))
        }
        Type::Custom(s) => {
            // Parser may represent `(A|B)` as Custom("(A | B)") inside Optional(...).
            // Normalize such cases by parsing the inner as a union.
            if s.starts_with('(') && s.ends_with(')') && s.contains('|') {
                let inner = &s[1..s.len() - 1];
                let mut keys: Vec<&'static str> = vec![];
                for part in inner.split('|') {
                    match part.trim() {
                        "string" => keys.push("String"),
                        "i32" => keys.push("I32"),
                        "i64" => keys.push("I64"),
                        "f32" => keys.push("F32"),
                        "f64" => keys.push("F64"),
                        _ => return None,
                    }
                }

                keys.sort();
                let name = format!("Union{}", keys.join(""));
                let u = union_types.iter().find(|u| u.name == name)?;
                let is_optional = label == "Optional";
                return Some((
                    format!("crate::api::{}::union::{}", u.domain, u.name),
                    is_optional,
                ));
            }
            None
        }
        Type::Group(inner) => {
            // Group(...) is only syntactic grouping.
            union_enum_path_for_ty(union_types, fn_name, label, inner)
        }
        Type::Traced(inner) => {
            // Recurse into Traced wrapper
            union_enum_path_for_ty(union_types, fn_name, label, inner)
        }
        Type::Union(types) => {
            let mut keys: Vec<&'static str> = vec![];
            let mut nullable = false;

            for t in types {
                match t {
                    Type::String => keys.push("String"),
                    Type::I32 => keys.push("I32"),
                    Type::I64 => keys.push("I64"),
                    Type::F32 => keys.push("F32"),
                    Type::F64 => keys.push("F64"),
                    Type::Null => nullable = true,
                    _ => {}
                }
            }
            keys.sort();
            keys.dedup();
            let name = if keys.is_empty() {
                "Union".to_string()
            } else {
                format!("Union{}", keys.join(""))
            };

            let u = union_types.iter().find(|u| u.name == name)?;
            // v1 semantic (strategy A): `T1 | T2 | null` is sugar for `(T1|T2)?`.
            // Also, `label=="Optional"` is used by outer Optional(...) wrapper for params.
            let is_optional = nullable || label == "Optional";
            Some((
                format!("crate::api::{}::union::{}", u.domain, u.name),
                is_optional,
            ))
        }
        _ => None,
    }
}

fn group_union_types_by_domain(union_types: Vec<TemplateUnionType>) -> Vec<TemplateUnionDomain> {
    let mut domains: Vec<String> = vec![];
    for u in &union_types {
        if !domains.iter().any(|d| d == &u.domain) {
            domains.push(u.domain.clone());
        }
    }

    let mut out: Vec<TemplateUnionDomain> = vec![];
    for d in domains {
        let unions = union_types
            .iter()
            .filter(|u| u.domain == d)
            .cloned()
            .collect::<Vec<_>>();
        out.push(TemplateUnionDomain { domain: d, unions });
    }
    out
}

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
mod filters;
mod naming;
mod union_types;

fn generate_register_h_and_symbols(
    ridl_files: &[String],
    output_dir: &str,
) -> Result<Option<AggregateIR>, Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(output_dir);

    // Ensure aggregate header exists even when there are no RIDL modules.
    // mquickjs-build includes this header unconditionally.
    // IMPORTANT: JS_RIDL_EXTENSIONS must not reference any js_* symbols in this case.
    // V1: We still emit an empty require-table so require() can exist and report "not found".
    if ridl_files.is_empty() {
        std::fs::write(
            out_dir.join("mquickjs_ridl_register.h"),
            // Same contract as the non-empty header: no RidlRequireEntry typedef
            // (the runtime shape lives in mquickjs_ridl_api.h) and no table
            // definition (provided by mquickjs_ridl_register.c).
            "/* Generated by ridl-tool: no RIDL modules selected */\n#ifndef MJS_RIDL_REGISTER_H\n#define MJS_RIDL_REGISTER_H\n\n/* File-scope declarations/definitions for RIDL extensions */\n#define JS_RIDL_DECLS /* empty */\n\n/* Hook used by mqjs_stdlib_template.c */\n#define JS_RIDL_EXTENSIONS /* empty */\n\n#endif /* MJS_RIDL_REGISTER_H */\n",
        )?;

        std::fs::write(
            out_dir.join("ridl_symbols.rs"),
            "// Generated by ridl-tool: no RIDL modules selected\n\n#[inline(always)]\npub fn ensure_symbols() {}\n",
        )?;

        // The C build (mquickjs-build --ridl-register-h) copies these siblings
        // from the aggregate dir unconditionally, so a module-less aggregate
        // must still emit them. Keep the class-id macro list argument-less and
        // keep js_ridl_require declared (require.c links against it).
        std::fs::write(
            out_dir.join("mquickjs_ridl_module_class_ids.h"),
            "/* Generated by ridl-tool: no RIDL modules selected */\n#ifndef MJS_RIDL_MODULE_CLASS_IDS_H\n#define MJS_RIDL_MODULE_CLASS_IDS_H\n\n#define MJS_RIDL_MODULE_CLASS_IDS(X)\n\n#endif /* MJS_RIDL_MODULE_CLASS_IDS_H */\n",
        )?;

        std::fs::write(
            out_dir.join("mquickjs_ridl_api.h"),
            // Mirrors the runtime section of the non-empty api.h template:
            // require.c (compiled whenever ridl-extensions is on) needs
            // js_ridl_require, the RidlRequireEntry shape and the extern table.
            "/* Generated by ridl-tool: no RIDL modules selected */\n#ifndef MJS_RIDL_API_H\n#define MJS_RIDL_API_H\n\n#include \"mquickjs.h\"\n\nJSValue js_ridl_require(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv);\n\ntypedef struct {\n    const char *module_full_name;\n    const char *module_base;\n    uint16_t v_major;\n    uint16_t v_minor;\n    uint16_t v_patch;\n\n    /* ensure_class_ids[0] must be module_class_id */\n    int module_class_id;\n    const int *ensure_class_ids;\n    int ensure_class_ids_len;\n} RidlRequireEntry;\n\nextern const RidlRequireEntry js_ridl_require_table[];\nextern const int js_ridl_require_table_len;\n\n#endif /* MJS_RIDL_API_H */\n",
        )?;

        return Ok(None);
    }

    // Parse ridl files as modules (1 file = 1 module). Module name defaults to GLOBAL.
    let mut modules: Vec<TemplateModule> = Vec::new();

    for ridl_file in ridl_files {
        let content = std::fs::read_to_string(ridl_file)?;
        let parsed = crate::parser::parse_ridl_file(&content)?;

        let module_name = parsed
            .module
            .as_ref()
            .map(|m| m.module_path.clone())
            .unwrap_or_else(|| "GLOBAL".to_string());

        let mut functions: Vec<TemplateFunction> = Vec::new();
        let mut interfaces: Vec<TemplateInterface> = Vec::new();
        let mut classes: Vec<TemplateClass> = Vec::new();
        let mut singletons: Vec<TemplateSingleton> = Vec::new();

        for item in parsed.items {
            match item {
                crate::parser::ast::IDLItem::Function(mut f) => {
                    f.module = parsed.module.clone();
                    let ridl_module_name = f
                        .module
                        .as_ref()
                        .map(|m| m.module_path.clone())
                        .unwrap_or_else(|| "GLOBAL".to_string());
                    let module_name_normalized =
                        crate::generator::filters::normalize_ident(&ridl_module_name)
                            .unwrap_or_else(|_| "GLOBAL".to_string());
                    functions.push(TemplateFunction::from_with_mode(
                        f,
                        parsed.mode,
                        module_name_normalized,
                    ))
                }
                crate::parser::ast::IDLItem::Interface(mut i) => {
                    i.module = parsed.module.clone();
                    let ridl_module_name = i
                        .module
                        .as_ref()
                        .map(|m| m.module_path.clone())
                        .unwrap_or_else(|| "GLOBAL".to_string());
                    let module_name_normalized =
                        crate::generator::filters::normalize_ident(&ridl_module_name)
                            .unwrap_or_else(|_| "GLOBAL".to_string());
                    interfaces.push(TemplateInterface::from_with_mode(
                        i,
                        parsed.mode,
                        module_name_normalized,
                    ))
                }
                crate::parser::ast::IDLItem::Singleton(mut s) => {
                    s.module = parsed.module.clone();
                    let module_name = s
                        .module
                        .as_ref()
                        .map(|m| m.module_path.as_str())
                        .unwrap_or("GLOBAL")
                        .to_string();
                    singletons.push(TemplateSingleton::from_ast(s, module_name, parsed.mode))
                }
                crate::parser::ast::IDLItem::Class(c) => {
                    let module_name_normalized =
                        crate::generator::filters::normalize_ident(&module_name)
                            .unwrap_or_else(|_| "GLOBAL".to_string());
                    classes.push(TemplateClass::from_with_mode(
                        module_name.clone(),
                        module_name_normalized,
                        c,
                        parsed.mode,
                    ))
                }
                _ => {}
            }
        }

        let (require_full_name, require_base, require_v_major, require_v_minor, require_v_patch) =
            if let Some(m) = &parsed.module {
                let ver = m
                    .version
                    .as_ref()
                    .expect("module version is required by parser");
                let v = crate::parser::require_spec::Version::parse_no_ws(ver)
                    .expect("module version is validated by parser");
                (
                    format!("{}@{}", m.module_path, ver),
                    m.module_path.clone(),
                    v.major,
                    v.minor,
                    v.patch,
                )
            } else {
                (String::new(), String::new(), 0, 0, 0)
            };

        modules.push(TemplateModule {
            module_name,
            module_decl: parsed.module,
            file_mode: parsed.mode,
            require_full_name,
            require_base,
            require_v_major,
            require_v_minor,
            require_v_patch,
            module_class_id: 0,
            interfaces,
            functions,
            singletons,
            classes,
        });
    }

    // Assign a global, monotonic class_id across all modules in this app aggregate.
    // This matches the ROM/build expectation: JS class ids are allocated as
    // JS_CLASS_USER + i (i in [0, JS_CLASS_COUNT)).
    let mut next_class_id: u32 = 0;

    // Allocate module object class ids first (stable across modules ordering).
    for m in &mut modules {
        if m.module_decl.is_some() {
            m.module_class_id = next_class_id;
            next_class_id += 1;
        }
    }

    // Allocate user class ids.
    let mut classes: Vec<TemplateClass> = Vec::new();
    for m in &mut modules {
        for c in &mut m.classes {
            c.class_id = next_class_id;
            next_class_id += 1;
        }
        classes.extend(m.classes.iter().cloned());
    }

    // Generate RIDL register headers:
    // - mquickjs_ridl_api.h: declarations only (public for runtime compilation)
    // - mquickjs_ridl_register.h: definitions for ROM build injection (host tool)
    let ridl_register_all = MquickjsRidlRegisterHeaderTemplate {
        module_name: "global".to_string(),
        modules: modules.clone(),
        classes: classes.clone(),
        next_class_id,
    };

    std::fs::write(
        out_dir.join("mquickjs_ridl_api.h"),
        MquickjsRidlApiHeaderTemplate {
            modules: ridl_register_all.modules.clone(),
            next_class_id: ridl_register_all.next_class_id,
        }
        .render()?,
    )?;

    std::fs::write(
        out_dir.join("mquickjs_ridl_register.h"),
        MquickjsRidlRegisterHeaderTemplate {
            module_name: ridl_register_all.module_name.clone(),
            modules: ridl_register_all.modules.clone(),
            classes: ridl_register_all.classes.clone(),
            next_class_id: ridl_register_all.next_class_id,
        }
        .render()?,
    )?;

    std::fs::write(
        out_dir.join("mquickjs_ridl_module_class_ids.h"),
        MquickjsRidlModuleClassIdsHeaderTemplate {
            modules: ridl_register_all.modules.clone(),
        }
        .render()?,
    )?;

    // NOTE: mquickjs_ridl_register.c is generated from the aggregate plan (needs ridl_files).

    // Aggregated symbols (extern declarations + keep-alive references).
    let agg_symbols = AggSymbolsTemplate { modules };

    std::fs::write(out_dir.join("ridl_symbols.rs"), agg_symbols.render()?)?;

    Ok(Some(AggregateIR {
        classes: ridl_register_all.classes,
    }))
}

// singleton aggregation (Option A: erased slots)
pub mod singleton_aggregate;

mod template_modules;
use template_modules::build_template_modules;

mod mquickjs_register_c;
use mquickjs_register_c::generate_mquickjs_ridl_register_c;

#[derive(Debug, Clone)]
pub(super) struct AggregateIR {
    pub(super) classes: Vec<TemplateClass>,
}

#[derive(Template)]
#[template(path = "mquickjs_ridl_register.h.j2", escape = "none")]
struct MquickjsRidlRegisterHeaderTemplate {
    // Used only for stdlib macro namespace (JS_STDLIB_EXTENSIONS_<...>).
    module_name: String,
    modules: Vec<TemplateModule>,
    // Flattened classes for templates that need global counts.
    classes: Vec<TemplateClass>,
    // Total count of allocated JS class ids (module objects + user classes).
    next_class_id: u32,
}

#[derive(Template)]
#[template(path = "mquickjs_ridl_api.h.j2", escape = "none")]
struct MquickjsRidlApiHeaderTemplate {
    modules: Vec<TemplateModule>,
    next_class_id: u32,
}

#[derive(Template)]
#[template(path = "mquickjs_ridl_module_class_ids.h.j2", escape = "none")]
struct MquickjsRidlModuleClassIdsHeaderTemplate {
    modules: Vec<TemplateModule>,
}

trait RustGlueLikeTemplate {
    fn interfaces_mut(&mut self) -> &mut Vec<TemplateInterface>;
    fn functions_mut(&mut self) -> &mut Vec<TemplateFunction>;
    fn singletons_mut(&mut self) -> &mut Vec<TemplateSingleton>;
    fn classes_mut(&mut self) -> &mut Vec<TemplateClass>;
}

#[derive(Template)]
// escape = "none": every interpolation in this template is generated code
// (identifiers, numeric literals, pre-escaped string literals via the
// escape_rust_string filter). Askama's default HTML escaping would silently
// corrupt emitted string literals (`"` -> `&quot;`) while still compiling.
#[template(path = "rust_glue.rs.j2", escape = "none")]
struct RustGlueTemplate {
    #[allow(dead_code)]
    module_name: String,
    #[allow(dead_code)]
    module_decl: Option<crate::parser::ast::ModuleDeclaration>,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
    singletons: Vec<TemplateSingleton>,
    classes: Vec<TemplateClass>,
}

impl RustGlueLikeTemplate for RustGlueTemplate {
    fn interfaces_mut(&mut self) -> &mut Vec<TemplateInterface> {
        &mut self.interfaces
    }
    fn functions_mut(&mut self) -> &mut Vec<TemplateFunction> {
        &mut self.functions
    }
    fn singletons_mut(&mut self) -> &mut Vec<TemplateSingleton> {
        &mut self.singletons
    }
    fn classes_mut(&mut self) -> &mut Vec<TemplateClass> {
        &mut self.classes
    }
}

#[derive(Template)]
#[template(path = "rust_api.rs.j2", escape = "none")]
#[allow(dead_code)]
struct RustApiTemplate {
    module_name: String,
    module_decl: Option<crate::parser::ast::ModuleDeclaration>,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
    singletons: Vec<TemplateSingleton>,
    classes: Vec<TemplateClass>,
    enums: Vec<TemplateEnum>,
    structs: Vec<TemplateStruct>,
    using_aliases: Vec<TemplateUsingAlias>,

    union_types_by_domain: Vec<TemplateUnionDomain>,
}

impl RustGlueLikeTemplate for RustApiTemplate {
    fn interfaces_mut(&mut self) -> &mut Vec<TemplateInterface> {
        &mut self.interfaces
    }
    fn functions_mut(&mut self) -> &mut Vec<TemplateFunction> {
        &mut self.functions
    }
    fn singletons_mut(&mut self) -> &mut Vec<TemplateSingleton> {
        &mut self.singletons
    }
    fn classes_mut(&mut self) -> &mut Vec<TemplateClass> {
        &mut self.classes
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TemplateUnionDomain {
    domain: String,
    unions: Vec<TemplateUnionType>,
}

#[derive(Debug, Clone)]
struct TemplateUnionType {
    /// The Rust module path for the type domain (global/module).
    /// e.g. `global` or `foo_bar`
    domain: String,

    /// A short PascalCase name within the union namespace.
    /// e.g. `EchoStringOrInt`
    name: String,

    members: Vec<TemplateUnionMember>,
}

#[derive(Debug, Clone)]
struct TemplateUnionMember {
    variant: String,
    rust_ty: String,
}

#[derive(Template)]
#[template(path = "ridl_symbols.rs.j2")]
#[allow(dead_code)]
struct AggSymbolsTemplate {
    modules: Vec<TemplateModule>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct TemplateModule {
    pub(super) module_name: String,
    module_decl: Option<crate::parser::ast::ModuleDeclaration>,
    file_mode: crate::parser::FileMode,

    // For require-table generation (only meaningful when module_decl.is_some()).
    require_full_name: String,
    require_base: String,
    require_v_major: u16,
    require_v_minor: u16,
    require_v_patch: u16,

    // JS class id for the module instance object (allocated globally).
    module_class_id: u32,

    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
    singletons: Vec<TemplateSingleton>,
    pub(super) classes: Vec<TemplateClass>,
}

#[derive(Debug, Clone)]
struct TemplateSingleton {
    name: String,
    module_name: String,
    module_name_normalized: String,
    methods: Vec<TemplateMethod>,
    properties: Vec<crate::parser::ast::Property>,
    /// JS-only `var` fields installed by JS_RIDL_StdlibInit (GLOBAL mode only).
    /// Never rendered by the Rust glue/api templates: glue has no access to the
    /// singleton's JSValue (ctx-slot dispatch only).
    js_fields: Vec<TemplateJsField>,
}

impl TemplateSingleton {
    fn from_ast(
        s: crate::parser::ast::Singleton,
        module_name: String,
        file_mode: crate::parser::FileMode,
    ) -> Self {
        let module_name_normalized = crate::generator::filters::normalize_ident(&module_name)
            .unwrap_or_else(|_| "GLOBAL".to_string());
        TemplateSingleton {
            name: s.name,
            module_name_normalized,
            module_name,
            methods: s
                .methods
                .into_iter()
                .map(|m| TemplateMethod::from_with_mode(m, file_mode))
                .collect(),
            properties: s.properties,
            js_fields: s
                .js_fields
                .into_iter()
                .map(|f| TemplateJsField {
                    name: f.name,
                    field_type: f.field_type,
                    init_literal: f.init_literal,
                    is_proto: f
                        .modifiers
                        .contains(&crate::parser::ast::PropertyModifier::Proto),
                    kind: f.kind,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct TemplateInterface {
    name: String,
    module_name_normalized: String,
    #[allow(dead_code)]
    slot_index: u32,
    methods: Vec<TemplateMethod>,
    #[allow(dead_code)]
    properties: Vec<crate::parser::ast::Property>,
}

#[derive(Debug, Clone)]
pub(super) struct TemplateClass {
    pub(super) name: String,
    pub(super) module_name: String,
    pub(super) module_name_normalized: String,
    pub(super) class_id: u32,
    constructor: Option<TemplateFunction>,
    methods: Vec<TemplateMethod>,
    properties: Vec<crate::parser::ast::Property>,
    js_fields: Vec<TemplateJsField>,
    pub(super) opaque_fields: Vec<TemplateOpaqueField>,
}

#[derive(Debug, Clone)]
struct TemplateJsField {
    name: String,
    field_type: crate::parser::ast::Type,
    init_literal: String,
    is_proto: bool,
    kind: crate::parser::ast::JsFieldKind,
}

#[derive(Debug, Clone)]
pub(super) struct TemplateOpaqueField {
    pub(super) name: String,
    pub(super) field_type: crate::parser::ast::Type,
}

#[derive(Debug, Clone)]
struct TemplateMethod {
    name: String,
    params: Vec<TemplateParam>,
    return_type: Type,
    return_rust_ty: String,
    /// Phase D: named-type info when the return type is a bare struct/enum ref.
    return_named: Option<NamedTypeInfo>,
    has_variadic: bool,
    needs_scope: bool,
    decorators: Vec<TemplateDecorator>,
}

#[derive(Debug, Clone)]
struct TemplateDecorator {
    name: String,
    args: Vec<TemplateDecoratorArg>,
}

#[derive(Debug, Clone)]
enum TemplateDecoratorArg {
    Integer(i64),
    String(String),
}

impl std::fmt::Display for TemplateDecoratorArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateDecoratorArg::Integer(i) => write!(f, "{}", i),
            TemplateDecoratorArg::String(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateParam {
    pub(crate) name: String,
    pub(crate) rust_name: String,
    pub(crate) ty: Type,
    pub(crate) variadic: bool,
    pub(crate) file_mode: crate::parser::FileMode,

    // Filled during template construction.
    // For union types this will be a fully qualified path under `crate::api::{domain}::union::*`.
    pub(crate) rust_ty: String,

    /// Phase D: named-type info when this param's RIDL type is a bare struct/enum
    /// reference resolved by the named-type override pass (bare `Type::Custom`).
    /// Filled by `apply_named_type_rust_ty_overrides`; the conversion emitters
    /// (filters.rs) switch on this to generate struct/enum conversion code.
    pub(crate) named: Option<NamedTypeInfo>,
}

/// Phase D: resolved named-type description carried on template nodes so the
/// conversion emitters can generate struct/enum JS<->Rust code without touching
/// stateless filter signatures (mirrors how union overrides rewrite rust_ty).
#[derive(Debug, Clone)]
pub(crate) struct NamedTypeInfo {
    /// Original RIDL name (e.g. `Address`).
    pub(crate) ridl_name: String,
    /// Generated Rust path (e.g. `crate::api::Address`).
    pub(crate) rust_path: String,
    pub(crate) shape: NamedTypeShape,
}

#[derive(Debug, Clone)]
pub(crate) enum NamedTypeShape {
    /// Struct with its fields (in RIDL declaration order).
    Struct(Vec<TemplateStructField>),
    /// C-like enum: (RIDL raw variant name, Rust PascalCase variant name).
    /// JS string form is the RIDL raw name (e.g. "RED").
    Enum(Vec<(String, String)>),
}

#[derive(Debug, Clone)]
struct TemplateEnum {
    name: String,
    variants: Vec<TemplateEnumVariant>,
}

#[derive(Debug, Clone)]
struct TemplateEnumVariant {
    name: String,
    value: Option<i32>,
}

#[derive(Debug, Clone)]
struct TemplateStruct {
    name: String,
    fields: Vec<TemplateStructField>,
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateStructField {
    name: String,
    rust_name: String,
    ty: Type,
    rust_ty: String,
    /// Phase D: resolved shape for a nested struct field (`Custom` field) or
    /// for the element type of an `array<struct>` field. None for
    /// primitive/string/array<primitive> fields (conversion emitters use this
    /// instead of reaching a stateful named-type table).
    nested: Option<Box<NamedTypeInfo>>,
}

#[derive(Debug, Clone)]
struct TemplateUsingAlias {
    name: String,
    rust_ty: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TemplateFunction {
    name: String,
    module_name_normalized: String,
    params: Vec<TemplateParam>,
    return_type: Type,
    return_rust_ty: String,
    /// Phase D: named-type info when the return type is a bare struct/enum ref.
    return_named: Option<NamedTypeInfo>,
}

impl TemplateInterface {
    fn from_with_mode(
        interface: Interface,
        file_mode: crate::parser::FileMode,
        module_name_normalized: String,
    ) -> Self {
        Self {
            name: interface.name,
            module_name_normalized,
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

        let has_variadic = params.iter().any(|p| p.variadic);

        fn is_any_like(ty: &Type) -> bool {
            matches!(ty, Type::Any)
                || matches!(ty, Type::Optional(inner) if matches!(inner.as_ref(), Type::Any))
        }

        let needs_scope = params.iter().any(|p| is_any_like(&p.ty))
            || (has_variadic && params.iter().any(|p| p.variadic && is_any_like(&p.ty)))
            || is_any_like(&method.return_type);

        let return_type = method.return_type;
        let mut return_rust_ty = crate::generator::filters::rust_type_from_idl(&return_type)
            .unwrap_or_else(|_| {
                // Fall back to unit only for truly unsupported types.
                // `Type::Custom` should not silently become `()`, because it can hide bugs.
                "()".to_string()
            });

        // Object-safety rule: `any` return must not carry call-site lifetimes.
        // Keep `any` param as borrowed `Local<'_, Value>`, but map `any` return to owned `ReturnAny`.
        if matches!(return_type, Type::Any) {
            return_rust_ty = "mquickjs_rs::handles::return_safe::ReturnAny".to_string();
        }
        if matches!(return_type, Type::Optional(ref inner) if matches!(inner.as_ref(), Type::Any)) {
            return_rust_ty = "Option<mquickjs_rs::handles::return_safe::ReturnAny>".to_string();
        }

        // Convert decorators
        let decorators: Vec<TemplateDecorator> = method
            .decorators
            .into_iter()
            .map(|d| TemplateDecorator {
                name: d.name,
                args: d
                    .args
                    .into_iter()
                    .map(|a| match a {
                        DecoratorArg::Integer(i) => TemplateDecoratorArg::Integer(i),
                        DecoratorArg::String(s) => TemplateDecoratorArg::String(s),
                    })
                    .collect(),
            })
            .collect();

        Self {
            name: method.name,
            params,
            return_type,
            return_rust_ty,
            return_named: None,
            has_variadic,
            needs_scope,
            decorators,
        }
    }
}

impl TemplateParam {
    fn from_with_mode(param: Param, file_mode: crate::parser::FileMode) -> Self {
        let ty = param.param_type;
        let rust_ty =
            crate::generator::filters::rust_type_from_idl(&ty).unwrap_or_else(|_| "()".to_string());
        let rust_name = crate::generator::filters::rust_ident(
            &crate::generator::naming::to_snake_case(&param.name),
        )
        .unwrap_or_else(|_| "_".to_string());

        Self {
            name: param.name,
            rust_name,
            ty,
            variadic: param.variadic,
            file_mode,
            rust_ty,
            named: None,
        }
    }
}

impl TemplateFunction {
    fn from_with_mode(
        function: Function,
        file_mode: crate::parser::FileMode,
        module_name_normalized: String,
    ) -> Self {
        let params: Vec<TemplateParam> = function
            .params
            .into_iter()
            .map(|p| TemplateParam::from_with_mode(p, file_mode))
            .collect();

        let return_type = function.return_type;
        let return_rust_ty = crate::generator::filters::rust_type_from_idl(&return_type)
            .unwrap_or_else(|_| "()".to_string());

        Self {
            name: function.name,
            module_name_normalized,
            params,
            return_type,
            return_rust_ty,
            return_named: None,
        }
    }
}

impl TemplateClass {
    fn from_with_mode(
        module_name: String,
        module_name_normalized: String,
        class: Class,
        file_mode: crate::parser::FileMode,
    ) -> Self {
        let module_name_normalized_cloned = module_name_normalized.clone();

        Self {
            module_name,
            module_name_normalized,
            name: class.name,
            class_id: 0,
            constructor: class.constructor.map(|c| {
                TemplateFunction::from_with_mode(
                    c,
                    file_mode,
                    module_name_normalized_cloned.clone(),
                )
            }),
            methods: class
                .methods
                .into_iter()
                .map(|m| TemplateMethod::from_with_mode(m, file_mode))
                .collect(),
            properties: class.properties,
            js_fields: class
                .js_fields
                .into_iter()
                .map(|f| TemplateJsField {
                    name: f.name,
                    field_type: f.field_type,
                    init_literal: f.init_literal,
                    is_proto: f
                        .modifiers
                        .contains(&crate::parser::ast::PropertyModifier::Proto),
                    kind: f.kind,
                })
                .collect(),
            opaque_fields: class
                .opaque_fields
                .into_iter()
                .map(|f| TemplateOpaqueField {
                    name: f.name,
                    field_type: f.field_type,
                })
                .collect(),
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
                crate::parser::ast::IDLItem::Singleton(mut s) => {
                    // In aggregate mode, singletons inherit file-level module decl.
                    s.module = module.clone();
                    singletons.push(s)
                }
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
    module_decl: Option<crate::parser::ast::ModuleDeclaration>,
    file_mode: crate::parser::FileMode,
    output_path: &Path,
    module_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut functions = Vec::new();
    let mut interfaces = Vec::new();
    let mut classes = Vec::new();
    let mut enums = Vec::new();
    let mut structs = Vec::new();
    let mut using_aliases = Vec::new();

    // Phase D: pre-collect same-module named-type names so struct fields can
    // resolve nested struct references regardless of definition order.
    let struct_names: std::collections::HashSet<String> = items
        .iter()
        .filter_map(|it| match it {
            crate::parser::ast::IDLItem::Struct(s) => Some(s.name.clone()),
            _ => None,
        })
        .collect();
    let enum_names: std::collections::HashSet<String> = items
        .iter()
        .filter_map(|it| match it {
            crate::parser::ast::IDLItem::Enum(e) => Some(e.name.clone()),
            _ => None,
        })
        .collect();

    for item in items {
        match item {
            crate::parser::ast::IDLItem::Function(f) => {
                let ridl_module_name = module_decl
                    .as_ref()
                    .map(|m| m.module_path.as_str())
                    .unwrap_or("GLOBAL");
                functions.push(TemplateFunction::from_with_mode(
                    f.clone(),
                    file_mode,
                    crate::generator::filters::normalize_ident(ridl_module_name)
                        .unwrap_or_else(|_| "GLOBAL".to_string()),
                ))
            }
            crate::parser::ast::IDLItem::Interface(i) => {
                let ridl_module_name = module_decl
                    .as_ref()
                    .map(|m| m.module_path.as_str())
                    .unwrap_or("GLOBAL");
                interfaces.push(TemplateInterface::from_with_mode(
                    i.clone(),
                    file_mode,
                    crate::generator::filters::normalize_ident(ridl_module_name)
                        .unwrap_or_else(|_| "GLOBAL".to_string()),
                ))
            }
            crate::parser::ast::IDLItem::Class(c) => {
                let ridl_module_name = module_decl
                    .as_ref()
                    .map(|m| m.module_path.as_str())
                    .unwrap_or("GLOBAL");
                let module_name_normalized =
                    crate::generator::filters::normalize_ident(ridl_module_name)
                        .unwrap_or_else(|_| "GLOBAL".to_string());
                classes.push(TemplateClass::from_with_mode(
                    ridl_module_name.to_string(),
                    module_name_normalized,
                    c.clone(),
                    file_mode,
                ))
            }
            crate::parser::ast::IDLItem::Enum(e) => {
                enums.push(TemplateEnum {
                    name: e.name.clone(),
                    variants: e
                        .values
                        .iter()
                        .map(|v| TemplateEnumVariant {
                            name: v.name.clone(),
                            value: v.value,
                        })
                        .collect(),
                });
            }
            crate::parser::ast::IDLItem::Struct(s) => {
                structs.push(build_template_struct(s, &struct_names, &enum_names)?);
            }
            crate::parser::ast::IDLItem::Using(u) => {
                let rust_ty = crate::generator::filters::rust_type_from_idl(&u.alias_type)
                    .unwrap_or_else(|_| "JSValue".to_string());
                using_aliases.push(TemplateUsingAlias {
                    name: u.name.clone(),
                    rust_ty,
                });
            }
            // 其他类型暂不处理，可根据需要添加
            _ => {}
        }
    }

    // 生成Rust胶水代码
    // NOTE: singletons are modelled as interface-like shapes for method glue generation.
    // We keep the original singleton name (`s.name`) so templates can generate stable VT symbol
    // names (RIDL_<NAME>_CTX_SLOT_VT) and enforce the `ridl_create_<name>_singleton` contract.
    let mut singletons = Vec::new();
    for item in items {
        if let crate::parser::ast::IDLItem::Singleton(s) = item {
            let singleton_module_name = module_decl
                .as_ref()
                .map(|m| m.module_path.as_str())
                .unwrap_or("GLOBAL")
                .to_string();
            singletons.push(TemplateSingleton::from_ast(
                s.clone(),
                singleton_module_name,
                file_mode,
            ));
        }
    }

    let mut rust_glue_template = RustGlueTemplate {
        module_name: module_name.to_string(),
        module_decl,
        interfaces: interfaces.clone(),
        functions: functions.clone(),
        singletons,
        classes: classes.clone(),
    };

    let union_types = collect_union_types(
        &module_name.to_string(),
        rust_glue_template.module_decl.clone(),
        &interfaces,
        &rust_glue_template.functions,
        &rust_glue_template.singletons,
        &classes,
    )?;
    // Phase D order contract: union overrides FIRST, named-type overrides SECOND.
    apply_union_rust_ty_overrides(&union_types, &mut rust_glue_template);

    let named_types = build_named_type_table(&enums, &structs)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    apply_named_type_rust_ty_overrides(&named_types, &mut rust_glue_template)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let rust_glue_code = rust_glue_template.render()?;
    std::fs::write(output_path.join("glue.rs"), rust_glue_code)?;

    // 生成 Rust API（trait/类型声明），供用户 impl 层与 glue 层共享引用。
    // 注意：这里不生成任何 `todo!()` 实现骨架，避免误导用户编辑 OUT_DIR 生成物。
    let union_types = union_types;

    let mut rust_api_template = RustApiTemplate {
        module_name: module_name.to_string(),
        module_decl: rust_glue_template.module_decl.clone(),
        interfaces: interfaces.clone(),
        functions: functions.clone(),
        singletons: rust_glue_template.singletons.clone(),
        classes: classes.clone(),
        enums: enums.clone(),
        structs: structs.clone(),
        using_aliases: using_aliases.clone(),
        union_types_by_domain: group_union_types_by_domain(union_types.clone()),
    };

    // Keep API trait signatures consistent with glue by applying the same union
    // overrides, then the same named-type overrides (same order as glue).
    apply_union_rust_ty_overrides(&union_types, &mut rust_api_template);
    apply_named_type_rust_ty_overrides(&named_types, &mut rust_api_template)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let rust_api_code = rust_api_template.render()?;
    std::fs::write(output_path.join("api.rs"), rust_api_code)?;

    // 注意：模块命令只生成 Rust glue 与 API，其他文件在 aggregate 命令中生成

    Ok(())
}

// ---------------------------------------------------------------------------
// Phase D: named-type table + struct field rejection matrix
// ---------------------------------------------------------------------------

/// Build the per-module named-type table consumed by the override pass.
///
/// `structs` must already have passed the struct-field rejection matrix (see
/// `build_template_struct`). Nested struct shapes are resolved here with
/// memoization + cycle detection and stored on `TemplateStructField::nested`
/// so the conversion emitters stay stateless.
fn build_named_type_table(
    enums: &[TemplateEnum],
    structs: &[TemplateStruct],
) -> Result<Vec<NamedTypeInfo>, String> {
    let mut out: Vec<NamedTypeInfo> = Vec::new();
    let mut memo: std::collections::HashMap<String, NamedTypeInfo> =
        std::collections::HashMap::new();

    for s in structs {
        out.push(resolve_struct_shape(&s.name, structs, &mut memo, &mut Vec::new())?);
    }

    for e in enums {
        let variants = e
            .variants
            .iter()
            .map(|v| {
                let pascal = crate::generator::filters::to_pascal_case(&v.name)
                    .unwrap_or_else(|_| v.name.clone());
                (v.name.clone(), pascal)
            })
            .collect();
        out.push(NamedTypeInfo {
            ridl_name: e.name.clone(),
            rust_path: format!(
                "crate::api::{}",
                crate::generator::naming::to_upper_camel_case(&e.name)
            ),
            shape: NamedTypeShape::Enum(variants),
        });
    }

    Ok(out)
}

/// Resolve a struct's full conversion shape, filling `TemplateStructField::nested`
/// recursively. `stack` carries the current resolution chain for cycle
/// diagnostics (`struct A { b: B } struct B { a: A }` is an infinitely sized
/// value and is rejected with a named chain instead of a rustc size error).
fn resolve_struct_shape(
    name: &str,
    structs: &[TemplateStruct],
    memo: &mut std::collections::HashMap<String, NamedTypeInfo>,
    stack: &mut Vec<String>,
) -> Result<NamedTypeInfo, String> {
    if let Some(info) = memo.get(name) {
        return Ok(info.clone());
    }
    if let Some(pos) = stack.iter().position(|s| s == name) {
        let mut chain = stack[pos..].to_vec();
        chain.push(name.to_string());
        return Err(format!(
            "recursive struct nesting is not supported (chain: {} — the generated Rust type would be infinitely sized)",
            chain.join(" -> ")
        ));
    }

    let s = structs
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("unknown struct '{name}' in named-type table"))?;

    stack.push(name.to_string());
    let mut fields = s.fields.clone();
    for f in &mut fields {
        // A nested reference is either a bare `Custom` field or the element of
        // an `array<Custom>` field; the field matrix already restricted both to
        // same-module structs (enum fields are rejected).
        let target = match &f.ty {
            Type::Custom(n) => Some(n.clone()),
            Type::Array(inner) => match inner.as_ref() {
                Type::Custom(n) => Some(n.clone()),
                _ => None,
            },
            _ => None,
        };
        if let Some(target) = target {
            let info = resolve_struct_shape(&target, structs, memo, stack)?;
            f.nested = Some(Box::new(info));
        }
    }
    stack.pop();

    let info = NamedTypeInfo {
        ridl_name: name.to_string(),
        rust_path: format!(
            "crate::api::{}",
            crate::generator::naming::to_upper_camel_case(name)
        ),
        shape: NamedTypeShape::Struct(fields),
    };
    memo.insert(name.to_string(), info.clone());
    Ok(info)
}

/// Build a `TemplateStruct`, enforcing the Phase D field rejection matrix.
///
/// 允许集：基元(i32/i64/f32/f64/bool) / string / 嵌套 struct / array<允许集>。
/// 明确拒绝（全部带可诊断错误，指明 struct 与字段）：
/// - `Traced<T>`（Traced 是 class opaque 专用）
/// - union 成员
/// - ClassRef（class 同名 Custom 已被 parser 的 class_ref_rewrite 改写为
///   ClassRef，struct 字段引用 class 会与 class 语义互相劫持）
/// - map
/// - `T?` 字段
/// - enum 类型字段（v1 允许集未包含；enum 作为方法参数/返回使用）
///
/// 旧实现中 `rust_type_from_idl` 失败会静默回退 `rust_ty = "JSValue"`；该回退
/// 已移除：所有允许集分支都能确定地给出 rust_ty，其余一律硬错误。
fn build_template_struct(
    s: &crate::parser::ast::StructDef,
    struct_names: &std::collections::HashSet<String>,
    enum_names: &std::collections::HashSet<String>,
) -> Result<TemplateStruct, Box<dyn std::error::Error>> {
    let mut fields: Vec<TemplateStructField> = Vec::new();
    for f in &s.fields {
        let rust_ty = struct_field_rust_ty(&s.name, &f.name, &f.field_type, struct_names, enum_names)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        fields.push(TemplateStructField {
            name: f.name.clone(),
            rust_name: crate::generator::filters::rust_ident(&f.name)
                .unwrap_or_else(|_| f.name.clone()),
            ty: f.field_type.clone(),
            rust_ty,
            nested: None,
        });
    }
    Ok(TemplateStruct {
        name: s.name.clone(),
        fields,
    })
}

fn struct_field_rust_ty(
    owner: &str,
    field_name: &str,
    ty: &Type,
    struct_names: &std::collections::HashSet<String>,
    enum_names: &std::collections::HashSet<String>,
) -> Result<String, String> {
    let reject = |why: String| -> Result<String, String> {
        Err(format!(
            "struct '{owner}': field '{field_name}' of type '{ty}': {why}"
        ))
    };

    match ty {
        Type::Bool | Type::I32 | Type::I64 | Type::F32 | Type::F64 | Type::String => {
            crate::generator::filters::rust_type_from_idl(ty)
                .map_err(|e| format!("struct '{owner}': field '{field_name}': {e}"))
        }
        Type::Custom(name) if !name.starts_with('(') => {
            if struct_names.contains(name) {
                Ok(format!(
                    "crate::api::{}",
                    crate::generator::naming::to_upper_camel_case(name)
                ))
            } else if enum_names.contains(name) {
                reject(
                    "struct fields of enum type are not supported in v1 (allowed field set: \
                     primitives/string/nested struct/array<allowed>); use the enum directly as a \
                     method parameter/return type"
                        .to_string(),
                )
            } else {
                reject(format!(
                    "unknown named type '{name}': struct fields may only reference structs \
                     defined in the same module"
                ))
            }
        }
        Type::Array(inner) => {
            let inner_rust = struct_field_rust_ty(owner, field_name, inner, struct_names, enum_names)?;
            Ok(format!("Vec<{}>", inner_rust))
        }
        Type::Optional(_) => reject(
            "optional struct fields (`T?`) are not supported in v1 (allowed field set: \
             primitives/string/nested struct/array<allowed>)"
                .to_string(),
        ),
        Type::Map(_, _) => reject(
            "map fields are not supported in v1 (allowed field set: primitives/string/nested \
             struct/array<allowed>)"
                .to_string(),
        ),
        Type::Union(_) => reject(
            "union fields are not supported in v1 (allowed field set: primitives/string/nested \
             struct/array<allowed>)"
                .to_string(),
        ),
        Type::Traced(_) => reject(
            "Traced<T> fields are not supported in structs (Traced is for class opaque fields)"
                .to_string(),
        ),
        Type::ClassRef(name) => reject(format!(
            "class type '{name}' is not allowed as a struct field (classes are reference types; \
             the reference was rewritten from a named-type mention by class_ref_rewrite)"
        )),
        other => reject(format!("unsupported struct field type: {other:?}")),
    }
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

pub fn generate_aggregate_consolidated(
    plan: &crate::plan::RidlPlan,
    output_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // (1) mquickjs_ridl_register.h + ridl_symbols.rs
    let ridl_files: Vec<String> = plan
        .modules
        .iter()
        .flat_map(|m| m.ridl_files.iter())
        .map(|p| p.display().to_string())
        .collect();

    {
        let out_dir_str = output_dir.to_str().ok_or("invalid output dir (non-utf8)")?;

        // Make sure the consolidated C-side register header matches the same
        // C ABI symbol naming convention as Rust glue (snake_case, '_' separated).
        // The underlying generator is in deps/mquickjs and uses RIDL names.
        // We normalize the RIDL sources here to avoid cross-tool naming drift.
        let mut ridl_files = ridl_files.clone();
        for p in &mut ridl_files {
            *p = p.clone();
        }

        let ir = generate_register_h_and_symbols(&ridl_files, out_dir_str)?;

        // (2) ridl_context_ext.rs (ctx_ext + slot indices + ridl_context_init)
        // Reuse the same class_id allocation produced by register.h generation.
        crate::generator::singleton_aggregate::generate_ridl_context_ext(plan, output_dir)?;

        // (2.1) mquickjs_ridl_register.c (runtime glue): stdlib normalization init.
        // It uses the same AggregateIR class_id mapping for proto vars.
        generate_mquickjs_ridl_register_c(plan, output_dir, ir.as_ref())?;
    }

    // (3) ridl_bootstrap.rs (modules keep-alive + process initialize)
    let mut crate_names: Vec<&str> = plan.modules.iter().map(|m| m.crate_name.as_str()).collect();
    crate_names.sort();
    crate_names.dedup();

    #[derive(askama::Template)]
    #[template(path = "ridl_bootstrap.rs.j2")]
    struct RidlBootstrapTemplate<'a> {
        crate_names: Vec<&'a str>,
    }

    let t = RidlBootstrapTemplate { crate_names };
    std::fs::write(output_dir.join("ridl_bootstrap.rs"), t.render()?)?;

    Ok(())
}
