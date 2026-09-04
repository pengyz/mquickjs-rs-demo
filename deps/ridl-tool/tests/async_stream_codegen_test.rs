//! AsyncStream 代码生成测试

use std::fs;
use tempfile::TempDir;

#[test]
fn test_async_stream_basic() {
    let idl = r#"
        class HttpClient {
            fn fetch(url: string, cb: callback(error: string?, data: string?)) -> void;
        }
    "#;

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    fs::create_dir_all(&output_path).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(idl).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_path,
        "test_module",
    );

    // 读取生成的代码
    let api_path = output_path.join("api.rs");
    let code = fs::read_to_string(&api_path).unwrap();

    // 验证生成的代码包含 callback 相关内容
    assert!(code.contains("callback") || code.contains("cb"));
}

#[test]
fn test_async_stream_with_decorator() {
    let idl = r#"
        class FileService {
            @nonCancellable
            fn save(data: string, cb: callback(error: string?, success: bool)) -> void;
        }
    "#;

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    fs::create_dir_all(&output_path).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(idl).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_path,
        "test_module",
    );

    // 读取生成的代码
    let api_path = output_path.join("api.rs");
    let code = fs::read_to_string(&api_path).unwrap();

    // 验证生成的代码包含 nonCancellable 相关内容
    assert!(code.contains("nonCancellable") || code.contains("spawn_non_cancellable"));
}

#[test]
fn test_async_stream_with_timeout() {
    let idl = r#"
        class CacheService {
            @timeout(5000)
            fn update(key: string, cb: callback(error: string?, success: bool)) -> void;
        }
    "#;

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    fs::create_dir_all(&output_path).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(idl).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_path,
        "test_module",
    );

    // 读取生成的代码
    let api_path = output_path.join("api.rs");
    let code = fs::read_to_string(&api_path).unwrap();

    // 验证生成的代码包含 timeout 相关内容
    assert!(code.contains("timeout") || code.contains("spawn_with_timeout"));
}

#[test]
fn test_async_stream_multiple_params() {
    let idl = r#"
        class DataService {
            fn process(input: string, options: object, cb: callback(error: string?, result: string?)) -> void;
        }
    "#;

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    fs::create_dir_all(&output_path).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(idl).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_path,
        "test_module",
    );

    // 读取生成的代码
    let api_path = output_path.join("api.rs");
    let code = fs::read_to_string(&api_path).unwrap();

    // 验证生成的代码包含多个参数处理
    assert!(code.contains("input") && code.contains("options"));
}

#[test]
fn test_async_stream_error_handling() {
    let idl = r#"
        class ErrorHandler {
            fn riskyOperation(cb: callback(error: string?, data: string?)) -> void;
        }
    "#;

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    fs::create_dir_all(&output_path).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(idl).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_path,
        "test_module",
    );

    // 读取生成的代码
    let api_path = output_path.join("api.rs");
    let code = fs::read_to_string(&api_path).unwrap();

    // 验证生成的代码包含错误处理
    assert!(code.contains("error") || code.contains("Error"));
}