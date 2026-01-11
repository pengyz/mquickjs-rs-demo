use std::{env, fs, process};

use mquickjs_demo::Context;

// App-level facade: mquickjs-rs is framework-only; RIDL module set is chosen by this app.
// The actual facade type will be introduced in src/lib.rs (mquickjs_demo::Context).

fn main() {
    mquickjs_rs::ridl_initialize!();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <javascript-file>", args[0]);
        return;
    }

    let filename = &args[1];
    let script = fs::read_to_string(filename).expect("Failed to read JavaScript file");

    // 使用默认内存容量创建上下文
    let mut context = Context::default();

    // 执行JavaScript代码
    match context.eval(&script) {
        Ok(result) => {
            println!("Result: {}", result);
        }
        Err(error) => {
            eprintln!("Error: {}", error);
            process::exit(1);
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
