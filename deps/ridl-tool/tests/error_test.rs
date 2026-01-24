use ridl_tool;

#[test]
fn syntax_error_is_reported() {
    // 缺少右括号
    let invalid_ridl = r#"interface Test { fn method(i32 x) -> string; "#;

    let result = ridl_tool::parse_ridl(invalid_ridl);
    assert!(result.is_err());
}

#[test]
fn valid_ridl_parses() {
    let valid_ridl = r#"
module test@1.0
interface Test {
    fn method(x: i32) -> string;
}
"#;

    let items = ridl_tool::parse_ridl(valid_ridl).expect("valid ridl must parse");
    assert!(!items.is_empty());
}
