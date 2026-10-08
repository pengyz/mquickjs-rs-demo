//! C.3 渲染断言：GLOBAL singleton 的 plain var 字段安装进 `JS_RIDL_StdlibInit`。
//!
//! 设计裁定（Phase C，对抗复核）：
//! - 安装机制不经 Rust glue：register.c 的 `JS_RIDL_StdlibInit` 内
//!   `JS_GetGlobalObject` → `JS_GetPropertyStr(singleton 名)` → 逐字段
//!   `JS_SetPropertyStr`（writable），任一失败 `return -1`。
//! - 仅 GLOBAL 模式（module_decl.is_none()）的 singleton 生成安装代码。
//! - init_literal 是解码后原文，嵌入 C 字符串前必须经 `escape_c_string` 转义。

use std::fs;
use std::path::PathBuf;

use ridl_tool::generator::generate_aggregate_consolidated;
use ridl_tool::plan::{GeneratedPaths, RidlModule, RidlPlan};

fn run_generate(ridl_files: &[PathBuf], out_dir: &PathBuf) {
    let module = RidlModule {
        crate_name: "m1".to_string(),
        name: "m1".to_string(),
        crate_dir: PathBuf::from("."),
        ridl_files: ridl_files.to_vec(),
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

    generate_aggregate_consolidated(&plan, out_dir).unwrap();
}

#[test]
fn stdlib_init_installs_global_singleton_var_fields() {
    let tempdir = tempfile::tempdir().unwrap();
    let out_dir = tempdir.path().to_path_buf();

    run_generate(
        &[PathBuf::from("tests/fixtures_singleton_var.ridl")],
        &out_dir,
    );

    let c = fs::read_to_string(out_dir.join("mquickjs_ridl_register.c")).unwrap();

    // Per-singleton install function exists and resolves the singleton object
    // from the global object.
    assert!(
        c.contains("ridl_install_singleton_store_fields"),
        "register.c should contain a Store field install function:\n{c}"
    );
    assert!(
        c.contains(r#"JS_GetPropertyStr(ctx, global_obj, "Store")"#),
        "install code must fetch the singleton object by name:\n{c}"
    );

    // i32 field.
    assert!(
        c.contains("JS_NewInt32(ctx, 42)"),
        "count field should be initialized with JS_NewInt32(ctx, 42):\n{c}"
    );
    // bool field. JS_NewBool is the 1-arg static inline (no ctx) in mquickjs.h;
    // the RIDL bool literal is mapped to 1/0 (no stdbool in this C file).
    assert!(
        c.contains("JS_NewBool(1)") && !c.contains("JS_NewBool(0)"),
        "flag=true field should be initialized with JS_NewBool(1):\n{c}"
    );
    // string field: escaped quote must survive into the generated C literal.
    assert!(
        c.contains(r#"JS_NewString(ctx, "he said \"hi\"")"#),
        "title field should carry the escape_c_string'd literal:\n{c}"
    );
    // null field.
    assert!(
        c.contains("JS_NULL"),
        "nothing field should be initialized with JS_NULL:\n{c}"
    );

    // Fields are set as (writable) own properties on the singleton object.
    assert!(
        c.contains(r#"JS_SetPropertyStr(ctx, obj, "count""#)
            && c.contains(r#"JS_SetPropertyStr(ctx, obj, "flag""#)
            && c.contains(r#"JS_SetPropertyStr(ctx, obj, "title""#)
            && c.contains(r#"JS_SetPropertyStr(ctx, obj, "nothing""#),
        "install code must set each field on the singleton object:\n{c}"
    );

    // The install is wired into JS_RIDL_StdlibInit.
    let init_body = c
        .split("int JS_RIDL_StdlibInit(JSContext *ctx)")
        .nth(1)
        .expect("JS_RIDL_StdlibInit must exist");
    assert!(
        init_body.contains("ridl_install_singleton_store_fields"),
        "JS_RIDL_StdlibInit must call the singleton field install:\n{init_body}"
    );
}

#[test]
fn module_mode_singleton_fields_are_not_installed() {
    let tempdir = tempfile::tempdir().unwrap();
    let out_dir = tempdir.path().to_path_buf();

    run_generate(
        &[PathBuf::from("tests/fixtures_singleton_var_module.ridl")],
        &out_dir,
    );

    let c = fs::read_to_string(out_dir.join("mquickjs_ridl_register.c")).unwrap();
    assert!(
        !c.contains("ridl_install_singleton"),
        "module-mode singletons must not generate field install code:\n{c}"
    );
}

#[test]
fn register_h_and_glue_are_unaffected_by_singleton_var_fields() {
    // 行为不变性：js_fields 只挂 JS 对象；register.h 的 props 表与 Rust glue
    // 都不为 var 字段生成任何成员。
    let tempdir = tempfile::tempdir().unwrap();
    let out_dir = tempdir.path().to_path_buf();

    run_generate(
        &[PathBuf::from("tests/fixtures_singleton_var.ridl")],
        &out_dir,
    );

    let hdr = fs::read_to_string(out_dir.join("mquickjs_ridl_register.h")).unwrap();
    assert!(
        !hdr.contains(r#""count""#) && !hdr.contains(r#""title""#),
        "register.h singleton props must not include js-only var fields:\n{hdr}"
    );
}
