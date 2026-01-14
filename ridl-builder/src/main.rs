mod aggregate;
mod module_discovery;
mod probe_bindgen;
mod unit_graph;

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Deserialize;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
        std::process::exit(2);
    };

    match cmd.as_str() {
        "build-tools" => build_tools(),
        "build-mquickjs" => build_mquickjs(args.collect()),
        "aggregate" => aggregate_cmd(args.collect()),
        "prepare" => prepare_cmd(args.collect()),
        "export-unit-graph" => export_unit_graph_cmd(args.collect()),
        "export-deps" => export_deps_cmd(args.collect()),
        "probe-bindgen" => probe_bindgen::run(),
        _ => {
            eprintln!("Unknown command: {cmd}");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    eprintln!("Usage: cargo run -p ridl-builder -- <command>");
    eprintln!("Commands:");
    eprintln!("  build-tools        Build internal tool binaries used by build.rs");
    eprintln!("  build-mquickjs     Build quickjs + generated headers (requires build-tools)");
    eprintln!("  aggregate          Generate ridl-manifest.json and mquickjs_ridl_register.h");
    eprintln!("  prepare            Build tools, aggregate RIDL, then build mquickjs with the aggregated header");
    eprintln!("  probe-bindgen      Compile a tiny crate to probe bindgen API");
    eprintln!("");
    eprintln!("Debug/Audit commands:");
    eprintln!("  export-unit-graph  Export raw cargo -Z --unit-graph JSON (debugging only)");
    eprintln!("  export-deps        Export parsed direct deps snapshot as ridl-deps.json (debugging only)");
    eprintln!("");
    eprintln!("aggregate/prepare options:");
    eprintln!("  --cargo-toml <path>   App Cargo.toml (required)");
    eprintln!("  --app-id <id>         Override app id (optional)");
    eprintln!("  --cargo-subcommand build|test  Use cargo unit-graph to derive direct deps (preferred)");
    eprintln!("  --cargo-args <args>            Extra args forwarded to cargo (features/target/profile), e.g. --cargo-args \"--features foo\"");
    eprintln!("  --intent build|test            Legacy fallback if --cargo-subcommand is not provided (default: build)");
    eprintln!("");
    eprintln!("export-unit-graph/export-deps options:");
    eprintln!("  --cargo-toml <path>   App Cargo.toml (required, absolute)");
    eprintln!("  --cargo-subcommand build|test  Required (unit-graph requires nightly)");
    eprintln!("  --cargo-args <args>            Forwarded to cargo");
    eprintln!("  --out <path>                   Write output to file instead of stdout");
}

fn aggregate_cmd(args: Vec<String>) {
    let opts = parse_aggregate_opts(&args);
    let modules = module_discovery::discover_ridl_modules(&opts);

    let out = aggregate::aggregate(&opts.target_dir, &opts.app_id, &modules)
        .unwrap_or_else(|e| panic!("aggregate failed: {e}"));

    eprintln!("wrote {}", out.manifest_path.display());
    eprintln!("wrote {}", out.ridl_register_h.display());

    // Write unit-graph + deps snapshots for audit/debug when using the preferred unit-graph path.
    if let Some(sc) = opts.cargo_subcommand {
        let meta = cargo_metadata(&opts.cargo_toml);
        let app_pkg = select_app_package(&meta, &opts.cargo_toml);
        let (raw_path, deps_path) = write_unit_graph_and_deps_snapshots(&opts, &meta, app_pkg, sc)
            .unwrap_or_else(|e| panic!("write unit-graph/deps snapshots failed: {e}"));
        eprintln!("wrote {}", raw_path.display());
        eprintln!("wrote {}", deps_path.display());
    }
}

fn find_workspace_root() -> PathBuf {
    // Walk up from crate dir until we find a Cargo.toml containing [workspace]
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return dir;
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    panic!("Unable to locate workspace root");
}

fn parse_opt(args: &[String], key: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == key {
            return it.next().cloned();
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Build,
    Test,
}

impl Intent {
    fn parse(s: &str) -> Self {
        match s {
            "build" => Self::Build,
            "test" => Self::Test,
            _ => panic!("invalid --intent '{s}', expected: build|test"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CargoSubcommand {
    Build,
    Test,
}

impl CargoSubcommand {
    fn parse(s: &str) -> Self {
        match s {
            "build" => Self::Build,
            "test" => Self::Test,
            _ => panic!("invalid --cargo-subcommand '{s}', expected: build|test"),
        }
    }

    fn implied_intent(self) -> Intent {
        match self {
            Self::Build => Intent::Build,
            Self::Test => Intent::Test,
        }
    }
}

#[derive(Debug, Clone)]
struct AggregateOpts {
    cargo_toml: PathBuf,
    app_id: String,
    intent: Intent,
    cargo_subcommand: Option<CargoSubcommand>,
    cargo_args: Vec<String>,
    target_dir: PathBuf,
}

fn parse_aggregate_opts(args: &[String]) -> AggregateOpts {
    let cargo_toml = parse_opt(args, "--cargo-toml")
        .unwrap_or_else(|| panic!("--cargo-toml is required"));
    let cargo_toml = PathBuf::from(cargo_toml);
    if !cargo_toml.is_absolute() {
        // Keep things explicit/stable across different cwd.
        panic!("--cargo-toml must be an absolute path: {}", cargo_toml.display());
    }

    let cargo_subcommand = parse_opt(args, "--cargo-subcommand")
        .as_deref()
        .map(CargoSubcommand::parse);

    let intent = if let Some(sc) = cargo_subcommand {
        sc.implied_intent()
    } else {
        parse_opt(args, "--intent")
            .as_deref()
            .map(Intent::parse)
            .unwrap_or(Intent::Build)
    };

    let cargo_args = parse_opt(args, "--cargo-args")
        .as_deref()
        .map(split_shell_words)
        .unwrap_or_default();

    let meta = cargo_metadata(&cargo_toml);
    let target_dir = env::var("MQUICKJS_RIDL_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| meta.target_directory.clone());

    let app_pkg = select_app_package(&meta, &cargo_toml);

    let app_id = parse_opt(args, "--app-id")
        .unwrap_or_else(|| normalize_app_id(&app_pkg.name));

    AggregateOpts {
        cargo_toml,
        app_id,
        intent,
        cargo_subcommand,
        cargo_args,
        target_dir,
    }
}

fn split_shell_words(s: &str) -> Vec<String> {
    // Minimal splitter: supports simple quoting with '...' and "...".
    // This is intentionally small to avoid extra deps.
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;

    for ch in s.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, c) if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }

    if quote.is_some() {
        panic!("invalid --cargo-args: unterminated quote");
    }

    if !cur.is_empty() {
        out.push(cur);
    }

    out
}

#[derive(Debug, Clone)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    resolve: CargoResolve,
    target_directory: PathBuf,
}

#[derive(Debug, Clone)]
struct CargoPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
struct CargoResolve {
    nodes: Vec<CargoNode>,
}

#[derive(Debug, Clone)]
struct CargoNode {
    id: String,
    deps: Vec<CargoDep>,
}

#[derive(Debug, Clone)]
struct CargoDep {
    pkg: String,
    dep_kinds: Vec<CargoDepKind>,
}

#[derive(Debug, Clone)]
struct CargoDepKind {
    kind: Option<String>,
}

fn cargo_metadata(manifest_path: &Path) -> CargoMetadata {
    let mut cmd = Command::new("cargo");
    cmd.arg("metadata")
        .arg("--format-version=1")
        .arg("--manifest-path")
        .arg(manifest_path);

    let out = cmd.output().expect("failed to run cargo metadata");
    if !out.status.success() {
        panic!(
            "cargo metadata failed (exit={:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[derive(Deserialize)]
    struct RawMeta {
        packages: Vec<RawPackage>,
        resolve: RawResolve,
        target_directory: PathBuf,
    }

    #[derive(Deserialize)]
    struct RawPackage {
        id: String,
        name: String,
        manifest_path: PathBuf,
    }

    #[derive(Deserialize)]
    struct RawResolve {
        nodes: Vec<RawNode>,
    }

    #[derive(Deserialize)]
    struct RawNode {
        id: String,
        deps: Vec<RawDep>,
    }

    #[derive(Deserialize)]
    struct RawDep {
        pkg: String,
        dep_kinds: Vec<RawDepKind>,
    }

    #[derive(Deserialize)]
    struct RawDepKind {
        kind: Option<String>,
    }

    let raw: RawMeta = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("failed to parse cargo metadata json: {e}"));

    CargoMetadata {
        packages: raw
            .packages
            .into_iter()
            .map(|p| CargoPackage {
                id: p.id,
                name: p.name,
                manifest_path: p.manifest_path,
            })
            .collect(),
        resolve: CargoResolve {
            nodes: raw
                .resolve
                .nodes
                .into_iter()
                .map(|n| CargoNode {
                    id: n.id,
                    deps: n
                        .deps
                        .into_iter()
                        .map(|d| CargoDep {
                            pkg: d.pkg,
                            dep_kinds: d
                                .dep_kinds
                                .into_iter()
                                .map(|k| CargoDepKind { kind: k.kind })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        },
        target_directory: raw.target_directory,
    }
}

fn select_app_package<'a>(meta: &'a CargoMetadata, cargo_toml: &Path) -> &'a CargoPackage {
    let want = cargo_toml
        .canonicalize()
        .unwrap_or_else(|e| panic!("failed to canonicalize --cargo-toml {}: {e}", cargo_toml.display()));

    meta.packages
        .iter()
        .find(|p| {
            p.manifest_path
                .canonicalize()
                .map(|p| p == want)
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            let mut msg = String::new();
            msg.push_str("app package not found by manifest_path match. want=\n");
            msg.push_str(&format!("  {}\n", want.display()));
            msg.push_str("candidates=\n");
            for p in &meta.packages {
                msg.push_str(&format!("  {} ({})\n", p.name, p.manifest_path.display()));
            }
            panic!("{msg}");
        })
}

fn normalize_app_id(s: &str) -> String {
    s.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn resolve_target_triple() -> String {
    // `cargo run` does not set TARGET by default (it builds for host), but our build layout
    // uses an explicit target triple directory. Prefer explicit overrides, then fall back to
    // the host triple.
    if let Ok(t) = env::var("TARGET") {
        return t;
    }
    if let Ok(t) = env::var("HOST") {
        return t;
    }
    // `rustc -vV` prints `host: <triple>`.
    let out = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("failed to run rustc -vV");
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            return rest.trim().to_string();
        }
    }
    panic!("Unable to resolve target triple (no TARGET/HOST env, and rustc -vV has no host)");
}


fn build_tools() {
    // Build tool binaries in one cargo invocation to avoid repeated locking.
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("ridl-tool")
        .arg("-p")
        .arg("mquickjs-build");
    run(cmd);

    // Print their expected locations for convenience.
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let bin_dir = if profile == "release" {
        "target/release"
    } else {
        "target/debug"
    };
    eprintln!("Built tools under {bin_dir}/ (ridl-tool, mquickjs-build)");
}

fn build_mquickjs(args: Vec<String>) {
    // This command is intended to be run manually before `cargo build/test`.
    // It avoids executing tool binaries from within build.rs (ETXTBSY issues in some envs).

    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-p").arg("mquickjs-build").arg("--");

    // Default args: build base QuickJS + stdlib (no RIDL extensions).
    // Allow passing through extra args (e.g. --ridl-register-h ...).
    if args.is_empty() {
        let target_triple = resolve_target_triple();
        let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
        let mode = if profile == "release" { "release" } else { "debug" };

        let out_dir = format!("target/mquickjs-build/framework/{target_triple}/{mode}/base");

        cmd.arg("build")
            .arg("--mquickjs-dir")
            .arg("deps/mquickjs")
            .arg("--out")
            .arg(out_dir);
    } else {
        cmd.args(args);
    }

    run(cmd);
}

fn prepare_cmd(args: Vec<String>) {
    let opts = parse_aggregate_opts(&args);

    // 1) build tool binaries
    build_tools();

    // 2) aggregate ridl modules into stable outputs
    let modules = module_discovery::discover_ridl_modules(&opts);
    let out = aggregate::aggregate(&opts.target_dir, &opts.app_id, &modules)
        .unwrap_or_else(|e| panic!("aggregate failed: {e}"));
    eprintln!("wrote {}", out.manifest_path.display());
    eprintln!("wrote {}", out.ridl_register_h.display());

    // Write unit-graph + deps snapshots for audit/debug when using the preferred unit-graph path.
    if let Some(sc) = opts.cargo_subcommand {
        let meta = cargo_metadata(&opts.cargo_toml);
        let app_pkg = select_app_package(&meta, &opts.cargo_toml);
        let (raw_path, deps_path) = write_unit_graph_and_deps_snapshots(&opts, &meta, app_pkg, sc)
            .unwrap_or_else(|e| panic!("write unit-graph/deps snapshots failed: {e}"));
        eprintln!("wrote {}", raw_path.display());
        eprintln!("wrote {}", deps_path.display());
    }

    let target_triple = resolve_target_triple();
    let cargo_profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let mode = if cargo_profile == "release" { "release" } else { "debug" };

    // TODO: this profile directory will be made app-id aware as we formalize multi-app mquickjs-build outputs.
    let out_dir = format!("target/mquickjs-build/framework/{target_triple}/{mode}/ridl");

    // 3) build mquickjs using the aggregated register header
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("-p")
        .arg("mquickjs-build")
        .arg("--")
        .arg("build")
        .arg("--mquickjs-dir")
        .arg("deps/mquickjs")
        .arg("--ridl-register-h")
        .arg(out.ridl_register_h)
        .arg("--out")
        .arg(out_dir);
    run(cmd);

    // TODO: class-id rs generation will be added once the header is stabilized.
}

fn export_unit_graph_cmd(args: Vec<String>) {
    let opts = parse_export_opts(&args);

    let raw = unit_graph::run_unit_graph(&opts.cargo_toml, opts.cargo_subcommand, &opts.cargo_args);
    write_or_print(&raw, opts.out.as_deref());
}

fn export_deps_cmd(args: Vec<String>) {
    let opts = parse_export_opts(&args);

    let meta = cargo_metadata(&opts.cargo_toml);
    let app_pkg = select_app_package(&meta, &opts.cargo_toml);

    let raw = unit_graph::run_unit_graph(&opts.cargo_toml, opts.cargo_subcommand, &opts.cargo_args);
    let direct = unit_graph::direct_deps_from_unit_graph_raw(&meta, app_pkg, opts.cargo_subcommand, &raw);

    let snapshot = build_deps_snapshot(app_pkg, opts.cargo_subcommand, &opts.cargo_args, &direct);
    let json = serde_json::to_vec_pretty(&snapshot)
        .unwrap_or_else(|e| panic!("failed to serialize ridl-deps.json: {e}"));

    write_or_print(&json, opts.out.as_deref());
}

#[derive(Debug, Clone)]
struct ExportOpts {
    cargo_toml: PathBuf,
    cargo_subcommand: CargoSubcommand,
    cargo_args: Vec<String>,
    out: Option<PathBuf>,
}

fn parse_export_opts(args: &[String]) -> ExportOpts {
    let cargo_toml = parse_opt(args, "--cargo-toml")
        .unwrap_or_else(|| panic!("--cargo-toml is required"));
    let cargo_toml = PathBuf::from(cargo_toml);
    if !cargo_toml.is_absolute() {
        panic!("--cargo-toml must be an absolute path: {}", cargo_toml.display());
    }

    let cargo_subcommand = parse_opt(args, "--cargo-subcommand")
        .as_deref()
        .map(CargoSubcommand::parse)
        .unwrap_or_else(|| panic!("--cargo-subcommand is required for export"));

    let cargo_args = parse_opt(args, "--cargo-args")
        .as_deref()
        .map(split_shell_words)
        .unwrap_or_default();

    let out = parse_opt(args, "--out").map(PathBuf::from);

    ExportOpts {
        cargo_toml,
        cargo_subcommand,
        cargo_args,
        out,
    }
}

#[derive(serde::Serialize)]
struct RidlDepsSnapshot {
    schema_version: u32,
    cargo_subcommand: String,
    cargo_args: Vec<String>,
    root: RidlDepsRoot,
    direct_deps: Vec<RidlDepsPkg>,
}

#[derive(serde::Serialize)]
struct RidlDepsRoot {
    pkg_id: String,
    name: String,
    manifest_path: String,
}

#[derive(serde::Serialize)]
struct RidlDepsPkg {
    pkg_id: String,
    name: String,
    manifest_path: String,
}

fn build_deps_snapshot(app_pkg: &CargoPackage, sc: CargoSubcommand, cargo_args: &[String], direct: &[&CargoPackage]) -> RidlDepsSnapshot {
    let mut deps: Vec<RidlDepsPkg> = direct
        .iter()
        .map(|p| RidlDepsPkg {
            pkg_id: p.id.clone(),
            name: p.name.clone(),
            manifest_path: p.manifest_path.display().to_string(),
        })
        .collect();
    deps.sort_by(|a, b| a.name.cmp(&b.name).then(a.pkg_id.cmp(&b.pkg_id)));
    deps.dedup_by(|a, b| a.pkg_id == b.pkg_id);

    RidlDepsSnapshot {
        schema_version: 1,
        cargo_subcommand: match sc {
            CargoSubcommand::Build => "build".to_string(),
            CargoSubcommand::Test => "test".to_string(),
        },
        cargo_args: cargo_args.to_vec(),
        root: RidlDepsRoot {
            pkg_id: app_pkg.id.clone(),
            name: app_pkg.name.clone(),
            manifest_path: app_pkg.manifest_path.display().to_string(),
        },
        direct_deps: deps,
    }
}

fn write_unit_graph_and_deps_snapshots(
    opts: &AggregateOpts,
    meta: &CargoMetadata,
    app_pkg: &CargoPackage,
    sc: CargoSubcommand,
) -> std::io::Result<(PathBuf, PathBuf)> {
    let out_dir = aggregate::default_out_dir(&opts.target_dir, &opts.app_id);
    std::fs::create_dir_all(&out_dir)?;

    let raw = unit_graph::run_unit_graph(&opts.cargo_toml, sc, &opts.cargo_args);
    let raw_path = out_dir.join("ridl-unit-graph.json");
    std::fs::write(&raw_path, &raw)?;

    let direct = unit_graph::direct_deps_from_unit_graph_raw(meta, app_pkg, sc, &raw);
    let snapshot = build_deps_snapshot(app_pkg, sc, &opts.cargo_args, &direct);
    let json = serde_json::to_vec_pretty(&snapshot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let deps_path = out_dir.join("ridl-deps.json");
    std::fs::write(&deps_path, &json)?;

    Ok((raw_path, deps_path))
}

fn write_or_print(bytes: &[u8], out: Option<&Path>) {
    if let Some(p) = out {
        std::fs::write(p, bytes).unwrap_or_else(|e| panic!("failed to write {}: {e}", p.display()));
        return;
    }

    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(bytes)
        .unwrap_or_else(|e| panic!("failed to write stdout: {e}"));
}

fn run(mut cmd: Command) {
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd.status().expect("failed to run command");
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
