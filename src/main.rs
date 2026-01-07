use std::env;
use std::fs;

use mquickjs_rs::Context;

// 引入生成的模块
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/generated_modules.rs"));

// 使用宏引入RIDL扩展符号，防止链接时被优化掉
mquickjs_rs::mquickjs_ridl_extensions!();

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: {} <javascript-file>", args[0]);
        return;
    }
    
    let filename = &args[1];
    let script = fs::read_to_string(filename)
        .expect("Failed to read JavaScript file");
    
    // 使用默认内存容量创建上下文
    let mut context = Context::default();
    
    // 执行JavaScript代码
    match context.eval(&script) {
        Ok(result) => {
            println!("Result: {}", result);
        }
        Err(error) => {
            eprintln!("Error: {}", error);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_quickjs_eval() {
        // 测试代码
    }
}