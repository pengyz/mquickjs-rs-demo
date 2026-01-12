use std::{
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
    let script =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    // Make sure process-level RIDL registration (symbols + modules) is done.
    // This is idempotent and required for scripts relying on RIDL extensions.
    crate::ridl_initialize::ridl_initialize::initialize();

    // Each JS file runs in an isolated context. Use the local Context wrapper so
    // ridl_context_init() is applied and singleton slots are filled.
    let mut context = crate::Context::default();

    context
        .eval(&script)
        .map(|_result| ())
        .map_err(|e| format!("eval failed: {e}"))
}
