use super::*;

impl Compiler {
    pub fn CompileTopLevel(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(decl) => self.CompileFunctionDecl(decl),
            TopLevel::Struct(decl) => self.CompileStructDecl(decl),
            TopLevel::Class(decl) => self.CompileClassDecl(decl),
            TopLevel::DataClass(decl) => self.CompileDataClassDecl(decl),
            TopLevel::Enum(decl) => self.CompileEnumDecl(decl),
            TopLevel::Trait(decl) => self.CompileTraitDecl(decl),
            TopLevel::Impl(decl) => self.CompileImplDecl(decl),
            TopLevel::Stmt(stmt) => self.CompileStmt(stmt),
        }
    }

    fn CompileFunctionDecl(&mut self, decl: &FunctionDecl) {
        let name = decl.name.clone();
        let mut child = Compiler::new();
        child.functionName = name.clone();
        child.BeginScope();
        for param in &decl.params {
            child.AddLocal(&param.name);
        }
        child.CompileBlock(&decl.body);
        child.EndScope();
        child.Emit(Instruction::new(Opcode::Return));
        let funcIdx = self.AddConstant(Value::Function(0));
        let reg = self.AddLocal(&name);
        self.Emit(Instruction::rrk(
            Opcode::Closure,
            reg as u8,
            0,
            funcIdx as u8,
        ));
    }

    fn CompileStructDecl(&mut self, _decl: &StructDecl) {
        self.Emit(Instruction::new(Opcode::Loadnil));
    }

    fn CompileClassDecl(&mut self, _decl: &ClassDecl) {
        self.Emit(Instruction::new(Opcode::Loadnil));
    }

    fn CompileDataClassDecl(&mut self, _decl: &DataClassDecl) {
        self.Emit(Instruction::new(Opcode::Loadnil));
    }

    fn CompileEnumDecl(&mut self, _decl: &EnumDecl) {
        self.Emit(Instruction::new(Opcode::Loadnil));
    }

    fn CompileTraitDecl(&mut self, _decl: &TraitDecl) {
        self.Emit(Instruction::new(Opcode::Loadnil));
    }

    fn CompileImplDecl(&mut self, _decl: &ImplDecl) {
        self.Emit(Instruction::new(Opcode::Loadnil));
    }
}
