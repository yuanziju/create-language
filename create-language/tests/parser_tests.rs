use create_language::lexer::Lexer;
use create_language::parser::Parser;

fn parse(
    source: &str,
) -> Result<create_language::ast::Program, create_language::parser::ParserError> {
    let tokens = Lexer::new(source).lex().unwrap();
    Parser::new(tokens).parse()
}

#[test]
fn parse_hello_function() {
    let source = r#"
        fun main(): int {
            return 0;
        }
    "#;
    let program = parse(source).unwrap();
    assert_eq!(program.items.len(), 1);
}

#[test]
fn parse_variable_and_if() {
    let source = r#"
        fun max(a: int, b: int): int {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    "#;
    let program = parse(source).unwrap();
    assert_eq!(program.items.len(), 1);
}

#[test]
fn parse_lambda() {
    let source = r#"
        fun apply(x: int, f: func(int): int): int {
            return f(x);
        }
    "#;
    let program = parse(source).unwrap();
    assert_eq!(program.items.len(), 1);
}
