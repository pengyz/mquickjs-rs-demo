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

    ridl_tool::generator::generate_module_api_file_default(out_dir).expect("generate module api");

    // Also generate module-local symbols file (we'll include it from OUT_DIR).
    // The current generator doesn't have a dedicated per-module symbols command, so we
    // reuse shared-file generation on a single ridl.
    ridl_tool::generator::generate_shared_files(
        &[ridl_path.to_string_lossy().to_string()],
        &out_dir.to_string_lossy(),
    )
    .expect("generate symbols");

    // We want the symbols file name to be module-specific to avoid collisions.
    // generate_shared_files writes ridl_symbols.rs; rename it.
    let from = out_dir.join("ridl_symbols.rs");
    let to = out_dir.join("ridl_module_demo_default_symbols.rs");
    let _ = std::fs::remove_file(&to);
    std::fs::rename(from, to).expect("rename symbols");

    // Shared slot indices from app-level aggregation.
    // Temporary convention: mquickjs-demo build.rs writes stable copies under $OUT_DIR/ridl/.
    {
        let from =
            Path::new("../../target/debug/build/mquickjs-demo-*/out/ridl/ridl_slot_indices.rs");
        let _ = from;
    }
}
