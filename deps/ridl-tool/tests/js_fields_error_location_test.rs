use ridl_tool::parser::parse_ridl_file;
use ridl_tool::validator::SemanticValidator;

#[test]
fn js_field_error_has_location() {
    let ridl = r#"
mode strict;

class A {
  fn js_var() -> void;
  var js_var: i32 = 1;
}
"#;

    let parsed = parse_ridl_file(ridl).expect("parse ridl");
    let mut v = SemanticValidator::new("<mem>".to_string());
    let mut idl = ridl_tool::parser::ast::IDL {
        module: parsed.module.clone(),
        interfaces: vec![],
        classes: vec![],
        enums: vec![],
        structs: vec![],
        functions: vec![],
        using: vec![],
        imports: vec![],
        singletons: vec![],
        callbacks: vec![],
    };
    for item in &parsed.items {
        if let ridl_tool::parser::ast::IDLItem::Class(c) = item {
            idl.classes.push(c.clone());
        }
    }

    let err = v.validate(&idl).expect_err("expected validation error");

    // Expect at least one error and it should have a non-zero line/column.
    assert!(!err.is_empty());
    assert!(err[0].line > 0, "line should be > 0, got {}", err[0].line);
    assert!(
        err[0].column > 0,
        "column should be > 0, got {}",
        err[0].column
    );
}
