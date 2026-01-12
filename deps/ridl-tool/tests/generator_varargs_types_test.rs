use std::fs;

#[test]
fn glue_varargs_string_double_bool_have_rel_and_conversions() {
    let ridl = r#"
fn f(...rest_s: string) -> void;
fn g(...rest_d: double) -> void;
fn h(...rest_b: bool) -> void;
"#;

    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    ridl_tool::validator::validate_with_mode(&parsed.items, parsed.mode).expect("validate ridl");

    let tmp = tempfile::tempdir().expect("tempdir");
    ridl_tool::generator::generate_module_files(&parsed.items, parsed.mode, tmp.path(), "demo")
        .expect("generate module files");

    let glue = fs::read_to_string(tmp.path().join("demo_glue.rs")).expect("read glue");

    // Each varargs loop should compute rel.
    assert!(glue.contains("let rel = i - 0"));

    // string varargs: check + to cstring.
    assert!(glue.contains("invalid string argument: rest_s["));
    assert!(glue.contains("JS_IsString"));
    assert!(glue.contains("JS_ToCString"));

    // double varargs: check + to number.
    assert!(glue.contains("invalid double argument: rest_d["));
    assert!(glue.contains("JS_ToNumber"));

    // bool varargs: tag check.
    assert!(glue.contains("invalid bool argument: rest_b["));
    assert!(glue.contains("JS_TAG_SPECIAL_BITS"));
}
