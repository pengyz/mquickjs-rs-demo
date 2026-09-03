/// Integration test for gc_mark function generation.
/// Verifies that:
/// - Classes with Traced<T> fields generate gc_mark functions
/// - Optional<Traced<T>> fields are unwrapped before gc_mark
/// - Non-Traced fields are not included in gc_mark
/// - Classes without Traced fields don't generate gc_mark

#[test]
fn test_gc_mark_generation_basic() {
    let ridl_input = r#"
class traced_node {
    opaque {
        held: Traced<i32>
        count: i32
    }

    fn getValue() -> i32;
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

    // Verify gc_mark function is generated
    assert!(
        api_content.contains("impl TracedNodeOpaque"),
        "Should generate impl block for TracedNodeOpaque"
    );
    assert!(
        api_content.contains(
            "pub(crate) unsafe fn gc_mark(&self, mf: *const mquickjs_rs::mquickjs_ffi::JSMarkFunc)"
        ),
        "Should generate gc_mark function signature"
    );
    assert!(
        api_content.contains("self.held.gc_mark(mf)"),
        "Should call gc_mark on Traced field"
    );
    // count is i32, should NOT be in gc_mark
    assert!(
        !api_content.contains("self.count.gc_mark"),
        "Should not call gc_mark on non-Traced field"
    );
}

#[test]
fn test_gc_mark_generation_optional_traced() {
    let ridl_input = r#"
class optional_node {
    opaque {
        optional_held: Traced<string>?
        regular_field: i32
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

    // Verify Optional<Traced<T>> unwrapping
    assert!(
        api_content.contains("if let Some(ref inner) = self.optional_held"),
        "Should unwrap Optional before calling gc_mark"
    );
    assert!(
        api_content.contains("inner.gc_mark(mf)"),
        "Should call gc_mark on unwrapped value"
    );
}

#[test]
fn test_gc_mark_not_generated_without_traced_fields() {
    let ridl_input = r#"
class no_traced_fields {
    opaque {
        id: i32
        label: string
    }

    fn getId() -> i32;
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

    // Verify struct is generated but no gc_mark
    assert!(
        api_content.contains("pub struct NoTracedFieldsOpaque"),
        "Should generate opaque struct"
    );
    assert!(
        !api_content.contains("impl NoTracedFieldsOpaque"),
        "Should not generate impl block without Traced fields"
    );
    assert!(
        !api_content.contains("fn gc_mark"),
        "Should not generate gc_mark without Traced fields"
    );
}

#[test]
fn test_gc_mark_mixed_fields() {
    let ridl_input = r#"
class mixed_fields {
    opaque {
        name: string
        traced_data: Traced<i32>
        regular_count: i32
        optional_traced: Traced<string>?
    }

    fn getName() -> string;
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

    // Verify only Traced fields are marked
    assert!(
        api_content.contains("self.traced_data.gc_mark(mf)"),
        "Should mark traced_data"
    );
    assert!(
        api_content.contains("if let Some(ref inner) = self.optional_traced"),
        "Should mark optional_traced with unwrap"
    );
    // Non-Traced fields should not be mentioned
    assert!(
        !api_content.contains("self.name.gc_mark"),
        "Should not mark string field"
    );
    assert!(
        !api_content.contains("self.regular_count.gc_mark"),
        "Should not mark i32 field"
    );
}
