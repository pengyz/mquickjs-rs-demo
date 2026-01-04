#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use crate::parser::IDLParser;
    use crate::parser::Rule;

    #[test]
    fn test_identifier_parsing() {
        let result = IDLParser::parse(Rule::identifier, "TestInterface");
        assert!(result.is_ok());
    }

    #[test]
    fn test_interface_parsing() {
        let result = IDLParser::parse(Rule::interface_def, "interface TestInterface { void doSomething(int value); }");
        assert!(result.is_ok());
    }
}