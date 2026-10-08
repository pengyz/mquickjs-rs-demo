//! 保留字检查的位置信息补全。
//!
//! `validate_identifiers` → `check_for_keyword_usage` 的保留字报错此前
//! 硬编码 line=0, col=0（Interface/Enum/StructDef AST 节点未携带 pos）。
//! 现为定义节点（interface/class/enum/struct/singleton）线程化 pos，
//! 报错 line/column 必须取自定义所在的真实位置。
//!
//! 选词说明：grammar 的 `keyword` 规则（`identifier = @{ !keyword ~ ... }`）
//! 在词法层面已拦截绝大多数保留字（module/class/singleton 等做定义名会在
//! parse 期报 "expected identifier"，到不了 validator）。validator 关键字表
//! 中只有 `callback` 不在 grammar keyword 表里，能以定义名身份通过 parse、
//! 由 validator 报保留字错误，因此本测试用 `callback` 作为保留字。

use ridl_tool::parser::{ast::IDL, parse_ridl_file};
use ridl_tool::validator::{RIDLError, SemanticValidator};

fn first_error(src: &str) -> RIDLError {
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
            ridl_tool::parser::ast::IDLItem::Interface(i) => idl.interfaces.push(i.clone()),
            ridl_tool::parser::ast::IDLItem::Class(c) => idl.classes.push(c.clone()),
            ridl_tool::parser::ast::IDLItem::Enum(e) => idl.enums.push(e.clone()),
            ridl_tool::parser::ast::IDLItem::Struct(s) => idl.structs.push(s.clone()),
            ridl_tool::parser::ast::IDLItem::Singleton(s) => idl.singletons.push(s.clone()),
            _ => {}
        }
    }
    let err = SemanticValidator::new("<mem>".to_string())
        .validate(&idl)
        .expect_err("reserved word must be rejected");
    assert!(!err.is_empty(), "expected at least one error");
    err.into_iter().next().expect("non-empty error list")
}

#[test]
fn interface_named_callback_reports_line_and_column() {
    // 第 1 行第 1 列起始的定义：line==1, column==1
    let err = first_error("interface callback { fn ping() -> void; }\n");
    assert!(
        err.message.contains("callback") && err.message.contains("interface"),
        "message should name identifier and context, got: {}",
        err.message
    );
    assert_eq!(err.line, 1, "interface on line 1");
    assert_eq!(err.column, 1, "interface starting at column 1");
}

#[test]
fn enum_named_callback_reports_line_and_column() {
    // 前导空行使定义落在第 3 行
    let src = "\n\nenum callback { A = 0 }\n";
    let err = first_error(src);
    assert!(
        err.message.contains("callback") && err.message.contains("enum"),
        "message should name identifier and context, got: {}",
        err.message
    );
    assert_eq!(err.line, 3, "enum defined on line 3");
    assert_eq!(err.column, 1, "enum starting at column 1");
}

#[test]
fn struct_named_callback_reports_line_and_column() {
    let err = first_error("struct callback { x: i32; }\n");
    assert!(
        err.message.contains("callback") && err.message.contains("struct"),
        "message should name identifier and context, got: {}",
        err.message
    );
    assert_eq!(err.line, 1, "struct on line 1");
    assert_eq!(err.column, 1, "struct starting at column 1");
}

#[test]
fn class_and_singleton_reserved_names_report_line_and_column() {
    // class/singleton 节点原本已有 pos，保留字检查同样应线程化
    let err = first_error("class callback { }\n");
    assert!(
        err.message.contains("callback") && err.message.contains("class"),
        "message should name identifier and context, got: {}",
        err.message
    );
    assert_eq!(err.line, 1, "class on line 1");
    assert_eq!(err.column, 1, "class starting at column 1");

    let err = first_error("singleton callback { fn ping() -> void; }\n");
    assert!(
        err.message.contains("callback") && err.message.contains("singleton"),
        "message should name identifier and context, got: {}",
        err.message
    );
    assert_eq!(err.line, 1, "singleton on line 1");
    assert_eq!(err.column, 1, "singleton starting at column 1");
}
