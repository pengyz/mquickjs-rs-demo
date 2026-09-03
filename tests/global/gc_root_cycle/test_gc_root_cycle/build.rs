use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let ridl = Path::new(&manifest_dir)
        .join("src")
        .join("test_gc_root_cycle.ridl");

    ridl_builder::ridl_build::build_one(&ridl);
}
