use std::fs;
use std::path::PathBuf;

use ridl_tool::generator::generate_aggregate_consolidated;
use ridl_tool::plan::{GeneratedPaths, RidlModule, RidlPlan};

#[test]
fn gc_mark_registered_in_header_for_traced_class_only() {
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

    // GcNode has a Traced<i32> opaque field -> gc_mark decl + registration expected.
    assert!(
        hdr.contains("void js_m1_class_gcnode_gc_mark(\n    JSContext *ctx,\n    void *opaque,\n    const JSMarkFunc *mf\n);"),
        "expected gc_mark declaration for GcNode, got:\n{hdr}"
    );
    assert!(
        hdr.contains("js_m1_class_gcnode_finalizer,\n        js_m1_class_gcnode_gc_mark"),
        "expected gc_mark wired into JS_CLASS_DEF for GcNode, got:\n{hdr}"
    );
    assert!(
        hdr.contains("(void)&js_m1_class_gcnode_gc_mark;"),
        "expected gc_mark referenced in keepalive function for GcNode, got:\n{hdr}"
    );

    // PlainNode has no Traced fields -> no gc_mark decl, NULL passed instead.
    assert!(
        !hdr.contains("js_m1_class_plainnode_gc_mark"),
        "did not expect gc_mark symbol for PlainNode, got:\n{hdr}"
    );
    assert!(
        hdr.contains("js_m1_class_plainnode_finalizer,\n        NULL\n    );"),
        "expected NULL gc_mark for PlainNode, got:\n{hdr}"
    );

    // Also verify mquickjs_ridl_api.h gets the declaration (this is the header the
    // ROM build actually includes for js_c_mark_table scanning).
    let api_hdr = fs::read_to_string(out_dir.join("mquickjs_ridl_api.h")).unwrap();
    assert!(
        api_hdr.contains("void js_m1_class_gcnode_gc_mark(\n    JSContext *ctx,\n    void *opaque,\n    const JSMarkFunc *mf\n);"),
        "expected gc_mark declaration for GcNode in mquickjs_ridl_api.h, got:\n{api_hdr}"
    );
    assert!(
        !api_hdr.contains("js_m1_class_plainnode_gc_mark"),
        "did not expect gc_mark symbol for PlainNode in mquickjs_ridl_api.h, got:\n{api_hdr}"
    );
}
