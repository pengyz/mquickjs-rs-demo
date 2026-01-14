use crate::plan::RidlPlan;
use askama::Template;
use std::collections::BTreeMap;
use std::path::Path;


#[derive(Template)]
#[template(path = "ridl_runtime_support.rs.j2")]
struct RidlRuntimeSupportTemplate {
    slots: Vec<Slot>,
    singleton_inits: Vec<SingletonInit>,
}

#[derive(Debug, Clone)]
struct SingletonInit {
    crate_name: String,
    slot_index: u32,
    vt_ident: String,
    singleton_key: String,
}


#[derive(Debug, Clone)]
struct Slot {
    /// sanitized Rust identifier
    name: String,
    index: u32,
}



pub fn generate_ridl_runtime_support(
    plan: &RidlPlan,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // We share the same slot ordering and singleton init ordering with the legacy generator.
    let mut slots: Vec<Slot> = Vec::new();
    let mut singleton_inits: Vec<SingletonInit> = Vec::new();
    let mut slot_map: BTreeMap<String, u32> = BTreeMap::new();

    for m in &plan.modules {
        for ridl_file in &m.ridl_files {
            let src = std::fs::read_to_string(ridl_file)?;
            let parsed = crate::parser::parse_ridl_file(&src)?;

            for item in parsed.items {
                let crate::parser::ast::IDLItem::Singleton(s) = item else {
                    continue;
                };

                let name = sanitize_ident(&s.name);
                if !slots.iter().any(|x| x.name == name) {
                    slots.push(Slot {
                        name: name.clone(),
                        index: 0,
                    });
                }

                singleton_inits.push(SingletonInit {
                    crate_name: m.crate_name.clone(),
                    slot_index: 0,
                    vt_ident: format!("RIDL_{}_SINGLETON_VT", name.to_uppercase()),
                    singleton_key: name.to_lowercase(),
                });
            }
        }
    }

    slots.sort_by(|a, b| a.name.cmp(&b.name));
    for (i, s) in slots.iter_mut().enumerate() {
        s.index = i as u32;
        slot_map.insert(s.name.to_lowercase(), s.index);
    }

    for s in singleton_inits.iter_mut() {
        if let Some(idx) = slot_map.get(&s.singleton_key) {
            s.slot_index = *idx;
        }
    }
    singleton_inits.sort_by(|a, b| {
        (a.crate_name.as_str(), a.slot_index).cmp(&(b.crate_name.as_str(), b.slot_index))
    });

    let t = RidlRuntimeSupportTemplate {
        slots,
        singleton_inits,
    };
    std::fs::write(out_dir.join("ridl_runtime_support.rs"), t.render()?)?;
    Ok(())
}


fn sanitize_ident(name: &str) -> String {
    // Keep it simple: allow [A-Za-z_][A-Za-z0-9_]*; otherwise map to underscores.
    // Also avoid Rust keywords minimally.
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        let ok = if i == 0 {
            ch == '_' || ch.is_ascii_alphabetic()
        } else {
            ch == '_' || ch.is_ascii_alphanumeric()
        };
        out.push(if ok { ch } else { '_' });
    }
    if out.is_empty() {
        out.push_str("singleton");
    }

    // Avoid keywords we care about.
    match out.as_str() {
        "type" | "match" | "mod" | "crate" | "self" | "super" => format!("{out}_"),
        _ => out,
    }
}
