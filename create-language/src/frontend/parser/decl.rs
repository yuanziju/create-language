use super::*;

impl Parser {
    pub fn parse_top_level(&mut self) -> Result<TopLevel, ParserError> {
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

    pub fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParserError> {
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

    pub fn parse_type(&mut self) -> Result<Type, ParserError> {
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
                Ok(Type::Result(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                ))
            }
            "Option" if generic_args.len() == 1 => {
                Ok(Type::Option(Box::new(generic_args[0].clone())))
            }
            _ => Ok(Type::Named(name, generic_args)),
        }
    }

    pub fn parse_optional_generic_args(&mut self) -> Result<Vec<Type>, ParserError> {
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
}
