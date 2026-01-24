use std::path::PathBuf;

use crate::parser;

use super::{TemplateClass, TemplateFunction, TemplateInterface, TemplateModule, TemplateSingleton};

pub(super) fn build_template_modules(
    ridl_files: &[PathBuf],
    classes: &[TemplateClass],
) -> Result<Vec<TemplateModule>, Box<dyn std::error::Error>> {
    // This function mirrors the module parsing + allocation logic in generator/mod.rs,
    // but accepts the already-allocated TemplateClass list (from AggregateIR) as the
    // source of truth for class_id values.

    // Parse modules.
    let mut modules: Vec<TemplateModule> = Vec::new();

    for ridl_file in ridl_files {
        let content = std::fs::read_to_string(ridl_file)?;
        let parsed = parser::parse_ridl_file(&content)?;

        let module_name = parsed
            .module
            .as_ref()
            .map(|m| m.module_path.clone())
            .unwrap_or_else(|| "GLOBAL".to_string());

        let mut interfaces: Vec<TemplateInterface> = Vec::new();
        let mut functions: Vec<TemplateFunction> = Vec::new();
        let singletons: Vec<TemplateSingleton> = Vec::new();
        let mut local_classes: Vec<TemplateClass> = Vec::new();

        for item in &parsed.items {
            match item {
                parser::ast::IDLItem::Function(f) => {
                    functions.push(TemplateFunction::from_with_mode(f.clone(), parsed.mode))
                }
                parser::ast::IDLItem::Interface(i) => {
                    interfaces.push(TemplateInterface::from_with_mode(i.clone(), parsed.mode))
                }
                parser::ast::IDLItem::Singleton(s) => {
                    // Singleton aggregation is not needed for mquickjs_ridl_register.c generation.
                    let _ = (s, &module_name);
                }
                parser::ast::IDLItem::Class(c) => {
                    local_classes.push(TemplateClass::from_with_mode(module_name.clone(), c.clone(), parsed.mode))
                }
                _ => {}
            }
        }

        let (require_full_name, require_base, require_v_major, require_v_minor, require_v_patch) =
            if let Some(m) = &parsed.module {
                let ver = m.version.as_ref().expect("module version is required by parser");
                let v = crate::parser::require_spec::Version::parse_no_ws(ver)
                    .expect("module version is validated by parser");
                (
                    format!("{}@{}", m.module_path, ver),
                    m.module_path.clone(),
                    v.major,
                    v.minor,
                    v.patch,
                )
            } else {
                (String::new(), String::new(), 0, 0, 0)
            };

        modules.push(TemplateModule {
            module_name,
            module_decl: parsed.module,
            file_mode: parsed.mode,
            require_full_name,
            require_base,
            require_v_major,
            require_v_minor,
            require_v_patch,
            module_class_id: 0,
            interfaces,
            functions,
            singletons,
            classes: local_classes,
        });
    }

    // Assign module_class_id in stable order.
    let mut next_module_id: u32 = 0;
    for m in &mut modules {
        if m.module_decl.is_some() {
            m.module_class_id = next_module_id;
            next_module_id += 1;
        }
    }

    // Apply class_id allocation from AggregateIR classes.
    // We match by (module_name, class_name).
    for m in &mut modules {
        for c in &mut m.classes {
            if let Some(found) = classes
                .iter()
                .find(|x| x.module_name == c.module_name && x.name == c.name)
            {
                c.class_id = found.class_id;
            }
        }
    }

    Ok(modules)
}
