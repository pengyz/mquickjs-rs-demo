#[test]
fn ridl_registry_manifest_env_is_set() {
    // registry/build.rs should export this so downstream build scripts can consume it.
    let manifest = std::env::var("RIDL_REGISTRY_MANIFEST")
        .expect("RIDL_REGISTRY_MANIFEST should be set by ridl_registry build script");

    let path = std::path::Path::new(&manifest);
    assert!(
        path.is_file(),
        "RIDL_REGISTRY_MANIFEST must point to a file"
    );

    let content =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {manifest}: {e}"));

    assert!(
        content.contains("stdlib_demo.ridl"),
        "Expected manifest to include stdlib_demo.ridl"
    );
}
