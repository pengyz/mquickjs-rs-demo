use std::env;

fn main() {
    // 设置Askama模板目录
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rerun-if-env-changed=ASKAMA_TEMPLATE_DIRS");
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rustc-env=ASKAMA_TEMPLATE_DIRS={}/templates", crate_dir);
}