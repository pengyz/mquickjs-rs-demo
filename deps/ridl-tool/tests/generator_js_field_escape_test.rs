//! P0 债项 1（伴生修复）：class js_fields / proto var 的字符串字面量转义。
//!
//! init_literal 存的是解码后原文；嵌入生成源码前必须经转义 filter
//! （singleton 安装段 Phase C 已用 `escape_c_string`，本文件钉死
//! class glue 与 C proto var 两处同款修复）。

use std::fs;
use std::path::PathBuf;

use ridl_tool::generator::{generate_aggregate_consolidated, generate_module_files};
use ridl_tool::parser::parse_ridl_file;
use ridl_tool::plan::{GeneratedPaths, RidlModule, RidlPlan};
use ridl_tool::validator::validate_with_mode;

const ESCAPED_FIELD_RIDL: &str = "class Node {\n    var s: string = \"he said \\\"hi\\\"\";\n    fn f() -> void;\n}\n";

#[test]
fn class_js_field_string_literal_is_escaped_in_glue() {
    let parsed = parse_ridl_file(ESCAPED_FIELD_RIDL).expect("parse ridl");
    validate_with_mode(&parsed.items, parsed.mode).expect("validate ridl");
    let tmp = tempfile::tempdir().expect("tempdir");
    generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        tmp.path(),
        "demo",
    )
    .expect("generate module files");
    let glue = fs::read_to_string(tmp.path().join("glue.rs")).expect("read glue");

    // 转义后的 Rust 源码形态：`he said \"hi\"`（反斜杠 + 引号）
    assert!(
        glue.contains("he said \\\"hi\\\""),
        "glue must escape embedded quotes in js field literals, glue:\n{glue}"
    );
    // 裸引号会生成非法 Rust（CString::new(""hi"...)）——必须不存在
    assert!(
        !glue.contains("he said \"hi\""),
        "raw unescaped quote must not appear in generated glue"
    );
}

#[test]
fn class_proto_var_string_literal_is_escaped_in_aggregate_c() {
    let ridl = "class Box {\n    proto var ps: string = \"a \\\"b\\\" c\";\n    fn f() -> void;\n}\n";
    let fixture_path = PathBuf::from("tests/fixtures_proto_var_escape.ridl");
    fs::write(&fixture_path, ridl).expect("write fixture");

    let tempdir = tempfile::tempdir().unwrap();
    let out_dir = tempdir.path().to_path_buf();

    let module = RidlModule {
        crate_name: "m1".to_string(),
        name: "m1".to_string(),
        crate_dir: PathBuf::from("."),
        ridl_files: vec![fixture_path.clone()],
    };
    let plan = RidlPlan {
        schema_version: 0,
        cargo_toml: PathBuf::from("Cargo.toml"),
        modules: vec![module],
        generated: GeneratedPaths {
            out_dir: out_dir.clone(),
            mquickjs_ridl_register_h: out_dir.join("mquickjs_ridl_register.h"),
            mquickjs_ridl_module_class_ids_h: out_dir.join("mquickjs_ridl_module_class_ids.h"),
            mqjs_ridl_user_class_ids_h: out_dir.join("mqjs_ridl_user_class_ids.h"),
            ridl_class_id_rs: out_dir.join("ridl_class_id.rs"),
        },
        inputs: vec![],
    };
    generate_aggregate_consolidated(&plan, &out_dir).unwrap();

    let c = fs::read_to_string(out_dir.join("mquickjs_ridl_register.c")).unwrap();
    // C 转义形态：`a \"b\" c`
    assert!(
        c.contains("a \\\"b\\\" c"),
        "aggregate register.c must escape proto var string literals, c:\n{c}"
    );

    let _ = fs::remove_file(&fixture_path);
}
