use std::{env, path::Path, process::Command};

#[test]
fn mquickjs_build_base_does_not_emit_ridl_headers() {
    // Base build: do not provide --ridl-register-h.
    // Expectation: output include dir should NOT contain any ridl headers.

    let tmp = env::temp_dir()
        .join("mquickjs-demo-tests")
        .join("base_build_no_ridl_headers");
    std::fs::create_dir_all(&tmp).expect("create temp dir");

    let out_dir = tmp.join("out");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).expect("clean previous out dir");
    }

    let mquickjs_build_exe = {
        // Prefer Cargo-provided path (available when the binary is a test target dependency).
        // Otherwise fall back to `target/<profile>/mquickjs-build` and ensure it's built.
        if let Ok(v) = env::var("CARGO_BIN_EXE_mquickjs-build") {
            Path::new(&v).to_path_buf()
        } else {
            let status = Command::new("cargo")
                .arg("build")
                .arg("-p")
                .arg("mquickjs-build")
                .status()
                .expect("cargo build -p mquickjs-build");
            assert!(status.success(), "failed to build mquickjs-build");

            let profile = if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            };
            env::current_dir()
                .unwrap()
                .join("target")
                .join(profile)
                .join("mquickjs-build")
        }
    };

    let status = Command::new(&mquickjs_build_exe)
        .arg("build")
        .arg("--mquickjs-dir")
        .arg(env::current_dir().unwrap().join("deps/mquickjs"))
        .arg("--out")
        .arg(&out_dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", mquickjs_build_exe.display()));

    assert!(status.success(), "mquickjs-build base build failed");

    let include_dir = out_dir.join("include");
    assert!(
        include_dir.is_dir(),
        "missing include dir: {}",
        include_dir.display()
    );

    for name in [
        "mquickjs_ridl_register.h",
        "mquickjs_ridl_module_class_ids.h",
        "mquickjs_ridl_api.h",
        "mquickjs_ridl_register.c",
    ] {
        assert!(
            !include_dir.join(name).exists(),
            "base build should not generate {name}"
        );
    }
}
