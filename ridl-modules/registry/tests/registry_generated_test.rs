#[test]
fn generated_registry_register_all_mentions_stdlib_demo() {
    // Build scripts run before tests; they should have generated this file.
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let generated = out_dir.join("registry_generated.rs");

    assert!(
        generated.exists(),
        "Expected {} to exist. registry/build.rs should generate it.",
        generated.display()
    );

    let content = std::fs::read_to_string(&generated)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", generated.display()));

    assert!(
        content.contains("stdlib_demo::ensure_linked"),
        "Expected generated register_all() to call stdlib_demo::ensure_linked()."
    );
}
