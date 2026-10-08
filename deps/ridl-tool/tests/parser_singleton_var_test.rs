//! C.1: singleton 成员支持 plain `var`（js-only 字段）。
//!
//! 设计裁定（docs/planning/2026/2026-10-08-todo-items-resolution.md Phase C）：
//! - singleton 复用 class 侧 `JsField` 结构（`Singleton.js_fields`）。
//! - `proto var` 在 singleton 上也要能被 parser 接受（由 validator 拒绝，
//!   错误信息更友好——grammar 层不做排除）。
//! - string 字面量的 `init_literal` 是解码后原文（与 class 侧一致）。

use ridl_tool::parser::ast::{IDLItem, JsFieldKind, PropertyModifier, Type};

#[test]
fn test_parse_singleton_plain_var_fields() {
    let input = r#"
        singleton Store {
            var count: i32 = 42;
            var flag: bool = true;
            var title: string = "hello";
            var nothing: null = null;
            fn describe() -> string;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Singleton(s) => {
            assert_eq!(s.name, "Store");
            assert_eq!(s.methods.len(), 1);
            assert_eq!(s.methods[0].name, "describe");
            assert_eq!(s.js_fields.len(), 4);

            let f0 = &s.js_fields[0];
            assert_eq!(f0.name, "count");
            assert_eq!(f0.field_type, Type::I32);
            assert_eq!(f0.init_literal, "42");
            assert_eq!(f0.kind, JsFieldKind::Var);
            assert!(!f0.modifiers.contains(&PropertyModifier::Proto));

            let f1 = &s.js_fields[1];
            assert_eq!(f1.name, "flag");
            assert_eq!(f1.field_type, Type::Bool);
            assert_eq!(f1.init_literal, "true");

            let f2 = &s.js_fields[2];
            assert_eq!(f2.name, "title");
            assert_eq!(f2.field_type, Type::String);
            // init_literal stores the decoded text (no surrounding quotes).
            assert_eq!(f2.init_literal, "hello");

            let f3 = &s.js_fields[3];
            assert_eq!(f3.name, "nothing");
            assert_eq!(f3.field_type, Type::Null);
            assert_eq!(f3.init_literal, "null");
        }
        _ => panic!("Expected Singleton item"),
    }
}

#[test]
fn test_parse_singleton_var_escaped_string_literal() {
    // Escape convention follows decode_ridl_string_literal (parser/mod.rs):
    // - `\"` decodes to a literal quote;
    // - other escapes use the doubled-backslash form (`\\n` -> LF).
    let input = r#"
        singleton Store {
            var escaped: string = "he said \"hi\"";
            var multiline: string = "l1\\nl2";
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    match &items[0] {
        IDLItem::Singleton(s) => {
            assert_eq!(s.js_fields.len(), 2);
            assert_eq!(s.js_fields[0].init_literal, "he said \"hi\"");
            assert_eq!(s.js_fields[1].init_literal, "l1\nl2");
        }
        _ => panic!("Expected Singleton item"),
    }
}

#[test]
fn test_parse_singleton_proto_var_parses_for_validator_rejection() {
    // proto var must PARSE (so the validator can reject it with a friendly,
    // located message) — the grammar layer must not exclude it.
    let input = r#"
        singleton Store {
            proto var pcount: i32 = 7;
            fn ping() -> i32;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("proto var must parse");
    match &items[0] {
        IDLItem::Singleton(s) => {
            assert_eq!(s.js_fields.len(), 1);
            let f = &s.js_fields[0];
            assert_eq!(f.name, "pcount");
            assert_eq!(f.field_type, Type::I32);
            assert_eq!(f.init_literal, "7");
            assert!(
                f.modifiers.contains(&PropertyModifier::Proto),
                "proto modifier must be preserved for validator rejection"
            );
        }
        _ => panic!("Expected Singleton item"),
    }
}

#[test]
fn test_parse_singleton_without_var_has_empty_js_fields() {
    let input = r#"
        singleton TestPing {
            fn ping() -> string;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    match &items[0] {
        IDLItem::Singleton(s) => {
            assert_eq!(s.js_fields.len(), 0);
        }
        _ => panic!("Expected Singleton item"),
    }
}

#[test]
fn test_singleton_js_field_types_are_not_class_ref_rewritten() {
    // js_fields are JS-only and restricted to primitive/null literals by the
    // validator; like the class side, their types must NOT be rewritten from
    // Custom to ClassRef (class_ref_rewrite.rs deliberately skips js_fields).
    let input = r#"
        class Widget {
            fn w() -> void;
        }
        singleton Store {
            var thing: Widget = null;
            fn make() -> Widget;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    let s = match &items[1] {
        IDLItem::Singleton(s) => s,
        _ => panic!("Expected Singleton item"),
    };
    assert_eq!(s.methods[0].return_type, Type::ClassRef("Widget".into()));
    assert_eq!(
        s.js_fields[0].field_type,
        Type::Custom("Widget".into()),
        "js field type must stay Custom (no ClassRef rewrite)"
    );
}
