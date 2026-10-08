//! Phase D.2: named-type override pass + TemplateStruct 硬错误 + 拒绝矩阵。
//!
//! 设计裁定（计划 D.2/D.4）：
//! - 机制 = 渲染前 named-type override pass（对齐 apply_union_rust_ty_overrides
//!   范式）；**执行顺序在 union 覆写之后**（先 union 后 named-type，本文件钉死）。
//! - TemplateStruct 字段 rust_ty 的 JSValue 静默回退改为硬错误（指明字段与类型）。
//! - 拒绝矩阵（全部带可诊断错误）：
//!   - union 成员含命名类型 → 生成期报错（此前 union_types.rs 静默丢弃非基元成员）；
//!   - struct 字段拒绝：Traced<T>、union、ClassRef、map、`T?`。

use std::fs;

use ridl_tool::generator::generate_module_files;
use ridl_tool::parser::parse_ridl_file;
use ridl_tool::validator::validate_with_mode;

fn generate(ridl: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let parsed = parse_ridl_file(ridl)?;
    validate_with_mode(&parsed.items, parsed.mode)?;
    let tmp = tempfile::tempdir().expect("tempdir");
    generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        tmp.path(),
        "demo",
    )?;
    let glue = fs::read_to_string(tmp.path().join("glue.rs"))?;
    let api = fs::read_to_string(tmp.path().join("api.rs"))?;
    // tempdir 在此drop，但内容已读入内存
    Ok((glue, api))
}

fn expect_gen_error(ridl: &str) -> String {
    let parsed = parse_ridl_file(ridl).expect("parse ridl");
    // validator 放行（矩阵项是生成期错误，不是 validator 错误）
    let _ = ridl_tool::validator::validate_with_mode(&parsed.items, parsed.mode);
    let tmp = tempfile::tempdir().expect("tempdir");
    let err = generate_module_files(
        &parsed.items,
        parsed.module.clone(),
        parsed.mode,
        tmp.path(),
        "demo",
    )
    .expect_err("expected generation error");
    err.to_string()
}

// ---------------------------------------------------------------------------
// override pass（执行顺序：union 先行、named-type 随后）
// ---------------------------------------------------------------------------

#[test]
fn named_type_override_applies_after_union_override() {
    // 同一模块中 union 参数与 named-type 参数共存：
    // - union 参数由 union 覆写改写为 Union 枚举路径（先行）；
    // - named-type 参数由 named-type 覆写改写为 crate::api:: 路径（随后）；
    // 两条断言同时成立证明两个 pass 均生效且互不干扰（顺序由实现中的
    // apply_union -> apply_named_type 序列钉死，见 generator/mod.rs）。
    // 注：api.rs 只为 interface/singleton/class 生成 trait（顶层 fn 直接以
    // 自由函数被 glue 调用），故 trait 签名断言使用 singleton。
    let ridl = r#"
struct Address { street: string; num: i32; }
enum Color { RED, GREEN, BLUE }
singleton S {
    fn echoUnion(v: string | i32) -> string | i32;
    fn echoAddress(a: Address) -> Address;
    fn nextColor(c: Color) -> Color;
}
"#;
    let (glue, api) = generate(ridl).expect("generate");

    // union 覆写结果（先行 pass 的产物）
    assert!(
        glue.contains("crate::api::global::union::UnionI32String"),
        "union param must be rewritten by the union override pass, glue:\n{glue}"
    );
    assert!(
        api.contains("crate::api::global::union::UnionI32String"),
        "api trait must keep the union enum path"
    );

    // named-type 覆写结果（后续 pass 的产物）
    assert!(
        glue.contains("a: crate::api::Address"),
        "struct param must be rewritten to the generated type path, glue:\n{glue}"
    );
    assert!(
        glue.contains("c: crate::api::Color"),
        "enum param must be rewritten to the generated type path"
    );
    assert!(
        api.contains("a: crate::api::Address") && api.contains(") -> crate::api::Address;"),
        "api trait must use the named-type path, api:\n{api}"
    );
    assert!(
        api.contains("c: crate::api::Color") && api.contains(") -> crate::api::Color;"),
        "api trait must use the enum path"
    );
}

#[test]
fn named_type_override_covers_functions_and_singleton_methods() {
    let ridl = r#"
struct Address { street: string; }
singleton S {
    fn makeAddress(street: string) -> Address;
    fn echoAddress(a: Address) -> Address;
}
fn makeAddress(street: string) -> Address;
"#;
    let (glue, api) = generate(ridl).expect("generate");

    // singleton 方法签名
    assert!(
        api.contains("a: crate::api::Address"),
        "singleton method param must use the named-type path, api:\n{api}"
    );
    // 全局函数没有 api trait（rust_api.rs.j2 只为 interface/singleton/class
    // 生成 trait）；glue 以自由函数调用并做返回转换。
    assert!(
        glue.contains("let result = make_address(street);"),
        "global function must be called as a free fn with the param, glue:\n{glue}"
    );
    assert!(
        glue.contains("let __ridl_obj = unsafe { mquickjs_rs::mquickjs_ffi::JS_NewObject(ctx) };"),
        "global function struct return must be injected as a JS object"
    );
    // glue 侧参数绑定
    assert!(
        glue.contains("let a: crate::api::Address ="),
        "glue must bind named-type params to the generated path, glue:\n{glue}"
    );
}

