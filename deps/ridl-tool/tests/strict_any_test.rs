#[test]
fn strict_mode_forbids_any_outside_variadic() {
    let ridl = r#"
mode strict;

fn bad(x: any) -> void;
"#;

    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    let err = ridl_tool::validator::validate_with_mode(&parsed.items, parsed.mode)
        .expect_err("strict should reject any outside variadic params");

    let msg = err.to_string();
    assert!(
        msg.contains("strict mode forbids `any` outside variadic parameters"),
        "unexpected error: {msg}"
    );
}

#[test]
fn strict_mode_allows_any_in_variadic() {
    let ridl = r#"
mode strict;

fn ok(...args: any) -> void;
"#;

    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    ridl_tool::validator::validate_with_mode(&parsed.items, parsed.mode)
        .expect("strict should allow any in variadic params");
}
