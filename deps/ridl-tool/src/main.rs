use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

mod generator;
mod parser;
mod validator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        eprintln!("Commands:");
        eprintln!("  module <ridl-files...> <output-dir> - Generate module-specific files");
        eprintln!("  aggregate <ridl-files...> <output-dir> - Generate shared aggregation files");
        eprintln!("  resolve --cargo-toml <path> --out <plan.json> - Resolve RIDL modules from Cargo.toml deps");
        eprintln!("  generate --plan <plan.json> --out <dir> - Generate glue + aggregate header from plan");
        std::process::exit(1);
    }

    let command = &args[1];
    let remaining_args: Vec<&String> = args.iter().skip(2).collect();

    match command.as_str() {
        "module" => {
            if remaining_args.len() < 2 {
                eprintln!("Usage: {} module <ridl-files...> <output-dir>", args[0]);
                std::process::exit(1);
            }

            // 最后一个参数是输出目录
            let output_dir = remaining_args.last().unwrap();
            // 其余参数是ridl文件
            let ridl_files = &remaining_args[..remaining_args.len() - 1];

            // 生成模块特定文件
            for ridl_file in ridl_files {
                if !ridl_file.ends_with(".ridl") {
                    eprintln!("Warning: Skipping non-ridl file: {}", ridl_file);
                    continue;
                }

                // 解析RIDL文件
                let content = std::fs::read_to_string(ridl_file)?;
                let parsed = parser::parse_ridl_file(&content)?;
                let items = parsed.items;
                validator::validate(&items)?;

                // 从文件路径提取模块名
                let module_name = Path::new(ridl_file)
                    .file_stem()
                    .ok_or("Invalid ridl file path")?
                    .to_str()
                    .ok_or("Invalid UTF-8 in file name")?
                    .to_string();

                // 生成模块特定文件
                generator::generate_module_files(&items, parsed.mode, Path::new(output_dir), &module_name)?;
            }
        }
        "aggregate" => {
            if remaining_args.len() < 2 {
                eprintln!("Usage: {} aggregate <ridl-files...> <output-dir>", args[0]);
                std::process::exit(1);
            }

            // 最后一个参数是输出目录
            let output_dir = remaining_args.last().unwrap();
            // 其余参数是ridl文件
            let ridl_files: Vec<String> = remaining_args[..remaining_args.len() - 1]
                .iter()
                .map(|s| (*s).clone())
                .collect();

            // 生成共享聚合文件
            generator::generate_shared_files(&ridl_files, output_dir)?;
        }
        "resolve" => {
            // ridl-tool resolve --cargo-toml <path> --out <plan.json>
            let mut cargo_toml: Option<PathBuf> = None;
            let mut out: Option<PathBuf> = None;
            let mut it = remaining_args.into_iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--cargo-toml" => cargo_toml = it.next().map(|s| PathBuf::from(s)),
                    "--out" => out = it.next().map(|s| PathBuf::from(s)),
                    _ => {
                        eprintln!("Unknown arg: {}", a);
                        std::process::exit(1);
                    }
                }
            }

            let Some(cargo_toml) = cargo_toml else {
                eprintln!("Missing --cargo-toml");
                std::process::exit(1);
            };
            let Some(out_path) = out else {
                eprintln!("Missing --out");
                std::process::exit(1);
            };

            let out_dir = out_path
                .parent()
                .unwrap_or_else(|| Path::new("."));
            fs::create_dir_all(out_dir)?;
            let plan = ridl_tool::resolve::resolve_from_cargo_toml(&cargo_toml, out_dir)
                .map_err(|e| format!("resolve failed: {e}"))?;

            let json = serde_json::to_string_pretty(&plan)?;
            fs::write(&out_path, json)?;
        }
        "generate" => {
            // ridl-tool generate --plan <plan.json> --out <dir>
            let mut plan_path: Option<PathBuf> = None;
            let mut out_dir: Option<PathBuf> = None;
            let mut it = remaining_args.into_iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--plan" => plan_path = it.next().map(|s| PathBuf::from(s)),
                    "--out" => out_dir = it.next().map(|s| PathBuf::from(s)),
                    _ => {
                        eprintln!("Unknown arg: {}", a);
                        std::process::exit(1);
                    }
                }
            }

            let Some(plan_path) = plan_path else {
                eprintln!("Missing --plan");
                std::process::exit(1);
            };
            let Some(out_dir) = out_dir else {
                eprintln!("Missing --out");
                std::process::exit(1);
            };

            fs::create_dir_all(&out_dir)?;

            let plan_text = fs::read_to_string(&plan_path)?;
            let plan: ridl_tool::plan::RidlPlan = serde_json::from_str(&plan_text)?;

            let mut ridl_files: Vec<String> = Vec::new();
            for m in &plan.modules {
                for f in &m.ridl_files {
                    ridl_files.push(f.display().to_string());
                }
            }

            // per-module glue/impl
            let module_out = out_dir.join("ridl");
            fs::create_dir_all(&module_out)?;
            for m in &plan.modules {
                for f in &m.ridl_files {
                    let ridl_file = f.display().to_string();
                    let content = fs::read_to_string(&ridl_file)?;
                    let parsed = parser::parse_ridl_file(&content)?;
                    let items = parsed.items;
                    validator::validate(&items)?;
                    let module_name = Path::new(&ridl_file)
                        .file_stem()
                        .ok_or("Invalid ridl file path")?
                        .to_str()
                        .ok_or("Invalid UTF-8 in file name")?
                        .to_string();
                    generator::generate_module_files(&items, parsed.mode, &module_out, &module_name)?;
                }

            }

            // aggregate header + symbols
            generator::generate_shared_files(&ridl_files, out_dir.to_str().ok_or("Invalid out dir")?)?;

            // app-side module initialization aggregator (derived from plan.modules crate names)
            let init_path = out_dir.join("ridl_modules_initialize.rs");
            let mut init_rs = String::new();
            init_rs.push_str("// Generated module initialization for RIDL extensions\n");
            init_rs.push_str("pub fn initialize_modules() {\n");
            for m in &plan.modules {
                init_rs.push_str(&format!("    {crate_name}::initialize_module();\n", crate_name = m.crate_name));
            }
            init_rs.push_str("}\n");
            fs::write(&init_path, init_rs)?;

            // unified initializer entrypoint (used by macro include)
            let unified_path = out_dir.join("ridl_initialize.rs");
            let unified_rs = "// Generated RIDL initializer entrypoint\n\
pub mod ridl_initialize {\n\
    mod symbols {\n\
        include!(concat!(env!(\"OUT_DIR\"), \"/ridl_symbols.rs\"));\n\
    }\n\
    mod modules {\n\
        include!(concat!(env!(\"OUT_DIR\"), \"/ridl_modules_initialize.rs\"));\n\
    }\n\
\n\
    pub fn initialize() {\n\
        modules::initialize_modules();\n\
        symbols::ensure_symbols();\n\
    }\n\
}\n";
            fs::write(&unified_path, unified_rs)?;
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}
