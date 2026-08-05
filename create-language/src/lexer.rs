use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    fn new(kind: TokenKind, lexeme: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            line,
            column,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Package,
    Import,
    Fun,
    Func,
    Async,
    Await,
    Struct,
    Data,
    Class,
    Enum,
    Trait,
    Impl,
    Init,
    Val,
    Var,
    If,
    Else,
    Match,
    While,
    Until,
    Do,
    For,
    In,
    Return,
    Break,
    Continue,
    Throw,
    Try,
    Catch,
    Finally,
    True,
    False,
    Null,
    Spawn,
    Receive,
    As,

    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Char(char),

    // Identifiers
    Ident(String),

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,

    // Operators
    Arrow,
    FatArrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Question,
    DoubleBang,
    Ampersand,
    And,
    Pipe,
    Or,
    At,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    Eq,
    NotEq,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    Range,

    // Special
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TokenKind::Package => "package",
            TokenKind::Import => "import",
            TokenKind::Fun => "fun",
            TokenKind::Func => "func",
            TokenKind::Async => "async",
            TokenKind::Await => "await",
            TokenKind::Struct => "struct",
            TokenKind::Data => "data",
            TokenKind::Class => "class",
            TokenKind::Enum => "enum",
            TokenKind::Trait => "trait",
            TokenKind::Impl => "impl",
            TokenKind::Init => "init",
            TokenKind::Val => "val",
            TokenKind::Var => "var",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::Match => "match",
            TokenKind::While => "while",
            TokenKind::Until => "until",
            TokenKind::Do => "do",
            TokenKind::For => "for",
            TokenKind::In => "in",
            TokenKind::Return => "return",
            TokenKind::Break => "break",
            TokenKind::Continue => "continue",
            TokenKind::Throw => "throw",
            TokenKind::Try => "try",
            TokenKind::Catch => "catch",
            TokenKind::Finally => "finally",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Null => "null",
            TokenKind::Spawn => "spawn",
            TokenKind::Receive => "receive",
            TokenKind::As => "as",
            TokenKind::Int(_) => "integer",
            TokenKind::Float(_) => "float",
            TokenKind::String(_) => "string",
            TokenKind::Char(_) => "char",
            TokenKind::Ident(_) => "identifier",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::Comma => ",",
            TokenKind::Semicolon => ";",
            TokenKind::Colon => ":",
            TokenKind::Dot => ".",
            TokenKind::Arrow => "->",
            TokenKind::FatArrow => "=>",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::Bang => "!",
            TokenKind::Question => "?",
            TokenKind::DoubleBang => "!!",
            TokenKind::Ampersand => "&",
            TokenKind::And => "&&",
            TokenKind::Pipe => "|",
            TokenKind::Or => "||",
            TokenKind::At => "@",
            TokenKind::Less => "<",
            TokenKind::Greater => ">",
            TokenKind::LessEq => "<=",
            TokenKind::GreaterEq => ">=",
            TokenKind::Eq => "==",
            TokenKind::NotEq => "!=",
            TokenKind::Assign => "=",
            TokenKind::PlusAssign => "+=",
            TokenKind::MinusAssign => "-=",
            TokenKind::StarAssign => "*=",
            TokenKind::SlashAssign => "/=",
            TokenKind::PercentAssign => "%=",
            TokenKind::Range => "..",
            TokenKind::Eof => "EOF",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub struct LexerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}:{}", self.message, self.line, self.column)
    }
}

impl std::error::Error for LexerError {}

