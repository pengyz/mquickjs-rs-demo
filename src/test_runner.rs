use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

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
                stack.push(p);
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
        format!("eval failed: {e}\n  file: {}\n  prefix: {:?}", path.display(), prefix)
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
    // Grouping heuristics (stable, path-based):
    // - tests/global/<group>/... -> global/<group>
    // - tests/<mode>/<module>/... -> <mode>/<module>
    // - ridl-modules/<module>/tests/... -> module/<module>
    let parts: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if parts.len() >= 3 && parts[0] == "tests" && parts[1] == "global" {
        return format!("global/{}", parts[2]);
    }

    if parts.len() >= 5 && parts[0] == "ridl-modules" && parts[1] == "tests" {
        return format!("{}/{}", parts[2], parts[3]);
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
