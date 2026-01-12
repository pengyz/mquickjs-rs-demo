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

    // Shared ctx extension definition (generated using the same items).
    // This keeps stdlib build independent from the app crate OUT_DIR.
    // NOTE: currently only singletons are emitted into ridl_ctx_ext.rs.
    {
        // A minimal slot list derived from parsed items.
        let mut slots: Vec<String> = Vec::new();
        for it in &items {
            if let ridl_tool::parser::ast::IDLItem::Singleton(s) = it {
                slots.push(s.name.clone());
            }
        }

        // Generate ridl_ctx_ext.rs using ridl-tool template (same as aggregation).
        // For now, reuse the template via generator entrypoint by calling ridl-tool binary is avoided.
        let rendered = ridl_tool::generator::singleton_aggregate::render_ctx_ext_only(&slots)
            .expect("render ctx ext");
        std::fs::write(out_dir.join("ridl_ctx_ext.rs"), rendered).expect("write ctx ext");
    }

    ridl_tool::generator::generate_module_api_file_default(out_dir).expect("generate module api");

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
