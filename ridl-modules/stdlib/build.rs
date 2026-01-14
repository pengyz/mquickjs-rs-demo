use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/stdlib.ridl");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_dir = Path::new(&out_dir);

    let ridl_path = Path::new("src/stdlib.ridl");
    let content = std::fs::read_to_string(ridl_path).expect("read ridl");

    let parsed = ridl_tool::parser::parse_ridl_file(&content).expect("parse ridl");
    let items = parsed.items;
    ridl_tool::validator::validate_with_mode(&items, parsed.mode).expect("validate ridl");

    // Module-specific glue
    ridl_tool::generator::generate_module_files(&items, parsed.mode, out_dir, "stdlib")
        .expect("generate module files");

    // NOTE: stdlib previously generated ridl_ctx_ext.rs and a module-local symbols file.
    // With the module-level 2-file plan (api.rs + glue.rs), these are generated directly by ridl-tool
    // into glue.rs, and ctx ext is owned by app-level aggregation.
}
