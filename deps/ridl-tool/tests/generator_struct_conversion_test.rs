//! Phase D.3: struct 转换生成。
//!
//! 语义（计划 D.3）：
//! - struct 参数 = JS plain object 逐字段提取（递归复用
//!   emit_single_param_extract 的基础类型分支）；缺失字段 strict 报错；
//!   多余字段忽略。
//! - struct 返回 = 逐字段注入 JS object。
//! - 嵌套 struct / array<基元> / array<struct> 字段递归处理。

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
fn struct_param_extracts_fields_from_plain_object() {
    let ridl = r#"
struct Address { street: string; num: i32; }
singleton S {
    fn echoAddress(a: Address) -> Address;
}
"#;
    let (glue, _) = generate(ridl);

    // plain object 检查（与 map 参数同款：JS_CLASS_OBJECT）
    assert!(
        glue.contains("struct 'Address': expected object"),
        "struct param must reject non-object input, glue:\n{glue}"
    );
    // 对象固定绑定：字段属性一律从 __ridl_struct_obj 读取（字段提取会把
    // `v` 重绑为属性值；若从 `v` 读取，第二个字段起会拿到错误的对象）。
    assert!(
        glue.contains("let __ridl_struct_obj_a: JSValue = v;"),
        "struct extraction must pin the object binding (per-param name, inside the block), glue:\n{glue}"
    );
    // 逐字段 JS_GetPropertyStr（从对象固定绑定读取）
    assert!(glue.contains("JS_GetPropertyStr(ctx, __ridl_struct_obj_a, __ridl_prop_name_street.as_ptr())"));
    assert!(glue.contains("JS_GetPropertyStr(ctx, __ridl_struct_obj_a, __ridl_prop_name_num.as_ptr())"));
    // 字段基础提取复用 emit_single_param_extract_from_jsvalue（错误信息带字段名）
    assert!(
        glue.contains("invalid string argument: street"),
        "field extraction must reuse the base string extractor"
    );
    assert!(glue.contains("invalid i32 argument: num"));
    // 结构体字面量按字段装配
    assert!(
        glue.contains("crate::api::Address { street, num }"),
        "extraction must end in the generated struct literal, glue:\n{glue}"
    );
}

#[test]
fn struct_param_missing_field_is_strict_error() {
    let ridl = r#"
struct Address { street: string; num: i32; }
singleton S {
    fn echoAddress(a: Address) -> Address;
}
"#;
    let (glue, _) = generate(ridl);
    assert!(
        glue.contains("struct 'Address': missing field 'street'"),
        "missing field must throw with struct and field names, glue:\n{glue}"
    );
    assert!(
        glue.contains("struct 'Address': missing field 'num'"),
        "missing field check must cover every field"
    );
}

#[test]
fn struct_param_extra_fields_are_ignored() {
    // 多余字段忽略 = 只读取声明过的字段；生成物中不出现对未声明属性的读取。
    let ridl = r#"
struct Address { street: string; }
singleton S {
    fn echoAddress(a: Address) -> Address;
}
"#;
    let (glue, _) = generate(ridl);
    assert!(glue.contains("__ridl_prop_name_street"));
    assert!(
        !glue.contains("__ridl_prop_name_extra"),
        "undeclared fields must not be read"
    );
}

#[test]
fn struct_param_nested_struct_recurses() {
    let ridl = r#"
struct Address { street: string; }
struct Person { name: string; address: Address; note: string; }
singleton S {
    fn makePerson(p: Person) -> Person;
}
"#;
    let (glue, _) = generate(ridl);

    // 外层与内层都做对象检查
    assert!(glue.contains("struct 'Person': expected object"));
    assert!(glue.contains("struct 'Address': expected object"));
    // 嵌套绑定的作用域：内层绑定名唯一（__ridl_struct_obj_address），
    // 且必须位于内层块内，避免遮蔽外层对象绑定（回归钉死）。
    assert!(glue.contains("let __ridl_struct_obj_p: JSValue = v;"));
    assert!(glue.contains("let __ridl_struct_obj_address: JSValue = v;"));
    assert!(
        glue.contains("JS_GetPropertyStr(ctx, __ridl_struct_obj_p, __ridl_prop_name_note.as_ptr())"),
        "fields after a nested struct must be read from the OUTER object binding, glue:\n{glue}"
    );
    // 内层结构体字面量
    assert!(glue.contains("crate::api::Address { street }"));
    assert!(glue.contains("crate::api::Person { name, address, note }"));
}

