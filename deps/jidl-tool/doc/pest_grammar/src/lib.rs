use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct RIDLParser;

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    // 基础语法测试
    #[test]
    fn test_identifier() {
        let result = RIDLParser::parse(
            Rule::identifier,
            "validIdentifier123"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_literal() {
        let result = RIDLParser::parse(
            Rule::string_literal,
            "\"hello world\""
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_integer_literal() {
        let result = RIDLParser::parse(
            Rule::integer_literal,
            "12345"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_float_literal() {
        let result = RIDLParser::parse(
            Rule::float_literal,
            "12.34"
        );
        assert!(result.is_ok());
    }

    // 复杂类型测试
    #[test]
    fn test_nullable_type() {
        let result = RIDLParser::parse(
            Rule::r#type,
            "string?"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_union_type() {
        let result = RIDLParser::parse(
            Rule::r#type,
            "string | int | bool"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_array_type() {
        let result = RIDLParser::parse(
            Rule::r#type,
            "array<string>"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_map_type() {
        let result = RIDLParser::parse(
            Rule::r#type,
            "map<string, int>"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_group_type() {
        let result = RIDLParser::parse(
            Rule::r#type,
            "(Person | LogEntry | string)"
        );
        assert!(result.is_ok());
    }

    // 接口定义测试
    #[test]
    fn test_interface_definition() {
        let input = r#"
        interface TestInterface {
            fn getValue() -> int;
            fn process(input: string);
            fn optionalParam(name: string?);
        }
        "#;
        
        let result = RIDLParser::parse(
            Rule::interface_def,
            input
        );
        assert!(result.is_ok());
    }

    // 类定义测试
    #[test]
    fn test_class_definition() {
        let input = r#"
        class TestClass {
            name: string;
            age: int;
            const MAX_AGE: int = 150;
            readonly property enabled: bool;
            TestClass(name: string, age: int);
            fn getName() -> string;
            fn setAge(age: int) -> void;
        }
        "#;
        
        let result = RIDLParser::parse(
            Rule::class_def,
            input
        );
        assert!(result.is_ok());
    }

    // 枚举定义测试
    #[test]
    fn test_enum_definition() {
        let input = r#"
        enum TestEnum {
            VALUE1 = 0,
            VALUE2 = 1,
            VALUE3 = 2
        }
        "#;
        
        let result = RIDLParser::parse(
            Rule::enum_def,
            input
        );
        assert!(result.is_ok());
    }

    // 结构体定义测试
    #[test]
    fn test_struct_definition() {
        let input = r#"
        json struct TestStruct {
            field1: string;
            field2: int?;
            field3: array<string>;
        }
        "#;
        
        let result = RIDLParser::parse(
            Rule::struct_def,
            input
        );
        assert!(result.is_ok());
    }

    // 使用不同序列化格式的结构体
    #[test]
    fn test_msgpack_struct_definition() {
        let input = r#"
        msgpack struct TestStruct {
            field1: string;
            field2: int;
        }
        "#;
        
        let result = RIDLParser::parse(
            Rule::struct_def,
            input
        );
        assert!(result.is_ok());
    }

    // 回调定义测试
    #[test]
    fn test_callback_definition() {
        let input = r#"
        callback ProcessCallback(result: string | object, success: bool);
        "#;
        
        let result = RIDLParser::parse(
            Rule::callback_def,
            input
        );
        assert!(result.is_ok());
    }

    // 函数定义测试
    #[test]
    fn test_function_definition() {
        let input = r#"
        fn add(a: int, b: int) -> int;
        "#;
        
        let result = RIDLParser::parse(
            Rule::global_function,
            input
        );
        assert!(result.is_ok());
    }

    // using定义测试
    #[test]
    fn test_using_definition() {
        let input = r#"
        using UserId = int;
        "#;
        
        let result = RIDLParser::parse(
            Rule::using_def,
            input
        );
        assert!(result.is_ok());
    }

    // import语句测试
    #[test]
    fn test_import_definition() {
        let input = r#"
        import NetworkPacket from "Packet.proto";
        "#;
        
        let result = RIDLParser::parse(
            Rule::import_stmt,
            input
        );
        assert!(result.is_ok());
    }

    // 完整RIDL文件测试 - 修复：避免使用callback作为参数名
    #[test]
    fn test_complete_ridl_file() {
        let input = r#"
        // 完整的RIDL文件示例
        using UserId = int;
        import NetworkPacket from "Packet.proto";
        
        json struct Person {
            name: string;
            age: int;
            email: string?;
        }
        
        interface UserService {
            fn getUser(id: UserId) -> Person?;
            fn processUsers(users: array<Person>, cb: callback(success: bool));
        }
        
        class UserProcessor {
            cache: map<UserId, Person>?;
            UserProcessor();
            fn processUser(user: Person) -> bool;
        }
        
        enum Status {
            PENDING = 0,
            PROCESSING = 1,
            COMPLETED = 2
        }
        
        callback ResultCallback(success: bool, result: string?);
        
        fn setTimeout(cb: callback(success: bool), delay: int);
        "#;
        
        let result = RIDLParser::parse(
            Rule::idl,
            input
        );
        assert!(result.is_ok());
    }

    // 模块化定义测试 - 修复：使用idl规则而不是definition规则，移除分号
    #[test]
    fn test_module_definition() {
        let input = r#"
        module system.network@1.0
        interface Network {
            fn getStatus() -> string;
        }
        "#;
        
        let result = RIDLParser::parse(
            Rule::idl,
            input
        );
        assert!(result.is_ok());
    }

    // singleton定义测试
    #[test]
    fn test_singleton_definition() {
        let input = r#"
        singleton console {
            fn log(message: string);
            fn error(message: string);
            readonly property enabled: bool;
        }
        "#;
        
        let result = RIDLParser::parse(
            Rule::singleton_def,
            input
        );
        assert!(result.is_ok());
    }

    // 复杂联合类型测试
    #[test]
    fn test_complex_union_type() {
        let result = RIDLParser::parse(
            Rule::r#type,
            "string | int | array<string> | map<string, int> | Person"
        );
        assert!(result.is_ok());
    }

    // 复杂可空类型测试
    #[test]
    fn test_complex_nullable_type() {
        let result = RIDLParser::parse(
            Rule::r#type,
            "(string | int)?"
        );
        assert!(result.is_ok());
    }

    // 错误用例测试
    #[test]
    fn test_invalid_interface_missing_brace() {
        let input = r#"interface TestInterface { fn getValue() -> int; "#;  // 缺少闭合大括号
        let result = RIDLParser::parse(Rule::interface_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_type_definition_array() {
        let input = r#"array<"#;  // 不完整的数组类型定义
        let result = RIDLParser::parse(Rule::r#type, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_type_definition_map() {
        let input = r#"map<string"#;  // 不完整的映射类型定义
        let result = RIDLParser::parse(Rule::r#type, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_enum_definition() {
        let input = r#"enum TestEnum { VALUE1 = 0, "#;  // 缺少闭合大括号
        let result = RIDLParser::parse(Rule::enum_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_struct_definition() {
        let input = r#"struct TestStruct { field1: string; "#;  // 缺少闭合大括号
        let result = RIDLParser::parse(Rule::struct_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_function_definition() {
        let input = r#"fn add(a: int, "#;  // 不完整的函数定义
        let result = RIDLParser::parse(Rule::global_function, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_callback_definition() {
        let input = r#"callback ProcessCallback("#;  // 不完整的回调定义
        let result = RIDLParser::parse(Rule::callback_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_using_definition() {
        let input = r#"using UserId "#;  // 不完整的using定义
        let result = RIDLParser::parse(Rule::using_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_import_definition() {
        let input = r#"import NetworkPacket from "#;  // 不完整的import定义
        let result = RIDLParser::parse(Rule::import_stmt, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_class_definition() {
        let input = r#"class TestClass { name: string "#;  // 缺少分号
        let result = RIDLParser::parse(Rule::class_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_singleton_definition() {
        let input = r#"singleton console { fn log(message: string) "#;  // 缺少分号
        let result = RIDLParser::parse(Rule::singleton_def, input);
        assert!(result.is_err());
    }

    // 修复错误用例：使用更现实的无效语法测试
    #[test]
    fn test_invalid_complete_ridl_file_with_syntax_error() {
        // 一个更复杂的错误用例
        let input = r#"
        interface TestInterface {
            fn getValue() ->;  // 错误：缺少返回类型
        }
        "#;
        
        let result = RIDLParser::parse(Rule::idl, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_string_literal() {
        let input = r#""hello world"#;  // 缺少闭合引号
        let result = RIDLParser::parse(Rule::string_literal, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_identifier_with_keyword() {
        let input = r#"interface"#;  // 关键字不能作为标识符
        let result = RIDLParser::parse(Rule::identifier, input);
        assert!(result.is_err());
    }
}