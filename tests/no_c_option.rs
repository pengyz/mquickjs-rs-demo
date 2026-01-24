use std::{
    env,
    path::Path,
    process::Command,
};

#[test]
fn mquickjs_build_tool_rejects_c_option() {
    // We intentionally test the *host tool* in-tree: deps/mquickjs/mquickjs_build.c.
    // The deprecated `-c`/`mqjs_ridl_class_id.h` pipeline has been removed.

    // Prefer the already-built host tool if it exists (e.g. after `ridl-builder prepare`).
    let preferred = Path::new("target/mquickjs-build/framework")
        .join(env::consts::ARCH)
        .join(env::consts::OS)
        .join("debug")
        .join("ridl")
        .join("build")
        .join("mqjs_ridl_stdlib");

    let exe = if preferred.is_file() {
        preferred
    } else {
        // Fallback: build it locally in a temp dir via `gcc`.
        // This stays within Cargo's test temp directory.
        let tmp = env::temp_dir().join("mquickjs-demo-tests").join("no_c_option");
        std::fs::create_dir_all(&tmp).expect("create temp dir");
        let out = tmp.join("mqjs_ridl_stdlib");

        let status = Command::new("gcc")
            .current_dir(&tmp)
            .arg("-O2")
            .arg("-D__HOST__")
            .arg(env::current_dir().unwrap().join("deps/mquickjs/mquickjs_build.c"))
            .arg("-D")
            .arg("main=mqjs_ridl_stdlib_main")
            .arg(env::current_dir().unwrap().join("deps/mquickjs/mqjs_stdlib.c"))
            .arg("-D")
            .arg("mqjs_ridl_stdlib_main=main")
            .arg("-o")
            .arg(&out)
            .arg("-I")
            .arg(env::current_dir().unwrap().join("deps/mquickjs"))
            .status()
            .expect("failed to run gcc");
        assert!(status.success(), "failed to build mqjs_ridl_stdlib via gcc");
        out
    };

    let out = Command::new(&exe)
        .arg("-c")
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));

    assert!(
        !out.status.success(),
        "unexpectedly accepted -c: {}",
        exe.display()
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid argument") || stderr.contains("usage:"),
        "unexpected stderr for -c rejection:\n{stderr}"
    );
}
