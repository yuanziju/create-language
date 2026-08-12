use crate::ast::*;
use crate::token::{Token, TokenKind};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ParserError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}:{}", self.message, self.line, self.column)
    }
}

impl std::error::Error for ParserError {}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut package = None;
        let mut imports = Vec::new();
        let mut items = Vec::new();

        if self.check(&TokenKind::Package) {
            package = Some(self.parse_package_decl()?);
        }

        while self.check(&TokenKind::Import) {
            imports.push(self.parse_import_stmt()?);
        }

        while !self.is_at_end() {
            items.push(self.parse_top_level()?);
        }

        Ok(Program {
            package,
            imports,
            items,
        })
    }

    fn parse_package_decl(&mut self) -> Result<PackageDecl, ParserError> {
        self.expect(TokenKind::Package)?;
        let path = self.parse_module_path()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(PackageDecl { path })
    }

    fn parse_import_stmt(&mut self) -> Result<ImportStmt, ParserError> {
        self.expect(TokenKind::Import)?;
        let path = self.expect_string()?;
        let alias = if self.match_token(&TokenKind::As) {
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(ImportStmt { path, alias })
    }

    fn parse_module_path(&mut self) -> Result<Vec<Identifier>, ParserError> {
        let mut path = vec![self.expect_ident()?];
        while self.match_token(&TokenKind::Dot) {
            path.push(self.expect_ident()?);
        }
        Ok(path)
    }

    pub fn match_literal(&mut self) -> Option<Literal> {
        match self.peek_kind() {
            Some(TokenKind::Int(v)) => {
                let val = v;
                self.advance();
                Some(Literal::Int(val))
            }
            Some(TokenKind::Float(v)) => {
                let val = v;
                self.advance();
                Some(Literal::Float(val))
            }
            Some(TokenKind::String(ref s)) => {
                let s = s.clone();
                self.advance();
                Some(Literal::String(s))
            }
            Some(TokenKind::Char(c)) => {
                let ch = c;
                self.advance();
                Some(Literal::Char(ch))
            }
            Some(TokenKind::True) => {
                self.advance();
                Some(Literal::Bool(true))
            }
            Some(TokenKind::False) => {
                self.advance();
                Some(Literal::Bool(false))
            }
            Some(TokenKind::Null) => {
                self.advance();
                Some(Literal::Null)
            }
            _ => None,
        }
    }

    pub fn match_ident(&mut self) -> Option<Identifier> {
        if let Some(TokenKind::Ident(name)) = self.peek_kind() {
            let name = name.clone();
            self.advance();
            Some(name)
        } else {
            None
        }
    }

    pub fn expect_ident(&mut self) -> Result<Identifier, ParserError> {
        self.match_ident()
            .ok_or_else(|| self.error("expected identifier".to_string()))
    }

    pub fn expect_string(&mut self) -> Result<String, ParserError> {
        if let Some(TokenKind::String(s)) = self.peek_kind() {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(self.error("expected string literal".to_string()))
        }
    }

    pub fn expect(&mut self, kind: TokenKind) -> Result<(), ParserError> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!(
                "expected '{}', found '{}'",
                kind,
                self.peek_kind()
                    .map_or("EOF".to_string(), |k| k.to_string())
            )))
        }
    }

    pub fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind().is_some_and(|k| k == *kind)
    }

    pub fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.tokens[self.pos].kind, TokenKind::Eof)
    }

    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.pos).map(|t| t.kind.clone())
    }

    pub fn peek_ahead_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens.get(self.pos + offset).map(|t| t.kind.clone())
    }

    pub fn match_assignment_op(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Assign)
                | Some(TokenKind::PlusAssign)
                | Some(TokenKind::MinusAssign)
                | Some(TokenKind::StarAssign)
                | Some(TokenKind::SlashAssign)
                | Some(TokenKind::PercentAssign)
        )
    }

    pub fn parse_assign_op(&mut self) -> Result<AssignOp, ParserError> {
        let op = match self.peek_kind() {
            Some(TokenKind::Assign) => AssignOp::Assign,
            Some(TokenKind::PlusAssign) => AssignOp::AddAssign,
            Some(TokenKind::MinusAssign) => AssignOp::SubAssign,
            Some(TokenKind::StarAssign) => AssignOp::MulAssign,
            Some(TokenKind::SlashAssign) => AssignOp::DivAssign,
            Some(TokenKind::PercentAssign) => AssignOp::ModAssign,
            _ => return Err(self.error("expected assignment operator".to_string())),
        };
        self.advance();
        Ok(op)
    }

    pub fn error(&self, message: String) -> ParserError {
        let token = self.tokens.get(self.pos).cloned().unwrap_or_else(|| Token {
            kind: TokenKind::Eof,
            lexeme: "".to_string(),
            line: 0,
            column: 0,
        });
        ParserError {
            message,
            line: token.line,
            column: token.column,
        }
    }
}

mod decl;
mod expr;
mod stmt;
