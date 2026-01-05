use std::fs;
use std::path::Path;
use crate::parser::ast::{IDLItem, Interface, Class, Enum, Function, Type, Field, Property, Method, StructDef, SerializationFormat};

/// 生成代码
pub fn generate_code(items: &[IDLItem], output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = Path::new(output_dir);
    
    // 创建输出目录
    fs::create_dir_all(output_path)?;
    
    // 生成 Rust 胶水代码
    generate_rust_glue(items, output_path)?;
    
    // 生成 C 绑定代码
    generate_c_bindings(items, output_path)?;
    
    // 生成标准库描述
    generate_stdlib_descriptions(items, output_path)?;
    
    Ok(())
}

/// 生成 Rust 胶水代码
fn generate_rust_glue(items: &[IDLItem], output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut rust_code = String::new();
    rust_code.push_str("// Auto-generated Rust glue code\n\n");

    for item in items {
        match item {
            IDLItem::Interface(interface) => {
                rust_code.push_str(&generate_rust_interface_glue(interface));
            }
            IDLItem::Class(class) => {
                rust_code.push_str(&generate_rust_class_glue(class));
            }
            IDLItem::Enum(enum_def) => {
                rust_code.push_str(&generate_rust_enum_glue(enum_def));
            }
            IDLItem::Struct(struct_def) => {
                rust_code.push_str(&generate_rust_struct_glue(struct_def));
            }
            IDLItem::Function(function) => {
                rust_code.push_str(&generate_rust_function_glue(function));
            }
            _ => {
                // 其他类型暂时不处理
            }
        }
    }

    // 将生成的代码写入文件
    fs::write(output_path.join("rust_glue.rs"), rust_code)?;

    Ok(())
}

/// 生成 C 绑定代码
pub fn generate_c_bindings(items: &[IDLItem], output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // 生成 C 绑定代码
    let mut c_code = String::new();
    c_code.push_str("// Auto-generated C bindings\n\n");
    c_code.push_str("#include \"mquickjs.h\"\n\n");

    for item in items {
        match item {
            IDLItem::Interface(interface) => {
                c_code.push_str(&generate_c_interface_glue(interface));
            }
            IDLItem::Class(class) => {
                c_code.push_str(&generate_c_class_glue(class));
            }
            IDLItem::Enum(enum_def) => {
                c_code.push_str(&generate_c_enum_glue(enum_def));
            }
            IDLItem::Struct(struct_def) => {
                c_code.push_str(&generate_c_struct_glue(struct_def));
            }
            IDLItem::Function(function) => {
                c_code.push_str(&generate_c_function_glue(function));
            }
            _ => {
                // 其他类型暂时不处理
            }
        }
    }

    // 将生成的代码写入文件
    fs::write(output_path.join("c_bindings.c"), c_code)?;

    Ok(())
}

/// 生成标准库描述
fn generate_stdlib_descriptions(items: &[IDLItem], output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let stdlib_path = output_path.join("stdlib_descriptions.txt");
    let mut content = String::new();
    
    content.push_str("// Auto-generated standard library descriptions\n\n");

    for item in items {
        match item {
            IDLItem::Interface(interface) => {
                content.push_str(&format!("// Interface: {}\n", interface.name));
                for method in &interface.methods {
                    content.push_str(&format!("// Method: {}::{}\n", interface.name, method.name));
                }
            }
            IDLItem::Class(class) => {
                content.push_str(&format!("// Class: {}\n", class.name));
                if let Some(ref constructor) = class.constructor {
                    content.push_str(&format!("// Constructor: {}::new\n", class.name));
                }
                for method in &class.methods {
                    content.push_str(&format!("// Method: {}::{}\n", class.name, method.name));
                }
            }
            IDLItem::Function(function) => {
                content.push_str(&format!("// Function: {}\n", function.name));
            }
            _ => {}
        }
    }
    
    fs::write(stdlib_path, content)?;
    Ok(())
}

// 生成Rust接口胶水代码
fn generate_rust_interface_glue(interface: &Interface) -> String {
    let mut code = format!("// Interface: {}\n", interface.name);
    code.push_str(&format!("pub struct {} {{\n", interface.name));
    code.push_str("    // Interface implementation\n");
    code.push_str("}\n\n");
    
    for method in &interface.methods {
        code.push_str(&generate_rust_method_glue(&interface.name, method));
    }
    
    code
}

// 生成Rust类胶水代码
fn generate_rust_class_glue(class: &Class) -> String {
    let mut code = format!("// Class: {}\n", class.name);
    code.push_str(&format!("pub struct {} {{\n", class.name));
    code.push_str("    // Class implementation\n");
    code.push_str("}\n\n");
    
    if let Some(ref constructor) = class.constructor {
        code.push_str(&generate_rust_constructor_glue(&class.name, constructor));
    }
    
    for method in &class.methods {
        code.push_str(&generate_rust_method_glue(&class.name, method));
    }
    
    code
}

// 生成Rust枚举胶水代码
fn generate_rust_enum_glue(enum_def: &Enum) -> String {
    let mut code = format!("// Enum: {}\n", enum_def.name);
    code.push_str(&format!("pub enum {} {{\n", enum_def.name));
    
    for value in &enum_def.values {
        if let Some(val) = value.value {
            code.push_str(&format!("    {} = {},\n", value.name, val));
        } else {
            code.push_str(&format!("    {},\n", value.name));
        }
    }
    
    code.push_str("}\n\n");
    code
}

