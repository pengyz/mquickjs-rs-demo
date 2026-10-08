//! P0 债项 1：class js_fields 类型矩阵收窄（validator 与 glue 对齐）。
//!
//! 背景：validator 曾允许 `{Bool, I32, I64, F32, F64, String, Null}`（外加
//! Any / Optional / Custom-null 特例），但 glue 模板只实现
//! `{String, I32, Bool, Null}`，其余类型在类构造时 `unreachable!()` panic。
//! 收窄到端到端真实可用的 `{i32, bool, string, null}`，错误信息列支持集。

use ridl_tool::parser::parse_ridl_file;
use ridl_tool::validator::validate_with_mode;

fn validate(src: &str) -> Result<(), String> {
    let parsed = parse_ridl_file(src).expect("parse ridl");
    validate_with_mode(&parsed.items, parsed.mode).map_err(|e| e.to_string())
}

fn class_with_field(field: &str) -> String {
    format!("class Node {{\n    var {field}\n    fn f() -> void;\n}}\n")
}

#[test]
fn supported_types_still_accepted() {
    for field in [
        "n: i32 = 1;",
        "b: bool = true;",
        "s: string = \"hi\";",
        "z: null = null;",
    ] {
        let src = class_with_field(field);
        validate(&src)
            .unwrap_or_else(|e| panic!("js field `{field}` should be accepted, got: {e}"));
    }
}

#[test]
fn i64_f32_f64_rejected_with_supported_set() {
    // 注：grammar 的 var 字面量不支持浮点（1.5 解析期即拒），但类型本身的
    // 拒绝发生在 validator——用整数字面量触发类型维度的拒绝。
    for field in ["v: i64 = 1;", "x: f64 = 1;", "y: f32 = 1;"] {
        let src = class_with_field(field);
        let err = validate(&src).expect_err("expected rejection");
        assert!(
            err.contains("supported types are i32, bool, string, null"),
            "rejection for `{field}` must name the supported set, got: {err}"
        );
    }
}

#[test]
fn any_optional_custom_previously_allowed_shapes_now_rejected() {
    // 这些形态此前被 validator 放行（Any 静默、Optional null/string、
    // Custom null），但 glue 构造时同样 panic —— 一并收窄。
    for field in [
        "a: any = null;",
        "o: string? = null;",
        "c: Widget = null;",
    ] {
        let src = class_with_field(field);
        let err = validate(&src).expect_err("expected rejection");
        assert!(
            err.contains("supported types are i32, bool, string, null"),
            "rejection for `{field}` must name the supported set, got: {err}"
        );
    }
}
