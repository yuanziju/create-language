use crate::ast::*;
use crate::lexer::{Token, TokenKind};
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

    fn parse_top_level(&mut self) -> Result<TopLevel, ParserError> {
        match self.peek_kind() {
            Some(TokenKind::Fun) | Some(TokenKind::Async) => {
                Ok(TopLevel::Function(self.parse_function_decl()?))
            }
            Some(TokenKind::Struct) => Ok(TopLevel::Struct(self.parse_struct_decl()?)),
            Some(TokenKind::Data) => Ok(TopLevel::DataClass(self.parse_data_class_decl()?)),
            Some(TokenKind::Class) => Ok(TopLevel::Class(self.parse_class_decl()?)),
            Some(TokenKind::Enum) => Ok(TopLevel::Enum(self.parse_enum_decl()?)),
            Some(TokenKind::Trait) => Ok(TopLevel::Trait(self.parse_trait_decl()?)),
            Some(TokenKind::Impl) => Ok(TopLevel::Impl(self.parse_impl_decl()?)),
            _ => Ok(TopLevel::Stmt(self.parse_stmt()?)),
        }
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParserError> {
        let is_async = self.match_token(&TokenKind::Async);
        self.expect(TokenKind::Fun)?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generic_params()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_optional_params()?;
        self.expect(TokenKind::RParen)?;
        let return_type = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(FunctionDecl {
            is_async,
            name,
            generics,
            params,
            return_type,
            body,
        })
    }

    fn parse_optional_generic_params(&mut self) -> Result<Vec<GenericParam>, ParserError> {
        if self.check(&TokenKind::Less) {
            self.parse_generic_params()
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, ParserError> {
        self.expect(TokenKind::Less)?;
        let mut params = Vec::new();
        params.push(self.parse_generic_param()?);
        while self.match_token(&TokenKind::Comma) {
            params.push(self.parse_generic_param()?);
        }
        self.expect(TokenKind::Greater)?;
        Ok(params)
    }

    fn parse_generic_param(&mut self) -> Result<GenericParam, ParserError> {
        let name = self.expect_ident()?;
        let bound = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        Ok(GenericParam { name, bound })
    }

    fn parse_optional_params(&mut self) -> Result<Vec<Param>, ParserError> {
        if self.check(&TokenKind::RParen) {
            return Ok(Vec::new());
        }
        let mut params = Vec::new();
        params.push(self.parse_param()?);
        while self.match_token(&TokenKind::Comma) {
            params.push(self.parse_param()?);
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, ParserError> {
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let default = if self.match_token(&TokenKind::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Param { name, ty, default })
    }

    fn parse_struct_decl(&mut self) -> Result<StructDecl, ParserError> {
        self.expect(TokenKind::Struct)?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generic_params()?;
        self.expect(TokenKind::LBrace)?;
        let fields = self.parse_optional_fields()?;
        self.expect(TokenKind::RBrace)?;
        Ok(StructDecl {
            name,
            generics,
            fields,
        })
    }

    fn parse_optional_fields(&mut self) -> Result<Vec<Field>, ParserError> {
        if self.check(&TokenKind::RBrace) {
            return Ok(Vec::new());
        }
        let mut fields = Vec::new();
        fields.push(self.parse_field()?);
        while self.match_token(&TokenKind::Comma) {
            if self.check(&TokenKind::RBrace) {
                break;
            }
            fields.push(self.parse_field()?);
        }
        Ok(fields)
    }

    fn parse_field(&mut self) -> Result<Field, ParserError> {
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(Field { name, ty })
    }

    fn parse_data_class_decl(&mut self) -> Result<DataClassDecl, ParserError> {
        self.expect(TokenKind::Data)?;
        self.expect(TokenKind::Class)?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generic_params()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_optional_constructor_params()?;
        self.expect(TokenKind::RParen)?;
        Ok(DataClassDecl {
            name,
            generics,
            params,
        })
    }

    fn parse_optional_constructor_params(&mut self) -> Result<Vec<ConstructorParam>, ParserError> {
        if self.check(&TokenKind::RParen) {
            return Ok(Vec::new());
        }
        let mut params = Vec::new();
        params.push(self.parse_constructor_param()?);
        while self.match_token(&TokenKind::Comma) {
            params.push(self.parse_constructor_param()?);
        }
        Ok(params)
    }

    fn parse_constructor_param(&mut self) -> Result<ConstructorParam, ParserError> {
        let is_val = if self.match_token(&TokenKind::Val) {
            true
        } else if self.match_token(&TokenKind::Var) {
            false
        } else {
            return Err(self.error("expected 'val' or 'var' in data class parameter".to_string()));
        };
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let default = if self.match_token(&TokenKind::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(ConstructorParam {
            is_val,
            name,
            ty,
            default,
        })
    }

    fn parse_class_decl(&mut self) -> Result<ClassDecl, ParserError> {
        self.expect(TokenKind::Class)?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generic_params()?;
        let supers = if self.match_token(&TokenKind::Colon) {
            self.parse_type_list()?
        } else {
            Vec::new()
        };
        self.expect(TokenKind::LBrace)?;
        let mut members = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            members.push(self.parse_class_member()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ClassDecl {
            name,
            generics,
            supers,
            members,
        })
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, ParserError> {
        if self.check(&TokenKind::Val) || self.check(&TokenKind::Var) {
            let decl = self.parse_field_decl()?;
            self.expect(TokenKind::Semicolon)?;
            Ok(ClassMember::Field(decl))
        } else if self.check(&TokenKind::Init) {
            Ok(ClassMember::Constructor(self.parse_constructor_decl()?))
        } else {
            Ok(ClassMember::Function(self.parse_function_decl()?))
        }
    }

    fn parse_field_decl(&mut self) -> Result<FieldDecl, ParserError> {
        let is_val = self.match_token(&TokenKind::Val);
        if !is_val {
            self.expect(TokenKind::Var)?;
        }
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let init = if self.match_token(&TokenKind::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(FieldDecl {
            is_val,
            name,
            ty,
            init,
        })
    }

    fn parse_constructor_decl(&mut self) -> Result<ConstructorDecl, ParserError> {
        self.expect(TokenKind::Init)?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_optional_params()?;
        self.expect(TokenKind::RParen)?;
        let body = self.parse_block()?;
        Ok(ConstructorDecl { params, body })
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, ParserError> {
        self.expect(TokenKind::Enum)?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generic_params()?;
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        variants.push(self.parse_enum_variant()?);
        while self.match_token(&TokenKind::Comma) {
            if self.check(&TokenKind::RBrace) {
                break;
            }
            variants.push(self.parse_enum_variant()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(EnumDecl {
            name,
            generics,
            variants,
        })
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariant, ParserError> {
        let name = self.expect_ident()?;
        let types = if self.check(&TokenKind::LParen) {
            self.expect(TokenKind::LParen)?;
            let types = self.parse_optional_types()?;
            self.expect(TokenKind::RParen)?;
            types
        } else {
            Vec::new()
        };
        Ok(EnumVariant { name, types })
    }

    fn parse_optional_types(&mut self) -> Result<Vec<Type>, ParserError> {
        if self.check(&TokenKind::RParen) {
            return Ok(Vec::new());
        }
        let mut types = Vec::new();
        types.push(self.parse_type()?);
        while self.match_token(&TokenKind::Comma) {
            types.push(self.parse_type()?);
        }
        Ok(types)
    }

    fn parse_trait_decl(&mut self) -> Result<TraitDecl, ParserError> {
        self.expect(TokenKind::Trait)?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generic_params()?;
        self.expect(TokenKind::LBrace)?;
        let mut members = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            members.push(self.parse_trait_member()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(TraitDecl {
            name,
            generics,
            members,
        })
    }

    fn parse_trait_member(&mut self) -> Result<TraitMember, ParserError> {
        self.expect(TokenKind::Fun)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_optional_params()?;
        self.expect(TokenKind::RParen)?;
        let return_type = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        if self.match_token(&TokenKind::Semicolon) {
            Ok(TraitMember::Signature(FunctionSignature {
                name,
                params,
                return_type,
            }))
        } else {
            let body = self.parse_block()?;
            Ok(TraitMember::Function(FunctionDecl {
                is_async: false,
                name,
                generics: Vec::new(),
                params,
                return_type,
                body,
            }))
        }
    }

    fn parse_impl_decl(&mut self) -> Result<ImplDecl, ParserError> {
        self.expect(TokenKind::Impl)?;
        let first = self.parse_type()?;
        let (trait_ty, ty) = if self.match_token(&TokenKind::For) {
            (Some(first), self.parse_type()?)
        } else {
            (None, first)
        };
        self.expect(TokenKind::LBrace)?;
        let mut functions = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            functions.push(self.parse_function_decl()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ImplDecl {
            trait_ty,
            ty,
            functions,
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParserError> {
        match self.peek_kind() {
            Some(TokenKind::LBrace) => Ok(Stmt::Block(self.parse_block()?)),
            Some(TokenKind::Val) | Some(TokenKind::Var) => {
                let decl = self.parse_var_decl()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::VarDecl(decl))
            }
            Some(TokenKind::If) => Ok(Stmt::If(self.parse_if_stmt()?)),
            Some(TokenKind::Match) => Ok(Stmt::Match(self.parse_match_stmt()?)),
            Some(TokenKind::While) => Ok(Stmt::While(self.parse_while_stmt()?)),
            Some(TokenKind::Do) => Ok(Stmt::DoWhile(self.parse_do_while_stmt()?)),
            Some(TokenKind::For) => self.parse_for_stmt(),
            Some(TokenKind::Return) => {
                self.advance();
                let expr = if self.check(&TokenKind::Semicolon) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Return(expr))
            }
            Some(TokenKind::Break) => {
                self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Break)
            }
            Some(TokenKind::Continue) => {
                self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Continue)
            }
            Some(TokenKind::Throw) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Throw(expr))
            }
            Some(TokenKind::Try) => Ok(Stmt::Try(self.parse_try_stmt()?)),
            _ => {
                let expr = self.parse_expr()?;
                if self.match_assignment_op() {
                    let op = self.parse_assign_op()?;
                    let value = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::Assign(Assign { target: expr, op, value }))
                } else {
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, ParserError> {
        let is_val = self.match_token(&TokenKind::Val);
        if !is_val {
            self.expect(TokenKind::Var)?;
        }
        let name = self.expect_ident()?;
        let ty = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.match_token(&TokenKind::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(VarDecl {
            is_val,
            name,
            ty,
            init,
        })
    }

    fn parse_if_stmt(&mut self) -> Result<IfStmt, ParserError> {
        self.expect(TokenKind::If)?;
        let cond = Box::new(self.parse_parenthesized_expr()?);
        let then_branch = self.parse_block()?;
        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                Some(Box::new(ElseBranch::If(self.parse_if_stmt()?)))
            } else {
                Some(Box::new(ElseBranch::Block(self.parse_block()?)))
            }
        } else {
            None
        };
        Ok(IfStmt {
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_match_stmt(&mut self) -> Result<MatchStmt, ParserError> {
        self.expect(TokenKind::Match)?;
        let expr = Box::new(self.parse_parenthesized_expr()?);
        self.expect(TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(MatchStmt { expr, arms })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParserError> {
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::FatArrow)?;
        let body = if self.check(&TokenKind::LBrace) {
            Expr::Block(self.parse_block()?)
        } else {
            self.parse_expr()?
        };
        Ok(MatchArm { pattern, body })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParserError> {
        if self.match_ident() == Some("_".to_string()) {
            return Ok(Pattern::Wildcard);
        }
        if let Some(lit) = self.match_literal() {
            return Ok(Pattern::Literal(lit));
        }
        let name = self.expect_ident()?;
        if self.match_token(&TokenKind::At) {
            let inner = self.parse_pattern()?;
            return Ok(Pattern::At(name, Box::new(inner)));
        }
        if self.check(&TokenKind::LParen) {
            self.expect(TokenKind::LParen)?;
            let mut patterns = Vec::new();
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                patterns.push(self.parse_pattern()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
            return Ok(Pattern::Constructor(name, patterns));
        }
        Ok(Pattern::Binding(name))
    }

    fn parse_while_stmt(&mut self) -> Result<WhileStmt, ParserError> {
        self.expect(TokenKind::While)?;
        let until = self.match_token(&TokenKind::Until);
        let cond = if self.check(&TokenKind::LParen) {
            self.parse_parenthesized_expr()?
        } else {
            self.parse_expr()?
        };
        let cond = if until {
            Expr::Unary(UnaryExpr {
                op: UnaryOp::Not,
                expr: Box::new(cond),
            })
        } else {
            cond
        };
        let body = self.parse_block()?;
        Ok(WhileStmt { cond, body })
    }

    fn parse_do_while_stmt(&mut self) -> Result<DoWhileStmt, ParserError> {
        self.expect(TokenKind::Do)?;
        let body = self.parse_block()?;
        self.expect(TokenKind::While)?;
        let cond = self.parse_parenthesized_expr()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(DoWhileStmt { body, cond })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParserError> {
        self.expect(TokenKind::For)?;
        if self.check(&TokenKind::LParen) {
            self.expect(TokenKind::LParen)?;
            let init = if self.check(&TokenKind::Semicolon) {
                None
            } else {
                Some(Box::new(self.parse_for_init()?))
            };
            self.expect(TokenKind::Semicolon)?;
            let cond = if self.check(&TokenKind::Semicolon) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect(TokenKind::Semicolon)?;
            let step = if self.check(&TokenKind::RParen) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect(TokenKind::RParen)?;
            let body = self.parse_block()?;
            return Ok(Stmt::For(ForStmt {
                init,
                cond,
                step,
                body,
            }));
        }

        let name = self.expect_ident()?;
        self.expect(TokenKind::In)?;
        let expr = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::ForIn(ForInStmt { name, expr, body }))
    }

    fn parse_for_init(&mut self) -> Result<Stmt, ParserError> {
        if self.check(&TokenKind::Val) || self.check(&TokenKind::Var) {
            Ok(Stmt::VarDecl(self.parse_var_decl()?))
        } else {
            Ok(Stmt::Expr(self.parse_expr()?))
        }
    }

    fn parse_try_stmt(&mut self) -> Result<TryStmt, ParserError> {
        self.expect(TokenKind::Try)?;
        let body = self.parse_block()?;
        let mut catches = Vec::new();
        while self.check(&TokenKind::Catch) {
            catches.push(self.parse_catch_clause()?);
        }
        let finally = if self.match_token(&TokenKind::Finally) {
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(TryStmt {
            body,
            catches,
            finally,
        })
    }

    fn parse_catch_clause(&mut self) -> Result<CatchClause, ParserError> {
        self.expect(TokenKind::Catch)?;
        self.expect(TokenKind::LParen)?;
        let name = if self.peek_ahead_kind(1) == Some(TokenKind::Colon) {
            let n = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            Some(n)
        } else {
            None
        };
        let ty = self.parse_type()?;
        self.expect(TokenKind::RParen)?;
        let body = self.parse_block()?;
        Ok(CatchClause { name, ty, body })
    }

    fn parse_block(&mut self) -> Result<Block, ParserError> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_expr(&mut self) -> Result<Expr, ParserError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_and_expr()?;
        while self.match_token(&TokenKind::Or) {
            let right = self.parse_and_expr()?;
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_equality_expr()?;
        while self.match_token(&TokenKind::And) {
            let right = self.parse_equality_expr()?;
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_equality_expr(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_relational_expr()?;
        loop {
            let op = if self.match_token(&TokenKind::Eq) {
                Some(BinaryOp::Eq)
            } else if self.match_token(&TokenKind::NotEq) {
                Some(BinaryOp::NotEq)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_relational_expr()?;
                left = Expr::Binary(BinaryExpr {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_relational_expr(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_range_expr()?;
        loop {
            let op = if self.match_token(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.match_token(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.match_token(&TokenKind::LessEq) {
                Some(BinaryOp::LessEq)
            } else if self.match_token(&TokenKind::GreaterEq) {
                Some(BinaryOp::GreaterEq)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_range_expr()?;
                left = Expr::Binary(BinaryExpr {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_range_expr(&mut self) -> Result<Expr, ParserError> {
        let left = self.parse_additive_expr()?;
        if self.match_token(&TokenKind::Range) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::Range,
                right: Box::new(right),
            }))
        } else {
            Ok(left)
        }
    }

    fn parse_additive_expr(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_multiplicative_expr()?;
        loop {
            let op = if self.match_token(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.match_token(&TokenKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_multiplicative_expr()?;
                left = Expr::Binary(BinaryExpr {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplicative_expr(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_unary_expr()?;
        loop {
            let op = if self.match_token(&TokenKind::Star) {
                Some(BinaryOp::Mul)
            } else if self.match_token(&TokenKind::Slash) {
                Some(BinaryOp::Div)
            } else if self.match_token(&TokenKind::Percent) {
                Some(BinaryOp::Mod)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_unary_expr()?;
                left = Expr::Binary(BinaryExpr {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, ParserError> {
        let op = if self.match_token(&TokenKind::Bang) {
            Some(UnaryOp::Not)
        } else if self.match_token(&TokenKind::Minus) {
            Some(UnaryOp::Neg)
        } else if self.match_token(&TokenKind::Plus) {
            Some(UnaryOp::Plus)
        } else if self.match_token(&TokenKind::Ampersand) {
            Some(UnaryOp::Ref)
        } else if self.match_token(&TokenKind::Star) {
            Some(UnaryOp::Deref)
        } else {
            None
        };
        if let Some(op) = op {
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::Unary(UnaryExpr {
                op,
                expr: Box::new(expr),
            }));
        }
        self.parse_await_expr()
    }

    fn parse_await_expr(&mut self) -> Result<Expr, ParserError> {
        if self.match_token(&TokenKind::Await) {
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::Await(Box::new(expr)));
        }
        self.parse_postfix_expr()
    }

    fn parse_postfix_expr(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_primary_expr()?;
        loop {
            if self.match_token(&TokenKind::LParen) {
                let args = self.parse_optional_args()?;
                self.expect(TokenKind::RParen)?;
                expr = Expr::Call(CallExpr {
                    callee: Box::new(expr),
                    args,
                });
            } else if self.match_token(&TokenKind::Dot) {
                let field = self.expect_ident()?;
                expr = Expr::FieldAccess(FieldAccessExpr {
                    object: Box::new(expr),
                    field,
                });
            } else if self.match_token(&TokenKind::Question) {
                expr = Expr::Nullable(Box::new(expr));
            } else if self.match_token(&TokenKind::DoubleBang) {
                expr = Expr::NonNull(Box::new(expr));
            } else if self.match_token(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                expr = Expr::Index(IndexExpr {
                    object: Box::new(expr),
                    index: Box::new(index),
                });
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_optional_args(&mut self) -> Result<Vec<Expr>, ParserError> {
        if self.check(&TokenKind::RParen) {
            return Ok(Vec::new());
        }
        let mut args = Vec::new();
        args.push(self.parse_expr()?);
        while self.match_token(&TokenKind::Comma) {
            args.push(self.parse_expr()?);
        }
        Ok(args)
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, ParserError> {
        if self.match_token(&TokenKind::Spawn) {
            let expr = self.parse_expr()?;
            return Ok(Expr::Spawn(Box::new(expr)));
        }
        if self.match_token(&TokenKind::Receive) {
            let expr = if self.check(&TokenKind::LParen) {
                self.expect(TokenKind::LParen)?;
                let e = if self.check(&TokenKind::RParen) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                self.expect(TokenKind::RParen)?;
                e
            } else {
                None
            };
            return Ok(Expr::Receive(expr));
        }
        if self.check(&TokenKind::LParen) {
            if self.looks_like_lambda() {
                return self.parse_lambda_expr();
            }
            self.expect(TokenKind::LParen)?;
            let expr = self.parse_expr()?;
            self.expect(TokenKind::RParen)?;
            return Ok(Expr::Grouping(Box::new(expr)));
        }
        if let Some(lit) = self.match_literal() {
            return Ok(Expr::Literal(lit));
        }
        if let Some(name) = self.match_ident() {
            if self.check(&TokenKind::LBrace) {
                return self.parse_struct_or_data_literal(name);
            }
            return Ok(Expr::Identifier(name));
        }
        if self.check(&TokenKind::LBrace) {
            if self.looks_like_lambda_brace() {
                return self.parse_lambda_expr();
            }
            return Ok(Expr::Block(self.parse_block()?));
        }
        if self.check(&TokenKind::If) {
            return Ok(Expr::If(Box::new(self.parse_if_stmt()?)));
        }
        if self.check(&TokenKind::Match) {
            return Ok(Expr::Match(Box::new(self.parse_match_stmt()?)));
        }
        if self.check(&TokenKind::LBracket) {
            self.expect(TokenKind::LBracket)?;
            let mut items = Vec::new();
            while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                items.push(self.parse_expr()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBracket)?;
            return Ok(Expr::ArrayLiteral(items));
        }
        Err(self.error(format!(
            "unexpected token '{}'",
            self.peek_kind().map_or("EOF".to_string(), |k| k.to_string())
        )))
    }

    fn parse_struct_or_data_literal(&mut self, name: Identifier) -> Result<Expr, ParserError> {
        let generic_args = self.parse_optional_generic_args()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let field_name = self.expect_ident()?;
            let value = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            fields.push(FieldInit {
                name: field_name,
                value,
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::StructLiteral(StructLiteralExpr {
            name,
            generic_args,
            fields,
        }))
    }

    fn parse_lambda_expr(&mut self) -> Result<Expr, ParserError> {
        let (params, return_type) = if self.check(&TokenKind::LParen) {
            self.expect(TokenKind::LParen)?;
            let params = self.parse_optional_lambda_params()?;
            self.expect(TokenKind::RParen)?;
            let return_type = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(TokenKind::Arrow)?;
            (params, return_type)
        } else {
            self.expect(TokenKind::LBrace)?;
            let params = self.parse_optional_lambda_params()?;
            self.expect(TokenKind::Arrow)?;
            let return_type = None;
            (params, return_type)
        };
        let body = if self.check(&TokenKind::LBrace) {
            Expr::Block(self.parse_block()?)
        } else {
            self.parse_expr()?
        };
        Ok(Expr::Lambda(LambdaExpr {
            params,
            return_type,
            body: Box::new(body),
        }))
    }

    fn parse_optional_lambda_params(&mut self) -> Result<Vec<LambdaParam>, ParserError> {
        let end_tokens = [
            TokenKind::RParen,
            TokenKind::Arrow,
            TokenKind::Colon,
        ];
        if end_tokens.iter().any(|t| self.check(t)) {
            return Ok(Vec::new());
        }
        let mut params = Vec::new();
        params.push(self.parse_lambda_param()?);
        while self.match_token(&TokenKind::Comma) {
            params.push(self.parse_lambda_param()?);
        }
        Ok(params)
    }

    fn parse_lambda_param(&mut self) -> Result<LambdaParam, ParserError> {
        let name = self.expect_ident()?;
        let ty = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        Ok(LambdaParam { name, ty })
    }

    fn looks_like_lambda(&mut self) -> bool {
        let mut depth = 0;
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        if i + 1 < self.tokens.len() {
                            match &self.tokens[i + 1].kind {
                                TokenKind::Arrow | TokenKind::Colon => return true,
                                _ => return false,
                            }
                        }
                        return false;
                    }
                }
                TokenKind::Comma | TokenKind::Ident(_) | TokenKind::Colon => {}
                _ => return false,
            }
            i += 1;
            if depth == 0 {
                return false;
            }
        }
        false
    }

    fn looks_like_lambda_brace(&mut self) -> bool {
        let mut i = self.pos + 1;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::Arrow => return true,
                TokenKind::RBrace | TokenKind::LBrace => return false,
                TokenKind::Comma | TokenKind::Ident(_) | TokenKind::Colon => {}
                _ => return false,
            }
            i += 1;
        }
        false
    }

    fn parse_parenthesized_expr(&mut self) -> Result<Expr, ParserError> {
        self.expect(TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(expr)
    }

    fn parse_type(&mut self) -> Result<Type, ParserError> {
        let mut types = vec![self.parse_nullable_type()?];
        while self.match_token(&TokenKind::Pipe) {
            types.push(self.parse_nullable_type()?);
        }
        if types.len() == 1 {
            Ok(types.into_iter().next().unwrap())
        } else {
            Ok(Type::Union(types))
        }
    }

    fn parse_nullable_type(&mut self) -> Result<Type, ParserError> {
        let ty = self.parse_primary_type()?;
        if self.match_token(&TokenKind::Question) {
            Ok(Type::Nullable(Box::new(ty)))
        } else {
            Ok(ty)
        }
    }

    fn parse_primary_type(&mut self) -> Result<Type, ParserError> {
        if self.match_token(&TokenKind::LParen) {
            let mut types = Vec::new();
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                types.push(self.parse_type()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
            return Ok(Type::Tuple(types));
        }
        if self.match_token(&TokenKind::LBracket) {
            let ty = self.parse_type()?;
            self.expect(TokenKind::RBracket)?;
            return Ok(Type::Array(Box::new(ty)));
        }
        if self.match_token(&TokenKind::Func) {
            self.expect(TokenKind::LParen)?;
            let params = self.parse_optional_types()?;
            self.expect(TokenKind::RParen)?;
            let return_type = if self.match_token(&TokenKind::Colon) {
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };
            return Ok(Type::Func(params, return_type));
        }
        let name = self.expect_ident()?;
        let generic_args = self.parse_optional_generic_args()?;
        match name.as_str() {
            "Result" if generic_args.len() == 2 => {
                let args = generic_args;
                Ok(Type::Result(Box::new(args[0].clone()), Box::new(args[1].clone())))
            }
            "Option" if generic_args.len() == 1 => {
                Ok(Type::Option(Box::new(generic_args[0].clone())))
            }
            _ => Ok(Type::Named(name, generic_args)),
        }
    }

    fn parse_optional_generic_args(&mut self) -> Result<Vec<Type>, ParserError> {
        if self.check(&TokenKind::Less) {
            self.parse_generic_args()
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_generic_args(&mut self) -> Result<Vec<Type>, ParserError> {
        self.expect(TokenKind::Less)?;
        let mut args = Vec::new();
        args.push(self.parse_type()?);
        while self.match_token(&TokenKind::Comma) {
            args.push(self.parse_type()?);
        }
        self.expect(TokenKind::Greater)?;
        Ok(args)
    }

    fn parse_type_list(&mut self) -> Result<Vec<Type>, ParserError> {
        let mut types = Vec::new();
        types.push(self.parse_type()?);
        while self.match_token(&TokenKind::Comma) {
            types.push(self.parse_type()?);
        }
        Ok(types)
    }

    fn match_literal(&mut self) -> Option<Literal> {
        match self.peek_kind() {
            Some(TokenKind::Int(v)) => {
                let v = v;
                self.advance();
                Some(Literal::Int(v))
            }
            Some(TokenKind::Float(v)) => {
                let v = v;
                self.advance();
                Some(Literal::Float(v))
            }
            Some(TokenKind::String(ref s)) => {
                let s = s.clone();
                self.advance();
                Some(Literal::String(s))
            }
            Some(TokenKind::Char(c)) => {
                let c = c;
                self.advance();
                Some(Literal::Char(c))
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

    fn match_ident(&mut self) -> Option<Identifier> {
        if let Some(TokenKind::Ident(name)) = self.peek_kind() {
            let name = name.clone();
            self.advance();
            Some(name)
        } else {
            None
        }
    }

    fn expect_ident(&mut self) -> Result<Identifier, ParserError> {
        self.match_ident()
            .ok_or_else(|| self.error("expected identifier".to_string()))
    }

    fn expect_string(&mut self) -> Result<String, ParserError> {
        if let Some(TokenKind::String(s)) = self.peek_kind() {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(self.error("expected string literal".to_string()))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParserError> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!(
                "expected '{}', found '{}'",
                kind,
                self.peek_kind().map_or("EOF".to_string(), |k| k.to_string())
            )))
        }
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind().map_or(false, |k| k == *kind)
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
            || matches!(self.tokens[self.pos].kind, TokenKind::Eof)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.pos).map(|t| t.kind.clone())
    }

    fn peek_ahead_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens.get(self.pos + offset).map(|t| t.kind.clone())
    }

    fn match_assignment_op(&mut self) -> bool {
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

    fn parse_assign_op(&mut self) -> Result<AssignOp, ParserError> {
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

    fn error(&self, message: String) -> ParserError {
        let token = self.tokens.get(self.pos).cloned().unwrap_or_else(|| {
            Token {
                kind: TokenKind::Eof,
                lexeme: "".to_string(),
                line: 0,
                column: 0,
            }
        });
        ParserError {
            message,
            line: token.line,
            column: token.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Result<Program, ParserError> {
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
}
