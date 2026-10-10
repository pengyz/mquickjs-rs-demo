//! 保留字定义名的位置上报测试。
//!
//! 历史：validator 关键字表中曾有 `callback` 不在 grammar keyword 表里，
//! 能以定义名身份通过 parse、由 validator 报保留字错误。`callback` 语法
//! 关键字化后（回调桥切片 2），保留字定义名在 **parse 期**即被 pest 拒绝
//! （错误自带 line:column）——本文件相应改为 parse 级位置断言。

use ridl_tool::parser::parse_ridl_file;

fn parse_error_string(src: &str) -> String {
    match parse_ridl_file(src) {
        Ok(_) => panic!("reserved word must be rejected at parse time:\n{src}"),
        Err(e) => format!("{e}"),
    }
}

#[test]
fn interface_named_callback_reports_line_and_column() {
    // 第 1 行：`interface ` 后的保留字定义名， pest 定位到 1:11
    let err = parse_error_string("interface callback { fn ping() -> void; }\n");
    assert!(
        err.contains("--> 1:") && err.contains("11"),
        "pest should locate the bad identifier at 1:11, got: {err}"
    );
    assert!(
        err.contains("expected identifier"),
        "message should say expected identifier, got: {err}"
    );
}

#[test]
fn enum_named_callback_reports_line_and_column() {
    // 前导空行使定义落在第 3 行
    let err = parse_error_string("\n\nenum callback { A = 0 }\n");
    assert!(
        err.contains("--> 3:"),
        "pest should locate the bad identifier on line 3, got: {err}"
    );
    assert!(
        err.contains("expected identifier"),
        "message should say expected identifier, got: {err}"
    );
}

#[test]
fn struct_named_callback_reports_line_and_column() {
    let err = parse_error_string("struct callback { x: i32; }\n");
    assert!(
        err.contains("--> 1:"),
        "pest should locate the bad identifier on line 1, got: {err}"
    );
    assert!(
        err.contains("expected identifier"),
        "message should say expected identifier, got: {err}"
    );
}

#[test]
fn class_and_singleton_reserved_names_report_line_and_column() {
    // class 与 singleton 同样在 parse 期拒绝，且各自携带位置
    let err = parse_error_string("class callback { }\n");
    assert!(
        err.contains("--> 1:") && err.contains("expected identifier"),
        "got: {err}"
    );
    let err = parse_error_string("singleton callback { fn ping() -> void; }\n");
    assert!(
        err.contains("--> 1:") && err.contains("expected identifier"),
        "got: {err}"
    );
}
