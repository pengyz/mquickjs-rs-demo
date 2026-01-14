use std::{env, path::Path, process};

use mquickjs_demo::test_runner;

fn main() {
    #[cfg(feature = "ridl-extensions")]
    {
        mquickjs_rs::ridl_bootstrap!();
    }

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <js-file-or-dir>", args[0]);
        process::exit(2);
    }

    let path = Path::new(&args[1]);
    let files = match test_runner::collect_js_files(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    };

    if files.is_empty() {
        eprintln!("No .js files found under: {}", path.display());
        process::exit(2);
    }

    let mut failed = 0usize;
    for f in &files {
        match test_runner::run_one_js_file(f) {
            Ok(()) => {
                println!("PASS {}", f.display());
            }
            Err(err) => {
                failed += 1;
                println!("FAIL {}", f.display());
                eprintln!("  {err}");
            }
        }
    }

    let total = files.len();
    let passed = total - failed;
    println!("\nSummary: {passed}/{total} passed");

    if failed == 0 {
        process::exit(0);
    }
    process::exit(1);
}
