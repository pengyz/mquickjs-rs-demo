use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // 获取项目根目录路径
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ridl_tool_path = format!("{}/deps/ridl-tool", crate_dir);
    
    // 查找所有的RIDL文件
    let ridl_files = vec![
        format!("{}/ridl_modules/stdlib/stdlib.ridl", crate_dir),
        format!("{}/ridl_modules/stdlib_demo/src/stdlib_demo.ridl", crate_dir),
    ];
    
    // 确保generated目录存在
    let generated_dir = format!("{}/generated", crate_dir);
    fs::create_dir_all(&generated_dir).expect("Could not create generated directory");
    
    // 首先为每个RIDL文件生成模块级文件到generated目录
    for ridl_file in &ridl_files {
        println!("cargo:warning=Generating module files from {}", ridl_file);
        
        // 从RIDL文件路径提取模块名
        let ridl_path = std::path::Path::new(ridl_file);
        let module_name = ridl_path.file_stem()
            .expect("Invalid ridl file path")
            .to_str()
            .expect("Invalid UTF-8 in file name");
        
        // 确保输出目录存在
        let output_dir = &generated_dir;
        
        // 运行ridl-tool生成模块级文件
        let output = Command::new("cargo")
            .current_dir(&ridl_tool_path)
            .arg("run")
            .arg("--")
            .arg("module")
            .arg(ridl_file)
            .arg(output_dir)
            .output()
            .expect("Failed to execute ridl-tool module command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("ridl-tool module command failed: {}", stderr);
        }
        
        // 添加模块级文件到重新构建依赖
        let glue_file = format!("{}/{}_glue.rs", output_dir, module_name);
        let impl_file = format!("{}/{}_impl.rs", output_dir, module_name);
        println!("cargo:rerun-if-changed={}", ridl_file);
        if std::path::Path::new(&glue_file).exists() {
            println!("cargo:rerun-if-changed={}", glue_file);
        }
        if std::path::Path::new(&impl_file).exists() {
            println!("cargo:rerun-if-changed={}", impl_file);
        }
    }

    // 然后运行aggregate命令生成全局聚合文件
    println!("cargo:warning=Aggregating all RIDL files to generate shared files");
    
    let mut args = vec!["run", "--", "aggregate"];
    for ridl_file in &ridl_files {
        args.push(ridl_file);
    }
    args.push(&generated_dir.as_str());
    
    let output = Command::new("cargo")
        .current_dir(&ridl_tool_path)
        .args(&args)
        .output()
        .expect("Failed to execute ridl-tool aggregate command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("ridl-tool aggregate command failed: {}", stderr);
    }

    // 复制ridl_symbols.rs到项目根目录，以供mquickjs-rs库访问
    let src_symbols_file = format!("{}/ridl_symbols.rs", generated_dir);
    let dst_symbols_file = format!("{}/ridl_symbols.rs", crate_dir);
    
    if std::path::Path::new(&src_symbols_file).exists() {
        fs::copy(&src_symbols_file, &dst_symbols_file)
            .expect("Failed to copy ridl_symbols.rs to project root");
        println!("cargo:rerun-if-changed={}", dst_symbols_file);
    }
    
    // 复制C头文件到mquickjs-rs目录
    let src_header_file = format!("{}/mquickjs_ridl_register.h", generated_dir);
    let mquickjs_rs_dir = format!("{}/deps/mquickjs-rs", crate_dir);
    let dst_header_file = format!("{}/mquickjs_ridl_register.h", mquickjs_rs_dir);
    
    if std::path::Path::new(&src_header_file).exists() {
        fs::copy(&src_header_file, &dst_header_file)
            .expect("Failed to copy mquickjs_ridl_register.h to mquickjs-rs");
        println!("cargo:rerun-if-changed={}", dst_header_file);
    }

    // 将生成的模块文件复制到项目根目录，以便Rust编译器能找到它们
    let generated_files = fs::read_dir(&generated_dir).unwrap();
    for entry in generated_files {
        if let Ok(entry) = entry {
            let file_path = entry.path();
            if let Some(file_name) = file_path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if file_name_str.ends_with("_glue.rs") || file_name_str.ends_with("_impl.rs") {
                    let dst_path = format!("{}/{}", crate_dir, file_name_str);
                    fs::copy(&file_path, &dst_path)
                        .expect(&format!("Failed to copy {} to project root", file_name_str));
                    
                    // 添加到重新构建依赖
                    println!("cargo:rerun-if-changed={}", dst_path);
                }
            }
        }
    }

    println!("cargo:warning=Code generation completed successfully!");
    
    // 添加聚合文件到重新构建依赖
    let c_header_file = format!("{}/mquickjs_ridl_register.h", &generated_dir);
    let symbols_file = format!("{}/ridl_symbols.rs", &generated_dir);
    
    if std::path::Path::new(&c_header_file).exists() {
        println!("cargo:rerun-if-changed={}", c_header_file);
    }
    if std::path::Path::new(&symbols_file).exists() {
        println!("cargo:rerun-if-changed={}", symbols_file);
    }
}