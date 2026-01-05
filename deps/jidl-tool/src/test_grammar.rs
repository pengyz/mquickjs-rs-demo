#[cfg(test)]
mod tests {
    use pest::Parser;
    use crate::parser::{IDLParser, Rule};
    use crate::parser::ast::*;
    use crate::parser::*;

    #[test]
    fn test_identifier_parsing() {
        let result = IDLParser::parse(Rule::identifier, "TestInterface");
        assert!(result.is_ok());
    }

    #[test]
    fn test_basic_interface_parsing() {
        // 根据RIDL规范，接口方法必须有返回类型
        let result = IDLParser::parse(Rule::interface_def, 
            "interface TestInterface { fn doSomething(value: int) -> void; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_interface_parsing() {
        let ridl = r#"
        interface Console {
            fn log(message: string) -> void;
            fn error(message: string) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Interface(interface) => {
                assert_eq!(interface.name, "Console");
                assert_eq!(interface.methods.len(), 2);
                
                let method1 = &interface.methods[0];
                assert_eq!(method1.name, "log");
                assert_eq!(method1.params.len(), 1);
                assert_eq!(method1.params[0].name, "message");
                assert_eq!(method1.params[0].param_type, Type::String);
                assert_eq!(method1.return_type, Type::Void);
                
                let method2 = &interface.methods[1];
                assert_eq!(method2.name, "error");
                assert_eq!(method2.params.len(), 1);
                assert_eq!(method2.params[0].name, "message");
                assert_eq!(method2.params[0].param_type, Type::String);
                assert_eq!(method2.return_type, Type::Void);
            }
            _ => panic!("Expected Interface"),
        }
    }

    #[test]
    fn test_interface_with_nullable_types() {
        let ridl = r#"
        interface NullableExample {
            fn getName() -> string?;
            fn getAge() -> int?;
            fn processName(name: string?) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Interface(interface) => {
                assert_eq!(interface.name, "NullableExample");
                assert_eq!(interface.methods.len(), 3);
                
                let method1 = &interface.methods[0];
                assert_eq!(method1.name, "getName");
                // Note: Currently our AST doesn't distinguish nullable types properly
                // This would require enhancement to the AST
                
                let method2 = &interface.methods[1];
                assert_eq!(method2.name, "getAge");
                
                let method3 = &interface.methods[2];
                assert_eq!(method3.name, "processName");
                assert_eq!(method3.params.len(), 1);
                assert_eq!(method3.params[0].name, "name");
                // Check that it's a nullable string type
            }
            _ => panic!("Expected Interface"),
        }
    }

    #[test]
    fn test_interface_with_union_types() {
        let ridl = r#"
        interface DataProcessor {
            fn processInput(data: string | int | array<string>) -> void;
            fn validateData(input: string) -> (bool | object);
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_class_definition() {
        let ridl = r#"
        class Person {
            name: string;
            age: int;
            Person(name: string, age: int);
            fn getName() -> string;
            fn getAge() -> int;
            fn setAge(age: int) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Class(class) => {
                assert_eq!(class.name, "Person");
                assert_eq!(class.fields.len(), 2);
                assert!(class.constructor.is_some());
                assert_eq!(class.methods.len(), 3);
                
                // Check fields
                assert_eq!(class.fields[0].name, "name");
                assert_eq!(class.fields[0].field_type, Type::String);
                assert_eq!(class.fields[1].name, "age");
                assert_eq!(class.fields[1].field_type, Type::Int);
                
                // Check constructor
                let constructor = class.constructor.as_ref().unwrap();
                assert_eq!(constructor.name, "Person");
                assert_eq!(constructor.params.len(), 2);
                assert_eq!(constructor.params[0].name, "name");
                assert_eq!(constructor.params[0].param_type, Type::String);
                assert_eq!(constructor.params[1].name, "age");
                assert_eq!(constructor.params[1].param_type, Type::Int);
                
                // Check methods
                let getName = &class.methods[0];
                assert_eq!(getName.name, "getName");
                assert_eq!(getName.return_type, Type::String);
                
                let getAge = &class.methods[1];
                assert_eq!(getAge.name, "getAge");
                assert_eq!(getAge.return_type, Type::Int);
                
                let setAge = &class.methods[2];
                assert_eq!(setAge.name, "setAge");
                assert_eq!(setAge.return_type, Type::Void);
                assert_eq!(setAge.params.len(), 1);
                assert_eq!(setAge.params[0].name, "age");
                assert_eq!(setAge.params[0].param_type, Type::Int);
            }
            _ => panic!("Expected Class"),
        }
    }

    #[test]
    fn test_enum_definition() {
        let ridl = r#"
        enum LogLevel {
            DEBUG = 0,
            INFO = 1,
            WARN = 2,
            ERROR = 3
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Enum(enum_def) => {
                assert_eq!(enum_def.name, "LogLevel");
                assert_eq!(enum_def.values.len(), 4);
                
                assert_eq!(enum_def.values[0].name, "DEBUG");
                assert_eq!(enum_def.values[0].value, Some(0));
                
                assert_eq!(enum_def.values[1].name, "INFO");
                assert_eq!(enum_def.values[1].value, Some(1));
                
                assert_eq!(enum_def.values[2].name, "WARN");
                assert_eq!(enum_def.values[2].value, Some(2));
                
                assert_eq!(enum_def.values[3].name, "ERROR");
                assert_eq!(enum_def.values[3].value, Some(3));
            }
            _ => panic!("Expected Enum"),
        }
    }

    #[test]
    fn test_struct_definition() {
        let ridl = r#"
        struct Person {
            name: string;
            age: int;
            email: string?;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::StructDef(struct_def) => {
                assert_eq!(struct_def.name, "Person");
                assert_eq!(struct_def.fields.len(), 3);
                
                assert_eq!(struct_def.fields[0].name, "name");
                assert_eq!(struct_def.fields[0].field_type, Type::String);
                
                assert_eq!(struct_def.fields[1].name, "age");
                assert_eq!(struct_def.fields[1].field_type, Type::Int);
                
                assert_eq!(struct_def.fields[2].name, "email");
                // Note: Currently our AST doesn't distinguish nullable types properly
            }
            _ => panic!("Expected StructDef"),
        }
    }

    #[test]
    fn test_json_struct_definition() {
        let ridl = r#"
        json struct Address {
            street: string;
            city: string;
            country: string;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::StructDef(struct_def) => {
                assert_eq!(struct_def.name, "Address");
                assert_eq!(struct_def.fields.len(), 3);
                assert_eq!(struct_def.serialization_format, SerializationFormat::Json);
            }
            _ => panic!("Expected StructDef"),
        }
    }

    #[test]
    fn test_callback_definition() {
        let ridl = r#"
        callback ProcessCallback(success: bool, result: string);
        callback LogCallback(entry: LogEntry);
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
        
        match &items[0] {
            IDLItem::Function(function) => {
                assert_eq!(function.name, "ProcessCallback");
                assert_eq!(function.params.len(), 2);
                assert_eq!(function.params[0].name, "success");
                assert_eq!(function.params[0].param_type, Type::Bool);
                assert_eq!(function.params[1].name, "result");
                assert_eq!(function.params[1].param_type, Type::String);
            }
            _ => panic!("Expected Function (Callback)"),
        }
    }

    #[test]
    fn test_complex_interface_with_callback() {
        let ridl = r#"
        interface CallbackExample {
            fn processData(input: string, callback: ProcessCallback) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_global_function() {
        let ridl = r#"
        fn setTimeout(callback: function, delay: int) -> void;
        fn add(a: int, b: int) -> int;
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
        
        match &items[0] {
            IDLItem::Function(function) => {
                assert_eq!(function.name, "setTimeout");
                assert_eq!(function.params.len(), 2);
                assert_eq!(function.params[0].name, "callback");
                assert_eq!(function.params[0].param_type, Type::Function);
                assert_eq!(function.params[1].name, "delay");
                assert_eq!(function.params[1].param_type, Type::Int);
                assert_eq!(function.return_type, Type::Void);
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_array_and_map_types() {
        let ridl = r#"
        interface ComplexExample {
            fn getItems() -> array<string>;
            fn processArray(items: array<int>) -> void;
            fn getMetadata() -> map<string, string>;
            fn updateConfig(config: object) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_namespace_definition() {
        // RIDL不再支持namespace特性，因为JavaScript原生不支持该特性
        // 此测试保留作为说明，但不会通过
        let source = r#"
        namespace Console {
            fn log(message: string);
        }
        "#;
        
        let result = parse_idl(source);
        assert!(result.is_err() || result.unwrap().len() == 0);
    }

    #[test]
    fn test_import_statement() {
        let ridl = r#"
        import NetworkPacket as Packet from Packet.proto;
        import TypeA, TypeB from Types.proto;
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
    }
}