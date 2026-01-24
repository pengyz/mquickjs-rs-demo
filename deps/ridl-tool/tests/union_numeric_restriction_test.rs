use ridl_tool::{
    generator::generate_module_files,
    parser::parse_ridl_file,
    validator::validate_with_mode,
};

#[test]
fn union_rejects_multiple_numeric_primitives() {
    // V1 rule: union may contain at most one numeric primitive.
    // Rationale: avoid heuristic numeric discrimination/priority when decoding JS values.
    let ridl = r#"
module test@1.0

interface Test {
    fn bad1(v: i32 | i64) -> i64;
    fn bad2(v: f32 | f64) -> f64;
    fn bad3(v: i32 | f64 | i64) -> f64;
}
"#;

    let parsed = parse_ridl_file(ridl).unwrap();

    let err = validate_with_mode(&parsed.items, parsed.mode)
        .expect_err("multi-numeric union should be rejected");

    let msg = err.to_string();
    assert!(
        msg.contains("最多出现 1 个"),
        "unexpected error: {msg}"
    );
}

#[test]
fn union_allows_single_numeric_plus_other_members_and_nullable() {
    let ridl = r#"
module test@1.0

interface Test {
    fn ok1(v: string | i32) -> string | i32;
    fn ok2(v: string | i32 | null) -> (string | i32)?;
    fn ok3(v: (string | i32)?) -> (string | i32)?;
}
"#;

    let parsed = parse_ridl_file(ridl).unwrap();

    validate_with_mode(&parsed.items, parsed.mode).expect("expected no diagnostics");

    // Ensure codegen still works on allowed shapes.
    let out_dir = std::env::temp_dir().join("ridl-tool-tests").join("union-numeric-restriction");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();

    generate_module_files(&parsed.items, parsed.module.clone(), parsed.mode, &out_dir, "m").unwrap();
}
