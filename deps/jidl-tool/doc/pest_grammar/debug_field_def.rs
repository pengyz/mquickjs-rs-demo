use pest::Parser;
use ridl_pest_test::RIDLParser;
use ridl_pest_test::Rule;

fn main() {
    // 测试重复的可空类型
    let input = r#"field: string??"#;
    
    println!("Testing repeated nullable type in field def: {}", input);
    
    match RIDLParser::parse(Rule::field_def, input) {
        Ok(pairs) => {
            println!("✓ Repeated nullable type in field def parsed successfully: {:?}", pairs.as_str());
        }
        Err(e) => {
            println!("✗ Repeated nullable type in field def parsing failed: {}", e);
        }
    }
    
    // 测试不完整的联合类型
    let input2 = r#"field: string |"#;
    
    println!("\nTesting incomplete union type in field def: {}", input2);
    
    match RIDLParser::parse(Rule::field_def, input2) {
        Ok(pairs) => {
            println!("✓ Incomplete union type in field def parsed successfully: {:?}", pairs.as_str());
        }
        Err(e) => {
            println!("✗ Incomplete union type in field def parsing failed: {}", e);
        }
    }
    
    // 测试正常的字段定义
    let input3 = r#"field: string?"#;
    
    println!("\nTesting normal nullable type in field def: {}", input3);
    
    match RIDLParser::parse(Rule::field_def, input3) {
        Ok(pairs) => {
            println!("✓ Normal nullable type in field def parsed successfully: {:?}", pairs.as_str());
        }
        Err(e) => {
            println!("✗ Normal nullable type in field def parsing failed: {}", e);
        }
    }
}