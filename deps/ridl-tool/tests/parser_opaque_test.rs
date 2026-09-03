use ridl_tool::parser::ast::{IDLItem, Type};

#[test]
fn test_parse_opaque_block_single_field() {
    let input = r#"
        class Node {
            opaque {
                held: Traced<Value>
            }
            fn test() -> void;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "Node");
            assert_eq!(class.opaque_fields.len(), 1);
            assert_eq!(class.opaque_fields[0].name, "held");

            // Verify type is Traced<Custom("Value")>
            match &class.opaque_fields[0].field_type {
                Type::Traced(inner) => {
                    match **inner {
                        Type::Custom(ref name) => {
                            assert_eq!(name, "Value");
                        }
                        _ => panic!("Expected Custom type inside Traced"),
                    }
                }
                _ => panic!("Expected Traced type"),
            }
        }
        _ => panic!("Expected Class item"),
    }
}

#[test]
fn test_parse_opaque_block_multiple_fields() {
    let input = r#"
        class MultiFieldNode {
            opaque {
                held: Traced<Value>?
                count: i32
                other: Traced<object>
            }
            fn dummy() -> void;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "MultiFieldNode");
            assert_eq!(class.opaque_fields.len(), 3);

            // Field 1: held: Traced<Value>?
            assert_eq!(class.opaque_fields[0].name, "held");
            match &class.opaque_fields[0].field_type {
                Type::Optional(inner) => {
                    match **inner {
                        Type::Traced(ref traced_inner) => {
                            match **traced_inner {
                                Type::Custom(ref name) => {
                                    assert_eq!(name, "Value");
                                }
                                _ => panic!("Expected Custom type inside Traced"),
                            }
                        }
                        _ => panic!("Expected Traced inside Optional"),
                    }
                }
                _ => panic!("Expected Optional type for held"),
            }

            // Field 2: count: i32
            assert_eq!(class.opaque_fields[1].name, "count");
            assert_eq!(class.opaque_fields[1].field_type, Type::I32);

            // Field 3: other: Traced<object>
            assert_eq!(class.opaque_fields[2].name, "other");
            match &class.opaque_fields[2].field_type {
                Type::Traced(inner) => {
                    assert_eq!(**inner, Type::Object);
                }
                _ => panic!("Expected Traced type for other"),
            }
        }
        _ => panic!("Expected Class item"),
    }
}

#[test]
fn test_parse_class_without_opaque() {
    let input = r#"
        class Simple {
            fn test() -> void;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "Simple");
            assert_eq!(class.opaque_fields.len(), 0);
        }
        _ => panic!("Expected Class item"),
    }
}

#[test]
fn test_parse_opaque_missing_rbrace() {
    let input = r#"
        class Bad {
            opaque {
                field: i32
            fn method() -> void;
        }
    "#;

    let result = ridl_tool::parse_ridl(input);
    assert!(result.is_err(), "Should fail on missing right brace");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("expected") || err_msg.contains("}") || err_msg.contains("brace"),
        "Error message should mention missing brace, got: {}", err_msg
    );
}

#[test]
fn test_parse_traced_without_type_param() {
    let input = r#"
        class Bad {
            opaque {
                field: Traced
            }
        }
    "#;

    let result = ridl_tool::parse_ridl(input);
    assert!(result.is_err(), "Should fail on Traced without type parameter");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("expected") || err_msg.contains("<") || err_msg.contains("type"),
        "Error message should mention missing type parameter, got: {}", err_msg
    );
}

#[test]
fn test_parse_opaque_field_missing_type() {
    let input = r#"
        class Bad {
            opaque {
                field:
            }
        }
    "#;

    let result = ridl_tool::parse_ridl(input);
    assert!(result.is_err(), "Should fail on field missing type");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("expected") || err_msg.contains("type") || err_msg.contains("identifier"),
        "Error message should mention missing type, got: {}", err_msg
    );
}

