use std::path::Path;

fn main() {
    // Re-run if the IDL changes.
    println!("cargo:rerun-if-changed=src/ridl_module_demo_default.ridl");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_dir = Path::new(&out_dir);

    let ridl_path = Path::new("src/ridl_module_demo_default.ridl");

    // Generate module-specific files into OUT_DIR.
    // Note: we call the generator library directly to avoid relying on a prebuilt binary.
    let content = std::fs::read_to_string(ridl_path).expect("read ridl");
    let parsed = ridl_tool::parser::parse_ridl_file(&content).expect("parse ridl");
    let items = parsed.items;
    ridl_tool::validator::validate_with_mode(&items, parsed.mode).expect("validate ridl");

    ridl_tool::generator::generate_module_files(
        &items,
        parsed.mode,
        out_dir,
        "ridl_module_demo_default",
    )
    .expect("generate module files");
}
