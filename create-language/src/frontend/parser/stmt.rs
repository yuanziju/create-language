use super::*;

impl Parser {
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParserError> {
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
                    Ok(Stmt::Assign(Assign {
                        target: expr,
                        op,
                        value,
                    }))
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

    pub fn parse_if_stmt(&mut self) -> Result<IfStmt, ParserError> {
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

    pub fn parse_match_stmt(&mut self) -> Result<MatchStmt, ParserError> {
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

    pub fn parse_block(&mut self) -> Result<Block, ParserError> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts })
    }
}