// 生成Rust结构体胶水代码
fn generate_rust_struct_glue(struct_def: &StructDef) -> String {
    let mut code = format!("// Struct: {}\n", struct_def.name);
    
    // 添加序列化相关导入
    match struct_def.serialization_format {
        SerializationFormat::Json => {
            code.push_str("#[derive(serde::Serialize, serde::Deserialize)]\n");
        }
        SerializationFormat::MessagePack => {
            code.push_str("#[derive(serde::Serialize, serde::Deserialize)]\n");
        }
        SerializationFormat::Protobuf => {
            code.push_str("#[derive(protobuf::Message)]\n");
        }
    }
    
    code.push_str(&format!("pub struct {} {{\n", struct_def.name));
    
    for field in &struct_def.fields {
        code.push_str(&format!("    pub {}: {},\n", field.name, rust_type_name(&field.field_type)));
    }
    
    code.push_str("}\n\n");
    code
}

// 生成Rust函数胶水代码
fn generate_rust_function_glue(function: &Function) -> String {
    let params = function
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, rust_type_name(&p.param_type)))
        .collect::<Vec<_>>()
        .join(", ");
    
    format!(
        "pub fn {}({}) -> {} {{\n    todo!(\"Function implementation\");\n}}\n\n",
        function.name,
        params,
        rust_type_name(&function.return_type)
    )
}

// 生成Rust方法胶水代码
fn generate_rust_method_glue(class_name: &str, method: &Method) -> String {
    format!(
        "JSValue {}_{}_method(JSContext *ctx, JSValue this_val, int argc, JSValue *argv) {{\n    // TODO: Implement method\n    return JS_UNDEFINED;\n}}\n\n",
        class_name,
        method.name
    )
}

// 生成Rust构造函数胶水代码
fn generate_rust_constructor_glue(class_name: &str, constructor: &Function) -> String {
    format!(
        "JSValue {}_constructor(JSContext *ctx, JSValue new_target, int argc, JSValue *argv) {{\n    // TODO: Implement constructor\n    return JS_UNDEFINED;\n}}\n\n",
        class_name
    )
}

// 生成C接口绑定代码
fn generate_c_interface_glue(interface: &Interface) -> String {
    let mut code = format!("// C bindings for interface: {}\n", interface.name);
    
    for method in &interface.methods {
        code.push_str(&format!(
            "// Method: {}::{}\nJSValue {}_{}_method(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
            interface.name,
            method.name,
            interface.name,
            method.name
        ));
    }
    
    code
}

// 生成C类绑定代码
fn generate_c_class_glue(class: &Class) -> String {
    let mut code = format!("// C bindings for class: {}\n", class.name);
    
    // Constructor
    if let Some(ref _constructor) = class.constructor {
        code.push_str(&format!(
            "// Constructor: {}\nJSValue {}_constructor(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
            class.name,
            class.name
        ));
    }
    
    // Methods
    for method in &class.methods {
        code.push_str(&format!(
            "// Method: {}::{}\nJSValue {}_{}_method(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
            class.name,
            method.name,
            class.name,
            method.name
        ));
    }
    
    code
}

// 生成C枚举绑定代码
fn generate_c_enum_glue(enum_def: &Enum) -> String {
    format!("// C bindings for enum: {}\n// Values: {:?}\n\n", enum_def.name, enum_def.values)
}

// 生成C结构体绑定代码
fn generate_c_struct_glue(struct_def: &StructDef) -> String {
    let format_str = match struct_def.serialization_format {
        SerializationFormat::Json => "JSON",
        SerializationFormat::MessagePack => "MessagePack",
        SerializationFormat::Protobuf => "Protobuf",
    };
    
    format!(
        "// C bindings for struct: {} (serialized with {})\n// Fields: {}\n\n",
        struct_def.name,
        format_str,
        struct_def.fields.len()
    )
}

// 生成C函数绑定代码
fn generate_c_function_glue(function: &Function) -> String {
    format!(
        "// Global function: {}\nJSValue {}_function(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
        function.name,
        function.name
    )
}

// 辅助函数：将IDL类型转换为Rust类型名
fn rust_type_name(idl_type: &Type) -> String {
    match idl_type {
        Type::Bool => "bool".to_string(),
        Type::Int => "i32".to_string(),
        Type::Float => "f32".to_string(),
        Type::Double => "f64".to_string(),
        Type::String => "String".to_string(),
        Type::Void => "()".to_string(),
        Type::Object => "Object".to_string(),
        Type::Function => "Function".to_string(),
        Type::Callback => "Callback".to_string(),
        Type::Null => "Option<()>".to_string(),
        Type::Any => "serde_json::Value".to_string(),
        Type::Array(inner) => format!("Vec<{}>", rust_type_name(inner)),
        Type::Map(key, value) => format!("std::collections::HashMap<{}, {}>", rust_type_name(key), rust_type_name(value)),
        Type::Union(_) => "serde_json::Value".to_string(),
        Type::Optional(inner) => format!("Option<{}>", rust_type_name(inner)),
        Type::Custom(name) => name.clone(),
        Type::Group(inner) => rust_type_name(inner),
    }
}