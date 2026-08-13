use crate::ast::*;
use crate::constant_pool::{ConstantPool, Value};
use crate::instruction::Instruction;
use crate::opcode::Opcode;

pub mod decl;
pub mod expr;
pub mod stmt;

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

pub struct Compiler {
    instructions: Vec<Instruction>,
    constants: ConstantPool,
    nextRegister: usize,
    locals: Vec<Local>,
    scopes: Vec<Scope>,
    upvalues: Vec<UpvalueInfo>,
    enclosing: Option<Box<Compiler>>,
    functionName: String,
    loopStack: Vec<LoopInfo>,
    errors: Vec<CompileError>,
}

struct Local {
    name: String,
    depth: usize,
    register: usize,
    captured: bool,
}

struct Scope {
    depth: usize,
}

struct UpvalueInfo {
    isLocal: bool,
    localIndex: usize,
}

struct LoopInfo {
    startIp: usize,
    breakIps: Vec<usize>,
}

pub struct CompileResult {
    pub function: Function,
    pub errors: Vec<CompileError>,
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub location: SourceLocation,
}

pub struct Function {
    pub name: String,
    pub arity: usize,
    pub numRegisters: usize,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub upvalueCount: usize,
    pub upvalueDescs: Vec<crate::binary::UpvalueDesc>,
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler {
            instructions: Vec::new(),
            constants: ConstantPool::new(),
            nextRegister: 0,
            locals: Vec::new(),
            scopes: vec![Scope { depth: 0 }],
            upvalues: Vec::new(),
            enclosing: None,
            functionName: String::new(),
            loopStack: Vec::new(),
            errors: Vec::new(),
        }
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn Compile(&mut self, program: &Program) -> CompileResult {
        self.functionName = "<main>".to_string();
        self.BeginScope();
        for item in &program.items {
            self.CompileTopLevel(item);
        }
        self.EndScope();
        self.Emit(Instruction::new(Opcode::Halt));
        CompileResult {
            function: Function {
                name: self.functionName.clone(),
                arity: 0,
                numRegisters: self.nextRegister,
                instructions: self.instructions.clone(),
                constants: self.constants.get_constants().to_vec(),
                upvalueCount: self.upvalues.len(),
                upvalueDescs: self
                    .upvalues
                    .iter()
                    .map(|u| crate::binary::UpvalueDesc {
                        isLocal: u.isLocal,
                        index: u.localIndex,
                    })
                    .collect(),
            },
            errors: self.errors.clone(),
        }
    }

    fn BeginScope(&mut self) {
        let depth = self.scopes.last().map(|s| s.depth + 1).unwrap_or(0);
        self.scopes.push(Scope { depth });
    }

    fn EndScope(&mut self) {
        let scope = self.scopes.pop().unwrap();
        while self.locals.last().is_some_and(|l| l.depth >= scope.depth) {
            let local = self.locals.pop().unwrap();
            if local.captured {
                self.Emit(Instruction::new(Opcode::CloseUpvalue));
            }
        }
    }

    pub fn AddLocal(&mut self, name: &str) -> usize {
        let reg = self.nextRegister;
        self.nextRegister += 1;
        self.locals.push(Local {
            name: name.to_string(),
            depth: self.scopes.last().unwrap().depth,
            register: reg,
            captured: false,
        });
        reg
    }

    pub fn ResolveLocal(&self, name: &str) -> Option<usize> {
        self.locals
            .iter()
            .rev()
            .find(|l| l.name == name)
            .map(|l| l.register)
    }

    pub fn ResolveUpvalue(&mut self, name: &str) -> Option<usize> {
        if let Some(ref mut enclosing) = self.enclosing {
            if let Some(reg) = enclosing.ResolveLocal(name) {
                for local in &mut enclosing.locals {
                    if local.register == reg {
                        local.captured = true;
                        break;
                    }
                }
                return Some(self.AddUpvalue(true, reg));
            }
            if let Some(idx) = enclosing.ResolveUpvalue(name) {
                return Some(self.AddUpvalue(false, idx));
            }
        }
        None
    }

    fn AddUpvalue(&mut self, isLocal: bool, index: usize) -> usize {
        for (i, uv) in self.upvalues.iter().enumerate() {
            if uv.isLocal == isLocal && uv.localIndex == index {
                return i;
            }
        }
        let idx = self.upvalues.len();
        self.upvalues.push(UpvalueInfo {
            isLocal,
            localIndex: index,
        });
        idx
    }

    pub fn Emit(&mut self, inst: Instruction) -> usize {
        let ip = self.instructions.len();
        self.instructions.push(inst);
        ip
    }

    pub fn EmitTableSwitch(&mut self, reg: u8, base_const: u16) -> usize {
        self.Emit(Instruction::ri(Opcode::TableSwitch, reg, base_const))
    }

    pub fn EmitLookupSwitch(&mut self, reg: u8, base_const: u16) -> usize {
        self.Emit(Instruction::ri(Opcode::LookupSwitch, reg, base_const))
    }

    pub fn EmitMatch(&mut self, reg: u8, base_const: u16) -> usize {
        self.Emit(Instruction::ri(Opcode::Match, reg, base_const))
    }

    pub fn EmitVecAdd(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::VecAdd, a, b, c))
    }

    pub fn EmitVecSub(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::VecSub, a, b, c))
    }

    pub fn EmitVecMul(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::VecMul, a, b, c))
    }

    pub fn EmitVecDiv(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::VecDiv, a, b, c))
    }

    pub fn EmitVecCmpEq(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::VecCmpEq, a, b, c))
    }

    pub fn EmitVecCmpLt(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::VecCmpLt, a, b, c))
    }

    pub fn EmitVecCmpLe(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::VecCmpLe, a, b, c))
    }

    pub fn EmitVecLoad(&mut self, a: u8, b: u8, imm: u8) -> usize {
        self.Emit(Instruction::rri(Opcode::VecLoad, a, b, imm))
    }

    pub fn EmitVecStore(&mut self, a: u8, b: u8, imm: u8) -> usize {
        self.Emit(Instruction::rri(Opcode::VecStore, a, b, imm))
    }

    pub fn EmitSuspend(&mut self, a: u8, imm: u16) -> usize {
        self.Emit(Instruction::ri(Opcode::Suspend, a, imm))
    }

    pub fn EmitResume(&mut self, a: u8, imm: u16) -> usize {
        self.Emit(Instruction::ri(Opcode::Resume, a, imm))
    }

    pub fn EmitInvokeSpecial(&mut self, a: u8, b: u8, c: u8) -> usize {
        self.Emit(Instruction::rrr(Opcode::InvokeSpecial, a, b, c))
    }

    pub fn EmitJump(&mut self, opcode: Opcode, reg: u8) -> usize {
        let ip = self.instructions.len();
        self.instructions.push(Instruction::ri(opcode, reg, 0));
        ip
    }

    pub fn PatchJump(&mut self, offset: usize) {
        let jumpDist = self.instructions.len() - offset - 1;
        let inst = &mut self.instructions[offset];
        let op = inst.opcode();
        let reg = inst.a();
        *inst = Instruction::ri(op, reg, jumpDist as u16);
    }

    pub fn AddConstant(&mut self, value: Value) -> u16 {
        self.constants.add(value)
    }

    pub fn Error(&mut self, message: String, location: SourceLocation) {
        self.errors.push(CompileError { message, location });
    }
}
