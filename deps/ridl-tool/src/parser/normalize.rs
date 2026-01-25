use crate::parser::ast::{IDLItem, Param, Property, Type};

pub fn normalize_idl_items(
    items: Vec<IDLItem>,
) -> Result<Vec<IDLItem>, Box<dyn std::error::Error>> {
    items
        .into_iter()
        .map(normalize_item)
        .collect::<Result<Vec<_>, _>>()
}

fn normalize_item(item: IDLItem) -> Result<IDLItem, Box<dyn std::error::Error>> {
    Ok(match item {
        IDLItem::Interface(mut itf) => {
            for m in &mut itf.methods {
                for p in &mut m.params {
                    p.param_type = normalize_type(p.param_type.clone())?;
                }
                m.return_type = normalize_type(m.return_type.clone())?;
            }
            for p in &mut itf.properties {
                normalize_property(p)?;
            }
            IDLItem::Interface(itf)
        }
        IDLItem::Class(mut cls) => {
            if let Some(ctor) = &mut cls.constructor {
                for p in &mut ctor.params {
                    p.param_type = normalize_type(p.param_type.clone())?;
                }
                ctor.return_type = normalize_type(ctor.return_type.clone())?;
            }
            for m in &mut cls.methods {
                for p in &mut m.params {
                    p.param_type = normalize_type(p.param_type.clone())?;
                }
                m.return_type = normalize_type(m.return_type.clone())?;
            }
            for p in &mut cls.properties {
                normalize_property(p)?;
            }
            IDLItem::Class(cls)
        }
        IDLItem::Function(mut f) => {
            for p in &mut f.params {
                p.param_type = normalize_type(p.param_type.clone())?;
            }
            f.return_type = normalize_type(f.return_type.clone())?;
            IDLItem::Function(f)
        }
        IDLItem::Singleton(mut s) => {
            for m in &mut s.methods {
                for p in &mut m.params {
                    p.param_type = normalize_type(p.param_type.clone())?;
                }
                m.return_type = normalize_type(m.return_type.clone())?;
            }
            for p in &mut s.properties {
                normalize_property(p)?;
            }
            IDLItem::Singleton(s)
        }
        IDLItem::Struct(mut st) => {
            for f in &mut st.fields {
                f.field_type = normalize_type(f.field_type.clone())?;
            }
            IDLItem::Struct(st)
        }
        other => other,
    })
}

fn normalize_property(p: &mut Property) -> Result<(), Box<dyn std::error::Error>> {
    p.property_type = normalize_type(p.property_type.clone())?;
    Ok(())
}

fn normalize_type(ty: Type) -> Result<Type, Box<dyn std::error::Error>> {
    // First, normalize children.
    let ty = match ty {
        Type::Array(inner) => Type::Array(Box::new(normalize_type(*inner)?)),
        Type::Map(k, v) => Type::Map(Box::new(normalize_type(*k)?), Box::new(normalize_type(*v)?)),
        Type::Optional(inner) => Type::Optional(Box::new(normalize_type(*inner)?)),
        Type::Union(ts) => {
            let mut out = Vec::with_capacity(ts.len());
            for t in ts {
                out.push(normalize_type(t)?);
            }
            Type::Union(out)
        }
        Type::Group(inner) => Type::Group(Box::new(normalize_type(*inner)?)),
        Type::CallbackWithParams(mut params) => {
            for Param { param_type, .. } in &mut params {
                *param_type = normalize_type(param_type.clone())?;
            }
            Type::CallbackWithParams(params)
        }
        Type::ClassRef(name) => Type::ClassRef(name),
        other => other,
    };

    // Canonicalization rules:
    // 1) Optional(Group(X)) -> Optional(X)
    let ty = match ty {
        Type::Optional(inner) => match *inner {
            Type::Group(g) => Type::Optional(g),
            other => Type::Optional(Box::new(other)),
        },
        other => other,
    };

    // 2) Union(..., Null) -> Optional(Union(...))
    //    If the union becomes a single member after removing Null, collapse it:
    //    Union(T, Null) -> Optional(T)
    let ty = match ty {
        Type::Union(ts) => {
            let mut non_null: Vec<Type> = Vec::new();
            let mut has_null = false;
            for t in ts {
                if matches!(t, Type::Null) {
                    has_null = true;
                } else {
                    non_null.push(t);
                }
            }
            if has_null {
                match non_null.len() {
                    0 => Type::Optional(Box::new(Type::Union(non_null))),
                    1 => Type::Optional(Box::new(non_null.into_iter().next().unwrap())),
                    _ => Type::Optional(Box::new(Type::Union(non_null))),
                }
            } else {
                Type::Union(non_null)
            }
        }
        other => other,
    };

    // 3) Optional(Custom("(A | B)")) -> Optional(Union(A,B))
    let ty = match ty {
        Type::Optional(inner) => match *inner {
            Type::Custom(s) => {
                if let Some(u) = try_parse_paren_union(&s)? {
                    Type::Optional(Box::new(u))
                } else {
                    Type::Optional(Box::new(Type::Custom(s)))
                }
            }
            other => Type::Optional(Box::new(other)),
        },
        other => other,
    };

    // 4) Optional(Optional(X)) -> Optional(X)
    let ty = match ty {
        Type::Optional(inner) => match *inner {
            Type::Optional(x) => Type::Optional(x),
            other => Type::Optional(Box::new(other)),
        },
        other => other,
    };

    // 5) Optional(Union([T])) -> Optional(T)
    let ty = match ty {
        Type::Optional(inner) => match *inner {
            Type::Union(mut ts) if ts.len() == 1 => {
                Type::Optional(Box::new(ts.pop().unwrap()))
            }
            other => Type::Optional(Box::new(other)),
        },
        other => other,
    };

    Ok(ty)
}

fn try_parse_paren_union(s: &str) -> Result<Option<Type>, Box<dyn std::error::Error>> {
    let s = s.trim();
    if !(s.starts_with('(') && s.ends_with(')') && s.contains('|')) {
        return Ok(None);
    }

    let inner = s[1..s.len() - 1].trim();
    if inner.is_empty() {
        return Ok(None);
    }

    // Minimal, conservative parser: split by `|` at top-level.
    // Current RIDL type atoms for unions in v1 are primitives and custom identifiers.
    let mut members: Vec<Type> = Vec::new();
    for part in inner.split('|') {
        let part = part.trim();
        if part.is_empty() {
            return Ok(None);
        }
        members.push(parse_atom_type(part));
    }

    if members.len() < 2 {
        return Ok(None);
    }

    Ok(Some(Type::Union(members)))
}

fn parse_atom_type(s: &str) -> Type {
    match s {
        "bool" => Type::Bool,
        "i32" => Type::I32,
        "i64" => Type::I64,
        "f32" => Type::F32,
        "f64" => Type::F64,
        "string" => Type::String,
        "void" => Type::Void,
        "object" => Type::Object,
        "null" => Type::Null,
        "any" => Type::Any,
        other => Type::Custom(other.to_string()),
    }
}
