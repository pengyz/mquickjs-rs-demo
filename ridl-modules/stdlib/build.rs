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

    ridl_tool::generator::generate_module_api_file(out_dir)
        .expect("generate module api");

    // Module-local symbols file
    ridl_tool::generator::generate_shared_files(
        &[ridl_path.to_string_lossy().to_string()],
        &out_dir.to_string_lossy(),
    )
    .expect("generate symbols");

    // Rename ridl_symbols.rs -> stdlib_symbols.rs to avoid collisions
    let from = out_dir.join("ridl_symbols.rs");
    let to = out_dir.join("stdlib_symbols.rs");
    let _ = std::fs::remove_file(&to);
    std::fs::rename(from, to).expect("rename symbols");
}
