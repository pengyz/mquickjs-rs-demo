use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::fs::File;
use std::io::Write;

fn main() {
    // 获取项目根目录路径
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mquickjs_path = format!("{}/../mquickjs", crate_dir);
    
    // 生成标准库扩展
    generate_stdlib_extensions(&mquickjs_path, &crate_dir);
    
    // 编译mquickjs库组件
    compile_mquickjs_components(&mquickjs_path, &crate_dir);
    
    // 生成bindgen绑定
    generate_bindings();
    
    // 链接到mquickjs库组件
    println!("cargo:rustc-link-search=native={}/", mquickjs_path);
    
    // 链接mquickjs静态库
    println!("cargo:rustc-link-lib=static=mquickjs");
    
    // 也链接系统库
    println!("cargo:rustc-link-lib=m");
}

fn generate_stdlib_extensions(mquickjs_path: &str, _crate_dir: &str) {
    println!("cargo:warning=Generating stdlib extensions in {}", mquickjs_path);
    
    // 编译mquickjs_build.host.o用于构建工具
    let output = Command::new("gcc")
        .current_dir(mquickjs_path)
        .arg("-c")
        .arg("mquickjs_build.c")
        .arg("-o")
        .arg("mquickjs_build.host.o")
        .arg("-D__HOST__")
        .arg("-include")
        .arg("stddef.h")
        .output()
        .expect("Failed to compile mquickjs_build.c for host");

    if !output.status.success() {
        panic!("Failed to compile mquickjs_build.c for host: {}", String::from_utf8_lossy(&output.stderr));
    }

    // 编译mqjs_ridl_stdlib工具，使用仅包含RIDL函数的模板文件
    let output = Command::new("gcc")
        .current_dir(mquickjs_path)  // 改为在mquickjs目录执行，以确保路径正确
        .arg("-D__HOST__")  // 定义__HOST__宏，以便使用正确的头文件
        .arg("../mquickjs-rs/mqjs_stdlib_template.c")  // 使用新的仅包含RIDL函数的模板
        .arg("mquickjs_build.host.o")
        .arg("-o")
        .arg("mqjs_ridl_stdlib")
        .arg("-I.")
        .arg("-include")
        .arg("stddef.h")
        .output()
        .expect("Failed to compile mqjs_ridl_stdlib");

    if !output.status.success() {
        panic!("Failed to compile mqjs_ridl_stdlib: {}", String::from_utf8_lossy(&output.stderr));
    }

    // 运行mqjs_ridl_stdlib生成标准库定义
    let output = Command::new("./mqjs_ridl_stdlib")
        .current_dir(mquickjs_path)
        .output()
        .expect("Failed to run mqjs_ridl_stdlib");

    if !output.status.success() {
        panic!("Failed to run mqjs_ridl_stdlib: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // 将输出写入文件
    let mut file = File::create(format!("{}/mqjs_ridl_stdlib.h", mquickjs_path))
        .expect("Failed to create mqjs_ridl_stdlib.h");
    file.write_all(&output.stdout)
        .expect("Failed to write to mqjs_ridl_stdlib.h");
    
    // 运行mqjs_ridl_stdlib生成原子定义
    let output = Command::new("./mqjs_ridl_stdlib")
        .current_dir(mquickjs_path)
        .arg("-a")
        .output()
        .expect("Failed to run mqjs_ridl_stdlib for atoms");

    if !output.status.success() {
        panic!("Failed to run mqjs_ridl_stdlib for atoms: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // 将原子输出写入文件
    let mut atom_file = File::create(format!("{}/mquickjs_atom.h", mquickjs_path))
        .expect("Failed to create mquickjs_atom.h");
    atom_file.write_all(&output.stdout)
        .expect("Failed to write to mquickjs_atom.h");
}

fn compile_mquickjs_components(mquickjs_path: &str, _crate_dir: &str) {
    println!("cargo:warning=Compiling mquickjs components in {}", mquickjs_path);
    
    // 编译mquickjs库的所有组件，需要包含头文件
    let files_to_compile = [
        "mquickjs.c",
        "dtoa.c", 
        "libm.c",
        "cutils.c",
    ];
    
    for file in &files_to_compile {
        let output = Command::new("gcc")
            .current_dir(mquickjs_path)
            .arg("-c")
            .arg(file)
            .arg("-o")
            .arg(format!("{}/{}.o", mquickjs_path, file.replace(".c", "")))
            .arg("-I.")
            .arg("-include")
            .arg("stddef.h")  // 确保size_t等类型定义可用
            .output()
            .expect(&format!("Failed to compile {}", file));

        if !output.status.success() {
            panic!("Failed to compile {}: {}", file, String::from_utf8_lossy(&output.stderr));
        }
    }
    
    // 编译mqjs_stdlib_impl.c，该文件在mquickjs-rs目录中
    let output = Command::new("gcc")
        .current_dir(mquickjs_path)
        .arg("../mquickjs-rs/mqjs_stdlib_impl.c")  // 使用相对路径
        .arg("-c")
        .arg("-o")
        .arg("mqjs_stdlib_impl.o")
        .arg("-I.")
        .arg("-include")
        .arg("stddef.h")  // 确保size_t等类型定义可用
        .output()
        .expect("Failed to compile mqjs_stdlib_impl.c");

    if !output.status.success() {
        panic!("Failed to compile mqjs_stdlib_impl.c: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // 创建静态库，包含mquickjs-rs目录中的对象文件
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