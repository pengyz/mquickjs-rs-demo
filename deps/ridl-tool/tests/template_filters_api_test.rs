use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn collect_template_filters(repo_root: &Path) -> BTreeSet<String> {
    let templates_dir = repo_root.join("deps/ridl-tool/templates");
    let mut out = BTreeSet::new();

    for ent in fs::read_dir(&templates_dir).expect("read templates dir") {
        let ent = ent.expect("read templates entry");
        let path = ent.path();
        if path.extension().and_then(|s| s.to_str()) != Some("j2") {
            continue;
        }
        let s = fs::read_to_string(&path).expect("read template");

        // Very small parser: collect `|filter_name` occurrences.
        for cap in s.match_indices('|') {
            let i = cap.0 + 1;
            let rest = &s[i..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && name != "_" {
                out.insert(name);
            }
        }

        // Also consider built-in filters used by templates.
        for builtin in ["upper", "length"].iter() {
            if s.contains(&format!("|{}", builtin)) {
                out.insert((*builtin).to_string());
            }
        }
    }

    out
}

fn collect_pub_filters(repo_root: &Path) -> BTreeSet<String> {
    let filters_rs = repo_root.join("deps/ridl-tool/src/generator/filters.rs");
    let s = fs::read_to_string(&filters_rs).expect("read filters.rs");

    let mut out = BTreeSet::new();
    for line in s.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("pub fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && name != "_" {
                out.insert(name);
            }
        }
    }
    out
}

#[test]
fn template_filters_are_in_sync_with_filters_rs() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");

    let used = collect_template_filters(&repo_root);
    let exported = collect_pub_filters(&repo_root);

    // Filters that are currently used without a `|filter` pipe.
    // Example: `safe` is an Askama built-in. `capitalize/lower/length/upper/default` are filters,
    // but `capitalize/lower` are also built-ins in some engines; we still treat them as template
    // usage because the templates contain them.
    let allow_missing = BTreeSet::<String>::from_iter([
        "safe".to_string(),
        "capitalize".to_string(),
        "lower".to_string(),
        "upper".to_string(),
        "length".to_string(),
    ]);

    for f in used.iter() {
        if allow_missing.contains(f) {
            continue;
        }
        assert!(
            exported.contains(f),
            "template uses filter '{f}' but filters.rs does not export it"
        );
    }

    // Enforce that our exported filter API stays minimal.
    let allow_unused_exports = BTreeSet::<String>::from_iter([
        // Built-in filter names that we intentionally provide for compatibility/clarity.
        "upper".to_string(),
        "length".to_string(),
    ]);
    for f in exported.iter() {
        if allow_unused_exports.contains(f) {
            continue;
        }
        assert!(
            used.contains(f),
            "filters.rs exports pub filter '{f}' but no template uses it"
        );
    }
}
