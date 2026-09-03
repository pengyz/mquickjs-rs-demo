/// Integration test for opaque struct generation from RIDL opaque blocks.
/// Verifies that:
/// - Classes with opaque fields generate XxxOpaque structs
/// - Traced<T> types are correctly mapped to mquickjs_rs::Traced<T>
/// - Empty opaque blocks are handled (no struct generated)

#[test]
fn test_opaque_struct_generation() {
    let ridl_input = r#"
class traced_node {
    opaque {
        held: Traced<i32>
        count: i32
    }

    fn getValue() -> i32;
}

class empty_opaque {
    fn test() -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let ridl_file = tempdir.path().join("test.ridl");
    std::fs::write(&ridl_file, ridl_input).unwrap();

    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    // Generate module files
    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    // Read generated API file
    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // Verify TracedNodeOpaque struct is generated
    assert!(
        api_content.contains("pub struct TracedNodeOpaque"),
        "Should generate TracedNodeOpaque struct"
    );
    assert!(
        api_content.contains("pub held: mquickjs_rs::Traced<i32>"),
        "Should map Traced<i32> to mquickjs_rs::Traced<i32>"
    );
    assert!(
        api_content.contains("pub count: i32"),
        "Should include count field"
    );

    // Verify EmptyOpaque class does not generate an opaque struct
    assert!(
        !api_content.contains("pub struct EmptyOpaqueOpaque"),
        "Should not generate opaque struct for class without opaque fields"
    );

    // Verify TracedNodeClass trait is still generated
    assert!(
        api_content.contains("pub trait TracedNodeClass"),
        "Should still generate class trait"
    );
}

#[test]
fn test_opaque_nested_traced_types() {
    let ridl_input = r#"
class complex {
    opaque {
        optional_traced: Traced<string>?
        nested_traced: Traced<Traced<i32>>
    }

    fn test() -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // Verify nested Traced types are handled
    assert!(
        api_content.contains("pub struct ComplexOpaque"),
        "Should generate ComplexOpaque struct"
    );
    assert!(
        api_content.contains("pub optional_traced: Option<mquickjs_rs::Traced<String>>"),
        "Should handle Optional<Traced<T>>"
    );
    assert!(
        api_content.contains("pub nested_traced: mquickjs_rs::Traced<mquickjs_rs::Traced<i32>>"),
        "Should handle nested Traced<Traced<T>>"
    );
}
