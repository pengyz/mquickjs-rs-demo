use askama::Template;
use std::path::Path;

use crate::generator::filters;
use crate::plan::RidlPlan;

use super::singleton_aggregate;

#[derive(Template)]
#[template(path = "mquickjs_ridl_register.c.j2", escape = "none")]
pub(super) struct MquickjsRidlRegisterCTemplate {
    pub(super) modules: Vec<super::TemplateModule>,
    pub(super) proto_vars: Vec<singleton_aggregate::ProtoVarInit>,
}

pub(super) fn generate_mquickjs_ridl_register_c(
    plan: &RidlPlan,
    out_dir: &Path,
    ir: Option<&super::AggregateIR>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Reuse the same TemplateModule list and class_id allocation used by register.h generation.
    // We do this by re-parsing the same ridl files and then applying the same monotonic
    // allocation logic.
    let mut ridl_files: Vec<std::path::PathBuf> = Vec::new();
    for m in &plan.modules {
        ridl_files.extend(m.ridl_files.iter().cloned());
    }

    // ir == None means the app selected zero RIDL modules (see
    // generate_register_h_and_symbols). register.c is unconditionally consumed
    // by the ridl-variant stdlib build, so emit a valid empty table file
    // instead of failing — a module-less app is a legitimate aggregate.
    let Some(ir) = ir else {
        let t = MquickjsRidlRegisterCTemplate {
            modules: Vec::new(),
            proto_vars: Vec::new(),
        };
        std::fs::write(out_dir.join("mquickjs_ridl_register.c"), t.render()?)?;
        return Ok(());
    };

    let modules = super::build_template_modules(&ridl_files, &ir.classes)?;
    let proto_vars = singleton_aggregate::collect_proto_vars(plan)?;

    let t = MquickjsRidlRegisterCTemplate {
        modules,
        proto_vars,
    };
    std::fs::write(out_dir.join("mquickjs_ridl_register.c"), t.render()?)?;
    Ok(())
}
