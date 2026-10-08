//! Phase D.4: enum 转换生成。
//!
//! 语义（计划 D.3/D.4）：
//! - enum 为纯 C-like（grammar.pest:34,93 无载荷语法）；
//! - JS 字符串形态 = RIDL 原始变体名（如 "RED"），Rust 侧为 PascalCase 变体
//!   （rust_api.rs.j2 to_pascal_case），双向映射由生成器产出；
//! - 参数：JS 字符串 → 匹配原始变体名 → Rust 变体；未匹配报错；
//! - 返回：Rust 变体 → 原始变体名字符串。

use std::fs;

use ridl_tool::generator::generate_module_files;
use ridl_tool::parser::parse_ridl_file;
use ridl_tool::validator::validate_with_mode;

fn generate(ridl: &str) -> (String, String) {
    let parsed = parse_ridl_file(ridl).expect("parse ridl");
    validate_with_mode(&parsed.items, parsed.mode).expect("validate ridl");
    let tmp = tempfile::tempdir().expect("tempdir");
    generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        tmp.path(),
        "demo",
    )
    .expect("generate module files");
    let glue = fs::read_to_string(tmp.path().join("glue.rs")).expect("read glue");
    let api = fs::read_to_string(tmp.path().join("api.rs")).expect("read api");
    (glue, api)
}

#[test]
fn api_declares_c_like_enum_with_pascal_variants() {
    let ridl = "enum Color { RED, GREEN, BLUE }\nfn nextColor(c: Color) -> Color;\n";
    let (_, api) = generate(ridl);
    assert!(
        api.contains("pub enum Color {\n    Red,\n    Green,\n    Blue,\n}"),
        "api must declare the C-like enum with PascalCase variants, api:\n{api}"
    );
}

#[test]
fn enum_param_matches_ridl_raw_variant_names() {
    let ridl = "enum Color { RED, GREEN, BLUE }\nfn nextColor(c: Color) -> Color;\n";
    let (glue, _) = generate(ridl);

    // JS 字符串形态 = RIDL 原始变体名
    assert!(
        glue.contains("\"RED\" => crate::api::Color::Red"),
        "match arms must map raw variant names, glue:\n{glue}"
    );
    assert!(glue.contains("\"GREEN\" => crate::api::Color::Green"));
    assert!(glue.contains("\"BLUE\" => crate::api::Color::Blue"));
    // 非字符串参数直接拒绝
    assert!(
        glue.contains("invalid enum argument: c: expected string"),
        "enum param must require a JS string"
    );
}

#[test]
fn enum_param_unknown_variant_is_an_error() {
    let ridl = "enum Color { RED, GREEN, BLUE }\nfn nextColor(c: Color) -> Color;\n";
    let (glue, _) = generate(ridl);
    assert!(
        glue.contains("unknown variant"),
        "unmatched variant must throw, glue:\n{glue}"
    );
    assert!(
        glue.contains("expected one of: RED, GREEN, BLUE"),
        "error must list the accepted raw variant names"
    );
}

#[test]
fn enum_return_maps_rust_variant_back_to_raw_name() {
    let ridl = "enum Color { RED, GREEN, BLUE }\nfn nextColor(c: Color) -> Color;\n";
    let (glue, _) = generate(ridl);

    assert!(
        glue.contains("crate::api::Color::Red => \"RED\""),
        "return mapping must produce the raw variant string, glue:\n{glue}"
    );
    assert!(glue.contains("crate::api::Color::Green => \"GREEN\""));
    assert!(glue.contains("crate::api::Color::Blue => \"BLUE\""));
    assert!(glue.contains("JS_NewString(ctx, cstr.as_ptr())"));
}

#[test]
fn enum_with_snake_case_variant_maps_both_directions() {
    // 非全大写变体名同样以 RIDL 原始名为 JS 形态、PascalCase 为 Rust 名。
    let ridl = "enum Mode { read_only, ACTIVE }\nfn nextMode(m: Mode) -> Mode;\n";
    let (glue, api) = generate(ridl);
    assert!(api.contains("ReadOnly,"), "api must declare PascalCase variant, api:\n{api}");
    assert!(api.contains("Active,"));
    assert!(glue.contains("\"read_only\" => crate::api::Mode::ReadOnly"));
    assert!(glue.contains("\"ACTIVE\" => crate::api::Mode::Active"));
    assert!(glue.contains("crate::api::Mode::ReadOnly => \"read_only\""));
    assert!(glue.contains("crate::api::Mode::Active => \"ACTIVE\""));
}
