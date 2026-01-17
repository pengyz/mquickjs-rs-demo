use std::{env, path::Path, process};

use mquickjs_demo::test_runner;

fn main() {
    #[cfg(feature = "ridl-extensions")]
    {
        mquickjs_rs::ridl_bootstrap!();
    }

    let args: Vec<String> = env::args().collect();

    let files = if args.len() < 2 {
        // Temporary hard-coded defaults (per repo convention):
        // - tests/: framework-level integration tests
        // - ridl-modules/: module-level tests
        let mut all = Vec::new();
        for p in [Path::new("tests"), Path::new("ridl-modules")] {
            match test_runner::collect_js_files(p) {
                Ok(mut v) => all.append(&mut v),
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(2);
                }
            }
        }
        all
    } else {
        let path = Path::new(&args[1]);
        match test_runner::collect_js_files(path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(2);
            }
        }
    };

    if files.is_empty() {
        eprintln!("No .js files found under default roots (tests/, ridl-modules/) or provided path");
        process::exit(2);
    }

    println!("[==========] Running {} JS tests.", files.len());

    for f in &files {
        match test_runner::run_one_js_file(f) {
            Ok(()) => {
                println!("PASS {}", f.display());
            }
            Err(err) => {
                println!("FAIL {}", f.display());
                eprintln!("  {err}");
            }
        }
    }

    let summary = test_runner::run_files_with_summary(&files);

    println!("\n[----------] Group summary:");
    for (k, c) in &summary.by_group {
        println!(
            "[----------] {:<32} {} tests, {} passed, {} failed",
            k, c.total, c.passed, c.failed
        );
    }

    println!(
        "\n[==========] {} tests ran.",
        summary.total.total
    );
    println!("[  PASSED  ] {} tests.", summary.total.passed);
    if summary.total.failed > 0 {
        println!("[  FAILED  ] {} tests.", summary.total.failed);
    }

    if summary.total.failed == 0 {
        process::exit(0);
    }
    process::exit(1);
}
