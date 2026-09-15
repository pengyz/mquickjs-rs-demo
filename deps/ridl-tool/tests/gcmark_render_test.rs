//! class 级 `gc_mark` **已废弃**：头文件回归守卫。
//!
//! 此前本测试验证"含 `Traced<T>` 字段的 class 会在注册头与 API 头中声明
//! `gc_mark` 并接入 `JS_CLASS_DEF`"。该机制已被整体移除：
//!
//! - 生成的 Rust 定义是 4 参，而引擎契约是 3 参 `(ctx, opaque, mf)`，
//!   错位后会在 GC 时把 `JSMarkFunc` 当 fat pointer 解引用 → SIGSEGV；
//! - `Traced<T>` 现基于引擎的 `JSGCRef`，引擎已在 mark 与重定位两个阶段
//!   自动处理，class 回调纯属冗余。
//!
//! 因此现在断言的是：**任何 class 都不再产生 gc_mark 声明或注册表项**。

use std::fs;
use std::path::PathBuf;

use ridl_tool::generator::generate_aggregate_consolidated;
use ridl_tool::plan::{GeneratedPaths, RidlModule, RidlPlan};

#[test]
fn no_gc_mark_declared_or_registered_for_any_class() {
    let tempdir = tempfile::tempdir().unwrap();
    let out_dir = tempdir.path().to_path_buf();

    let module1 = RidlModule {
        crate_name: "m1".to_string(),
        name: "m1".to_string(),
        crate_dir: PathBuf::from("."),
        ridl_files: vec![PathBuf::from("tests/fixtures_gcmark_render.ridl")],
    };

    let plan = RidlPlan {
        schema_version: 0,
        cargo_toml: PathBuf::from("Cargo.toml"),
        modules: vec![module1],
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

    let hdr = fs::read_to_string(out_dir.join("mquickjs_ridl_register.h")).unwrap();
    let api_hdr = fs::read_to_string(out_dir.join("mquickjs_ridl_api.h")).unwrap();

    // 任何 class（含带 Traced 字段的 GcNode）都不应再有 gc_mark 符号。
    for name in ["gcnode", "plainnode"] {
        let sym = format!("js_m1_class_{name}_gc_mark");
        assert!(
            !hdr.contains(&sym),
            "注册头不应再出现 {sym}，实际:\n{hdr}"
        );
        assert!(
            !api_hdr.contains(&sym),
            "API 头不应再出现 {sym}，实际:\n{api_hdr}"
        );
    }

    // 注册表里的 gc_mark 槽位恒为 NULL（JS_CLASS_DEF 的 gc_mark 位置参数）。
    assert!(
        hdr.contains("js_m1_class_gcnode_finalizer,\n        NULL"),
        "GcNode 的 gc_mark 槽位应为 NULL，实际:\n{hdr}"
    );

    // keepalive 函数不应再引用 gc_mark 符号。
    assert!(
        !hdr.contains("js_m1_class_gcnode_gc_mark;"),
        "keepalive 不应再引用 gc_mark，实际:\n{hdr}"
    );
}