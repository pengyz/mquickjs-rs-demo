use crate::parser::ast::{PropertyModifier, Type};

// Rust类型转换辅助函数，用于模板
#[allow(dead_code)]
pub fn rust_type_from_idl(idl_type: &Type) -> Result<String, askama::Error> {
    let rust_type = match idl_type {
        Type::Bool => "bool".to_string(),
        Type::Int => "i32".to_string(),
        Type::Float => "f32".to_string(),
        Type::Double => "f64".to_string(),
        Type::String => "String".to_string(),
        Type::Void => "()".to_string(),
        Type::Object => "serde_json::Value".to_string(),
        Type::Array(inner) => format!("Vec<{}>", rust_type_from_idl(inner)?),
        Type::Map(key_type, value_type) => format!(
            "std::collections::HashMap<{}, {}>",
            rust_type_from_idl(key_type)?,
            rust_type_from_idl(value_type)?
        ),
        Type::Union(_) => "serde_json::Value".to_string(),
        Type::Optional(inner) => format!("Option<{}>", rust_type_from_idl(inner)?),
        Type::Custom(name) => name.clone(),
        Type::Callback => "Box<dyn Fn()>".to_string(),
        Type::CallbackWithParams(_) => "Box<dyn Fn()>".to_string(),
        Type::Group(inner) => rust_type_from_idl(inner)?,
        Type::Null => "std::option::Option<()>".to_string(),
        Type::Any => "serde_json::Value".to_string(),
    };
    Ok(rust_type)
}

// 修复此函数以正确处理字符串类型的Rust转换
#[allow(dead_code)]
pub fn rust_type_from_str_opt(opt_type: &Option<String>) -> ::askama::Result<String> {
    match opt_type {
        Some(type_str) => {
            // 根据类型字符串返回对应的Rust类型
            match type_str.as_str() {
                "bool" => Ok("bool".to_string()),
                "int" => Ok("i32".to_string()),
                "float" => Ok("f32".to_string()),
                "double" => Ok("f64".to_string()),
                "string" => Ok("String".to_string()),
                "void" => Ok("()".to_string()),
                "object" => Ok("serde_json::Value".to_string()),
                s => {
                    // 如果不是预定义类型，则保持原样
                    Ok(s.to_string())
                }
            }
        }
        None => Ok("()".to_string()),
    }
}

// 提供默认值的过滤器
pub fn default(s: &Option<String>, default: &str) -> ::askama::Result<String> {
    Ok(s.as_deref().unwrap_or(default).to_string())
}

// 将字符串转换为驼峰命名
#[allow(dead_code)]
pub fn camelcase(s: &str) -> ::askama::Result<String> {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c.is_alphanumeric() {
            if capitalize_next || result.is_empty() {
                result.push_str(&c.to_uppercase().to_string());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        } else {
            capitalize_next = true;
        }
    }

    Ok(result)
}

// 将字符串转换为小写
#[allow(dead_code)]
pub fn lower(s: &str) -> ::askama::Result<String> {
    Ok(s.to_lowercase())
}

// 将字符串转换为大写
#[allow(dead_code)]
pub fn upper(s: &str) -> ::askama::Result<String> {
    Ok(s.to_uppercase())
}

// 为切片定义长度过滤器，适用于 Vec<T> 和 &[T]
pub fn length<T>(slice: &[T]) -> ::askama::Result<usize> {
    Ok(slice.len())
}

// JS类型转换辅助函数，用于模板
#[allow(dead_code)]
pub fn js_conversion_type(idl_type: &Type) -> Result<String, askama::Error> {
    let conversion_method = match idl_type {
        Type::Bool => "to_bool",
        Type::Int => "to_i32",
        Type::Float => "to_f32",
        Type::Double => "to_f64",
        Type::String => "to_string",
        _ => "to_js_value", // fallback for complex types
    }
    .to_string();

    Ok(conversion_method)
}

pub fn is_readonly_prop(modifiers: &[PropertyModifier]) -> ::askama::Result<bool> {
    Ok(modifiers.contains(&PropertyModifier::ReadOnly))
}

// Turn a singleton/interface name into a Rust type identifier suffix.
// E.g. "console" -> "Console", "my_console" -> "MyConsole".
#[allow(dead_code)]
pub fn to_rust_type_ident(name: &str) -> ::askama::Result<String> {
    camelcase(name)
}
