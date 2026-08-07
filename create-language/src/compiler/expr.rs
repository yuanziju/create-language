use super::*;

impl Compiler {
    pub fn CompileExpr(&mut self, expr: &Expr) -> u8 {
        match expr {
            Expr::Literal(lit) => self.CompileLiteral(lit),
            Expr::Identifier(name) => self.CompileIdent(name),
            Expr::Binary(BinaryExpr { left, op, right }) => self.CompileBinary(op, left, right),
            Expr::Unary(UnaryExpr { op, expr }) => self.CompileUnary(op, expr),
            Expr::Call(CallExpr { callee, args }) => self.CompileCall(callee, args),
            Expr::Lambda(LambdaExpr { params, body, .. }) => self.CompileLambda(params, body),
            Expr::If(ifStmt) => self.CompileIfExpr(ifStmt),
            Expr::Match(matchStmt) => self.CompileMatchExpr(matchStmt),
            Expr::ArrayLiteral(elements) => self.CompileArrayLiteral(elements),
            Expr::FieldAccess(FieldAccessExpr { object, field }) => {
                self.CompileFieldAccess(object, field)
            }
            Expr::Index(IndexExpr { object, index }) => self.CompileIndexAccess(object, index),
            Expr::StructLiteral(StructLiteralExpr { name, fields, .. }) => {
                self.CompileStructLiteral(name, fields)
            }
            Expr::Await(expr) | Expr::Spawn(expr) => self.CompileExpr(expr),
            Expr::Receive(opt) => {
                if let Some(inner) = opt {
                    self.CompileExpr(inner)
                } else {
                    let reg = self.nextRegister as u8;
                    self.nextRegister += 1;
                    self.Emit(Instruction::new(Opcode::Loadnil));
                    reg
                }
            }
            _ => {
                self.Error(
                    "unsupported expression".to_string(),
                    SourceLocation { line: 0, column: 0 },
                );
                let reg = self.nextRegister as u8;
                self.nextRegister += 1;
                self.Emit(Instruction::new(Opcode::Loadnil));
                reg
            }
        }
    }

    fn CompileLiteral(&mut self, lit: &Literal) -> u8 {
        let reg = self.nextRegister as u8;
        self.nextRegister += 1;
        match lit {
            Literal::Int(n) => {
                if *n >= 0 && *n <= 0xFFFF {
                    self.Emit(Instruction::ri(Opcode::Loadi, reg, *n as u16));
                } else {
                    let idx = self.AddConstant(Value::Int(*n));
                    self.Emit(Instruction::rrk(Opcode::Loadk, reg, 0, idx as u8));
                }
            }
            Literal::Float(f) => {
                let idx = self.AddConstant(Value::Float(*f));
                self.Emit(Instruction::rrk(Opcode::Loadk, reg, 0, idx as u8));
            }
            Literal::String(s) => {
                let idx = self.AddConstant(Value::String(s.clone()));
                self.Emit(Instruction::rrk(Opcode::Loadk, reg, 0, idx as u8));
            }
            Literal::Bool(b) => {
                self.Emit(Instruction::ri(Opcode::Loadbool, reg, *b as u16));
            }
            Literal::Char(c) => {
                let idx = self.AddConstant(Value::String(c.to_string()));
                self.Emit(Instruction::rrk(Opcode::Loadk, reg, 0, idx as u8));
            }
            Literal::Null => {
                self.Emit(Instruction::new(Opcode::Loadnil));
            }
        }
        reg
    }

    fn CompileIdent(&mut self, name: &str) -> u8 {
        if let Some(reg) = self.ResolveLocal(name) {
            return reg as u8;
        }
        if let Some(upIdx) = self.ResolveUpvalue(name) {
            let reg = self.nextRegister as u8;
            self.nextRegister += 1;
            self.Emit(Instruction::rri(Opcode::LoadUpvalue, reg, upIdx as u8, 0));
            return reg;
        }
        let reg = self.nextRegister as u8;
        self.nextRegister += 1;
        self.Emit(Instruction::new(Opcode::Loadnil));
        reg
    }

