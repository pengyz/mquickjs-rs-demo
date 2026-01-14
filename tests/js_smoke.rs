use std::path::{Path, PathBuf};

/// Collect `*.js` files under `tests/` (non-recursive) and run them via the
/// in-crate JS runner.
///
/// Keeping this as a Rust integration test means `cargo test` can act as CI.
#[test]
fn js_smoke_tests() {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let files = mquickjs_demo::test_runner::collect_js_files(&test_dir)
        .expect("collect_js_files(tests/) should succeed");

    // Guardrail: if the directory layout changes, we want CI to tell us.
    assert!(
        !files.is_empty(),
        "no .js files found under {}",
        test_dir.display()
    );

    // Ensure process-level RIDL initialization happens before running any scripts.
    // This matches src/main.rs behavior.
    #[cfg(feature = "ridl-extensions")]
    {
        mquickjs_rs::ridl_bootstrap!();
    }

    let total = files.len();

    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    for f in files {
        if let Err(e) = mquickjs_demo::test_runner::run_one_js_file(&f) {
            failures.push((f, e));
        }
    }

    if !failures.is_empty() {
        eprintln!("JS smoke failures: {}/{}", failures.len(), total);
        for (f, e) in &failures {
            eprintln!("- {}\n  {}", f.display(), e);
        }
        panic!("js smoke tests failed");
    }
}
