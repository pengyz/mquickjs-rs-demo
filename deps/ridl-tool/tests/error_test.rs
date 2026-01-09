use ridl_tool;

#[test]
fn syntax_error_is_reported() {
    // 缺少右括号
    let invalid_ridl = r#"interface Test { fn method(int x) -> string; "#;

    let result = ridl_tool::parse_ridl(invalid_ridl);
    assert!(result.is_err());
}

#[test]
fn valid_ridl_parses() {
    let valid_ridl = r#"
module test@1.0
interface Test {
    fn method(x: int) -> string;
}
"#;

    let items = ridl_tool::parse_ridl(valid_ridl).expect("valid ridl must parse");
    assert!(!items.is_empty());
}
