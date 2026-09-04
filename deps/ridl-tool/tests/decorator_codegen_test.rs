/// Decorator code generation test
/// Verify that decorators are correctly parsed and generate appropriate code comments

use ridl_tool::parser::FileMode;

/// Test that decorators are parsed and generate comments in the API
#[test]
fn test_decorator_noncancellable_codegen() {
    let ridl_input = r#"
interface TestInterface {
    @nonCancellable
    fn doSomething() -> void;
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

    // Verify that the decorator comment is generated
    assert!(
        api_content.contains("// Decorators: @nonCancellable"),
        "Should generate decorator comment for @nonCancellable"
    );
}

/// Test that timeout decorator with argument is parsed and generates comments
#[test]
fn test_decorator_timeout_codegen() {
    let ridl_input = r#"
interface TestInterface {
    @timeout(5000)
    fn doSomething() -> void;
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

    // Verify that the decorator comment is generated with argument
    assert!(
        api_content.contains("// Decorators: @timeout(5000)"),
        "Should generate decorator comment for @timeout(5000)"
    );
}

/// Test that multiple decorators are parsed and generate comments
#[test]
fn test_multiple_decorators_codegen() {
    let ridl_input = r#"
interface TestInterface {
    @nonCancellable
    @timeout(3000)
    fn doSomething() -> void;
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

    // Verify that both decorator comments are generated
    assert!(
        api_content.contains("// Decorators: @nonCancellable, @timeout(3000)"),
        "Should generate decorator comments for both decorators"
    );
}

/// Test that decorators on class methods are parsed and generate comments
#[test]
fn test_decorator_on_class_method_codegen() {
    let ridl_input = r#"
class TestClass {
    @nonCancellable
    fn doSomething() -> void;
    
    @timeout(2000)
    fn doOther() -> void;
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

    // Verify that decorator comments are generated for class methods
    assert!(
        api_content.contains("// Decorators: @nonCancellable"),
        "Should generate decorator comment for @nonCancellable on class method"
    );
    assert!(
        api_content.contains("// Decorators: @timeout(2000)"),
        "Should generate decorator comment for @timeout(2000) on class method"
    );
}

/// Test that decorators on singleton methods are parsed and generate comments
#[test]
fn test_decorator_on_singleton_method_codegen() {
    let ridl_input = r#"
singleton TestSingleton {
    @nonCancellable
    fn doSomething() -> void;
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

    // Verify that decorator comment is generated for singleton method
    assert!(
        api_content.contains("// Decorators: @nonCancellable"),
        "Should generate decorator comment for @nonCancellable on singleton method"
    );
}

/// Test that methods without decorators don't generate decorator comments
#[test]
fn test_no_decorator_codegen() {
    let ridl_input = r#"
interface TestInterface {
    fn doSomething() -> void;
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

    // Verify that no decorator comment is generated
    assert!(
        !api_content.contains("// Decorators:"),
        "Should not generate decorator comment when no decorators are present"
    );
}