#[test]
fn test_parse_empty_opaque_block() {
    let input = r#"
        class Empty {
            opaque {
            }
            fn method() -> void;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse should succeed for empty opaque");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "Empty");
            assert_eq!(class.opaque_fields.len(), 0);
        }
        _ => panic!("Expected Class item"),
    }
}

#[test]
fn test_parse_complex_nested_traced() {
    let input = r#"
        class Complex {
            opaque {
                callbacks: Traced<array<object>>
                data_map: Traced<map<string, object>>
            }
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "Complex");
            assert_eq!(class.opaque_fields.len(), 2);

            // Field 1: callbacks: Traced<array<object>>
            assert_eq!(class.opaque_fields[0].name, "callbacks");
            match &class.opaque_fields[0].field_type {
                Type::Traced(inner) => {
                    match **inner {
                        Type::Array(ref elem_type) => {
                            assert_eq!(**elem_type, Type::Object);
                        }
                        _ => panic!("Expected Array inside Traced"),
                    }
                }
                _ => panic!("Expected Traced type"),
            }

            // Field 2: data_map: Traced<map<string, object>>
            assert_eq!(class.opaque_fields[1].name, "data_map");
            match &class.opaque_fields[1].field_type {
                Type::Traced(inner) => {
                    match **inner {
                        Type::Map(ref key_type, ref val_type) => {
                            assert_eq!(**key_type, Type::String);
                            assert_eq!(**val_type, Type::Object);
                        }
                        _ => panic!("Expected Map inside Traced"),
                    }
                }
                _ => panic!("Expected Traced type"),
            }
        }
        _ => panic!("Expected Class item"),
    }
}

#[test]
fn test_parse_opaque_at_different_positions() {
    let input = r#"
        class MiddleOpaque {
            fn before() -> void;
            opaque {
                field: i32
            }
            fn after() -> void;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "MiddleOpaque");
            assert_eq!(class.methods.len(), 2);
            assert_eq!(class.opaque_fields.len(), 1);
            assert_eq!(class.opaque_fields[0].name, "field");
        }
        _ => panic!("Expected Class item"),
    }
}

#[test]
fn test_parse_multiple_opaque_blocks() {
    let input = r#"
        class MultiOpaque {
            opaque {
                first: i32
            }
            fn method() -> void;
            opaque {
                second: i32
            }
        }
    "#;

    // Multiple opaque blocks should either error or only keep one
    let result = ridl_tool::parse_ridl(input);
    if let Ok(items) = result {
        match &items[0] {
            IDLItem::Class(class) => {
                // If it parses, document the behavior (last one wins or merged)
                assert!(class.opaque_fields.len() > 0, "Should have at least one opaque field");
            }
            _ => panic!("Expected Class item"),
        }
    } else {
        // If it errors, that's also acceptable behavior
        assert!(result.is_err(), "Multiple opaque blocks may not be allowed");
    }
}

#[test]
fn test_parse_opaque_at_start() {
    let input = r#"
        class OpaqueFirst {
            opaque {
                first_field: i32
            }
            fn method() -> void;
            property prop: string;
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "OpaqueFirst");
            assert_eq!(class.opaque_fields.len(), 1);
            assert_eq!(class.opaque_fields[0].name, "first_field");
            assert!(class.methods.len() > 0 || class.properties.len() > 0);
        }
        _ => panic!("Expected Class item"),
    }
}

#[test]
fn test_parse_opaque_at_end() {
    let input = r#"
        class OpaqueLast {
            fn method() -> void;
            property prop: string;
            opaque {
                last_field: i32
            }
        }
    "#;

    let items = ridl_tool::parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);

    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "OpaqueLast");
            assert_eq!(class.opaque_fields.len(), 1);
            assert_eq!(class.opaque_fields[0].name, "last_field");
            assert!(class.methods.len() > 0 || class.properties.len() > 0);
        }
        _ => panic!("Expected Class item"),
    }
}
