use pest::{Parser};
use ridl_pest_test::{RIDLParser, Rule};

fn main() {
    // 测试 callback 作为类型
    let input = r#"fn setTimeout(cb: callback, delay: int);"#;
    
    println!("Testing: {}", input);
    
    match RIDLParser::parse(Rule::global_function, input) {
        Ok(pairs) => {
            println!("Parse successful:");
            for pair in pairs {
                println!("  Rule: {:?}, Span: {:?}", pair.as_rule(), pair.as_span());
                for inner_pair in pair.into_inner() {
                    println!("    Inner Rule: {:?}, Span: {:?}", inner_pair.as_rule(), inner_pair.as_span());
                    for inner_inner in inner_pair.into_inner() {
                        println!("      Inner Inner Rule: {:?}, Span: {:?}", inner_inner.as_rule(), inner_inner.as_span());
                    }
                }
            }
        }
        Err(e) => {
            println!("Parse failed: {}", e);
        }
    }
}