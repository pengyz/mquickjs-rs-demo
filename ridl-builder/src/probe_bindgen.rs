use std::process::{Command, Stdio};

pub fn run() {
    // Compile small snippets as separate temp crates so we can query bindgen's public API.
    let tmp_dir = std::env::temp_dir().join("mquickjs-bindgen-probe");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).expect("create temp dir");

    std::fs::write(
        tmp_dir.join("Cargo.toml"),
        r#"[package]
name = "bindgen_probe"
version = "0.0.0"
edition = "2024"

[dependencies]
bindgen = "0.72"
"#,
    )
    .unwrap();

    std::fs::create_dir_all(tmp_dir.join("src")).unwrap();

    let probes: &[(&str, &str)] = &[
        (
            "probe-a-rusttarget-default",
            r#"fn main() {
    let _b = bindgen::Builder::default();
    let _t = bindgen::RustTarget::default();
}
"#,
        ),
        (
            "probe-b-builder-rust_target-default",
            r#"fn main() {
    let b = bindgen::Builder::default();
    let _ = b.rust_target(bindgen::RustTarget::default());
}
"#,
        ),
        (
            "probe-c-builder-rust_edition-2024",
            r#"fn main() {
    let b = bindgen::Builder::default();
    let _ = b.rust_edition(bindgen::RustEdition::Edition2024);
}
"#,
        ),
        (
            "probe-d-builder-unsafe-externs",
            r#"fn main() {
    let b = bindgen::Builder::default();
    let _ = b.generate_unsafe_externs(true);
}
"#,
        ),
    ];

    for (name, src) in probes {
        std::fs::write(tmp_dir.join("src/main.rs"), src).unwrap();
        eprintln!("== {name} ==");
        let ok = cargo_check(&tmp_dir);
        if ok {
            eprintln!("OK: {name}");
        } else {
            eprintln!("FAIL: {name}");
        }
    }

    eprintln!(
        "(If none of the probes succeed, we will need to adjust the bindgen version or use a post-processing step.)"
    );
}

fn cargo_check(dir: &std::path::Path) -> bool {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(dir).arg("check");
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let st = cmd.status().expect("run cargo check");
    st.success()
}
