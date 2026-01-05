use std::fs;
use std::path::Path;

mod parser;
mod generator;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        println!("Usage: {} --input <input.ridl> --output <output_dir>", args[0]);
        std::process::exit(1);
    }
    
    let mut input_file = String::new();
    let mut output_dir = String::new();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                if i + 1 < args.len() {
                    input_file = args[i + 1].clone();
                    i += 1;
                } else {
                    eprintln!("Error: --input requires a file path");
                    std::process::exit(1);
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_dir = args[i + 1].clone();
                    i += 1;
                } else {
                    eprintln!("Error: --output requires a directory path");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }
    
    if input_file.is_empty() || output_dir.is_empty() {
        eprintln!("Error: Both --input and --output are required");
        std::process::exit(1);
    }
    
    // Read the RIDL file
    let ridl_content = match fs::read_to_string(&input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file {}: {}", input_file, e);
            std::process::exit(1);
        }
    };
    
    // Parse the RIDL content
    match parser::parse_idl(&ridl_content) {
        Ok(items) => {
            // Create output directory if it doesn't exist
            if !Path::new(&output_dir).exists() {
                if let Err(e) = fs::create_dir_all(&output_dir) {
                    eprintln!("Error creating output directory {}: {}", output_dir, e);
                    std::process::exit(1);
                }
            }
            
            // Generate C bindings
            match generator::generate_c_bindings(&items, Path::new(&output_dir)) {
                Ok(_) => {
                    println!("Successfully generated C bindings to {}", output_dir);
                }
                Err(e) => {
                    eprintln!("Error generating C bindings: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error parsing RIDL file: {}", e);
            std::process::exit(1);
        }
    }
}