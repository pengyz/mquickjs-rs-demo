use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct RidlPlan {
    generated: RidlGenerated,
}

#[derive(Debug, Deserialize)]
struct RidlGenerated {
    out_dir: PathBuf,
    mquickjs_ridl_register_h: PathBuf,
}

#[derive(Debug, Serialize)]
struct BuildOutput {
    schema_version: u32,
    lib_dir: PathBuf,
    include_dir: PathBuf,
    libs: Vec<String>,
    inputs: Vec<PathBuf>,
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        eprintln!("Usage: mquickjs-build build --mquickjs-dir <dir> --plan <ridl_plan.json> --out <out-dir>");
        std::process::exit(2);
    };

    match cmd.as_str() {
        "build" => build_cmd(args.collect()),
        _ => {
            eprintln!("Unknown command: {cmd}");
            std::process::exit(2);
        }
    }
}

fn build_cmd(argv: Vec<String>) {
    let mut mquickjs_dir: Option<PathBuf> = None;
    let mut plan_path: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;

    let mut it = argv.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--mquickjs-dir" => mquickjs_dir = it.next().map(PathBuf::from),
            "--plan" => plan_path = it.next().map(PathBuf::from),
            "--out" => out_dir = it.next().map(PathBuf::from),
            _ => {
                eprintln!("Unknown arg: {a}");
                std::process::exit(2);
            }
        }
    }

    let mquickjs_dir = mquickjs_dir.unwrap_or_else(|| die("Missing --mquickjs-dir"));
    let plan_path = plan_path.unwrap_or_else(|| die("Missing --plan"));
    let out_dir = out_dir.unwrap_or_else(|| die("Missing --out"));

    fs::create_dir_all(&out_dir).unwrap_or_else(|e| die(&format!("Failed to create out dir: {e}")));

    let plan_text = fs::read_to_string(&plan_path)
        .unwrap_or_else(|e| die(&format!("Failed to read plan {}: {e}", plan_path.display())));
    let plan: RidlPlan = serde_json::from_str(&plan_text)
        .unwrap_or_else(|e| die(&format!("Failed to parse plan JSON: {e}")));

    let include_dir = out_dir.join("include");
    let lib_dir = out_dir.join("lib");
    fs::create_dir_all(&include_dir).unwrap_or_else(|e| die(&format!("Failed to create include dir: {e}")));
    fs::create_dir_all(&lib_dir).unwrap_or_else(|e| die(&format!("Failed to create lib dir: {e}")));

    // Keep all compilation outputs isolated from the submodule directory.
    let build_dir = out_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap_or_else(|e| die(&format!("Failed to create build dir: {e}")));

    // Copy ridl register header into include dir (so we never write into deps/mquickjs).
    let ridl_h_dst = include_dir.join("mquickjs_ridl_register.h");
    copy_file(&plan.generated.mquickjs_ridl_register_h, &ridl_h_dst);

    // Copy primary public header for bindgen/consumers.
    copy_file(&mquickjs_dir.join("mquickjs.h"), &include_dir.join("mquickjs.h"));

    // 1) Build host object for tool compilation.
    let mut gcc = Command::new("gcc");
    gcc.current_dir(&build_dir)
        .arg("-c")
        .arg(mquickjs_dir.join("mquickjs_build.c"))
        .arg("-o")
        .arg("mquickjs_build.host.o")
        .arg("-D__HOST__")
        .arg("-include")
        .arg("stddef.h");
    run(gcc);

    // 2) Build mqjs_ridl_stdlib tool from template.
    // Template lives in deps/mquickjs-rs today.
    let template = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("mquickjs-rs")
        .join("mqjs_stdlib_template.c");

    let mut gcc = Command::new("gcc");
    gcc.current_dir(&build_dir)
        .arg("-D__HOST__")
        .arg(template)
        .arg("mquickjs_build.host.o")
        .arg("-o")
        .arg("mqjs_ridl_stdlib")
        .arg("-I")
        .arg(&include_dir)
        .arg("-I")
        .arg(&mquickjs_dir)
        .arg("-include")
        .arg("stddef.h");
    run(gcc);

    // 3) Run tool to generate atoms header.
    let mut tool = Command::new(build_dir.join("mqjs_ridl_stdlib"));
    tool.arg("-a");
    let atoms_out = run_capture(tool);
    fs::write(include_dir.join("mquickjs_atom.h"), atoms_out)
        .unwrap_or_else(|e| die(&format!("Failed to write mquickjs_atom.h: {e}")));

    // 4) Run tool to generate stdlib defs header.
    let stdlib_defs_out = run_capture(Command::new(build_dir.join("mqjs_ridl_stdlib")));
    fs::write(include_dir.join("mqjs_ridl_stdlib.h"), stdlib_defs_out)
        .unwrap_or_else(|e| die(&format!("Failed to write mqjs_ridl_stdlib.h: {e}")));

    // 5) Compile core objects.
    let core_sources = ["mquickjs.c", "dtoa.c", "libm.c", "cutils.c"];
    let mut objects: Vec<PathBuf> = Vec::new();
    for src in core_sources {
        let src_path = mquickjs_dir.join(src);
        let obj_path = build_dir.join(format!("{}.o", src.trim_end_matches(".c")));
        let mut gcc = Command::new("gcc");
        gcc.current_dir(&build_dir)
            .arg("-c")
            .arg(src_path)
            .arg("-o")
            .arg(&obj_path)
            .arg("-I")
            .arg(&mquickjs_dir)
            .arg("-include")
            .arg("stddef.h");
        run(gcc);
        objects.push(obj_path);
    }

    // 6) Compile stdlib implementation object.
    let stdlib_impl = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("mquickjs-rs")
        .join("mqjs_stdlib_impl.c");

    let stdlib_obj = build_dir.join("mqjs_stdlib_impl.o");
    let mut gcc = Command::new("gcc");
    gcc.current_dir(&build_dir)
        .arg("-c")
        .arg(stdlib_impl)
        .arg("-o")
        .arg(&stdlib_obj)
        .arg("-I")
        .arg(&include_dir)
        .arg("-I")
        .arg(&mquickjs_dir)
        .arg("-include")
        .arg("stddef.h")
        .arg("-include")
        .arg("mquickjs_ridl_register.h");
    run(gcc);

    objects.push(stdlib_obj);

    // 7) Pack final libmquickjs.a
    let lib_path = lib_dir.join("libmquickjs.a");
    let mut ar = Command::new("ar");
    ar.current_dir(&build_dir).arg("rcs").arg(&lib_path);
    for obj in &objects {
        ar.arg(obj);
    }
    run(ar);

    let build_output = BuildOutput {
        schema_version: 1,
        lib_dir: lib_dir.clone(),
        include_dir: include_dir.clone(),
        libs: vec!["mquickjs".to_string()],
        inputs: vec![
            plan_path,
            mquickjs_dir.join("mquickjs_build.c"),
            mquickjs_dir.join("mquickjs.c"),
            mquickjs_dir.join("dtoa.c"),
            mquickjs_dir.join("libm.c"),
            mquickjs_dir.join("cutils.c"),
            ridl_h_dst,
        ],
    };

    let out_json = serde_json::to_string_pretty(&build_output)
        .unwrap_or_else(|e| die(&format!("Failed to serialize build output: {e}")));
    fs::write(out_dir.join("mquickjs_build_output.json"), out_json)
        .unwrap_or_else(|e| die(&format!("Failed to write build output json: {e}")));
}

fn run(mut cmd: Command) {
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd.status().unwrap_or_else(|e| die(&format!("Failed to run command: {e}")));
    if !status.success() {
        die(&format!("Command failed with status {status}"));
    }
}

fn run_capture(mut cmd: Command) -> Vec<u8> {
    let out = cmd
        .output()
        .unwrap_or_else(|e| die(&format!("Failed to run command: {e}")));
    if !out.status.success() {
        die(&format!(
            "Command failed with status {}\nstderr:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    out.stdout
}

fn copy_file(src: &Path, dst: &Path) {
    fs::copy(src, dst)
        .unwrap_or_else(|e| die(&format!("Failed to copy {} -> {}: {e}", src.display(), dst.display())));
}

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1)
}
