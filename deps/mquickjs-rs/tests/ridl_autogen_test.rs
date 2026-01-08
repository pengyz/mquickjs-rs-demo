#[test]
fn generated_ridl_modules_rs_is_present() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let generated = out_dir.join("ridl_modules.rs");

    assert!(
        generated.exists(),
        "Expected {} to exist. Build script should generate it.",
        generated.display()
    );
}

#[test]
fn generated_ridl_modules_rs_mentions_stdlib_demo() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let generated = out_dir.join("ridl_modules.rs");

    let content = std::fs::read_to_string(&generated)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", generated.display()));

    assert!(
        content.contains("stdlib_demo"),
        "Expected ridl_modules.rs to mention stdlib_demo module"
    );
    assert!(
        content.contains("stdlib_demo__ridl_force_link"),
        "Expected ridl_modules.rs to reference stdlib_demo__ridl_force_link to keep symbols alive"
    );
}

#[test]
fn aggregate_header_is_present_in_out_dir() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));

    let register_h = out_dir.join("mquickjs_ridl_register.h");
    assert!(
        register_h.exists(),
        "Expected {} to exist. Build script should generate it via ridl-tool aggregate.",
        register_h.display()
    );
}
