use std::fs;

#[test]
fn glue_generation_uses_template_filters_for_params() {
    let ridl = r#"
fn f(a: int, b: string, c: double, d: bool, ...rest: any) -> void;
"#;

    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    ridl_tool::validator::validate_with_mode(&parsed.items, parsed.mode).expect("validate ridl");

    let tmp = tempfile::tempdir().expect("tempdir");
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.mode,
        tmp.path(),
        "demo",
    )
    .expect("generate module files");

    let glue = fs::read_to_string(tmp.path().join("demo_glue.rs")).expect("read glue");

    // Ensure there is no leftover placeholder replacement logic in output.
    assert!(!glue.contains("{IDX}"), "glue should not contain {{IDX}}");
    assert!(!glue.contains("{IDX0}"), "glue should not contain {{IDX0}}");

    // Spot-check: missing argument checks are generated for required params.
    assert!(glue.contains("missing argument: a"));
    assert!(glue.contains("missing argument: b"));

    // Spot-check: type checks + conversions.
    assert!(glue.contains("invalid int argument: a"));
    assert!(glue.contains("JS_ToInt32"));

    assert!(glue.contains("invalid string argument: b"));
    assert!(glue.contains("JS_IsString"));
    assert!(glue.contains("JS_ToCString"));

    assert!(glue.contains("invalid double argument: c"));
    assert!(glue.contains("JS_ToNumber"));

    assert!(glue.contains("invalid bool argument: d"));
    assert!(glue.contains("invalid bool argument: d"));

    // Spot-check: varargs any collects argv slice.
    // With 4 fixed params, varargs starts at idx0=4.
    assert!(glue.contains("let mut rest: Vec<JSValue>"));
    assert!(glue.contains("for i in 4..(argc as usize)"));

    // For `any` varargs we no longer emit `rel` (it was only used for error messages).
    // Keep this check flexible: we only care that argv is pushed and the index is `i`.
    assert!(glue.contains("rest.push("));
    assert!(glue.contains("*argv.add(i)"));
}
