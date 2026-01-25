use ridl_tool::{parser::parse_ridl_file, validator::validate_with_mode};

#[test]
fn map_key_must_be_primitive() {
    let bad = r#"
module test@1.0

interface Test {
    fn a(v: map<object, i32>) -> void;
    fn b(v: map<any, i32>) -> void;
    fn c(v: map<array<string>, i32>) -> void;
    fn d(v: map<(string | i32), i32>) -> void;
    fn e(v: map<string?, i32>) -> void;
}
"#;

    let parsed = parse_ridl_file(bad).unwrap();
    let err = validate_with_mode(&parsed.items, parsed.mode).expect_err("expected map key restriction");
    let msg = err.to_string();
    assert!(msg.contains("map<K, V>"), "unexpected error: {msg}");
}

#[test]
fn map_key_allows_primitives() {
    let ok = r#"
module test@1.0

interface Test {
    fn a(v: map<string, i32>) -> void;
    fn b(v: map<bool, any?>) -> void;
    fn c(v: map<i32, (string | i32)?>) -> void;
    fn d(v: map<i64, f64>) -> void;
}
"#;

    let parsed = parse_ridl_file(ok).unwrap();
    validate_with_mode(&parsed.items, parsed.mode).expect("expected ok");
}
