use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Locate the workspace root (the directory containing `mquickjs.build.toml`)
/// by walking up from this crate's manifest directory.
///
/// Integration tests and `cargo run` inherit the *package* root as CWD, but
/// workspace-level resources (`target/`, `deps/`, JS corpora under `tests/`
/// and `ridl-modules/`) live at the repo root — resolve them through here.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|p| p.join("mquickjs.build.toml").is_file())
        .expect("workspace root (mquickjs.build.toml) not found above manifest dir")
        .to_path_buf()
}

pub fn collect_js_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    let mut out = Vec::new();
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("js") {
            out.push(path.to_path_buf());
        }
        return Ok(out);
    }

    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd =
            fs::read_dir(&dir).map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?;
        for ent in rd {
            let ent = ent.map_err(|e| format!("failed to read dir entry: {e}"))?;
            let p = ent.path();
            if p.is_dir() {
                // Skip `_`-prefixed directories (scratch/diag output, not test corpora).
                let skip = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('_'));
                if !skip {
                    stack.push(p);
                }
                continue;
            }
            if p.extension().and_then(|s| s.to_str()) == Some("js") {
                out.push(p);
            }
        }
    }

    out.sort();
    Ok(out)
}

pub fn run_one_js_file(path: &Path) -> Result<(), String> {
    let mut script =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    // Some editors may add an UTF-8 BOM; QuickJS doesn't accept it.
    if script.starts_with('\u{feff}') {
        script = script.trim_start_matches('\u{feff}').to_string();
    }

    // Process-level RIDL initialization is owned by the application entrypoint.
    // Unit tests for this crate may run without ridl-extensions.

    // Each JS file runs in an isolated context. Use the local Context wrapper so
    // ridl_context_init() is applied and singleton slots are filled.
    let mut context = crate::Context::default();

    context.eval(&script).map(|_result| ()).map_err(|e| {
        // Include file path and a short prefix to help diagnose syntax errors.
        let prefix: String = script.chars().take(80).collect();
        format!(
            "eval failed: {e}\n  file: {}\n  prefix: {:?}",
            path.display(),
            prefix
        )
    })
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CaseCounts {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

impl CaseCounts {
    pub fn record_ok(&mut self) {
        self.total += 1;
        self.passed += 1;
    }

    pub fn record_fail(&mut self) {
        self.total += 1;
        self.failed += 1;
    }
}

#[derive(Debug, Default)]
pub struct RunSummary {
    pub by_group: BTreeMap<String, CaseCounts>,
    pub total: CaseCounts,
}

pub fn group_key_for_path(path: &Path) -> String {
    // Grouping heuristics (stable, path-shape based):
    // - tests/global/<group>/... -> global/<group>
    // - tests/<mode>/<module>/... -> <mode>/<module>
    // - ridl-modules/<module>/tests/... -> module/<module>
    //
    // Paths may be absolute (default roots are resolved against the workspace
    // root), so relativize against it first; keep the original path when it is
    // not under the workspace root.
    let rel = path
        .strip_prefix(workspace_root())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| path.to_path_buf());

    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if parts.len() >= 3 && parts[0] == "tests" && parts[1] == "global" {
        return format!("global/{}", parts[2]);
    }

    if parts.len() >= 3 && parts[0] == "ridl-modules" {
        return format!("module/{}", parts[1]);
    }

    "ungrouped".to_string()
}

pub fn run_files_with_summary(files: &[PathBuf]) -> RunSummary {
    let mut summary = RunSummary::default();

    for f in files {
        let group = group_key_for_path(f);
        let g = summary.by_group.entry(group).or_default();

        match run_one_js_file(f) {
            Ok(()) => {
                summary.total.record_ok();
                g.record_ok();
            }
            Err(_e) => {
                summary.total.record_fail();
                g.record_fail();
            }
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_js_files_skips_underscore_prefixed_dirs() {
        let root = tempfile::tempdir().expect("tempdir");

        std::fs::write(root.path().join("a.js"), "// ok").expect("write a.js");
        std::fs::create_dir(root.path().join("_diag")).expect("mkdir _diag");
        std::fs::write(root.path().join("_diag").join("b.js"), "// diag").expect("write b.js");
        std::fs::create_dir(root.path().join("_private")).expect("mkdir _private");
        std::fs::write(root.path().join("_private").join("c.js"), "// private").expect("write c.js");
        // Nested `_` dirs are skipped at any depth.
        std::fs::create_dir_all(root.path().join("sub").join("_deep")).expect("mkdir sub/_deep");
        std::fs::write(root.path().join("sub").join("d.js"), "// nested ok").expect("write d.js");
        std::fs::write(
            root.path().join("sub").join("_deep").join("e.js"),
            "// deep skipped",
        )
        .expect("write e.js");

        let mut files = collect_js_files(root.path()).expect("collect_js_files");
        files.sort();

        let names: Vec<String> = files
            .iter()
            .map(|p| {
                p.strip_prefix(root.path())
                    .expect("path under temp root")
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert_eq!(names, vec!["a.js", "sub/d.js"]);
    }
}
