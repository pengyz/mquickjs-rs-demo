use std::env;
use std::fs;

use mquickjs_rs::Context;

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
    
    // 也测试一下Value的使用
    println!("Testing Value conversions...");
    
    // 字符串转换测试
    match context.create_string("Hello from Rust!") {
        Ok(js_str_val) => {
            match context.get_string(js_str_val) {
                Ok(rust_str) => println!("String conversion: {}", rust_str),
                Err(e) => eprintln!("Error getting string: {}", e),
            }
        }
        Err(e) => eprintln!("Error creating string: {}", e),
    }
    
    // 数字转换测试
    match context.create_number(42.5) {
        Ok(js_num_val) => {
            match context.get_number(js_num_val) {
                Ok(rust_num) => println!("Number conversion: {}", rust_num),
                Err(e) => eprintln!("Error getting number: {}", e),
            }
        }
        Err(e) => eprintln!("Error creating number: {}", e),
    }
    
    // 布尔转换测试
    match context.create_boolean(true) {
        Ok(js_bool_val) => {
            match context.get_boolean(js_bool_val) {
                Ok(rust_bool) => println!("Boolean conversion: {}", rust_bool),
                Err(e) => eprintln!("Error getting boolean: {}", e),
            }
        }
        Err(e) => eprintln!("Error creating boolean: {}", e),
    }
    
    // 测试使用自定义内存容量创建上下文
    println!("Testing Context with custom memory capacity...");
    let _custom_context = Context::new(2 * 1024 * 1024) // 2MB
        .expect("Failed to create JavaScript context with custom memory");
    println!("Successfully created context with 2MB memory capacity");
    
    // 测试使用 Default trait 创建上下文
    println!("Testing Context::default()...");
    let _default_context = Context::default();
    println!("Successfully created context using Default trait");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_quickjs_eval() {
        // 测试代码
    }
}