use std::path::PathBuf;
use std::process::Command;

fn main() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mquickjs_path = crate_dir.join("../mquickjs");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // 编译 mquickjs 核心（无 RIDL 标准库扩展）
    let status = Command::new("make")
        .current_dir(&mquickjs_path)
        .arg("libmquickjs.a")
        .status()
        .expect("failed to run make libmquickjs.a");
    if !status.success() {
        panic!("make libmquickjs.a failed");
    }

    // 复制核心静态库到 OUT_DIR
    std::fs::copy(
        mquickjs_path.join("libmquickjs.a"),
        out_dir.join("libmquickjs.a"),
    )
    .expect("copy libmquickjs.a failed");

    // 运行 bindgen 生成 bindings（针对 mquickjs.h）
    let bindings = bindgen::Builder::default()
        .header(mquickjs_path.join("mquickjs.h").to_string_lossy())
        .clang_arg("-I")
        .clang_arg(mquickjs_path.to_string_lossy())
        .clang_arg("-include")
        .clang_arg("stddef.h")
        .allowlist_recursively(true)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=mquickjs");
}
