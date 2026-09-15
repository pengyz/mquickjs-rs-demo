//! 异步装饰器作用域约束测试。
//!
//! `@nonCancellable` / `@timeout` 目前**仅**在 singleton 方法上受支持。
//! class 方法上使用它们必须在 RIDL 校验阶段被拒绝 —— 否则模板会生成
//! 未经实现的异步 glue（历史上这里产生过含 UB 的死代码）。
//!
//! 背景见 `docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md`
//! 与 `deps/ridl-tool/templates/rust_glue.rs.j2` 中 class 方法段的说明。

use ridl_tool::{parser::parse_ridl_file, validator::validate_with_mode};

#[test]
fn async_decorator_rejected_on_class_method() {
    for decorator in ["@nonCancellable", "@timeout(1000)"] {
        let bad = format!(
            r#"
module test@1.0

class Worker {{
    {decorator}
    fn run() -> string;
}}
"#
        );

        let parsed = parse_ridl_file(&bad).expect("parse should succeed");
        let err = validate_with_mode(&parsed.items, parsed.mode)
            .expect_err("class 方法上的异步装饰器应被拒绝");
        let msg = err.to_string();

        assert!(
            msg.contains("singleton"),
            "错误信息应说明仅支持 singleton，实际: {msg}"
        );
        assert!(
            msg.contains("Worker.run"),
            "错误信息应指出具体方法，实际: {msg}"
        );
    }
}

#[test]
fn async_decorator_allowed_on_singleton_method() {
    for decorator in ["@nonCancellable", "@timeout(1000)"] {
        let ok = format!(
            r#"
module test@1.0

singleton Service {{
    {decorator}
    fn run() -> string;

    fn plain() -> string;
}}
"#
        );

        let parsed = parse_ridl_file(&ok).expect("parse should succeed");
        validate_with_mode(&parsed.items, parsed.mode)
            .unwrap_or_else(|e| panic!("singleton 上的异步装饰器应被接受，实际: {e}"));
    }
}

#[test]
fn plain_class_method_still_allowed() {
    let ok = r#"
module test@1.0

class Worker {
    fn run() -> string;
}
"#;

    let parsed = parse_ridl_file(ok).expect("parse should succeed");
    validate_with_mode(&parsed.items, parsed.mode).expect("无装饰器的 class 方法应正常通过");
}