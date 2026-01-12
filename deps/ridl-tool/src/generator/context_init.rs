use askama::Template;
use std::path::Path;

use crate::plan::RidlPlan;

#[derive(Debug, Clone)]
pub struct TemplateSingletonVTable {
    #[allow(dead_code)]
    pub singleton_name: String,
    pub vtable_struct_name: String,
    pub field_name: String,
    pub vtable_create_path: String,
}

pub fn generate_ridl_context_init(
    plan: &RidlPlan,
    out_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut singletons: Vec<TemplateSingletonVTable> = Vec::new();

    for m in &plan.modules {
        for ridl_file in &m.ridl_files {
            let src = std::fs::read_to_string(ridl_file)?;
            let parsed = crate::parser::parse_ridl_file(&src)?;

            for item in parsed.items {
                let crate::parser::ast::IDLItem::Singleton(s) = item else {
                    continue;
                };
                let field_name = s.name.to_lowercase();
                let vtable_struct_name = format!("Ridl{}VTable", to_rust_type_ident(&field_name));

                // vtable create function naming convention: ridl_<singleton>_vtable_create
                let vtable_fn = format!("ridl_{}_vtable_create", s.name.to_lowercase());
                let vtable_create_path = format!("{}::impls::{}", m.crate_name, vtable_fn);

                singletons.push(TemplateSingletonVTable {
                    singleton_name: s.name,
                    vtable_struct_name,
                    field_name,
                    vtable_create_path,
                });
            }
        }
    }

    singletons.sort_by(|a, b| a.field_name.cmp(&b.field_name));

    let tmpl = super::RidlContextInitTemplate {
        header_struct_name: "RidlCtxExtHeader".to_string(),
        singletons,
    };

    let code = tmpl.render()?;
    std::fs::write(out_path, code)?;
    Ok(())
}

fn to_rust_type_ident(name: &str) -> String {
    // Minimal PascalCase conversion for RIDL identifiers.
    let mut out = String::new();
    let mut upper = true;
    for ch in name.chars() {
        if ch == '_' || ch == '-' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        "Singleton".to_string()
    } else {
        out
    }
}
