use std::fs;
use std::path::PathBuf;

use ridl_tool::generator::generate_aggregate_consolidated;
use ridl_tool::plan::{GeneratedPaths, RidlModule, RidlPlan};

fn ridl_tmp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("mquickjs-ridl-test-{name}-{}", std::process::id()));
    dir
}

/// Aggregated ridl_bootstrap.rs emits `{{ crate_name }}::initialize_module()`
/// per module crate. Crate names there are PACKAGE names and may contain '-'
/// (first real case: the `mquickjs-ui` binding crate); the emitted Rust path
/// must be the ident-normalized lib target (hyphens -> underscores), or the
/// consumer crate fails to compile with E0425/E0433 on the generated file.
#[test]
fn aggregate_bootstrap_normalizes_hyphenated_crate_names() {
    let out_dir = ridl_tmp_dir("bootstrap-ident");
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).unwrap();

    let module = RidlModule {
        crate_name: "mquickjs-ui".to_string(),
        name: "mquickjs-ui".to_string(),
        crate_dir: PathBuf::from("."),
        ridl_files: vec![PathBuf::from(
            "tests/fixtures_bootstrap_crate_ident.ridl",
        )],
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

    let bootstrap = fs::read_to_string(out_dir.join("ridl_bootstrap.rs")).unwrap();
    assert!(
        bootstrap.contains("mquickjs_ui::initialize_module()"),
        "bootstrap must emit the ident-normalized crate path: {bootstrap}"
    );
    assert!(
        !bootstrap.contains("mquickjs-ui::"),
        "hyphenated package name is not a valid Rust path: {bootstrap}"
    );

    let _ = fs::remove_dir_all(&out_dir);
}