    fn CompileBinary(&mut self, op: &BinaryOp, left: &Expr, right: &Expr) -> u8 {
        let leftReg = self.CompileExpr(left);
        let rightReg = self.CompileExpr(right);
        let resultReg = self.nextRegister as u8;
        self.nextRegister += 1;
        match op {
            BinaryOp::Add => {
                self.Emit(Instruction::rrr(Opcode::Add, resultReg, leftReg, rightReg));
            }
            BinaryOp::Sub => {
                self.Emit(Instruction::rrr(Opcode::Sub, resultReg, leftReg, rightReg));
            }
            BinaryOp::Mul => {
                self.Emit(Instruction::rrr(Opcode::Mul, resultReg, leftReg, rightReg));
            }
            BinaryOp::Div => {
                self.Emit(Instruction::rrr(Opcode::Div, resultReg, leftReg, rightReg));
            }
            BinaryOp::Mod => {
                self.Emit(Instruction::rrr(Opcode::Mod, resultReg, leftReg, rightReg));
            }
            BinaryOp::Eq => {
                self.Emit(Instruction::rrr(Opcode::Eq, resultReg, leftReg, rightReg));
            }
            BinaryOp::Less => {
                self.Emit(Instruction::rrr(Opcode::Lt, resultReg, leftReg, rightReg));
            }
            BinaryOp::LessEq => {
                self.Emit(Instruction::rrr(Opcode::Le, resultReg, leftReg, rightReg));
            }
            BinaryOp::Greater => {
                self.Emit(Instruction::rrr(Opcode::Lt, resultReg, rightReg, leftReg));
            }
            BinaryOp::GreaterEq => {
                self.Emit(Instruction::rrr(Opcode::Le, resultReg, rightReg, leftReg));
            }
            BinaryOp::NotEq => {
                self.Emit(Instruction::rrr(Opcode::Eq, resultReg, leftReg, rightReg));
                self.Emit(Instruction::rrr(Opcode::Not, resultReg, resultReg, 0));
            }
            BinaryOp::And => {
                let jumpIdx = self.EmitJump(Opcode::JmpF, leftReg);
                let rightReg2 = self.CompileExpr(right);
                self.Emit(Instruction::rrr(Opcode::Mov, resultReg, rightReg2, 0));
                self.PatchJump(jumpIdx);
            }
            BinaryOp::Or => {
                let jumpIdx = self.EmitJump(Opcode::JmpT, leftReg);
                let rightReg2 = self.CompileExpr(right);
                self.Emit(Instruction::rrr(Opcode::Mov, resultReg, rightReg2, 0));
                self.PatchJump(jumpIdx);
            }
            _ => {
                self.Emit(Instruction::new(Opcode::Loadnil));
            }
        }
        resultReg
    }

    fn CompileUnary(&mut self, op: &UnaryOp, expr: &Expr) -> u8 {
        let reg = self.CompileExpr(expr);
        match op {
            UnaryOp::Neg => {
                let resultReg = self.nextRegister as u8;
                self.nextRegister += 1;
                self.Emit(Instruction::rrr(Opcode::Neg, resultReg, reg, 0));
                resultReg
            }
            UnaryOp::Not => {
                let resultReg = self.nextRegister as u8;
                self.nextRegister += 1;
                self.Emit(Instruction::rrr(Opcode::Not, resultReg, reg, 0));
                resultReg
            }
            _ => reg,
        }
    }

    fn CompileCall(&mut self, callee: &Expr, args: &[Expr]) -> u8 {
        let calleeReg = self.CompileExpr(callee);
        let baseReg = self.nextRegister as u8;
        self.nextRegister += 1 + args.len().max(1);
        self.Emit(Instruction::rrr(Opcode::Mov, baseReg, calleeReg, 0));
        for (i, arg) in args.iter().enumerate() {
            let argReg = self.CompileExpr(arg);
            self.Emit(Instruction::rrr(
                Opcode::Mov,
                baseReg + 1 + i as u8,
                argReg,
                0,
            ));
        }
        self.Emit(Instruction::rrr(
            Opcode::Call,
            baseReg,
            baseReg,
            args.len() as u8,
        ));
        baseReg
    }

    fn CompileLambda(&mut self, _params: &[LambdaParam], _body: &Expr) -> u8 {
        let mut child = Compiler::new();
        child.functionName = "<lambda>".to_string();
        child.enclosing = Some(Box::new(std::mem::replace(self, Compiler::new())));
        let _ = child;
        let reg = self.nextRegister as u8;
        self.nextRegister += 1;
        let idx = self.AddConstant(Value::Function(0));
        self.Emit(Instruction::rrk(Opcode::Closure, reg, 0, idx as u8));
        reg
    }

