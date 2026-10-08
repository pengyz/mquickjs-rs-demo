use std::fs;

fn glue_for(ridl: &str) -> String {
    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    let tmp = tempfile::tempdir().expect("tempdir");
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        tmp.path(),
        "demo",
    )
    .expect("generate module files");
    fs::read_to_string(tmp.path().join("glue.rs")).expect("read glue")
}

/// B2: unsupported varargs types must fail at compile time (behavior unchanged)
/// but with a per-type diagnostic instead of one generic message.
#[test]
fn varargs_optional_union_map_callback_have_specific_diagnostics() {
    // (ridl, expected diagnostic fragment, expected full compile_error line fragment)
    let cases: &[(&str, &str)] = &[
        (
            "fn f(...rest: i32?) -> void;",
            "v1 glue: varargs does not support Optional<T> for rest (v1): use a concrete supported type or any",
        ),
        (
            "fn f(...rest: string | i32) -> void;",
            "v1 glue: varargs does not support union types for rest (v1): use a concrete supported type or any",
        ),
        (
            "fn f(...rest: map<string, i32>) -> void;",
            "v1 glue: varargs does not support map<K, V> for rest (v1): use a concrete supported type or any",
        ),
        (
            "fn f(...rest: callback()) -> void;",
            "v1 glue: varargs does not support callback types for rest (v1): use a concrete supported type or any",
        ),
    ];

    for (ridl, expected) in cases {
        let glue = glue_for(ridl);
        let expected_line = format!("compile_error!(\"{}\");", expected);
        assert!(
            glue.contains(&expected_line),
            "ridl `{}`: expected targeted diagnostic\n  {}\nin glue:\n{}",
            ridl,
            expected_line,
            glue
        );
        // The generic catch-all message must be gone.
        assert!(
            !glue.contains("unsupported varargs type"),
            "ridl `{}`: generic varargs diagnostic should be replaced; got:\n{}",
            ridl,
            glue
        );
    }
}
