//! Phase D.1: named-type 语义校验。
//!
//! 规则（计划 D.4 拒绝矩阵的 validator 部分）：
//! - 跨 kind 唯一性：struct/enum/class/interface 之间重名一律拒绝
//!   （此前 `validate_duplicate_definitions` 是空实现，HashMap 覆盖写入，
//!   `struct X {} enum X {}` 静默通过）。
//! - Custom 引用必须解析到同模块（同文件）struct/enum 定义
//!   （此前 `validate_type` 的 Custom 分支显式跳过）。
//! - 跨模块命名类型引用 v1 不支持：validator 按单文件校验，引用其它模块的
//!   struct/enum 自然表现为"未知命名类型"错误（本文件用两个独立 parse 模拟）。

use std::collections::HashMap;

use ridl_tool::parser::ast::{IDL, IDLItem};
use ridl_tool::parser::parse_ridl_file;
use ridl_tool::validator::SemanticValidator;

/// 按真实构建路径构造完整 IDL（与 validator/mod.rs 的 validate_with_mode 相同的
/// 装配方式），再走 SemanticValidator::validate。
fn validate_ridl(src: &str) -> Result<(), Vec<ridl_tool::validator::RIDLError>> {
    let parsed = parse_ridl_file(src).expect("parse ridl");
    let mut idl = IDL {
        module: parsed.module.clone(),
        interfaces: vec![],
        classes: vec![],
        enums: vec![],
        structs: vec![],
        functions: vec![],
        using: vec![],
        imports: vec![],
        singletons: vec![],
        callbacks: vec![],
    };
    for item in &parsed.items {
        match item {
            IDLItem::Interface(i) => idl.interfaces.push(i.clone()),
            IDLItem::Class(c) => idl.classes.push(c.clone()),
            IDLItem::Enum(e) => idl.enums.push(e.clone()),
            IDLItem::Struct(s) => idl.structs.push(s.clone()),
            IDLItem::Function(f) => idl.functions.push(f.clone()),
            IDLItem::Singleton(s) => idl.singletons.push(s.clone()),
            _ => {}
        }
    }
    SemanticValidator::new("<mem>".to_string()).validate(&idl)
}

fn first_error_message(src: &str) -> String {
    let err = validate_ridl(src).expect_err("expected validation error");
    assert!(!err.is_empty(), "error list must not be empty");
    err[0].message.clone()
}

#[test]
fn duplicate_struct_and_enum_name_is_rejected() {
    let src = "struct Thing { a: i32; }\nenum Thing { A, B }\n";
    let msg = first_error_message(src);
    assert!(
        msg.contains("Thing") && msg.contains("struct") && msg.contains("enum"),
        "cross-kind duplicate must name both kinds, got: {msg}"
    );
}

#[test]
fn duplicate_struct_and_class_name_is_rejected() {
    let src = "struct Thing { a: i32; }\nclass Thing { fn m() -> void; }\n";
    let msg = first_error_message(src);
    assert!(
        msg.contains("Thing") && msg.contains("struct") && msg.contains("class"),
        "struct/class duplicate must name both kinds, got: {msg}"
    );
}

#[test]
fn duplicate_enum_and_class_name_is_rejected() {
    let src = "enum Color { RED }\nclass Color { fn m() -> void; }\n";
    let msg = first_error_message(src);
    assert!(
        msg.contains("Color") && msg.contains("enum") && msg.contains("class"),
        "enum/class duplicate must name both kinds, got: {msg}"
    );
}

#[test]
fn duplicate_interface_and_struct_name_is_rejected() {
    let src = "interface Shape { fn area() -> f64; }\nstruct Shape { a: i32; }\n";
    let msg = first_error_message(src);
    assert!(
        msg.contains("Shape") && msg.contains("interface") && msg.contains("struct"),
        "interface/struct duplicate must name both kinds, got: {msg}"
    );
}

#[test]
fn duplicate_same_kind_struct_names_are_rejected() {
    let src = "struct Point { x: i32; }\nstruct Point { y: i32; }\n";
    let msg = first_error_message(src);
    assert!(
        msg.contains("Point") && msg.contains("Duplicate"),
        "same-kind duplicate must be reported, got: {msg}"
    );
}

#[test]
fn distinct_names_still_validate() {
    let src = r#"
struct Address { street: string; num: i32; }
struct Person { name: string; address: Address; tags: array<string>; }
enum Color { RED, GREEN, BLUE }
class Widget { fn ping() -> void; }
interface Shape { fn area() -> f64; }
fn makeAddress(a: Address) -> Address;
fn nextColor(c: Color) -> Color;
"#;
    validate_ridl(src).unwrap_or_else(|e| panic!("distinct named types must validate: {e:?}"));
}

#[test]
fn custom_reference_to_defined_struct_passes() {
    let src = "struct Address { street: string; }\nfn echoAddress(a: Address) -> Address;\n";
    validate_ridl(src).unwrap_or_else(|e| panic!("resolved Custom must validate: {e:?}"));
}

#[test]
fn custom_reference_to_defined_enum_passes() {
    let src = "enum Color { RED, GREEN }\nfn nextColor(c: Color) -> Color;\n";
    validate_ridl(src).unwrap_or_else(|e| panic!("resolved enum Custom must validate: {e:?}"));
}

#[test]
fn custom_reference_to_unknown_name_is_rejected() {
    let src = "fn echoAddress(a: Address) -> Address;\n";
    let msg = first_error_message(src);
    assert!(
        msg.contains("Address") && (msg.contains("Unknown") || msg.contains("unknown")),
        "unknown named type must be reported, got: {msg}"
    );
}

#[test]
fn custom_reference_inside_struct_field_must_resolve() {
    let src = "struct Person { address: Missing; }\nstruct Address { street: string; }\n";
    let msg = first_error_message(src);
    assert!(
        msg.contains("Missing"),
        "unknown nested named type must be reported, got: {msg}"
    );
}

#[test]
fn cross_module_named_type_reference_is_not_resolvable() {
    // validator 按单文件（= 单模块）校验：模块 B 引用模块 A 的 struct 无法解析。
    // 跨模块命名类型引用 v1 不支持，在 validator 层以"未知命名类型"拒绝。
    let module_a = "struct Address { street: string; }\n";
    let module_b = "fn echoAddress(a: Address) -> Address;\n";

    let parsed_a = parse_ridl_file(module_a).expect("parse module a");
    let parsed_b = parse_ridl_file(module_b).expect("parse module b");

    let mut names: HashMap<String, ()> = HashMap::new();
    let _ = &mut names;

    let mut idl_b = IDL {
        module: parsed_b.module.clone(),
        interfaces: vec![],
        classes: vec![],
        enums: vec![],
        structs: vec![],
        functions: vec![],
        using: vec![],
        imports: vec![],
        singletons: vec![],
        callbacks: vec![],
    };
    for item in &parsed_b.items {
        if let IDLItem::Function(f) = item {
            idl_b.functions.push(f.clone());
        }
    }
    // 只装配模块 B 自身的定义集（不含模块 A 的 Address）——跨模块引用无法解析。
    let _ = parsed_a;
    let err = SemanticValidator::new("<mem>".to_string())
        .validate(&idl_b)
        .expect_err("cross-module reference must not resolve");
    assert!(
        err[0].message.contains("Address"),
        "cross-module reference must be rejected, got: {:?}",
        err[0].message
    );
}
