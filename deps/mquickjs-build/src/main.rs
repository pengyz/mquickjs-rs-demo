use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Serialize;

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
        eprintln!(
            "Usage: mquickjs-build build --mquickjs-dir <dir> --ridl-register-h <mquickjs_ridl_register.h> --out <out-dir>"
        );
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
    let mut ridl_register_h: Option<PathBuf> = None;
    let mut target: Option<String> = None;
    let mut out_dir: Option<PathBuf> = None;

    let mut it = argv.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--mquickjs-dir" => mquickjs_dir = it.next().map(PathBuf::from),
            "--ridl-register-h" => ridl_register_h = it.next().map(PathBuf::from),
            "--out" => out_dir = it.next().map(PathBuf::from),
            // 交叉编译目标（例如 thumbv7em-none-eabihf）。
            // 省略时按宿主目标构建。指定后引擎对象改用 clang 编译。
            "--target" => target = it.next().map(String::from),
            _ => {
                eprintln!("Unknown arg: {a}");
                std::process::exit(2);
            }
        }
    }

    let mquickjs_dir = mquickjs_dir.unwrap_or_else(|| die("Missing --mquickjs-dir"));
    let out_dir = out_dir.unwrap_or_else(|| die("Missing --out"));

    fs::create_dir_all(&out_dir).unwrap_or_else(|e| die(&format!("Failed to create out dir: {e}")));

    let ridl_register_h = ridl_register_h.map(|p| {
        if !p.exists() {
            die(&format!(
                "--ridl-register-h points to a non-existent path: {}",
                p.display()
            ));
        }
        p
    });

    let include_dir = out_dir.join("include");
    let lib_dir = out_dir.join("lib");
    fs::create_dir_all(&include_dir)
        .unwrap_or_else(|e| die(&format!("Failed to create include dir: {e}")));
    fs::create_dir_all(&lib_dir).unwrap_or_else(|e| die(&format!("Failed to create lib dir: {e}")));

    // Keep all compilation outputs isolated from the submodule directory.
    let build_dir = out_dir.join("build");
    fs::create_dir_all(&build_dir)
        .unwrap_or_else(|e| die(&format!("Failed to create build dir: {e}")));

    let ridl_h_dst = include_dir.join("mquickjs_ridl_register.h");
    let ridl_module_class_ids_dst = include_dir.join("mquickjs_ridl_module_class_ids.h");
    let ridl_api_dst = include_dir.join("mquickjs_ridl_api.h");
    let ridl_c_dst = include_dir.join("mquickjs_ridl_register.c");

    // Copy ridl register headers into include dir (so we never write into deps/mquickjs).
    //
    // We support a stable umbrella header name (mquickjs_ridl_register.h) plus split
    // RIDL C headers: public API + register (ROM build roots) + runtime glue C.
    //
    // NOTE: mqjs_stdlib_template.c only includes these headers when
    // MQUICKJS_ENABLE_RIDL_EXTENSIONS is defined. So the base build does not need
    // any RIDL headers at all.
    if let Some(src) = &ridl_register_h {
        copy_file(src, &ridl_h_dst);

        let src_dir = src
            .parent()
            .unwrap_or_else(|| die("--ridl-register-h has no parent directory"));

        copy_file(
            &src_dir.join("mquickjs_ridl_module_class_ids.h"),
            &ridl_module_class_ids_dst,
        );
        copy_file(&src_dir.join("mquickjs_ridl_api.h"), &ridl_api_dst);
        copy_file(&src_dir.join("mquickjs_ridl_register.c"), &ridl_c_dst);
    }

    // Copy primary public header for bindgen/consumers.
    copy_file(
        &mquickjs_dir.join("mquickjs.h"),
        &include_dir.join("mquickjs.h"),
    );

    // 0) 解析目标工具链。
    //
    // 生成器工具自身（下面第 1、2 步）**必须**用宿主编译器；只有引擎对象
    // 走目标编译器。没有完整 sysroot 时用本仓库的裸机桩头文件。
    let cross = target.map(|triple| CrossCc {
        triple,
        stubs: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("nostd-include")
            .canonicalize()
            .expect("canonicalize nostd-include"),
    });

    // 1) Build host object for tool compilation.
    let mut gcc = Command::new("gcc");
    let mquickjs_build_c = mquickjs_dir
        .join("mquickjs_build.c")
        .canonicalize()
        .expect("canonicalize mquickjs_build.c");

    gcc.current_dir(&build_dir)
        .arg("-c")
        .arg(&mquickjs_build_c)
        .arg("-o")
        .arg("mquickjs_build.host.o")
        .arg("-D__HOST__")
        .arg("-include")
        .arg("stddef.h");
    // mquickjs_build.c 负责**生成** js_stdlib 的定义文本，其链接属性
    // （base=weak / ridl=strong）由此宏决定，因此这里也必须与变体一致。
    if ridl_register_h.is_some() {
        gcc.arg("-DMQUICKJS_ENABLE_RIDL_EXTENSIONS");
    }
    run(gcc);

    // 2) Build mqjs_ridl_stdlib tool from template.
    // Template lives in deps/mquickjs-rs today.
    let template = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("mquickjs-rs")
        .join("mqjs_stdlib_template.c");

    let template = template.canonicalize().unwrap_or_else(|e| {
        die(&format!(
            "Failed to canonicalize mqjs_stdlib_template.c: {e}"
        ))
    });

    let include_dir_canon = include_dir
        .canonicalize()
        .unwrap_or_else(|e| die(&format!("Failed to canonicalize include dir: {e}")));

    let mquickjs_dir_canon = mquickjs_dir
        .canonicalize()
        .unwrap_or_else(|e| die(&format!("Failed to canonicalize mquickjs dir: {e}")));

    // Build mqjs_ridl_stdlib (host tool).
    // It generates the ROM tables (mqjs_ridl_stdlib.h / mquickjs_atom.h / etc.).
    // NOTE: this tool must see the same JS_RIDL_EXTENSIONS injection as the runtime
    // build, otherwise the generated stdlib table won't include things like require().
    // We moved runtime-only pieces (require table + ensure_all) into mquickjs_ridl_register.c,
    // so including mquickjs_ridl_register.h in the host compile is now safe.
    let mut gcc = Command::new("gcc");
    gcc.current_dir(&build_dir)
        .arg("-D__HOST__")
        .arg(&template)
        .arg("mquickjs_build.host.o")
        .arg("-o")
        .arg("mqjs_ridl_stdlib")
        .arg("-I")
        .arg(&include_dir_canon)
        .arg("-I")
        .arg(&mquickjs_dir_canon)
        .arg("-include")
        .arg("stddef.h");
    if ridl_register_h.is_some() {
        gcc.arg("-DMQUICKJS_ENABLE_RIDL_EXTENSIONS");
    }
    run(gcc);

    // 3) Run tool to generate atoms header.
    let mut tool = Command::new(build_dir.join("mqjs_ridl_stdlib"));
    tool.arg("-a");
    let atoms_out = run_capture(tool);
    fs::write(include_dir.join("mquickjs_atom.h"), atoms_out)
        .unwrap_or_else(|e| die(&format!("Failed to write mquickjs_atom.h: {e}")));

    // 4) Run tool to generate stdlib defs header.
    let stdlib_defs_out = run_capture(Command::new(build_dir.join("mqjs_ridl_stdlib")));
    fs::write(include_dir.join("mqjs_ridl_stdlib.h"), &stdlib_defs_out)
        .unwrap_or_else(|e| die(&format!("Failed to write mqjs_ridl_stdlib.h: {e}")));

    // 4.1) (ridl-extensions only) Dump ROMClass index mapping for ridl-builder.
    // ridl-builder will later generate mquickjs_ext_romclass_map.c from this mapping + selected modules.
    if ridl_register_h.is_some() {
        let out_path = include_dir.join("mquickjs_romclass_index.json");
        let mut cmd = Command::new(build_dir.join("mqjs_ridl_stdlib"));
        cmd.arg("-M");
        cmd.arg(&out_path);
        run(cmd);
    }

    // 4.2) RIDL user class ids are generated by ridl-tool into mquickjs_ridl_api.h.

    // 5) Compile core objects.
    let core_sources = ["mquickjs.c", "dtoa.c", "libm.c", "cutils.c"];
    let mut objects: Vec<PathBuf> = Vec::new();
    for src in core_sources {
        let src_path = mquickjs_dir
            .join(src)
            .canonicalize()
            .unwrap_or_else(|e| die(&format!("Failed to canonicalize source {src}: {e}")));
        let obj_path = PathBuf::from(format!("{}.o", src.trim_end_matches(".c")));
    let mut gcc = cc_target(&build_dir, cross.as_ref());
        gcc.current_dir(&build_dir)
            .arg("-c")
            .arg(src_path)
            .arg("-o")
            .arg(&obj_path)
            .arg("-I")
            .arg(&include_dir_canon)
            .arg("-I")
            .arg(&mquickjs_dir_canon)
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
        .join("mqjs_stdlib_impl.c")
        .canonicalize()
        .unwrap_or_else(|e| die(&format!("Failed to canonicalize mqjs_stdlib_impl.c: {e}")));

    let stdlib_obj = PathBuf::from("mqjs_stdlib_impl.o");
    let mut gcc = cc_target(&build_dir, cross.as_ref());
    gcc.current_dir(&build_dir)
        .arg("-c")
        .arg(stdlib_impl)
        .arg("-o")
        .arg(&stdlib_obj)
        .arg("-I")
        .arg(&include_dir_canon)
        .arg("-I")
        .arg(&mquickjs_dir_canon)
        .arg("-include")
        .arg("stddef.h");
    if ridl_register_h.is_some() {
        gcc.arg("-include").arg("mquickjs_ridl_api.h");
        gcc.arg("-DMQUICKJS_ENABLE_RIDL_EXTENSIONS");
    } else {
        // base 变体的导出符号（js_stdlib / js_date_constructor / js_date_now）
        // 以 weak 定义。原因：应用最终二进制会同时拿到两个变体的 stdlib ——
        // base 经 mquickjs-rs 的 rlib 元数据传播（供其自身测试与 trybuild
        // 这类嵌套构建），ridl 由应用自己的 build script 链接。
        // 若两者都是 strong 会 duplicate symbol；weak 可与之共存，
        // 且单独链接 base 时仍可用。
        gcc.arg("-DJS_STDLIB_LINKAGE=__attribute__((weak))");
    }
    run(gcc);

    objects.push(stdlib_obj);

    // 6.0) (ridl-extensions only) Compile generated runtime glue (require table).
    if ridl_register_h.is_some() {
        let ridl_reg_c = ridl_c_dst.canonicalize().unwrap_or_else(|e| {
            die(&format!(
                "Failed to canonicalize mquickjs_ridl_register.c: {e}"
            ))
        });

        let ridl_reg_obj = PathBuf::from("mquickjs_ridl_register.o");
    let mut gcc = cc_target(&build_dir, cross.as_ref());
        gcc.current_dir(&build_dir)
            .arg("-c")
            .arg(ridl_reg_c)
            .arg("-o")
            .arg(&ridl_reg_obj)
            .arg("-I")
            .arg(&include_dir_canon)
            .arg("-I")
            .arg(&mquickjs_dir_canon)
            .arg("-include")
            .arg("stddef.h")
            .arg("-include")
            .arg("mquickjs_ridl_api.h")
            .arg("-include")
            .arg("mquickjs_ridl_register.h")
            .arg("-DMQUICKJS_ENABLE_RIDL_EXTENSIONS");
        run(gcc);

        objects.push(ridl_reg_obj);
    }

    // 6.1) (ridl-extensions only) Compile require() support.
    // require() is injected via JS_RIDL_EXTENSIONS in the generated mquickjs_ridl_register.h,
    // so base builds must not reference or link it.
    if ridl_register_h.is_some() {
        let require_c = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("mquickjs-rs")
            .join("require.c")
            .canonicalize()
            .unwrap_or_else(|e| die(&format!("Failed to canonicalize require.c: {e}")));

        let require_obj = PathBuf::from("mqjs_require.o");
    let mut gcc = cc_target(&build_dir, cross.as_ref());
        gcc.current_dir(&build_dir)
            .arg("-c")
            .arg(require_c)
            .arg("-o")
            .arg(&require_obj)
            .arg("-I")
            .arg(&include_dir_canon)
            .arg("-I")
            .arg(&mquickjs_dir_canon)
            .arg("-include")
            .arg("stddef.h")
            .arg("-include")
            .arg("mquickjs_ridl_api.h")
            .arg("-DMQUICKJS_ENABLE_RIDL_EXTENSIONS");
        run(gcc);

        objects.push(require_obj);
    }

    // 7) Pack archives.
    //
    // 拆成三个归档，原因是「变体选择必须由叶子二进制决定，而不是由全局 feature 决定」：
    //
    // - libmquickjs_core.a   : 变体无关的引擎对象（两变体字节相同，已核实）
    // - libmquickjs_stdlib.a : 变体专属的 stdlib impl（+ ridl 专属对象）
    // - libmquickjs.a        : 两者合并（向后兼容：selftest / 外部 consumer 仍可整体链接）
    //
    // `js_stdlib` 只定义在 mqjs_stdlib_impl.o 中，而任何使用 mquickjs-rs 的二进制
    // 都必须引用它 —— 这是「变体专属对象」与「共享 rlib」之间唯一的连接点。
    // 拆开后：应用侧链接 ridl stdlib（完整 RIDL），而 mquickjs-rs 自身的 test
    // 目标链接 base stdlib（不引用任何 RIDL 模块符号），两者互不干扰。
    //
    // Keep lib output paths relative to build_dir to avoid toolchain path oddities.
    let core_objects: Vec<PathBuf> = objects.iter().take(core_sources.len()).cloned().collect();
    let stdlib_objects: Vec<PathBuf> = objects.iter().skip(core_sources.len()).cloned().collect();

    // stdlib 归档按**变体命名**，避免 base 与 ridl 下同名归档需要靠 `-L` 顺序区分。
    // 链接期用 `-lmquickjs_stdlib_<variant>` 解析，无歧义。
    let variant = if ridl_register_h.is_some() { "ridl" } else { "base" };
    let stdlib_name = format!("../lib/libmquickjs_stdlib_{variant}.a");

    // 清理历史遗留的通用名归档，避免与按变体命名的归档形成歧义。
    let _ = fs::remove_file(build_dir.join("../lib/libmquickjs_stdlib.a"));

    pack_archive(&build_dir, "../lib/libmquickjs.a", &objects);
    pack_archive(&build_dir, "../lib/libmquickjs_core.a", &core_objects);
    pack_archive(&build_dir, &stdlib_name, &stdlib_objects);

    let build_output = BuildOutput {
        schema_version: 1,
        lib_dir: lib_dir
            .canonicalize()
            .unwrap_or_else(|e| die(&format!("Failed to canonicalize lib dir: {e}"))),
        include_dir: include_dir
            .canonicalize()
            .unwrap_or_else(|e| die(&format!("Failed to canonicalize include dir: {e}"))),
        libs: vec![
            "mquickjs".to_string(),
            "mquickjs_core".to_string(),
            format!("mquickjs_stdlib_{variant}"),
        ],
        inputs: vec![
            mquickjs_build_c,
            mquickjs_dir.join("mquickjs.c"),
            mquickjs_dir.join("dtoa.c"),
            mquickjs_dir.join("libm.c"),
            mquickjs_dir.join("cutils.c"),
            ridl_h_dst,
            ridl_module_class_ids_dst,
            include_dir.join("mquickjs_romclass_index.json"),
        ],
    };

    let out_json = serde_json::to_string_pretty(&build_output)
        .unwrap_or_else(|e| die(&format!("Failed to serialize build output: {e}")));
    fs::write(out_dir.join("mquickjs_build_output.json"), out_json)
        .unwrap_or_else(|e| die(&format!("Failed to write build output json: {e}")));
}

