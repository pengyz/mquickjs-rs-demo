use std::fs;
use std::path::PathBuf;

use ridl_tool::generator::generate_aggregate_consolidated;
use ridl_tool::plan::{GeneratedPaths, RidlModule, RidlPlan};

fn ridl_tmp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("mquickjs-ridl-test-{name}-{}", std::process::id()));
    dir
}

#[test]
fn js_class_ids_are_global_monotonic_across_modules() {
    let out_dir = ridl_tmp_dir("class-id");
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).unwrap();

    // Two modules (with module decl), each has one class.
    // Module object ids must be 0,1 and user class ids must follow (2,3).
    let module1 = RidlModule {
        crate_name: "m1".to_string(),
        name: "m1".to_string(),
        crate_dir: PathBuf::from("."),
        ridl_files: vec![PathBuf::from("tests/fixtures_module_class_id_m1.ridl")],
    };

    let module2 = RidlModule {
        crate_name: "m2".to_string(),
        name: "m2".to_string(),
        crate_dir: PathBuf::from("."),
        ridl_files: vec![PathBuf::from("tests/fixtures_module_class_id_m2.ridl")],
    };

    let plan = RidlPlan {
        schema_version: 0,
        cargo_toml: PathBuf::from("Cargo.toml"),
        modules: vec![module1, module2],
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

    assert!(
        hdr.contains("#define JS_CLASS_M1_1_0_MODULE (JS_CLASS_USER + 0)"),
        "{hdr}"
    );
    assert!(
        hdr.contains("#define JS_CLASS_M2_1_0_MODULE (JS_CLASS_USER + 1)"),
        "{hdr}"
    );

    assert!(
        hdr.contains("#define JS_CLASS_M1_USERA (JS_CLASS_USER + 2)"),
        "{hdr}"
    );
    assert!(
        hdr.contains("#define JS_CLASS_M2_USERB (JS_CLASS_USER + 3)"),
        "{hdr}"
    );

    assert!(
        hdr.contains("#define JS_CLASS_COUNT (JS_CLASS_USER + 4)"),
        "{hdr}"
    );
}
