use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::fs;

fn main() {
    // 获取项目根目录路径
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mquickjs_path = format!("{}/../mquickjs", crate_dir);
    
    // 复制mqjs_stdlib_impl.c到mquickjs目录
    copy_stdlib_impl(&crate_dir, &mquickjs_path);
    
    // 生成标准库扩展
    generate_stdlib_extensions(&mquickjs_path);
    
    // 编译mquickjs库组件
    compile_mquickjs_components(&mquickjs_path);
    
    // 生成bindgen绑定
    generate_bindings();
    
    // 链接到mquickjs库组件
    println!("cargo:rustc-link-search=native={}/", mquickjs_path);
    
    // 链接mquickjs静态库
    println!("cargo:rustc-link-lib=static=mquickjs");
    
    // 也链接系统库
    println!("cargo:rustc-link-lib=m");
}

fn copy_stdlib_impl(crate_dir: &str, mquickjs_path: &str) {
    let src_path = format!("{}/mqjs_stdlib_impl.c", crate_dir);
    let dst_path = format!("{}/mqjs_stdlib_impl.c", mquickjs_path);
    
    // 复制mqjs_stdlib_impl.c到mquickjs目录
    fs::copy(&src_path, &dst_path)
        .expect("Failed to copy mqjs_stdlib_impl.c to mquickjs directory");
}

fn generate_stdlib_extensions(mquickjs_path: &str) {
    println!("cargo:warning=Generating stdlib extensions in {}", mquickjs_path);
    
    // 编译mqjs_stdlib工具
    let output = Command::new("make")
        .current_dir(mquickjs_path)
        .arg("mqjs_stdlib")
        .output()
        .expect("Failed to compile mqjs_stdlib");

    if !output.status.success() {
        panic!("Failed to compile mqjs_stdlib: {}", String::from_utf8_lossy(&output.stderr));
    }

    // 运行mqjs_stdlib生成标准库定义
    let output = Command::new("sh")
        .current_dir(mquickjs_path)
        .arg("-c")
        .arg("./mqjs_stdlib > mqjs_stdlib.h")
        .output()
        .expect("Failed to run mqjs_stdlib");

    if !output.status.success() {
        panic!("Failed to run mqjs_stdlib: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // 运行mqjs_stdlib生成原子定义
    let output = Command::new("sh")
        .current_dir(mquickjs_path)
        .arg("-c")
        .arg("./mqjs_stdlib -a > mquickjs_atom.h")
        .output()
        .expect("Failed to run mqjs_stdlib for atoms");

    if !output.status.success() {
        panic!("Failed to run mqjs_stdlib for atoms: {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn compile_mquickjs_components(mquickjs_path: &str) {
    println!("cargo:warning=Compiling mquickjs components in {}", mquickjs_path);
    
    // 编译mquickjs库的所有组件，需要包含头文件
    let files_to_compile = [
        "mquickjs.c",
        "dtoa.c", 
        "libm.c",
        "cutils.c",
        "mqjs_stdlib_impl.c"  // 新增包含标准库定义的文件
    ];
    
    for file in &files_to_compile {
        let output = Command::new("gcc")
            .current_dir(mquickjs_path)
            .arg("-c")
            .arg(file)
            .arg("-o")
            .arg(format!("{}.o", file.replace(".c", "")))
            .arg("-I.")
            .output()
            .expect(&format!("Failed to compile {}", file));

        if !output.status.success() {
            panic!("Failed to compile {}: {}", file, String::from_utf8_lossy(&output.stderr));
        }
    }
    
    // 创建静态库
    let output = Command::new("ar")
        .current_dir(mquickjs_path)
        .arg("rcs")
        .arg("libmquickjs.a")
        .arg("mquickjs.o")
        .arg("dtoa.o")
        .arg("libm.o")
        .arg("cutils.o")
        .arg("mqjs_stdlib_impl.o")  // 添加标准库定义对象文件
        .output()
        .expect("Failed to run ar command");

    if !output.status.success() {
        panic!("Failed to create static library: {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn generate_bindings() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mquickjs_path = format!("{}/../mquickjs", crate_dir);
    
    // 获取头文件路径
    let header_path = format!("{}/mquickjs.h", mquickjs_path);
    
    // 使用bindgen生成绑定
    let bindings = bindgen::Builder::default()
        .header(&header_path)
        .clang_arg("-I")
        .clang_arg(&mquickjs_path)
        .clang_arg("-include")
        .clang_arg("stddef.h")  // 包含stddef.h以定义size_t
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // 将生成的绑定写入文件
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}