use ridl_tool::generator::generate_module_files;
use ridl_tool::parser::parse_ridl_file;
use std::fs;

fn tmpdir(name: &str) -> std::path::PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("ridl_tool_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn nullable_union_is_normalized_in_api_and_glue() {
    let dir = tmpdir("union_nullable_normalization");
    let ridl_path = dir.join("m.ridl");

    fs::write(
        &ridl_path,
        r#"
module test@1.0
interface Test {
    fn a(v: string | i32 | null) -> string | i32 | null;
    fn b(v: string | i32 | null) -> string | i32 | null;
}
"#,
    )
    .unwrap();

    let out_dir = dir.join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let parsed = parse_ridl_file(&fs::read_to_string(&ridl_path).unwrap()).unwrap();
    generate_module_files(&parsed.items, parsed.module.clone(), parsed.mode, &out_dir, "m").unwrap();

    let api = fs::read_to_string(out_dir.join("api.rs")).unwrap();
    let glue = fs::read_to_string(out_dir.join("glue.rs")).unwrap();

    // Never leak raw RIDL union syntax into Rust type positions.
    assert!(
        !api.contains("(string | i32)"),
        "api.rs must not contain raw union syntax; got:\n{api}"
    );
    assert!(
        !glue.contains("(string | i32)"),
        "glue.rs must not contain raw union syntax; got:\n{glue}"
    );

    // Both nullable union spellings must be normalized to Option<...>.
    assert!(
        api.contains("Option<crate::api::test::union::UnionI32String>"),
        "expected Option<...UnionI32String> in api.rs; got:\n{api}"
    );
    assert!(
        glue.contains("Option<crate::api::test::union::UnionI32String>"),
        "expected Option<...UnionI32String> in glue.rs; got:\n{glue}"
    );
}