pub struct Lexer<'a> {
    _source: &'a str,
    chars: std::str::Chars<'a>,
    current: Option<char>,
    line: usize,
    column: usize,
    start_line: usize,
    start_column: usize,
    buffer: String,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut chars = source.chars();
        let current = chars.next();
        Self {
            _source: source,
            chars,
            current,
            line: 1,
            column: 1,
            start_line: 1,
            start_column: 1,
            buffer: String::new(),
        }
    }

    pub fn lex(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        self.skip_whitespace_and_comments()?;
        while self.current.is_some() {
            self.start_line = self.line;
            self.start_column = self.column;
            self.buffer.clear();
            let token = self.next_token()?;
            tokens.push(token);
            self.skip_whitespace_and_comments()?;
        }
        tokens.push(Token::new(TokenKind::Eof, "", self.line, self.column));
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexerError> {
        match self.current {
            Some('(') => self.single(TokenKind::LParen),
            Some(')') => self.single(TokenKind::RParen),
            Some('{') => self.single(TokenKind::LBrace),
            Some('}') => self.single(TokenKind::RBrace),
            Some('[') => self.single(TokenKind::LBracket),
            Some(']') => self.single(TokenKind::RBracket),
            Some(',') => self.single(TokenKind::Comma),
            Some(';') => self.single(TokenKind::Semicolon),
            Some(':') => self.single(TokenKind::Colon),
            Some('.') => {
                if self.peek() == Some('.') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::Range))
                } else {
                    self.single(TokenKind::Dot)
                }
            }
            Some('+') => self.two_char_assign('=', TokenKind::PlusAssign, TokenKind::Plus),
            Some('-') => {
                if self.peek() == Some('>') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::Arrow))
                } else {
                    self.two_char_assign('=', TokenKind::MinusAssign, TokenKind::Minus)
                }
            }
            Some('*') => self.two_char_assign('=', TokenKind::StarAssign, TokenKind::Star),
            Some('%') => self.two_char_assign('=', TokenKind::PercentAssign, TokenKind::Percent),
            Some('/') => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::SlashAssign))
                } else {
                    self.single(TokenKind::Slash)
                }
            }
            Some('!') => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::NotEq))
                } else if self.peek() == Some('!') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::DoubleBang))
                } else {
                    self.single(TokenKind::Bang)
                }
            }
            Some('=') => {
                if self.peek() == Some('>') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::FatArrow))
                } else if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::Eq))
                } else {
                    self.single(TokenKind::Assign)
                }
            }
            Some('<') => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::LessEq))
                } else {
                    self.single(TokenKind::Less)
                }
            }
            Some('>') => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::GreaterEq))
                } else {
                    self.single(TokenKind::Greater)
                }
            }
            Some('|') => {
                if self.peek() == Some('|') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::Or))
                } else {
                    self.single(TokenKind::Pipe)
                }
            }
            Some('&') => {
                if self.peek() == Some('&') {
                    self.advance();
                    self.advance();
                    Ok(self.token(TokenKind::And))
                } else {
                    self.single(TokenKind::Ampersand)
                }
            }
            Some('?') => self.single(TokenKind::Question),
            Some('@') => self.single(TokenKind::At),
            Some('"') => self.string(),
            Some('\'') => self.char_literal(),
            Some(c) if c.is_ascii_digit() => self.number(),
            Some(c) if is_ident_start(c) => self.identifier_or_keyword(),
            Some(c) => Err(self.error(format!("unexpected character '{}'", c))),
            None => Err(self.error("unexpected end of input".to_string())),
        }
    }

    fn single(&mut self, kind: TokenKind) -> Result<Token, LexerError> {
        self.advance();
        Ok(self.token(kind))
    }

    fn two_char_assign(
        &mut self,
        second: char,
        assign_kind: TokenKind,
        single_kind: TokenKind,
    ) -> Result<Token, LexerError> {
        if self.peek() == Some(second) {
            self.advance();
            self.advance();
            Ok(self.token(assign_kind))
        } else {
            self.single(single_kind)
        }
    }

    fn token(&self, kind: TokenKind) -> Token {
        Token::new(
            kind,
            self.buffer.clone(),
            self.start_line,
            self.start_column,
        )
    }

    fn advance(&mut self) {
        if let Some(c) = self.current {
            self.buffer.push(c);
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.current = self.chars.next();
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexerError> {
        while let Some(c) = self.current {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' && self.peek() == Some('/') {
                self.skip_line_comment();
            } else if c == '/' && self.peek() == Some('*') {
                self.skip_block_comment()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.current {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), LexerError> {
        self.advance(); // '/'
        self.advance(); // '*'
        let start_line = self.line;
        let start_column = self.column;
        let mut depth = 1;
        while depth > 0 {
            match self.current {
                Some('/') if self.peek() == Some('*') => {
                    self.advance();
                    self.advance();
                    depth += 1;
                }
                Some('*') if self.peek() == Some('/') => {
                    self.advance();
                    self.advance();
                    depth -= 1;
                }
                Some(_) => self.advance(),
                None => {
                    return Err(LexerError {
                        message: "unterminated block comment".to_string(),
                        line: start_line,
                        column: start_column,
                    })
                }
            }
        }
        Ok(())
    }

    fn string(&mut self) -> Result<Token, LexerError> {
        self.advance(); // opening quote
        let mut value = String::new();
        while let Some(c) = self.current {
            if c == '"' {
                self.advance();
                let token = self.token(TokenKind::String(value));
                return Ok(token);
            } else if c == '\\' {
                self.advance();
                match self.current {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('r') => value.push('\r'),
                    Some('\\') => value.push('\\'),
                    Some('"') => value.push('"'),
                    Some('\'') => value.push('\''),
                    Some(other) => {
                        return Err(self.error(format!("invalid escape sequence '\\{}'", other)))
                    }
                    None => return Err(self.error("unterminated string escape".to_string())),
                }
                self.advance();
            } else if c == '\n' {
                return Err(self.error("unterminated string literal".to_string()));
            } else {
                value.push(c);
                self.advance();
            }
        }
        Err(self.error("unterminated string literal".to_string()))
    }

    fn char_literal(&mut self) -> Result<Token, LexerError> {
        self.advance(); // opening quote
        let value = match self.current {
            Some('\\') => {
                self.advance();
                let c = match self.current {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('\\') => '\\',
                    Some('"') => '"',
                    Some('\'') => '\'',
                    Some(other) => {
                        return Err(self.error(format!("invalid escape sequence '\\{}'", other)))
                    }
                    None => return Err(self.error("unterminated char escape".to_string())),
                };
                self.advance();
                c
            }
            Some(c) => {
                self.advance();
                c
            }
            None => return Err(self.error("unterminated char literal".to_string())),
        };
        if self.current != Some('\'') {
            return Err(self.error("unterminated char literal".to_string()));
        }
        self.advance();
        Ok(self.token(TokenKind::Char(value)))
    }

    fn number(&mut self) -> Result<Token, LexerError> {
        if self.current == Some('0') {
            let next = self.peek();
            if next == Some('x') || next == Some('X') {
                self.advance();
                self.advance();
                return self.hex_number();
            } else if next == Some('b') || next == Some('B') {
                self.advance();
                self.advance();
                return self.binary_number();
            }
        }
        self.decimal_number()
    }

    fn decimal_number(&mut self) -> Result<Token, LexerError> {
        while let Some(c) = self.current {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        if self.current == Some('.') && self.peek().map_or(false, |c| c.is_ascii_digit()) {
            self.advance();
            while let Some(c) = self.current {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.current == Some('e') || self.current == Some('E') {
                self.advance();
                if self.current == Some('+') || self.current == Some('-') {
                    self.advance();
                }
                if !self.current.map_or(false, |c| c.is_ascii_digit()) {
                    return Err(self.error("expected exponent digits".to_string()));
                }
                while let Some(c) = self.current {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            let value: f64 = self.buffer.parse().unwrap();
            Ok(self.token(TokenKind::Float(value)))
        } else {
            let value: i64 = self.buffer.parse().map_err(|_| {
                self.error(format!("integer literal '{}' out of range", self.buffer))
            })?;
            Ok(self.token(TokenKind::Int(value)))
        }
    }

    fn hex_number(&mut self) -> Result<Token, LexerError> {
        let mut value: i64 = 0;
        let mut has_digit = false;
        while let Some(c) = self.current {
            if let Some(d) = c.to_digit(16) {
                value = value * 16 + d as i64;
                has_digit = true;
                self.advance();
            } else {
                break;
            }
        }
        if !has_digit {
            return Err(self.error("expected hex digits".to_string()));
        }
        Ok(self.token(TokenKind::Int(value)))
    }

    fn binary_number(&mut self) -> Result<Token, LexerError> {
        let mut value: i64 = 0;
        let mut has_digit = false;
        while let Some(c) = self.current {
            if let Some(d) = c.to_digit(2) {
                value = value * 2 + d as i64;
                has_digit = true;
                self.advance();
            } else {
                break;
            }
        }
        if !has_digit {
            return Err(self.error("expected binary digits".to_string()));
        }
        Ok(self.token(TokenKind::Int(value)))
    }

    fn identifier_or_keyword(&mut self) -> Result<Token, LexerError> {
        while let Some(c) = self.current {
            if is_ident_continue(c) {
                self.advance();
            } else {
                break;
            }
        }
        let word = self.buffer.clone();
        let kind = match word.as_str() {
            "package" => TokenKind::Package,
            "import" => TokenKind::Import,
            "fun" => TokenKind::Fun,
            "func" => TokenKind::Func,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "struct" => TokenKind::Struct,
            "data" => TokenKind::Data,
            "class" => TokenKind::Class,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "init" => TokenKind::Init,
            "val" => TokenKind::Val,
            "var" => TokenKind::Var,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "while" => TokenKind::While,
            "until" => TokenKind::Until,
            "do" => TokenKind::Do,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "throw" => TokenKind::Throw,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "spawn" => TokenKind::Spawn,
            "receive" => TokenKind::Receive,
            "as" => TokenKind::As,
            _ => TokenKind::Ident(word),
        };
        Ok(self.token(kind))
    }

    fn error(&self, message: String) -> LexerError {
        LexerError {
            message,
            line: self.line,
            column: self.column,
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_keywords_and_symbols() {
        let source = "fun add(a: int, b: int): int { return a + b; }";
        let tokens = Lexer::new(source).lex().unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(matches!(kinds[0], TokenKind::Fun));
        assert!(matches!(kinds[1], TokenKind::Ident(_)));
        assert!(matches!(kinds[2], TokenKind::LParen));
    }

    #[test]
    fn lex_string_and_comments() {
        let source = r#"// line comment
        "hello" /* block */"#;
        let tokens = Lexer::new(source).lex().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::String(_)));
    }

    #[test]
    fn lex_numbers() {
        let source = "42 3.14 0xFF 0b1010";
        let tokens = Lexer::new(source).lex().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Int(42));
        assert!(matches!(tokens[1].kind, TokenKind::Float(_)));
        assert_eq!(tokens[2].kind, TokenKind::Int(255));
        assert_eq!(tokens[3].kind, TokenKind::Int(10));
    }
}
