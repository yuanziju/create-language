use super::*;

impl Compiler {
    pub fn CompileStmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr) => {
                self.CompileExpr(expr);
            }
            Stmt::VarDecl(VarDecl {
                is_val,
                name,
                ty,
                init,
            }) => self.CompileVarDecl(*is_val, name, ty, init.as_ref()),
            Stmt::Assign(Assign { target, op, value }) => {
                self.CompileAssign(target, op, value);
            }
            Stmt::If(stmt) => self.CompileIfStmt(stmt),
            Stmt::While(WhileStmt { cond, body }) => self.CompileWhileStmt(cond, body),
            Stmt::Until(UntilStmt { cond, body }) => self.CompileUntilStmt(cond, body),
            Stmt::DoWhile(DoWhileStmt { body, cond }) => self.CompileDoWhileStmt(body, cond),
            Stmt::For(stmt) => self.CompileForStmt(stmt),
            Stmt::Match(MatchStmt { expr, arms }) => {
                self.CompileMatchExpr(&MatchStmt {
                    expr: expr.clone(),
                    arms: arms.clone(),
                });
            }
            Stmt::Return(expr) => self.CompileReturnStmt(expr.as_ref()),
            Stmt::Break => self.CompileBreakStmt(),
            Stmt::Continue => self.CompileContinueStmt(),
            Stmt::Throw(expr) => self.CompileThrowStmt(expr),
            Stmt::Try(TryStmt {
                body,
                catches,
                finally,
            }) => {
                self.CompileTryStmt(body, catches, finally.as_ref());
            }
            Stmt::Block(block) => self.CompileBlock(block),
            _ => {}
        }
    }

    pub fn CompileBlock(&mut self, block: &Block) {
        self.BeginScope();
        for stmt in &block.stmts {
            self.CompileStmt(stmt);
        }
        self.EndScope();
    }

    fn CompileIfStmt(&mut self, stmt: &IfStmt) {
        let condReg = self.CompileExpr(&stmt.cond);
        let thenJump = self.EmitJump(Opcode::JmpF, condReg);
        self.CompileBlock(&stmt.then_branch);
        let mut endJumps = Vec::new();
        if stmt.else_branch.is_some() {
            endJumps.push(self.EmitJump(Opcode::Jmp, 0));
        }
        self.PatchJump(thenJump);
        if let Some(ref elseBranch) = stmt.else_branch {
            match elseBranch.as_ref() {
                ElseBranch::Block(block) => {
                    self.CompileBlock(block);
                }
                ElseBranch::If(innerIf) => {
                    self.CompileIfStmt(innerIf);
                }
            }
            for jump in &endJumps {
                self.PatchJump(*jump);
            }
        }
    }

    fn CompileWhileStmt(&mut self, cond: &Expr, body: &Block) {
        let loopStart = self.instructions.len();
        let condReg = self.CompileExpr(cond);
        let exitJump = self.EmitJump(Opcode::JmpF, condReg);
        self.loopStack.push(LoopInfo {
            startIp: loopStart,
            breakIps: Vec::new(),
        });
        self.CompileBlock(body);
        self.Emit(Instruction::i(Opcode::Jmp, loopStart as u32));
        self.PatchJump(exitJump);
        let loopInfo = self.loopStack.pop().unwrap();
        for breakIp in loopInfo.breakIps {
            self.PatchJump(breakIp);
        }
    }

    fn CompileUntilStmt(&mut self, cond: &Expr, body: &Block) {
        let loopStart = self.instructions.len();
        let condReg = self.CompileExpr(cond);
        let exitJump = self.EmitJump(Opcode::JmpT, condReg);
        self.loopStack.push(LoopInfo {
            startIp: loopStart,
            breakIps: Vec::new(),
        });
        self.CompileBlock(body);
        self.Emit(Instruction::i(Opcode::Jmp, loopStart as u32));
        self.PatchJump(exitJump);
        let loopInfo = self.loopStack.pop().unwrap();
        for breakIp in loopInfo.breakIps {
            self.PatchJump(breakIp);
        }
    }

    fn CompileDoWhileStmt(&mut self, body: &Block, cond: &Expr) {
        let loopStart = self.instructions.len();
        self.loopStack.push(LoopInfo {
            startIp: loopStart,
            breakIps: Vec::new(),
        });
        self.CompileBlock(body);
        let condReg = self.CompileExpr(cond);
        self.Emit(Instruction::ri(Opcode::JmpT, condReg, loopStart as u16));
        let loopInfo = self.loopStack.pop().unwrap();
        for breakIp in loopInfo.breakIps {
            self.PatchJump(breakIp);
        }
    }

    fn CompileForStmt(&mut self, stmt: &ForStmt) {
        if let Some(ref init) = stmt.init {
            self.CompileStmt(init);
        }
        let loopStart = self.instructions.len();
        let mut exitJumpOpt = None;
        if let Some(ref cond) = stmt.cond {
            let condReg = self.CompileExpr(cond);
            let exitJump = self.EmitJump(Opcode::JmpF, condReg);
            exitJumpOpt = Some(exitJump);
        }
        let mut breakIps = Vec::new();
        if let Some(jump) = exitJumpOpt {
            breakIps.push(jump);
        }
        self.loopStack.push(LoopInfo {
            startIp: loopStart,
            breakIps,
        });
        self.CompileBlock(&stmt.body);
        if let Some(ref step) = stmt.step {
            self.CompileExpr(step);
        }
        self.Emit(Instruction::i(Opcode::Jmp, loopStart as u32));
        let loopInfo = self.loopStack.pop().unwrap();
        for breakIp in loopInfo.breakIps {
            self.PatchJump(breakIp);
        }
    }

    fn CompileReturnStmt(&mut self, expr: Option<&Expr>) {
        if let Some(expr) = expr {
            let reg = self.CompileExpr(expr);
            self.Emit(Instruction::ri(Opcode::Return, reg, 0));
        } else {
            self.Emit(Instruction::ri(Opcode::Return, 0, 0));
        }
    }

    fn CompileVarDecl(
        &mut self,
        _isVal: bool,
        name: &str,
        _ty: &Option<Type>,
        init: Option<&Expr>,
    ) {
        let reg = self.AddLocal(name);
        if let Some(init) = init {
            let initReg = self.CompileExpr(init);
            self.Emit(Instruction::rrr(Opcode::Mov, reg as u8, initReg, 0));
        } else {
            self.Emit(Instruction::new(Opcode::Loadnil));
        }
    }

    fn CompileBreakStmt(&mut self) {
        let jumpIp = self.EmitJump(Opcode::Jmp, 0);
        if let Some(loopInfo) = self.loopStack.last_mut() {
            loopInfo.breakIps.push(jumpIp);
        }
    }

    fn CompileContinueStmt(&mut self) {
        if let Some(loopInfo) = self.loopStack.last() {
            self.Emit(Instruction::i(Opcode::Jmp, loopInfo.startIp as u32));
        }
    }

    fn CompileThrowStmt(&mut self, expr: &Expr) {
        let reg = self.CompileExpr(expr);
        self.Emit(Instruction::ri(Opcode::Throw, reg, 0));
    }

    fn CompileTryStmt(
        &mut self,
        tryBlock: &Block,
        catches: &[CatchClause],
        finallyBlock: Option<&Block>,
    ) {
        let tryStart = self.instructions.len();
        self.Emit(Instruction::ri(Opcode::Try, 0, 0));
        self.CompileBlock(tryBlock);
        self.Emit(Instruction::new(Opcode::EndTry));
        let skipCatch;
        if !catches.is_empty() {
            skipCatch = self.EmitJump(Opcode::Jmp, 0);
            let catchStart = self.instructions.len();
            for catch in catches {
                self.CompileBlock(&catch.body);
            }
            self.PatchJump(skipCatch);
            let catchOffset = (catchStart - tryStart - 1) as u16;
            self.instructions[tryStart] = Instruction::ri(Opcode::Try, 0, catchOffset);
        }
        if let Some(finally) = finallyBlock {
            self.CompileBlock(finally);
        }
    }
}