#[test]
fn struct_param_array_of_primitives_field_loops() {
    let ridl = r#"
struct Person { name: string; tags: array<string>; }
singleton S {
    fn makePerson(p: Person) -> Person;
}
"#;
    let (glue, _) = generate(ridl);

    // array 字段按 length 循环、逐元素 JS_GetPropertyUint32
    assert!(
        glue.contains("for __ridl_i_tags in 0..__ridl_len_tags"),
        "array field must loop over its length, glue:\n{glue}"
    );
    // 数组性检查：非 JS_CLASS_ARRAY 输入可诊断拒绝（字符串有 length，
    // 不检查会静默按字符拆解）。
    assert!(
        glue.contains("struct 'Person': field 'tags': expected array"),
        "array field must reject non-arrays, glue:\n{glue}"
    );
    assert!(glue.contains("JS_GetPropertyUint32(ctx, __ridl_arr_tags, __ridl_i_tags as u32)"));
    assert!(glue.contains("tags.push(tags_elem)"));
    assert!(glue.contains("let mut tags: Vec<String> = Vec::new();"));
}

#[test]
fn struct_param_array_of_structs_field_loops() {
    let ridl = r#"
struct Address { street: string; }
struct Person { rows: array<Address>; }
singleton S {
    fn makePerson(p: Person) -> Person;
}
"#;
    let (glue, _) = generate(ridl);
    assert!(glue.contains("let mut rows: Vec<crate::api::Address> = Vec::new();"));
    assert!(glue.contains("crate::api::Address { street }"));
    assert!(glue.contains("rows.push(rows_elem)"));
}

#[test]
fn struct_return_injects_fields_into_js_object() {
    let ridl = r#"
struct Address { street: string; num: i32; }
singleton S {
    fn makeAddress(street: string, num: i32) -> Address;
}
"#;
    let (glue, _) = generate(ridl);

    assert!(
        glue.contains("let __ridl_obj = unsafe { mquickjs_rs::mquickjs_ffi::JS_NewObject(ctx) };"),
        "struct return must build a JS object, glue:\n{glue}"
    );
    assert!(glue.contains("JS_SetPropertyStr(ctx, __ridl_obj, __ridl_field_name_street.as_ptr()"));
    assert!(glue.contains("JS_SetPropertyStr(ctx, __ridl_obj, __ridl_field_name_num.as_ptr()"));
    // 逐字段转换：string 走 CString/JS_NewString，i32 走 JS_NewInt32
    assert!(glue.contains("CString::new(result.street.as_str())"));
    assert!(glue.contains("JS_NewInt32(ctx, result.num)"));
    // 注入以对象表达式收尾
    assert!(glue.contains("__ridl_obj\n}"));
}

#[test]
fn struct_return_nested_and_array_inject() {
    let ridl = r#"
struct Address { street: string; }
struct Person { name: string; address: Address; tags: array<string>; }
singleton S {
    fn makePerson(p: Person) -> Person;
}
"#;
    let (glue, _) = generate(ridl);

    // 嵌套 struct 字段：内层对象注入
    assert!(
        glue.contains("CString::new(result.address.street.as_str())"),
        "nested struct field must be injected recursively, glue:\n{glue}"
    );
    // array<string> 字段：JS_NewArray + SetPropertyUint32
    assert!(glue.contains("JS_NewArray(ctx, result.tags.len() as i32)"));
    assert!(glue.contains("JS_SetPropertyUint32(ctx, __ridl_arr, __ridl_i as u32"));
}
