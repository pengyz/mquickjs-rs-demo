use std::fs;

#[test]
fn glue_varargs_string_double_bool_have_rel_and_conversions() {
    let ridl = r#"
fn f(...rest_s: string) -> void;
fn g(...rest_d: f64) -> void;
fn h(...rest_b: bool) -> void;
"#;

    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    ridl_tool::validator::validate_with_mode(&parsed.items, parsed.mode).expect("validate ridl");

    let tmp = tempfile::tempdir().expect("tempdir");
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        tmp.path(),
        "demo",
    )
    .expect("generate module files");

    let glue = fs::read_to_string(tmp.path().join("glue.rs")).expect("read glue");

    // Each varargs loop should compute rel.
    assert!(glue.contains("let rel = i - 0"));

    // string varargs: check + to cstring.
    assert!(glue.contains("invalid string argument: rest_s["));
    assert!(glue.contains("JS_IsString"));
    assert!(glue.contains("JS_ToCString"));

    // f64 varargs: check + to number.
    assert!(glue.contains("invalid f64 argument: rest_d["));
    assert!(glue.contains("JS_ToNumber"));

    // bool varargs: tag check.
    assert!(glue.contains("invalid bool argument: rest_b["));
    assert!(glue.contains("JS_TAG_SPECIAL_BITS"));
}

#[test]
fn glue_varargs_string_collects_owned_strings_matching_trait() {
    // B1: string varargs must be collected as owned `String`s, one conversion
    // per element (JS_ToCString -> CStr::from_ptr -> to_string_lossy).
    //
    // The API trait (api.rs) already declares `Vec<String>` (rust_type_from_idl),
    // so a glue-side `Vec<*const c_char>` both mismatches the trait signature and
    // dangles: JS_ToCString returns a borrowed pointer (the caller's JSCStringBuf
    // stack buffer for short strings), which must not outlive the loop iteration.
    let ridl = r#"
singleton TestVarargs {
    fn joinAll(sep: string, ...parts: string) -> string;
}
"#;

    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    ridl_tool::validator::validate_with_mode(&parsed.items, parsed.mode).expect("validate ridl");

    let tmp = tempfile::tempdir().expect("tempdir");
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        tmp.path(),
        "demo",
    )
    .expect("generate module files");

    let glue = fs::read_to_string(tmp.path().join("glue.rs")).expect("read glue");
    let api = fs::read_to_string(tmp.path().join("api.rs")).expect("read api");

    // Collection vector is `Vec<String>`, filled with owned copies inside the loop.
    assert!(
        glue.contains("let mut parts: Vec<String> = Vec::new();"),
        "glue must declare Vec<String> for string varargs"
    );
    assert!(
        glue.contains("CStr::from_ptr(ptr)"),
        "glue must convert the borrowed JS_ToCString pointer via CStr::from_ptr"
    );
    assert!(
        glue.contains("to_string_lossy().into_owned()"),
        "glue must copy each element into an owned String inside the loop"
    );
    assert!(glue.contains("parts.push("), "glue must push converted strings");

    // No raw-pointer vector may survive for string varargs (dangling + trait mismatch).
    assert!(
        !glue.contains("Vec<*const core::ffi::c_char>"),
        "glue must not collect string varargs as Vec<*const c_char>"
    );

    // Trait surface stays `Vec<String>`; pin it so glue and trait cannot drift apart.
    assert!(
        api.contains("parts: Vec<String>"),
        "api trait must declare Vec<String> for string varargs"
    );
}
