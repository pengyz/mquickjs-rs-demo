use std::env;

fn main() {
    // 获取项目根目录路径
    let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let stdlib_dir = format!("{}/tests/ridl_tests/stdlib", project_root);
    let stdlib_demo_dir = format!("{}/tests/ridl_tests/stdlib_demo", project_root);
    let ridl_tests_dir = format!("{}/tests/ridl_tests", project_root);

    // 编译C胶水代码
    cc::Build::new()
        .file(format!("{}/stdlib_glue.c", stdlib_dir))
        .file(format!("{}/stdlib_demo_glue.c", stdlib_demo_dir))
        .include(format!("{}/deps/mquickjs", project_root))
        .include(&stdlib_dir)
        .include(&stdlib_demo_dir)
        .include(&ridl_tests_dir)  // 添加 ridl_tests 目录以包含 js_native_api.h
        .compile("stdlib_glue");

    // 重新生成绑定时通知 Cargo
    println!("cargo:rerun-if-changed={}/stdlib_glue.c", stdlib_dir);
    println!("cargo:rerun-if-changed={}/stdlib_glue.h", stdlib_dir);
    println!("cargo:rerun-if-changed={}/stdlib_demo_glue.c", stdlib_demo_dir);
    println!("cargo:rerun-if-changed={}/stdlib_demo_glue.h", stdlib_demo_dir);
    println!("cargo:rerun-if-changed={}/../mquickjs/mquickjs.h", project_root);
    
    // 未来可扩展支持其他RIDL模块
    // 例如：network、file等模块
}