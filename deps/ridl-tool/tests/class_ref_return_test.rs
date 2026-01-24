use ridl_tool::parser::parse_ridl_file;
use std::fs;

use ridl_tool::generator::generate_module_files;

fn tmpdir(name: &str) -> std::path::PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("ridl_tool_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn class_ref_and_optional_class_ref_returns_generate_glue() {
    let ridl = r#"
class User {
    fn getName() -> string;
}

singleton TestClass {
    fn makeUser(name: string) -> User;
    fn maybeUser(name: string) -> User?;
}
"#;

    let parsed = parse_ridl_file(ridl).unwrap();

    // Sanity: parser should classify class references strictly.
    let mut has_make_user = false;
    let mut has_maybe_user = false;
    for it in &parsed.items {
        if let ridl_tool::parser::ast::IDLItem::Singleton(s) = it {
            for m in &s.methods {
                match m.name.as_str() {
                    "makeUser" => {
                        has_make_user = true;
                        assert_eq!(
                            m.return_type,
                            ridl_tool::parser::ast::Type::ClassRef("User".to_string())
                        );
                    }
                    "maybeUser" => {
                        has_maybe_user = true;
                        assert_eq!(
                            m.return_type,
                            ridl_tool::parser::ast::Type::Optional(Box::new(
                                ridl_tool::parser::ast::Type::ClassRef("User".to_string())
                            ))
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(has_make_user);
    assert!(has_maybe_user);

    let dir = tmpdir("class_ref_return");
    let out_dir = dir.join("out");
    fs::create_dir_all(&out_dir).unwrap();

    generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        &out_dir,
        "m",
    )
    .unwrap();

    let glue = fs::read_to_string(out_dir.join("glue.rs")).unwrap();
    assert!(
        !glue.contains("v1 glue: unsupported return type"),
        "glue.rs still contains unsupported return type:\n{glue}"
    );
    assert!(
        glue.contains("ridl_boxed_user_to_js"),
        "glue.rs should reference ridl_boxed_user_to_js for class returns:\n{glue}"
    );
}
