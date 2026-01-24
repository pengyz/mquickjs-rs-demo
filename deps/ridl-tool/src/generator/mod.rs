use crate::parser::ast::{Class, Function, IDLItem, Interface, Method, Param, Type, IDL};

use union_types::collect_union_types;

fn apply_union_rust_ty_overrides(union_types: &[TemplateUnionType], tpl: &mut impl RustGlueLikeTemplate) {
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
    apply_union_rust_ty_overrides_ty(union_types, fn_name, "", &f.return_type, &mut f.return_rust_ty);
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
    apply_union_rust_ty_overrides_ty(union_types, fn_name, "", &m.return_type, &mut m.return_rust_ty);
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
        _ => false,
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
                        "int" => keys.push("Int"),
                        "float" => keys.push("Float"),
                        "double" => keys.push("Double"),
                        _ => return None,
                    }
                }

                keys.sort();
                let name = format!("Union{}", keys.join(""));
                let u = union_types.iter().find(|u| u.name == name)?;
                let is_optional = label == "Optional";
                return Some((format!("crate::api::{}::union::{}", u.domain, u.name), is_optional));
            }
            None
        }
        Type::Group(inner) => {
            // Group(...) is only syntactic grouping.
            union_enum_path_for_ty(union_types, fn_name, label, inner)
        }
        Type::Union(types) => {
            let mut keys: Vec<&'static str> = vec![];
            let mut nullable = false;

            for t in types {
                match t {
                    Type::String => keys.push("String"),
                    Type::Int => keys.push("Int"),
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
            Some((format!("crate::api::{}::union::{}", u.domain, u.name), is_optional))
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
            "/* Generated by ridl-tool: no RIDL modules selected */\n#ifndef MJS_RIDL_REGISTER_H\n#define MJS_RIDL_REGISTER_H\n\n#include <stdint.h>\n\n/* File-scope declarations/definitions for RIDL extensions */\n#define JS_RIDL_DECLS /* empty */\n\n/* Hook used by mqjs_stdlib_template.c */\n#define JS_RIDL_EXTENSIONS /* empty */\n\ntypedef struct {\n    const char *module_full_name;\n    const char *module_base;\n    uint16_t v_major;\n    uint16_t v_minor;\n    uint16_t v_patch;\n    int module_class_id;\n} RidlRequireEntry;\n\nstatic const RidlRequireEntry js_ridl_require_table[] = { };\nstatic const int js_ridl_require_table_len = 0;\n\n#endif /* MJS_RIDL_REGISTER_H */\n",
        )?;

        std::fs::write(
            out_dir.join("ridl_symbols.rs"),
            "// Generated by ridl-tool: no RIDL modules selected\n\n#[inline(always)]\npub fn ensure_symbols() {}\n",
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
                    let module_name_normalized = crate::generator::filters::normalize_ident(&ridl_module_name)
                        .unwrap_or_else(|_| "GLOBAL".to_string());
                    functions.push(TemplateFunction::from_with_mode(f, parsed.mode, module_name_normalized))
                }
                crate::parser::ast::IDLItem::Interface(mut i) => {
                    i.module = parsed.module.clone();
                    let ridl_module_name = i
                        .module
                        .as_ref()
                        .map(|m| m.module_path.clone())
                        .unwrap_or_else(|| "GLOBAL".to_string());
                    let module_name_normalized = crate::generator::filters::normalize_ident(&ridl_module_name)
                        .unwrap_or_else(|_| "GLOBAL".to_string());
                    interfaces.push(TemplateInterface::from_with_mode(i, parsed.mode, module_name_normalized))
                }
                crate::parser::ast::IDLItem::Singleton(mut s) => {
                    s.module = parsed.module.clone();
                    let module_name = s
                        .module
                        .as_ref()
                        .map(|m| m.module_path.as_str())
                        .unwrap_or("GLOBAL")
                        .to_string();
                    singletons.push(TemplateSingleton {
                        name: s.name,
                        module_name_normalized: crate::generator::filters::normalize_ident(&module_name)
                            .unwrap_or_else(|_| "GLOBAL".to_string()),
                        module_name,
                        methods: s
                            .methods
                            .into_iter()
                            .map(|m| TemplateMethod::from_with_mode(m, parsed.mode))
                            .collect(),
                        properties: s.properties,
                    })
                }
                crate::parser::ast::IDLItem::Class(c) => {
                    let module_name_normalized = crate::generator::filters::normalize_ident(&module_name)
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
#[template(path = "rust_glue.rs.j2")]
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
#[template(path = "rust_api.rs.j2")]
#[allow(dead_code)]
struct RustApiTemplate {
    module_name: String,
    module_decl: Option<crate::parser::ast::ModuleDeclaration>,
    interfaces: Vec<TemplateInterface>,
    functions: Vec<TemplateFunction>,
    singletons: Vec<TemplateSingleton>,
    classes: Vec<TemplateClass>,

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
}

#[derive(Debug, Clone)]
struct TemplateJsField {
    name: String,
    field_type: crate::parser::ast::Type,
    init_literal: String,
    is_proto: bool,
}

#[derive(Debug, Clone)]
struct TemplateMethod {
    name: String,
    params: Vec<TemplateParam>,
    return_type: Type,
    return_rust_ty: String,
    has_variadic: bool,
    needs_scope: bool,
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
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TemplateFunction {
    name: String,
    module_name_normalized: String,
    params: Vec<TemplateParam>,
    return_type: Type,
    return_rust_ty: String,
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

        Self {
            name: method.name,
            params,
            return_type,
            return_rust_ty,
            has_variadic,
            needs_scope,
        }
    }
}

impl TemplateParam {
    fn from_with_mode(param: Param, file_mode: crate::parser::FileMode) -> Self {
        let ty = param.param_type;
        let rust_ty = crate::generator::filters::rust_type_from_idl(&ty).unwrap_or_else(|_| "()".to_string());
        let rust_name = crate::generator::filters::rust_ident(&crate::generator::naming::to_snake_case(&param.name))
            .unwrap_or_else(|_| "_".to_string());

        Self {
            name: param.name,
            rust_name,
            ty,
            variadic: param.variadic,
            file_mode,
            rust_ty,
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
            constructor: class
                .constructor
                .map(|c| {
                    TemplateFunction::from_with_mode(c, file_mode, module_name_normalized_cloned.clone())
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
                let module_name_normalized = crate::generator::filters::normalize_ident(ridl_module_name)
                    .unwrap_or_else(|_| "GLOBAL".to_string());
                classes.push(TemplateClass::from_with_mode(
                    ridl_module_name.to_string(),
                    module_name_normalized,
                    c.clone(),
                    file_mode,
                ))
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
            singletons.push(TemplateSingleton {
                name: s.name.clone(),
                module_name_normalized: crate::generator::filters::normalize_ident(&singleton_module_name)
                    .unwrap_or_else(|_| "GLOBAL".to_string()),
                module_name: singleton_module_name,
                methods: s
                    .methods
                    .clone()
                    .into_iter()
                    .map(|m| TemplateMethod::from_with_mode(m, file_mode))
                    .collect(),
                properties: s.properties.clone(),
            });
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
    );
    apply_union_rust_ty_overrides(&union_types, &mut rust_glue_template);
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
        union_types_by_domain: group_union_types_by_domain(union_types.clone()),
    };

    // Keep API trait signatures consistent with glue by applying the same union overrides.
    apply_union_rust_ty_overrides(&union_types, &mut rust_api_template);
    let rust_api_code = rust_api_template.render()?;
    std::fs::write(output_path.join("api.rs"), rust_api_code)?;

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
        let out_dir_str = output_dir
            .to_str()
            .ok_or("invalid output dir (non-utf8)")?;

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
