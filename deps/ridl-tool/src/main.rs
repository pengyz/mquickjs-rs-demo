use std::env;
use std::fs;
use std::path::Path;

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
                let items = parser::parse_ridl(&content)?;
                validator::validate(&items)?;

                // 从文件路径提取模块名
                let module_name = Path::new(ridl_file)
                    .file_stem()
                    .ok_or("Invalid ridl file path")?
                    .to_str()
                    .ok_or("Invalid UTF-8 in file name")?
                    .to_string();

                // 生成模块特定文件
                generator::generate_module_files(&items, Path::new(output_dir), &module_name)?;
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
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}
