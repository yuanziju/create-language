use super::*;

impl Parser {
    pub fn parse_expr(&mut self) -> Result<Expr, ParserError> {
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
            self.peek_kind()
                .map_or("EOF".to_string(), |k| k.to_string())
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
        let end_tokens = [TokenKind::RParen, TokenKind::Arrow, TokenKind::Colon];
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

    pub fn parse_parenthesized_expr(&mut self) -> Result<Expr, ParserError> {
        self.expect(TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(expr)
    }
}
