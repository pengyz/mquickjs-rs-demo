use ridl_tool::parser::ast::{IDLItem, Type};
use ridl_tool::parser::parse_ridl_file;

#[test]
fn parser_builds_expected_union_optional_group_shapes() {
    let parsed = parse_ridl_file(
        r#"
module test@1.0
interface Test {
    fn a(v: string | i32 | null) -> void;
    fn b(v: (string | i32)?) -> void;
}
"#,
    )
    .unwrap();

    let itf = parsed
        .items
        .iter()
        .find_map(|it| match it {
            IDLItem::Interface(i) => Some(i),
            _ => None,
        })
        .expect("interface Test");

    let a = itf.methods.iter().find(|m| m.name == "a").unwrap();
    let b = itf.methods.iter().find(|m| m.name == "b").unwrap();

    // a: normalized as Optional(Union([String, I32])) (strategy A sugar)
    let a_v = &a.params[0].param_type;
    match a_v {
        Type::Optional(inner) => match inner.as_ref() {
            Type::Union(ts) => {
                assert!(ts.iter().any(|t| matches!(t, Type::String)));
                assert!(ts.iter().any(|t| matches!(t, Type::I32)));
                // null is normalized away into Optional wrapper
                assert!(!ts.iter().any(|t| matches!(t, Type::Null)));
            }
            other => panic!("expected Union inside Optional for a.v, got {other:?}"),
        },
        other => panic!("expected Optional(Union) for a.v, got {other:?}"),
    }

    // b: Optional(Union([String, I32])) (normalized from Optional(Custom("(string | i32)"))).
    let b_v = &b.params[0].param_type;
    match b_v {
        Type::Optional(inner) => match inner.as_ref() {
            Type::Union(ts) => {
                assert!(ts.iter().any(|t| matches!(t, Type::String)));
                assert!(ts.iter().any(|t| matches!(t, Type::I32)));
                assert!(!ts.iter().any(|t| matches!(t, Type::Null)));
            }
            other => panic!("expected Union inside Optional for b.v, got {other:?}"),
        },
        other => panic!("expected Optional for b.v, got {other:?}"),
    }
}