// ---------------------------------------------------------------------------
// TemplateStruct 硬错误（JSValue 静默回退移除）
// ---------------------------------------------------------------------------

#[test]
fn struct_field_with_unresolvable_custom_type_is_a_hard_error() {
    // 旧实现：rust_type_from_idl 失败 -> 静默回退 rust_ty = "JSValue"。
    // 新实现：硬错误，指明 struct 与字段。
    let err = expect_gen_error("struct S { v: Value; }\n");
    assert!(
        err.contains("struct 'S'") && err.contains("v"),
        "hard error must name the struct and the field, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// 拒绝矩阵：union 成员含命名类型（生成期报错）
// ---------------------------------------------------------------------------

#[test]
fn union_member_named_struct_is_rejected_at_generation() {
    // 旧实现：非基元成员被 map_union_member 静默丢弃，`string | Address`
    // 静默退化为 UnionString（只含 String 成员）。
    let err = expect_gen_error(
        "struct Address { street: string; }\nfn f(v: string | Address) -> void;\n",
    );
    assert!(
        err.contains("Address") && err.contains("union"),
        "union member with named type must be a generation error naming the type, got: {err}"
    );
}

#[test]
fn union_member_named_enum_is_rejected_at_generation() {
    let err = expect_gen_error("enum Color { RED }\nfn f(v: Color | i32) -> void;\n");
    assert!(
        err.contains("Color") && err.contains("union"),
        "union member with enum type must be a generation error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// 拒绝矩阵：struct 字段类型（全部生成期硬错误）
// ---------------------------------------------------------------------------

#[test]
fn struct_field_traced_is_rejected() {
    let err = expect_gen_error("struct S { v: Traced<i32>; }\n");
    assert!(
        err.contains("struct 'S'") && err.contains("v") && err.contains("Traced"),
        "Traced<T> field must be rejected naming struct/field/type, got: {err}"
    );
}

#[test]
fn struct_field_union_is_rejected() {
    let err = expect_gen_error("struct S { v: string | i32; }\n");
    assert!(
        err.contains("struct 'S'") && err.contains("v") && err.contains("union"),
        "union field must be rejected naming struct/field/type, got: {err}"
    );
}

#[test]
fn struct_field_class_ref_is_rejected() {
    // class 同名 Custom 已被 parser 改写为 ClassRef（class_ref_rewrite.rs
    // 覆盖 struct 字段），若放行会与 class 语义互相劫持。
    let err = expect_gen_error("struct S { w: Widget; }\nclass Widget { fn m() -> void; }\n");
    assert!(
        err.contains("struct 'S'") && err.contains("v") == false && err.contains("Widget"),
        "ClassRef field must be rejected naming struct/field/class, got: {err}"
    );
}

#[test]
fn struct_field_map_is_rejected() {
    let err = expect_gen_error("struct S { v: map<string, i32>; }\n");
    assert!(
        err.contains("struct 'S'") && err.contains("v") && err.contains("map"),
        "map field must be rejected naming struct/field/type, got: {err}"
    );
}

#[test]
fn struct_field_optional_is_rejected() {
    let err = expect_gen_error("struct S { v: i32?; }\n");
    assert!(
        err.contains("struct 'S'") && err.contains("v"),
        "optional struct field must be rejected naming struct/field, got: {err}"
    );
}

#[test]
fn struct_field_enum_type_is_rejected_in_v1() {
    // 允许集 = 基元/string/嵌套 struct/array<允许集>；enum 字段不在 v1 允许集。
    let err = expect_gen_error("enum Color { RED }\nstruct S { c: Color; }\n");
    assert!(
        err.contains("struct 'S'") && err.contains("c") && err.contains("Color"),
        "enum-typed struct field must be rejected with a diagnostic, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// 允许集：基元/string/嵌套 struct/array<允许集> 正常通过
// ---------------------------------------------------------------------------

#[test]
fn struct_allowed_field_set_generates_without_error() {
    let ridl = r#"
struct Address { street: string; num: i32; big: i64; ratio: f64; small: f32; flag: bool; }
struct Person { name: string; address: Address; tags: array<string>; points: array<i32>; rows: array<Address>; }
fn makePerson(p: Person) -> Person;
"#;
    let (glue, api) = generate(ridl).expect("allowed field set must generate");
    assert!(api.contains("pub struct Address"), "api must declare Address");
    assert!(api.contains("pub struct Person"), "api must declare Person");
    assert!(glue.contains("crate::api::Person"), "glue must reference Person");
}
