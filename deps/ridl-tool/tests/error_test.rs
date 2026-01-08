use jidl_tool;

fn main() {
    // 测试语法错误报告
    let invalid_ridl = r#"interface Test { fn method(int x) -> string; "#; // 缺少右括号
    let result = jidl_tool::parse_ridl_content(invalid_ridl, "test.ridl");

    match result {
        Ok(_) => println!("解析成功，没有检测到错误"),
        Err(errors) => {
            println!("捕获到 {} 个错误:", errors.len());
            for error in errors {
                println!("  - 错误: {}", error.message);
                println!("    位置: {}:{}", error.file, error.line);
                println!("    类型: {:?}", error.error_type);
            }
        }
    }

    // 测试有效RIDL的解析
    let valid_ridl = r#"
module test@1.0
interface Test {
    fn method(x: int) -> string;
}
"#;
    let result = jidl_tool::parse_ridl_content(valid_ridl, "valid_test.ridl");
    match result {
        Ok(items) => println!("有效RIDL解析成功，解析到 {} 个定义", items.len()),
        Err(errors) => {
            println!("有效RIDL解析失败:");
            for error in errors {
                println!("  - 错误: {}", error.message);
            }
        }
    }
}
