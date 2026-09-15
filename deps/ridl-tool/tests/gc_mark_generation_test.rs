//! class 级 `gc_mark` **已废弃**：回归守卫。
//!
//! # 背景
//!
//! 本文件此前验证的是"为含 `Traced<T>` 字段的 class 生成 gc_mark 函数"。
//! 该机制已被**整体移除**，原因有二：
//!
//! 1. **致命 ABI 缺陷**：生成的 Rust 定义是 4 参
//!    `(_ctx, _obj, opaque, mf)`，而引擎契约是 3 参
//!    `(ctx, opaque, mf)`（`mquickjs.h` 的 `JSCMark`；
//!    调用点 `mquickjs.c` 的 `c_mark_table[...](s->ctx, p->u.user.opaque, &mf)`）。
//!    寄存器错位后 `opaque` 实参收到 `&mf`（栈地址），被当作 `Box<dyn Trait>`
//!    fat pointer 解引用 → 虚表跳转 → **SIGSEGV**。
//!    只要 GC 时该类对象可达就会崩溃。
//!    复现见 `tests/gc_traced.rs::traced_node_reachable_during_gc_invokes_gc_mark`。
//!
//! 2. **已冗余**：`Traced<T>` 现基于引擎的 `JSGCRef`，该链在 mark 与重定位
//!    两个阶段都被引擎扫描，无需 class 回调辅助标记。
//!
//! 参见 `docs/knowledge/gotcha_mquickjs_gc_mark_signature.md` 与
//! `docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md`。

/// 含 `Traced<T>` 字段的 class：**不再**生成任何 class gc_mark。
///
/// 同时确认 opaque 字段的类型映射仍然正确（移除 gc_mark 不影响它）。
#[test]
fn class_with_traced_fields_generates_no_gc_mark() {
    let ridl_input = r#"
class traced_node {
    opaque {
        held: Traced<i32>
        count: i32
    }

    fn getValue() -> i32;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_content = std::fs::read_to_string(output_dir.join("api.rs")).unwrap();

    // 字段类型映射保持正确
    assert!(
        api_content.contains("pub struct TracedNodeOpaque"),
        "Opaque struct 仍应生成"
    );
    assert!(
        api_content.contains("pub held: mquickjs_rs::Traced<i32>"),
        "Traced 字段类型映射应保持不变"
    );

    // gc_mark 的定义与调用都不应出现
    // （生成文件里的说明注释也会含 "gc_mark" 字样，故检查定义/调用而非任意子串）
    assert!(
        !api_content.contains("unsafe fn gc_mark"),
        "不应再生成 Opaque::gc_mark 定义"
    );
    assert!(
        !api_content.contains(".gc_mark(mf)"),
        "不应再生成 gc_mark 调用"
    );
    assert!(
        !api_content.contains("fn gc_mark(&self"),
        "trait 里也不应再有 gc_mark 方法"
    );
}

/// 生成的 glue 里不应再有 class 级 `gc_mark` FFI 导出。
///
/// 该导出正是 ABI 缺陷的载体，必须彻底消失。
#[test]
fn glue_exports_no_class_gc_mark_ffi() {
    let ridl_input = r#"
class traced_node {
    opaque {
        held: Traced<i32>
    }

    fn getValue() -> i32;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let glue_content = std::fs::read_to_string(output_dir.join("glue.rs")).unwrap();
    assert!(
        !glue_content.contains("_class_traced_node_gc_mark"),
        "glue 不应再导出 class gc_mark FFI"
    );
}