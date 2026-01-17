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
fn strict_mode_is_propagated_into_generated_glue_any_varargs() {
    let dir = tmpdir("mode_propagation_strict_glue");
    let ridl_path = dir.join("m.ridl");

    fs::write(
        &ridl_path,
        r#"
mode strict;

fn ok(...args: any) -> void;
"#,
    )
    .unwrap();

    let out_dir = dir.join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let parsed = parse_ridl_file(&fs::read_to_string(&ridl_path).unwrap()).unwrap();
    generate_module_files(&parsed.items, parsed.module.clone(), parsed.mode, &out_dir, "m").unwrap();

    let glue = fs::read_to_string(out_dir.join("glue.rs")).unwrap();

    // In strict mode, varargs(any) should be collected as Vec<Local<Value>>.
    assert!(
        glue.contains("Vec<mquickjs_rs::handles::local::Local"),
        "expected strict varargs(any) -> Vec<Local<Value>> in glue; got:\n{glue}"
    );
}
