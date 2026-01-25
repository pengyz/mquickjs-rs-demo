use ridl_tool::parser::parse_ridl_file;
use std::fs;

fn tmpdir(name: &str) -> std::path::PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("ridl_tool_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn union_with_only_null_is_optional_of_single_member_not_union() {
    let dir = tmpdir("optional_singleton_from_union_null");
    let ridl_path = dir.join("m.ridl");

    fs::write(
        &ridl_path,
        r#"
module test@1.0
interface Test {
    fn a(v: f64 | null) -> f64 | null;
}
"#,
    )
    .unwrap();

    let parsed = parse_ridl_file(&fs::read_to_string(&ridl_path).unwrap()).unwrap();

    let itf = match &parsed.items[0] {
        ridl_tool::parser::ast::IDLItem::Interface(itf) => itf,
        other => panic!("expected Interface, got: {other:?}"),
    };

    let m = &itf.methods[0];

    assert_eq!(
        format!("{:?}", m.params[0].param_type),
        "Optional(F64)",
        "param type must normalize to Optional(F64), got: {:?}",
        m.params[0].param_type
    );

    assert_eq!(
        format!("{:?}", m.return_type),
        "Optional(F64)",
        "return type must normalize to Optional(F64), got: {:?}",
        m.return_type
    );
}
