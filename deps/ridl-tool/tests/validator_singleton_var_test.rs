//! C.2: singleton js_fields 的语义校验（Phase C，复核裁定）。
//!
//! 规则：
//! - plain `var` 字面量类型集为 `{i32, bool, string, null}`；I64/F32/F64 等
//!   其它类型在 singleton 侧**明确拒绝**（比 class 侧"允许但 glue panic"更严，
//!   这是有意的——错误信息需说明当前支持集）。
//! - `proto var` 在 singleton 上明确拒绝：singleton 是 `JS_OBJECT_DEF` 匿名
//!   class（无 class id、无可寻址 proto）。
//! - js_fields 与 methods/properties 名称冲突、js_fields 重复名均拒绝。
//! - 错误定位对齐 js_fields_error_location_test.rs（line/column > 0）。

use ridl_tool::parser::{ast::IDL, parse_ridl_file};
use ridl_tool::validator::SemanticValidator;

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
            ridl_tool::parser::ast::IDLItem::Singleton(s) => idl.singletons.push(s.clone()),
            ridl_tool::parser::ast::IDLItem::Class(c) => idl.classes.push(c.clone()),
            _ => {}
        }
    }
    SemanticValidator::new("<mem>".to_string()).validate(&idl)
}

fn first_error_message(src: &str) -> (String, usize, usize) {
    let err = validate_ridl(src).expect_err("expected validation error");
    assert!(!err.is_empty());
    let e = &err[0];
    (e.message.clone(), e.line, e.column)
}

const PRELUDE: &str = "singleton Store {\n";

#[test]
fn singleton_var_valid_supported_set_passes() {
    let src = r#"
singleton Store {
    var count: i32 = 42;
    var flag: bool = false;
    var title: string = "hello";
    var nothing: null = null;
    fn describe() -> string;
}
"#;
    validate_ridl(src).expect("supported literal set must validate");
}

#[test]
fn singleton_var_name_conflict_with_method_is_rejected_with_location() {
    let src = "singleton Store {\n    fn describe() -> void;\n    var describe: i32 = 1;\n}\n";
    let (msg, line, col) = first_error_message(src);
    assert!(
        msg.contains("describe") && msg.contains("singleton"),
        "message should name the field and singleton, got: {msg}"
    );
    assert!(line > 0, "line should be > 0, got {line}");
    assert!(col > 0, "column should be > 0, got {col}");
}

#[test]
fn singleton_var_name_conflict_with_property_is_rejected() {
    let src = "singleton Store {\n    property count: i32;\n    var count: i32 = 1;\n}\n";
    let (msg, _, _) = first_error_message(src);
    assert!(
        msg.contains("count"),
        "message should name the conflicting field, got: {msg}"
    );
}

#[test]
fn singleton_var_duplicate_names_are_rejected() {
    let src = "singleton Store {\n    var count: i32 = 1;\n    var count: i32 = 2;\n}\n";
    let (msg, line, col) = first_error_message(src);
    assert!(
        msg.contains("Duplicate") && msg.contains("count"),
        "message should report duplicate field, got: {msg}"
    );
    assert!(line > 0 && col > 0, "location should be set, got {line}:{col}");
}

#[test]
fn singleton_var_unsupported_type_names_supported_set() {
    // I64/F32/F64 are deliberately stricter than the class side (which allows
    // them but panics in glue): the error must state the supported set.
    for ty in ["i64", "f32", "f64"] {
        let src = format!("{PRELUDE}    var big: {ty} = 1;\n}}\n");
        let (msg, line, col) = first_error_message(&src);
        assert!(
            msg.contains("i32") && msg.contains("bool") && msg.contains("string") && msg.contains("null"),
            "error for '{ty}' should list supported set {{i32, bool, string, null}}, got: {msg}"
        );
        assert!(line > 0 && col > 0, "location should be set, got {line}:{col}");
    }
}

#[test]
fn singleton_var_literal_type_mismatch_is_rejected() {
    // i32 field with string literal
    let src = "singleton Store {\n    var count: i32 = \"x\";\n}\n";
    let (msg, _, _) = first_error_message(src);
    assert!(
        msg.contains("count"),
        "message should name the mismatched field, got: {msg}"
    );

    // bool field with integer literal
    let src = "singleton Store {\n    var flag: bool = 1;\n}\n";
    let (msg, _, _) = first_error_message(src);
    assert!(msg.contains("flag"), "got: {msg}");

    // null field with non-null literal
    let src = "singleton Store {\n    var nothing: null = 3;\n}\n";
    let (msg, _, _) = first_error_message(src);
    assert!(msg.contains("nothing"), "got: {msg}");
}

#[test]
fn singleton_proto_var_is_rejected_with_location() {
    let src = "singleton Store {\n    proto var pcount: i32 = 7;\n    fn ping() -> i32;\n}\n";
    let (msg, line, col) = first_error_message(src);
    assert!(
        msg.contains("proto") && msg.contains("pcount") && msg.contains("singleton"),
        "message should explain singleton proto var rejection, got: {msg}"
    );
    assert!(line > 0, "line should be > 0, got {line}");
    assert!(col > 0, "column should be > 0, got {col}");
}