/// 交叉编译配置。
struct CrossCc {
    triple: String,
    /// 裸机 libc 桩头文件目录。
    stubs: PathBuf,
}

/// 宿主编译器命令（用于构建生成器工具本身）。
fn cc_host(build_dir: &Path) -> Command {
    let mut c = Command::new("gcc");
    c.current_dir(build_dir);
    c
}

/// 目标编译器命令。
///
/// 无 `--target` 时退化为宿主 `gcc`；指定后使用 `clang --target=<triple>`
/// 并以 `-ffreestanding` + 桩头文件编译引擎对象（引擎不需要完整 libc）。
fn cc_target(build_dir: &Path, cross: Option<&CrossCc>) -> Command {
    let Some(x) = cross else {
        return cc_host(build_dir);
    };
    let mut c = Command::new("clang");
    c.current_dir(build_dir)
        .arg(format!("--target={}", x.triple))
        .arg("-ffreestanding")
        .arg("-I")
        .arg(&x.stubs);
    c
}

fn run(mut cmd: Command) {
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd
        .status()
        .unwrap_or_else(|e| die(&format!("Failed to run command: {e}")));
    if !status.success() {
        die(&format!("Command failed with status {status}"));
    }
}

/// 打包静态归档。
///
/// 先删除既有归档：`ar rcs` 对**同名**成员是替换，但当对象集合发生变化时
/// （例如变体切换导致成员增减）会残留陈旧成员，进而造成难以诊断的链接行为。
fn pack_archive(build_dir: &Path, rel_path: &str, objects: &[PathBuf]) {
    let _ = fs::remove_file(build_dir.join(rel_path));

    let mut ar = Command::new("ar");
    ar.current_dir(build_dir).arg("rcs").arg(rel_path);
    for obj in objects {
        ar.arg(obj);
    }
    run(ar);
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
    fs::copy(src, dst).unwrap_or_else(|e| {
        die(&format!(
            "Failed to copy {} -> {}: {e}",
            src.display(),
            dst.display()
        ))
    });
}

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1)
}
