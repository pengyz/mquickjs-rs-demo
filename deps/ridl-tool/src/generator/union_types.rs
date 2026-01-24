use crate::parser::ast::{ModuleDeclaration, Type};

use super::{TemplateClass, TemplateFunction, TemplateInterface, TemplateMethod, TemplateParam, TemplateSingleton, TemplateUnionMember, TemplateUnionType};
use super::filters::rust_type_from_idl;

pub(super) fn collect_union_types(
    _module_name: &str,
    module_decl: Option<ModuleDeclaration>,
    interfaces: &[TemplateInterface],
    functions: &[TemplateFunction],
    singletons: &[TemplateSingleton],
    classes: &[TemplateClass],
) -> Vec<TemplateUnionType> {
    let domain = domain_name(&module_decl);

    let mut out: Vec<TemplateUnionType> = vec![];

    for itf in interfaces {
        for m in &itf.methods {
            collect_from_method(&domain, m, &mut out);
        }
    }

    for f in functions {
        collect_from_function(&domain, f, &mut out);
    }

    for s in singletons {
        for m in &s.methods {
            collect_from_method(&domain, m, &mut out);
        }
    }

    for c in classes {
        if let Some(ctor) = &c.constructor {
            collect_from_function(&domain, ctor, &mut out);
        }
        for m in &c.methods {
            collect_from_method(&domain, m, &mut out);
        }
    }

    out
}

fn collect_from_method(domain: &str, m: &TemplateMethod, out: &mut Vec<TemplateUnionType>) {
    for p in &m.params {
        collect_from_param(domain, &m.name, p, out);
    }

    collect_from_return(domain, &m.name, &m.return_type, out);
}

fn collect_from_function(domain: &str, f: &TemplateFunction, out: &mut Vec<TemplateUnionType>) {
    for p in &f.params {
        collect_from_param(domain, &f.name, p, out);
    }

    collect_from_return(domain, &f.name, &f.return_type, out);
}

fn collect_from_param(domain: &str, fn_name: &str, p: &TemplateParam, out: &mut Vec<TemplateUnionType>) {
    collect_from_type(domain, fn_name, &p.name, &p.ty, out);
}

fn collect_from_return(domain: &str, fn_name: &str, ty: &Type, out: &mut Vec<TemplateUnionType>) {
    collect_from_type(domain, fn_name, "return", ty, out);
}

fn collect_from_type(domain: &str, fn_name: &str, label: &str, ty: &Type, out: &mut Vec<TemplateUnionType>) {
    match ty {
        Type::Optional(inner) => collect_from_type(domain, fn_name, label, inner, out),
        Type::Group(inner) => collect_from_type(domain, fn_name, label, inner, out),
        Type::Union(types) => {
            let (member_types, _nullable) = normalize_union(types);
            // Union enum name is derived only from the member set (v1):
            // - Same union type used for param/return
            // - Nullable unions are represented as Option<UnionEnum> (null is normalized out)
            let name = union_name_from_members(&member_types);

            let mut members: Vec<TemplateUnionMember> = vec![];
            for mt in member_types {
                if matches!(mt, Type::Null) {
                    continue;
                }
                if let Some(m) = map_union_member(&mt) {
                    members.push(m);
                }
            }

            let cand = TemplateUnionType {
                domain: domain.to_string(),
                name,
                members,
            };

            if !out.iter().any(|u| u.domain == cand.domain && u.name == cand.name) {
                out.push(cand);
            }
        }
        _ => {}
    }
}

fn map_union_member(ty: &Type) -> Option<TemplateUnionMember> {
    match ty {
        Type::String => Some(TemplateUnionMember {
            variant: "String".to_string(),
            rust_ty: "String".to_string(),
        }),
        Type::I32 => Some(TemplateUnionMember {
            variant: "I32".to_string(),
            rust_ty: "i32".to_string(),
        }),
        Type::I64 => Some(TemplateUnionMember {
            variant: "I64".to_string(),
            rust_ty: "i64".to_string(),
        }),
        Type::F32 => Some(TemplateUnionMember {
            variant: "F32".to_string(),
            rust_ty: "f32".to_string(),
        }),
        Type::F64 => Some(TemplateUnionMember {
            variant: "F64".to_string(),
            rust_ty: "f64".to_string(),
        }),
        // Keep narrow for now; other complex member types will be added in V1-B2.
        other => {
            let _ = rust_type_from_idl(other);
            None
        }
    }
}

fn normalize_union(types: &[Type]) -> (Vec<Type>, bool) {
    let mut members: Vec<Type> = vec![];
    let mut has_null = false;

    for t in types {
        match t {
            Type::Null => has_null = true,
            Type::Group(inner) => {
                let (inner_members, inner_null) = normalize_union(&[inner.as_ref().clone()]);
                members.extend(inner_members);
                has_null |= inner_null;
            }
            other => members.push(other.clone()),
        }
    }

    (members, has_null)
}

fn domain_name(module_decl: &Option<ModuleDeclaration>) -> String {
    match module_decl {
        None => "global".to_string(),
        Some(m) => normalize_ident(&m.module_path),
    }
}

fn normalize_ident(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn union_name_from_members(member_types: &[Type]) -> String {
    // v1: only supports a small set of discriminable unions.
    // Produce stable name by sorting a canonical key order.
    let mut keys: Vec<&'static str> = vec![];
    for t in member_types {
        match t {
            Type::String => keys.push("String"),
            Type::I32 => keys.push("I32"),
            Type::I64 => keys.push("I64"),
            Type::F32 => keys.push("F32"),
            Type::F64 => keys.push("F64"),
            _ => {}
        }
    }

    keys.sort();
    keys.dedup();

    if keys.is_empty() {
        return "Union".to_string();
    }

    format!("Union{}", keys.join(""))
}

#[allow(dead_code)]
fn to_upper_camel_case(s: &str) -> String {
    let mut out = String::new();
    let mut upper_next = true;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if upper_next {
                out.extend(ch.to_uppercase());
                upper_next = false;
            } else {
                out.push(ch);
            }
        } else {
            upper_next = true;
        }
    }
    if out.is_empty() {
        "X".to_string()
    } else {
        out
    }
}
