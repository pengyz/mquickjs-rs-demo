//! 聚合 register.c 的 proto-var 空表门控：聚合无 proto var 时不生成
//! `ridl_proto_var_entries[]`（空表无引用方，在 -Werror 构建下会以
//! unused-const-variable 失败——OpenVela/NuttX CMake 轨全局 -Werror）。

use std::fs;
use std::path::PathBuf;

use ridl_tool::generator::generate_aggregate_consolidated;
use ridl_tool::plan::{GeneratedPaths, RidlModule, RidlPlan};

fn render_aggregate(ridl: &str, fixture_name: &str) -> String {
    let fixture_path = PathBuf::from(fixture_name);
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
    let _ = fs::remove_file(&fixture_path);
    c
}

#[test]
fn no_proto_vars_aggregate_omits_proto_var_table() {
    // 只有方法/单例的聚合（如 sim 应用：stdlib console 单例），没有 proto var。
    let c = render_aggregate(
        "class Plain {\n    fn f() -> void;\n}\n",
        "fixtures_proto_var_empty.ridl",
    );
    assert!(
        !c.contains("ridl_proto_var_entries"),
        "aggregate without proto vars must not emit the unused table, c:\n{c}"
    );
}

#[test]
fn with_proto_vars_aggregate_still_emits_table() {
    // 有 proto var 时表照常生成（既有行为不回归；转义细节另见
    // generator_js_field_escape_test）。
    let c = render_aggregate(
        "class Box {\n    proto var n: i32 = 1;\n    fn f() -> void;\n}\n",
        "fixtures_proto_var_present.ridl",
    );
    assert!(
        c.contains("ridl_proto_var_entries"),
        "aggregate with proto vars must keep the table, c:\n{c}"
    );
    assert!(
        c.contains("ridl_install_proto_vars_all"),
        "install function must remain, c:\n{c}"
    );
}
