fn main() {
    println!("cargo:rerun-if-changed=src/test_class.ridl");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_dir = std::path::Path::new(&out_dir);

    let ridl_path = std::path::Path::new("src/test_class.ridl");

    let content = std::fs::read_to_string(ridl_path).expect("read ridl");
    let parsed = ridl_tool::parser::parse_ridl_file(&content).expect("parse ridl");
    let items = parsed.items;
    ridl_tool::validator::validate_with_mode(&items, parsed.mode).expect("validate ridl");

    ridl_tool::generator::generate_module_files(
        &items,
        parsed.module.clone(),
        parsed.mode,
        out_dir,
        "test_class",
    )
    .expect("generate module files");
}
