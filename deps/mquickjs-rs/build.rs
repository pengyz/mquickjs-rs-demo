use std::{env, path::PathBuf};

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 变体目录布局：<mode>/{base,ridl}/{include,lib}
    //   mquickjs_sys::include_dir() → <mode>/<variant>/include
    let variant_root = mquickjs_sys::include_dir()
        .parent()
        .expect("include_dir has parent")
        .to_path_buf(); // <mode>/<variant>
    let mode_root = variant_root
        .parent()
        .expect("variant root has parent")
        .to_path_buf(); // <mode>

    // 1) 链接**变体无关**的引擎对象。
    //
    // mquickjs_core.a 只含 mquickjs.o / dtoa.o / libm.o / cutils.o，
    // 这些对象在 base 与 ridl 变体中字节完全相同（已核实），
    // 因此可以随本 crate 的 rlib 传播给任何消费者。
    //
    // **不能**在这里链接含 stdlib 的归档：js_stdlib 只定义在
    // mqjs_stdlib_impl.o 中，而该对象是**变体专属**的 —— ridl 变体的它
    // 强引用应用的全部 RIDL 模块符号。哪些二进制需要 RIDL 是**叶子**的属性，
    // 不是全局 feature 的属性，因此由各叶子自行选择（见下）。
    println!(
        "cargo:rustc-link-search=native={}",
        variant_root.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=mquickjs_core");

    // 2) 本 crate **自身的 test 目标**固定使用 base 变体的 stdlib。
    //
    // base 的 js_c_function_table 不引用任何 RIDL 模块符号（实测 0 个），
    // 而 mquickjs-rs 不可能依赖那些模块（会形成 Cargo 依赖环）。
    //
    // `rustc-link-arg-tests` 只作用于**发出该指令的包自己的 test 目标**，
    // 不会随 rlib 传播给依赖方 —— 应用侧由自己的 build script 链接
    // ridl stdlib（见 mquickjs_ridl_glue::emit_native_stdlib_link）。
    //
    // 这里用 `-L<base/lib> -lmquickjs_stdlib_base`：stdlib 归档已**按变体命名**
    // （base / ridl 各自不同名），因此不存在"靠 -L 顺序选对归档"的歧义，
    // 同时 `-l` 由 cargo 放在库序列的正确位置（直接用归档绝对路径当 link-arg
    // 会因其出现在引用方之前而无法被回溯搜索，导致 js_stdlib 未定义）。
    let base_lib_dir = mode_root.join("base").join("lib");
    let base_stdlib = base_lib_dir.join("libmquickjs_stdlib_base.a");
    if base_stdlib.exists() {
        println!("cargo:rerun-if-changed={}", base_stdlib.display());
        // 既用 rustc-link-arg（本包 test 目标），也用 rustc-link-lib（可传播）。
        // 传播是必要的：trybuild 等会发起**嵌套 cargo 构建**，那些 crate 不是本包的
        // target，只能通过 rlib 元数据里的链接指令获得 stdlib。
        println!("cargo:rustc-link-search=native={}", base_lib_dir.display());
        println!("cargo:rustc-link-lib=static=mquickjs_stdlib_base");
        println!("cargo:rustc-link-arg=-lmquickjs_stdlib_base");
    }

    let include_dir = mquickjs_sys::include_dir();

    // mquickjs-sys::include_dir() points to the build output include dir for the active
    // profile (base/ridl). bindgen currently requires mquickjs_ridl_api.h, which only
    // exists in the "ridl" variant. Prefer that include dir when available.
    let ridl_include_dir = include_dir
        .parent()
        .expect("include_dir has parent")
        .parent()
        .expect("include_dir has parent parent")
        .join("ridl")
        .join("include");

    let include_dir = if ridl_include_dir.join("mquickjs_ridl_api.h").exists() {
        ridl_include_dir
    } else {
        include_dir
    };

    // Compile optional ROMClass handle exports produced by the ROM builder.
    // We intentionally compile this only if present so non-ridl profiles can still build.
    compile_optional_ext_romclass_map_c(&include_dir);

    // Generate a Rust module exposing *absolute* QuickJS class ids (JS_CLASS_*).
    // Source of truth is ridl-tool's aggregate header: mquickjs_ridl_register.h.
    generate_ridl_js_class_id_rs(&out_path, &include_dir);

    // Generate FFI bindings for the headers produced by mquickjs-build.
    // Use absolute paths so bindgen can't be confused by the build script cwd.
    let header_path = mquickjs_sys::header_path();

    println!("cargo:rerun-if-changed={}", header_path.display());

    let bindings = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .clang_arg("-I")
        .clang_arg(include_dir.to_string_lossy())
        .clang_arg("-include")
        .clang_arg("stddef.h")
        .clang_arg("-include")
        .clang_arg("mquickjs_ridl_api.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_recursively(true)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}

fn compile_optional_ext_romclass_map_c(include_dir: &std::path::Path) {
    let in_path = include_dir.join("mquickjs_ext_romclass_map.c");
    println!("cargo:rerun-if-changed={}", in_path.display());

    if !in_path.exists() {
        return;
    }

    // This translation unit depends on the engine-private ROM encoding helpers
    // (JS_VALUE_FROM_PTR/JS_ROM_VALUE) declared in mquickjs_priv.h, which lives in the
    // vendored mquickjs source tree (not in the generated include dir).
    let mquickjs_priv_include = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("mquickjs")
        .canonicalize()
        .expect("canonicalize deps/mquickjs");

    cc::Build::new()
        .file(&in_path)
        .include(include_dir)
        .include(mquickjs_priv_include)
        .flag("-include")
        .flag("stddef.h")
        .warnings(false)
        .compile("mquickjs_ext_romclass_map");
}

fn generate_ridl_js_class_id_rs(out_dir: &std::path::Path, include_dir: &std::path::Path) {
    let in_path = include_dir.join("mquickjs_ridl_register.h");
    println!("cargo:rerun-if-changed={}", in_path.display());

    let Ok(content) = std::fs::read_to_string(&in_path) else {
        let _ = std::fs::write(out_dir.join("ridl_js_class_id.rs"), "\n");
        return;
    };

    let mut items: Vec<(String, i32)> = Vec::new();

    for raw in content.lines() {
        let line = raw.trim();
        if !line.starts_with("#define JS_CLASS_") {
            continue;
        }
        // Expect: #define JS_CLASS_FOO (JS_CLASS_USER + N)
        let mut it = line.split_whitespace();
        let _ = it.next();
        let Some(name) = it.next() else { continue };
        let Some(expr0) = it.next() else { continue };
        let expr = std::iter::once(expr0)
            .chain(it)
            .collect::<Vec<_>>()
            .join(" ");

        let expr = expr.trim();
        let Some(inner) = expr.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
            continue;
        };
        let inner = inner.replace(' ', "");
        let Some(rhs) = inner.strip_prefix("JS_CLASS_USER+") else {
            continue;
        };
        let Ok(n) = rhs.parse::<i32>() else { continue };

        items.push((name.to_string(), n));
    }

    items.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    out.push_str("// @generated by mquickjs-rs/build.rs. DO NOT EDIT.\n\n");
    out.push_str("// QuickJS absolute class ids for RIDL-generated classes.\n\n");

    for (name, n) in items {
        out.push_str(&format!(
            "#[allow(non_upper_case_globals)]\n\
             pub const {name}: i32 = crate::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_USER as i32 + {n};\n\n"
        ));
    }

    std::fs::write(out_dir.join("ridl_js_class_id.rs"), out).expect("write ridl_js_class_id.rs");
}