    fn CompileIfExpr(&mut self, stmt: &IfStmt) -> u8 {
        let condReg = self.CompileExpr(&stmt.cond);
        let thenJump = self.EmitJump(Opcode::JmpF, condReg);
        let mut resultReg = self.nextRegister as u8;
        self.nextRegister += 1;
        let mut hasElse = false;
        // Compile the then branch block - take the last expression if any
        let thenBlock = &stmt.then_branch;
        if let Some(Stmt::Expr(expr)) = thenBlock.stmts.last() {
            let exprReg = self.CompileExpr(expr);
            resultReg = exprReg;
        } else {
            self.Emit(Instruction::new(Opcode::Loadnil));
        }
        let elseJump = self.EmitJump(Opcode::Jmp, 0);
        self.PatchJump(thenJump);
        if let Some(ref elseBranch) = stmt.else_branch {
            hasElse = true;
            match elseBranch.as_ref() {
                ElseBranch::Block(block) => {
                    if let Some(lastStmt) = block.stmts.last() {
                        match lastStmt {
                            Stmt::Expr(expr) => {
                                let elseReg = self.CompileExpr(expr);
                                self.Emit(Instruction::rrr(Opcode::Mov, resultReg, elseReg, 0));
                            }
                            _ => {
                                self.Emit(Instruction::new(Opcode::Loadnil));
                            }
                        }
                    }
                }
                ElseBranch::If(innerIf) => {
                    let elseReg = self.CompileIfExpr(innerIf);
                    self.Emit(Instruction::rrr(Opcode::Mov, resultReg, elseReg, 0));
                }
            }
        } else {
            self.Emit(Instruction::new(Opcode::Loadnil));
        }
        self.PatchJump(elseJump);
        if hasElse {
            resultReg
        } else {
            let nilReg = self.nextRegister as u8;
            self.nextRegister += 1;
            self.Emit(Instruction::new(Opcode::Loadnil));
            nilReg
        }
    }

    pub fn CompileMatchExpr(&mut self, matchStmt: &MatchStmt) -> u8 {
        let matchReg = self.CompileExpr(&matchStmt.expr);
        let resultReg = self.nextRegister as u8;
        self.nextRegister += 1;
        let mut endJumps = Vec::new();
        for arm in &matchStmt.arms {
            let condReg = self.nextRegister as u8;
            self.nextRegister += 1;
            self.Emit(Instruction::rrr(Opcode::Eq, condReg, matchReg, matchReg));
            let skipJump = self.EmitJump(Opcode::JmpF, condReg);
            let armReg = self.CompileExpr(&arm.body);
            self.Emit(Instruction::rrr(Opcode::Mov, resultReg, armReg, 0));
            let endJump = self.EmitJump(Opcode::Jmp, 0);
            endJumps.push(endJump);
            self.PatchJump(skipJump);
        }
        self.Emit(Instruction::new(Opcode::Loadnil));
        for jump in endJumps {
            self.PatchJump(jump);
        }
        resultReg
    }

    fn CompileArrayLiteral(&mut self, elements: &[Expr]) -> u8 {
        let reg = self.nextRegister as u8;
        self.nextRegister += 1;
        self.Emit(Instruction::new(Opcode::NewArray));
        for elem in elements {
            let elemReg = self.CompileExpr(elem);
            let lenReg = self.nextRegister as u8;
            self.nextRegister += 1;
            self.Emit(Instruction::rrr(Opcode::ALen, lenReg, reg, 0));
            self.Emit(Instruction::rrr(Opcode::ASet, reg, lenReg, elemReg));
        }
        reg
    }

    fn CompileFieldAccess(&mut self, object: &Expr, field: &str) -> u8 {
        let objReg = self.CompileExpr(object);
        let reg = self.nextRegister as u8;
        self.nextRegister += 1;
        let fieldIdx = self.AddConstant(Value::String(field.to_string()));
        self.Emit(Instruction::rrk(
            Opcode::GetField,
            reg,
            objReg,
            fieldIdx as u8,
        ));
        reg
    }

    fn CompileIndexAccess(&mut self, array: &Expr, index: &Expr) -> u8 {
        let arrReg = self.CompileExpr(array);
        let idxReg = self.CompileExpr(index);
        let reg = self.nextRegister as u8;
        self.nextRegister += 1;
        self.Emit(Instruction::rrr(Opcode::AGet, reg, arrReg, idxReg));
        reg
    }

    pub fn CompileAssign(&mut self, target: &Expr, _op: &AssignOp, value: &Expr) -> u8 {
        let valReg = self.CompileExpr(value);
        match target {
            Expr::Identifier(name) => {
                if let Some(reg) = self.ResolveLocal(name) {
                    self.Emit(Instruction::rrr(Opcode::Mov, reg as u8, valReg, 0));
                }
            }
            Expr::FieldAccess(FieldAccessExpr { object, field }) => {
                let objReg = self.CompileExpr(object);
                let fieldIdx = self.AddConstant(Value::String(field.clone()));
                self.Emit(Instruction::rrk(
                    Opcode::SetField,
                    valReg,
                    objReg,
                    fieldIdx as u8,
                ));
            }
            Expr::Index(IndexExpr { object, index }) => {
                let arrReg = self.CompileExpr(object);
                let idxReg = self.CompileExpr(index);
                self.Emit(Instruction::rrr(Opcode::ASet, arrReg, idxReg, valReg));
            }
            _ => {}
        }
        valReg
    }

    fn CompileStructLiteral(&mut self, _name: &str, _fields: &[FieldInit]) -> u8 {
        let reg = self.nextRegister as u8;
        self.nextRegister += 1;
        self.Emit(Instruction::new(Opcode::NewObject));
        reg
    }
}
