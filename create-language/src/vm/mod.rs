use std::collections::HashMap;

use crate::binary::ModuleFile;
use crate::instruction::Instruction;
use crate::opcode::Opcode;
use crate::vm::error::{Result, RuntimeError};
use crate::vm::frame::CallFrame;
use crate::vm::memory::{Heap, ObjectKind, Stack};
use crate::vm::value::{ClosureData, GcRef, Value};

pub type NativeFn = fn(&[Value]) -> std::result::Result<Value, RuntimeError>;

pub mod error;
pub mod frame;
pub mod memory;
pub mod value;

pub struct Vm {
    stack: Stack,
    frames: Vec<CallFrame>,
    heap: Heap,
    module: Option<ModuleFile>,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Vm {
            stack: Stack::new(),
            frames: Vec::new(),
            heap: Heap::new(),
            module: None,
        }
    }

    pub fn LoadModule(&mut self, module: ModuleFile) -> Result<()> {
        self.module = Some(module);
        Ok(())
    }

    pub fn Exec(&mut self) -> Result<()> {
        let module = self
            .module
            .as_ref()
            .ok_or_else(|| RuntimeError::Custom("no module loaded".into()))?;
        let entryPoint = module.entryPoint as usize;
        if entryPoint >= module.functions.len() {
            return Err(RuntimeError::Custom("invalid entry point".into()));
        }
        let func = &module.functions[entryPoint];
        let frame = CallFrame::new(entryPoint, func.numRegisters, 0, 0);
        self.frames.push(frame);
        self.ExecuteCurrentFrame()
    }

    pub fn ExecFunc(&mut self, funcIndex: usize, args: Vec<Value>) -> Result<Value> {
        let module = self
            .module
            .as_ref()
            .ok_or_else(|| RuntimeError::Custom("no module loaded".into()))?;
        if funcIndex >= module.functions.len() {
            return Err(RuntimeError::UndefinedFunction(format!(
                "function index {funcIndex}"
            )));
        }
        let func = &module.functions[funcIndex];
        if args.len() != func.arity {
            return Err(RuntimeError::Custom(format!(
                "expected {} arguments, got {}",
                func.arity,
                args.len()
            )));
        }
        let stackStart = self.stack.len();
        for arg in args {
            self.stack.push(arg);
        }
        let frame = CallFrame::new(funcIndex, func.numRegisters, stackStart, 0);
        self.frames.push(frame);
        self.ExecuteCurrentFrame()?;
        self.stack
            .pop()
            .ok_or(RuntimeError::Custom("no return value".into()))
    }

    fn ExecuteCurrentFrame(&mut self) -> Result<()> {
        loop {
            let frameIndex = self.frames.len() - 1;
            let inst = {
                let module = self.module.as_ref().unwrap();
                let frame = &self.frames[frameIndex];
                let func = &module.functions[frame.funcIndex];
                if frame.ip >= func.instructions.len() {
                    return Err(RuntimeError::Custom("program counter out of bounds".into()));
                }
                func.instructions[frame.ip]
            };
            if inst.opcode() == Opcode::Wide {
                let frameIndex = self.frames.len() - 1;
                let module = self.module.as_ref().unwrap();
                let frame = &mut self.frames[frameIndex];
                frame.ip += 1;
                let func = &module.functions[frame.funcIndex];
                if frame.ip >= func.instructions.len() {
                    return Err(RuntimeError::Custom("program counter out of bounds".into()));
                }
                let wideInst = func.instructions[frame.ip];
                self.ExecuteInstruction(wideInst, true)?;
                if self.frames.is_empty() || wideInst.opcode() == Opcode::Halt {
                    return Ok(());
                }
                // Don't auto-increment IP after Call/TailCall — new frame starts at IP 0
                if wideInst.opcode() != Opcode::Call && wideInst.opcode() != Opcode::TailCall {
                    let frameIndex = self.frames.len() - 1;
                    let frame = &mut self.frames[frameIndex];
                    frame.ip += 1;
                }
            } else {
                self.ExecuteInstruction(inst, false)?;
                if self.frames.is_empty() || inst.opcode() == Opcode::Halt {
                    return Ok(());
                }
                // Don't auto-increment IP after Call/TailCall — new frame starts at IP 0
                if inst.opcode() != Opcode::Call && inst.opcode() != Opcode::TailCall {
                    let frameIndex = self.frames.len() - 1;
                    let frame = &mut self.frames[frameIndex];
                    frame.ip += 1;
                }
            }
        }
    }

    fn ExecuteInstruction(&mut self, inst: Instruction, _wide: bool) -> Result<()> {
        match inst.opcode() {
            Opcode::Halt => {
                return Ok(());
            }
            Opcode::Mov => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let val = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[b].clone()
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = val;
            }
            Opcode::Loadk => {
                let a = inst.a() as usize;
                let k = inst.imm8() as usize;
                let val = {
                    let module = self.module.as_ref().unwrap();
                    let frame = self.frames.last().unwrap();
                    let func = &module.functions[frame.funcIndex];
                    if k >= func.constants.len() {
                        return Err(RuntimeError::Custom("constant index out of bounds".into()));
                    }
                    Value::from(&func.constants[k])
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = val;
            }
            Opcode::Loadi => {
                let a = inst.a() as usize;
                let val = inst.imm16_signed() as i64;
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Int(val);
            }
            Opcode::Loadbool => {
                let a = inst.a() as usize;
                let val = inst.imm16() != 0;
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Bool(val);
            }
            Opcode::Loadnil => {
                let a = inst.a() as usize;
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Nil;
            }
            Opcode::Add => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                let result = match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => Value::Int(l.wrapping_add(*r)),
                    (Value::Float(l), Value::Float(r)) => Value::Float(l + r),
                    (Value::Int(l), Value::Float(r)) => Value::Float(*l as f64 + r),
                    (Value::Float(l), Value::Int(r)) => Value::Float(l + *r as f64),
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "number",
                            found: lhs.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = result;
            }
            Opcode::Sub => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                let result = match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => Value::Int(l.wrapping_sub(*r)),
                    (Value::Float(l), Value::Float(r)) => Value::Float(l - r),
                    (Value::Int(l), Value::Float(r)) => Value::Float(*l as f64 - r),
                    (Value::Float(l), Value::Int(r)) => Value::Float(l - *r as f64),
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "number",
                            found: lhs.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = result;
            }
            Opcode::Mul => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                let result = match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => Value::Int(l.wrapping_mul(*r)),
                    (Value::Float(l), Value::Float(r)) => Value::Float(l * r),
                    (Value::Int(l), Value::Float(r)) => Value::Float(*l as f64 * r),
                    (Value::Float(l), Value::Int(r)) => Value::Float(l * *r as f64),
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "number",
                            found: lhs.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = result;
            }
            Opcode::Div => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                let result = match (&lhs, &rhs) {
                    (Value::Int(_), Value::Int(r)) if *r == 0 => {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    (Value::Int(l), Value::Int(r)) => Value::Int(l.wrapping_div(*r)),
                    (Value::Float(_), Value::Float(r)) if *r == 0.0 => {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    (Value::Float(l), Value::Float(r)) => Value::Float(l / r),
                    (Value::Int(l), Value::Float(r)) if *r == 0.0 => {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    (Value::Int(l), Value::Float(r)) => Value::Float(*l as f64 / r),
                    (Value::Float(l), Value::Int(r)) if *r == 0 => {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    (Value::Float(l), Value::Int(r)) => Value::Float(l / *r as f64),
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "number",
                            found: lhs.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = result;
            }
            Opcode::Mod => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                let result = match (&lhs, &rhs) {
                    (Value::Int(_), Value::Int(r)) if *r == 0 => {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    (Value::Int(l), Value::Int(r)) => Value::Int(l.wrapping_rem(*r)),
                    (Value::Float(_), Value::Float(r)) if *r == 0.0 => {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    (Value::Float(l), Value::Float(r)) => Value::Float(l % r),
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "integer",
                            found: lhs.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = result;
            }
            Opcode::Neg => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let val = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[b].clone()
                };
                let result = match &val {
                    Value::Int(v) => Value::Int(v.wrapping_neg()),
                    Value::Float(v) => Value::Float(-v),
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "number",
                            found: val.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = result;
            }
            Opcode::BitAnd => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => {
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Int(l & r);
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "int",
                            found: lhs.typeName(),
                        });
                    }
                }
            }
            Opcode::BitOr => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => {
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Int(l | r);
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "int",
                            found: lhs.typeName(),
                        });
                    }
                }
            }
            Opcode::BitXor => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => {
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Int(l ^ r);
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "int",
                            found: lhs.typeName(),
                        });
                    }
                }
            }
            Opcode::BitNot => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let val = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[b].clone()
                };
                match &val {
                    Value::Int(v) => {
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Int(!v);
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "int",
                            found: val.typeName(),
                        });
                    }
                }
            }
            Opcode::Shl => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => {
                        let shift = (*r & 0x3F) as u32;
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Int(l.wrapping_shl(shift));
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "int",
                            found: lhs.typeName(),
                        });
                    }
                }
            }
            Opcode::Shr => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => {
                        let shift = (*r & 0x3F) as u32;
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Int(l.wrapping_shr(shift));
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "int",
                            found: lhs.typeName(),
                        });
                    }
                }
            }
            Opcode::I2f => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let val = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[b].clone()
                };
                match &val {
                    Value::Int(v) => {
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Float(*v as f64);
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "int",
                            found: val.typeName(),
                        });
                    }
                }
            }
            Opcode::F2i => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let val = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[b].clone()
                };
                match &val {
                    Value::Float(v) => {
                        let frame = self.frames.last_mut().unwrap();
                        frame.registers[a] = Value::Int(*v as i64);
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "float",
                            found: val.typeName(),
                        });
                    }
                }
            }
            Opcode::Eq => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let result = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[b] == frame.registers[c]
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Bool(result);
            }
            Opcode::Lt => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                let result = match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => l < r,
                    (Value::Float(l), Value::Float(r)) => l < r,
                    (Value::Int(l), Value::Float(r)) => (*l as f64) < *r,
                    (Value::Float(l), Value::Int(r)) => *l < (*r as f64),
                    (Value::String(l), Value::String(r)) => l < r,
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "comparable",
                            found: lhs.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Bool(result);
            }
            Opcode::Le => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (lhs, rhs) = {
                    let frame = self.frames.last().unwrap();
                    (frame.registers[b].clone(), frame.registers[c].clone())
                };
                let result = match (&lhs, &rhs) {
                    (Value::Int(l), Value::Int(r)) => l <= r,
                    (Value::Float(l), Value::Float(r)) => l <= r,
                    (Value::Int(l), Value::Float(r)) => (*l as f64) <= *r,
                    (Value::Float(l), Value::Int(r)) => *l <= (*r as f64),
                    (Value::String(l), Value::String(r)) => l <= r,
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "comparable",
                            found: lhs.typeName(),
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Bool(result);
            }
            Opcode::Not => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let truthy = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[b].isTruthy()
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Bool(!truthy);
            }
            Opcode::IsType => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let typeCode = inst.c();
                let matches = {
                    let frame = self.frames.last().unwrap();
                    let val = &frame.registers[b];
                    match typeCode {
                        0 => matches!(val, Value::Nil),
                        1 => matches!(val, Value::Bool(_)),
                        2 => matches!(val, Value::Int(_)),
                        3 => matches!(val, Value::Float(_)),
                        4 => matches!(val, Value::String(_)),
                        5 => matches!(
                            val,
                            Value::Function(_) | Value::Closure(_) | Value::NativeFn(_)
                        ),
                        6 => matches!(val, Value::Object(_)),
                        7 => matches!(val, Value::Array(_)),
                        _ => false,
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Bool(matches);
            }
            Opcode::Jmp => {
                let frame = self.frames.last_mut().unwrap();
                let offset: isize = if inst.a() == 0 {
                    inst.imm16_signed() as isize
                } else {
                    inst.imm24_signed() as isize
                };
                if offset == 0 {
                    return Ok(());
                }
                let newIp = frame.ip as isize + offset;
                if newIp < 0 {
                    return Err(RuntimeError::Custom("jump to negative address".into()));
                }
                frame.ip = newIp as usize - 1;
            }
            Opcode::JmpT => {
                let a = inst.a() as usize;
                let offset = inst.imm16_signed() as isize;
                let truthy = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[a].isTruthy()
                };
                if truthy {
                    let frame = self.frames.last_mut().unwrap();
                    let newIp = frame.ip as isize + offset;
                    if newIp < 0 {
                        return Err(RuntimeError::Custom("jump to negative address".into()));
                    }
                    frame.ip = newIp as usize - 1;
                }
            }
            Opcode::JmpF => {
                let a = inst.a() as usize;
                let offset = inst.imm16_signed() as isize;
                let truthy = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[a].isTruthy()
                };
                if !truthy {
                    let frame = self.frames.last_mut().unwrap();
                    let newIp = frame.ip as isize + offset;
                    if newIp < 0 {
                        return Err(RuntimeError::Custom("jump to negative address".into()));
                    }
                    frame.ip = newIp as usize - 1;
                }
            }
            Opcode::Call => {
                let a = inst.a() as usize;
                let argCount = inst.c() as usize;
                let (callee, args) = {
                    let frame = self.frames.last().unwrap();
                    let callee = frame.registers[a].clone();
                    let mut args = Vec::with_capacity(argCount);
                    for i in 0..argCount {
                        args.push(frame.registers[a + 1 + i].clone());
                    }
                    (callee, args)
                };
                let funcIndex;
                let numRegisters;
                let arity;
                let isNative;
                let nativeFn: Option<NativeFn>;
                let closureUpvalues: Vec<GcRef>;
                match &callee {
                    Value::Function(idx) => {
                        let module = self.module.as_ref().unwrap();
                        let func = &module.functions[*idx];
                        funcIndex = *idx;
                        numRegisters = func.numRegisters;
                        arity = func.arity;
                        isNative = false;
                        nativeFn = None;
                        closureUpvalues = Vec::new();
                    }
                    Value::Closure(data) => {
                        let module = self.module.as_ref().unwrap();
                        let func = &module.functions[data.funcIndex];
                        funcIndex = data.funcIndex;
                        numRegisters = func.numRegisters;
                        arity = func.arity;
                        isNative = false;
                        nativeFn = None;
                        closureUpvalues = data.upvalues.clone();
                    }
                    Value::NativeFn(f) => {
                        isNative = true;
                        nativeFn = Some(*f);
                        funcIndex = 0;
                        numRegisters = 0;
                        arity = 0;
                        closureUpvalues = Vec::new();
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "function",
                            found: callee.typeName(),
                        });
                    }
                }
                if !isNative && args.len() != arity {
                    return Err(RuntimeError::Custom(format!(
                        "expected {arity} arguments, got {}",
                        args.len()
                    )));
                }
                if isNative {
                    let fnPtr = nativeFn.unwrap();
                    let result = fnPtr(&args)?;
                    let frame = self.frames.last_mut().unwrap();
                    frame.registers[a] = result;
                    return Ok(());
                }
                let stackStart = self.stack.len();
                let returnAddr = {
                    let frame = self.frames.last().unwrap();
                    frame.ip
                };
                let mut newFrame = CallFrame::new(funcIndex, numRegisters, stackStart, returnAddr);
                for (i, arg) in args.iter().enumerate() {
                    newFrame.registers[i] = arg.clone();
                }
                newFrame.openUpvalues = closureUpvalues;
                for arg in args {
                    self.stack.push(arg);
                }
                self.frames.push(newFrame);
                return Ok(());
            }
            Opcode::Return => {
                let a = inst.a() as usize;
                let (retVal, stackStart, returnAddr) = {
                    let frame = self.frames.last().unwrap();
                    (
                        frame.registers[a].clone(),
                        frame.stackStart,
                        frame.returnAddr,
                    )
                };
                self.CloseFrameUpvalues();
                self.frames.pop();
                if self.frames.is_empty() {
                    self.stack.push(retVal);
                    return Ok(());
                }
                self.stack.truncate(stackStart);
                self.stack.push(retVal);
                let frame = self.frames.last_mut().unwrap();
                frame.ip = returnAddr;
                return Ok(());
            }
            Opcode::TailCall => {
                let a = inst.a() as usize;
                let argCount = inst.b() as usize;
                let (callee, args, stackStart) = {
                    let frame = self.frames.last().unwrap();
                    let callee = frame.registers[a].clone();
                    let mut args = Vec::with_capacity(argCount);
                    for i in 0..argCount {
                        args.push(frame.registers[a + 1 + i].clone());
                    }
                    (callee, args, frame.stackStart)
                };
                self.CloseFrameUpvalues();
                self.frames.pop();
                let funcIndex;
                let numRegisters;
                let arity;
                let closureUpvalues: Vec<GcRef>;
                match &callee {
                    Value::Function(idx) => {
                        let module = self.module.as_ref().unwrap();
                        let func = &module.functions[*idx];
                        funcIndex = *idx;
                        numRegisters = func.numRegisters;
                        arity = func.arity;
                        closureUpvalues = Vec::new();
                    }
                    Value::Closure(data) => {
                        let module = self.module.as_ref().unwrap();
                        let func = &module.functions[data.funcIndex];
                        funcIndex = data.funcIndex;
                        numRegisters = func.numRegisters;
                        arity = func.arity;
                        closureUpvalues = data.upvalues.clone();
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "function",
                            found: callee.typeName(),
                        });
                    }
                }
                if args.len() != arity {
                    return Err(RuntimeError::Custom(format!(
                        "expected {arity} arguments, got {}",
                        args.len()
                    )));
                }
                self.stack.truncate(stackStart);
                let mut newFrame = CallFrame::new(funcIndex, numRegisters, stackStart, 0);
                for (i, arg) in args.iter().enumerate() {
                    newFrame.registers[i] = arg.clone();
                }
                newFrame.openUpvalues = closureUpvalues;
                for arg in args {
                    self.stack.push(arg);
                }
                self.frames.push(newFrame);
                return Ok(());
            }
            Opcode::Closure => {
                let a = inst.a() as usize;
                let k = inst.imm8() as usize;
                let (funcIdx, targetUpvalueDescs) = {
                    let module = self.module.as_ref().unwrap();
                    let frame = self.frames.last().unwrap();
                    let func = &module.functions[frame.funcIndex];
                    if k >= func.constants.len() {
                        return Err(RuntimeError::Custom("constant index out of bounds".into()));
                    }
                    let funcIdx = match &func.constants[k] {
                        crate::constant_pool::Value::Function(idx) => *idx as usize,
                        _ => {
                            return Err(RuntimeError::Custom("expected function constant".into()));
                        }
                    };
                    let targetFunc = &module.functions[funcIdx];
                    (funcIdx, targetFunc.upvalueDescs.clone())
                };
                let mut upvalues = Vec::new();
                for upDesc in &targetUpvalueDescs {
                    if upDesc.isLocal {
                        let existing = {
                            let frame = self.frames.last().unwrap();
                            frame
                                .openUpvalues
                                .iter()
                                .find(|gcref| {
                                    self.heap.get(**gcref).is_some_and(|obj| {
                                        if let ObjectKind::Upvalue(inner) = &obj.kind {
                                            inner.registerIndex == upDesc.index
                                        } else {
                                            false
                                        }
                                    })
                                })
                                .copied()
                        };
                        if let Some(gcref) = existing {
                            upvalues.push(gcref);
                        } else {
                            let gcref = self
                                .heap
                                .alloc(crate::vm::memory::GcObject::new_upvalue(upDesc.index));
                            let frame = self.frames.last_mut().unwrap();
                            frame.openUpvalues.push(gcref);
                            upvalues.push(gcref);
                        }
                    } else {
                        let frame = self.frames.last().unwrap();
                        if upDesc.index < frame.openUpvalues.len() {
                            upvalues.push(frame.openUpvalues[upDesc.index]);
                        } else {
                            return Err(RuntimeError::Custom(
                                "inherited upvalue index out of bounds".into(),
                            ));
                        }
                    }
                }
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Closure(ClosureData {
                    funcIndex: funcIdx,
                    upvalues,
                });
            }
            Opcode::LoadUpvalue => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let val = {
                    let frame = self.frames.last().unwrap();
                    let upvalueRef = *frame.openUpvalues.get(b).ok_or_else(|| {
                        RuntimeError::Custom("upvalue index out of bounds".into())
                    })?;
                    let obj = self
                        .heap
                        .get(upvalueRef)
                        .ok_or_else(|| RuntimeError::Custom("upvalue not found".into()))?;
                    match &obj.kind {
                        ObjectKind::Upvalue(inner) => {
                            if inner.closed {
                                inner.value.clone()
                            } else {
                                frame.registers[inner.registerIndex].clone()
                            }
                        }
                        _ => {
                            return Err(RuntimeError::Custom("expected upvalue object".into()));
                        }
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = val;
            }
            Opcode::StoreUpvalue => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let (val, upvalueRef) = {
                    let frame = self.frames.last().unwrap();
                    let val = frame.registers[b].clone();
                    let upvalueRef = *frame.openUpvalues.get(a).ok_or_else(|| {
                        RuntimeError::Custom("upvalue index out of bounds".into())
                    })?;
                    (val, upvalueRef)
                };
                let obj = self
                    .heap
                    .get_mut(upvalueRef)
                    .ok_or_else(|| RuntimeError::Custom("upvalue not found".into()))?;
                match &mut obj.kind {
                    ObjectKind::Upvalue(inner) => {
                        if inner.closed {
                            inner.value = val;
                        } else {
                            let frame = self.frames.last_mut().unwrap();
                            frame.registers[inner.registerIndex] = val;
                        }
                    }
                    _ => {
                        return Err(RuntimeError::Custom("expected upvalue object".into()));
                    }
                }
            }
            Opcode::CloseUpvalue => {
                let a = inst.a() as usize;
                self.CloseUpvaluesGE(a);
            }
            Opcode::NewObject => {
                let a = inst.a() as usize;
                let gcRef = self
                    .heap
                    .alloc(crate::vm::memory::GcObject::new_object(HashMap::new()));
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Object(gcRef);
            }
            Opcode::NewArray => {
                let a = inst.a() as usize;
                let gcRef = self
                    .heap
                    .alloc(crate::vm::memory::GcObject::new_array(Vec::new()));
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Array(gcRef);
            }
            Opcode::GetField => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let fieldIdx = inst.imm8() as usize;
                let (fieldName, objRef) = {
                    let module = self.module.as_ref().unwrap();
                    let frame = self.frames.last().unwrap();
                    let func = &module.functions[frame.funcIndex];
                    if fieldIdx >= func.constants.len() {
                        return Err(RuntimeError::Custom("constant index out of bounds".into()));
                    }
                    let fieldName = match &func.constants[fieldIdx] {
                        crate::constant_pool::Value::String(s) => s.clone(),
                        _ => {
                            return Err(RuntimeError::Custom("expected string constant".into()));
                        }
                    };
                    let objRef = match &frame.registers[b] {
                        Value::Object(gc) => *gc,
                        _ => {
                            return Err(RuntimeError::TypeMismatch {
                                expected: "object",
                                found: frame.registers[b].typeName(),
                            });
                        }
                    };
                    (fieldName, objRef)
                };
                let obj = self.heap.get(objRef).ok_or(RuntimeError::NullReference)?;
                let val = match &obj.kind {
                    ObjectKind::Object(fields) => {
                        fields.get(&fieldName).cloned().unwrap_or(Value::Nil)
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "object",
                            found: "non-object",
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = val;
            }
            Opcode::SetField => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let fieldIdx = inst.imm8() as usize;
                let (fieldName, val, objRef) = {
                    let module = self.module.as_ref().unwrap();
                    let frame = self.frames.last().unwrap();
                    let func = &module.functions[frame.funcIndex];
                    if fieldIdx >= func.constants.len() {
                        return Err(RuntimeError::Custom("constant index out of bounds".into()));
                    }
                    let fieldName = match &func.constants[fieldIdx] {
                        crate::constant_pool::Value::String(s) => s.clone(),
                        _ => {
                            return Err(RuntimeError::Custom("expected string constant".into()));
                        }
                    };
                    let val = frame.registers[a].clone();
                    let objRef = match &frame.registers[b] {
                        Value::Object(gc) => *gc,
                        _ => {
                            return Err(RuntimeError::TypeMismatch {
                                expected: "object",
                                found: frame.registers[b].typeName(),
                            });
                        }
                    };
                    (fieldName, val, objRef)
                };
                let obj = self
                    .heap
                    .get_mut(objRef)
                    .ok_or(RuntimeError::NullReference)?;
                match &mut obj.kind {
                    ObjectKind::Object(fields) => {
                        fields.insert(fieldName, val);
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "object",
                            found: "non-object",
                        });
                    }
                }
            }
            Opcode::AGet => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (arrRef, index) = {
                    let frame = self.frames.last().unwrap();
                    let arrRef = match &frame.registers[b] {
                        Value::Array(gc) => *gc,
                        _ => {
                            return Err(RuntimeError::TypeMismatch {
                                expected: "array",
                                found: frame.registers[b].typeName(),
                            });
                        }
                    };
                    let index = match &frame.registers[c] {
                        Value::Int(i) => *i as usize,
                        _ => {
                            return Err(RuntimeError::TypeMismatch {
                                expected: "int",
                                found: frame.registers[c].typeName(),
                            });
                        }
                    };
                    (arrRef, index)
                };
                let arr = self.heap.get(arrRef).ok_or(RuntimeError::NullReference)?;
                let val = match &arr.kind {
                    ObjectKind::Array(elems) => {
                        if index >= elems.len() {
                            return Err(RuntimeError::IndexOutOfBounds {
                                len: elems.len(),
                                index,
                            });
                        }
                        elems[index].clone()
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "array",
                            found: "non-array",
                        });
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = val;
            }
            Opcode::ASet => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let c = inst.c() as usize;
                let (arrRef, index, val) = {
                    let frame = self.frames.last().unwrap();
                    let arrRef = match &frame.registers[a] {
                        Value::Array(gc) => *gc,
                        _ => {
                            return Err(RuntimeError::TypeMismatch {
                                expected: "array",
                                found: frame.registers[a].typeName(),
                            });
                        }
                    };
                    let index = match &frame.registers[b] {
                        Value::Int(i) => *i as usize,
                        _ => {
                            return Err(RuntimeError::TypeMismatch {
                                expected: "int",
                                found: frame.registers[b].typeName(),
                            });
                        }
                    };
                    let val = frame.registers[c].clone();
                    (arrRef, index, val)
                };
                let arr = self
                    .heap
                    .get_mut(arrRef)
                    .ok_or(RuntimeError::NullReference)?;
                match &mut arr.kind {
                    ObjectKind::Array(elems) => {
                        if index >= elems.len() {
                            elems.resize(index + 1, Value::Nil);
                        }
                        elems[index] = val;
                    }
                    _ => {
                        return Err(RuntimeError::TypeMismatch {
                            expected: "array",
                            found: "non-array",
                        });
                    }
                }
            }
            Opcode::ALen => {
                let a = inst.a() as usize;
                let b = inst.b() as usize;
                let len = {
                    let frame = self.frames.last().unwrap();
                    match &frame.registers[b] {
                        Value::Array(gc) => {
                            let arr = self.heap.get(*gc).ok_or(RuntimeError::NullReference)?;
                            match &arr.kind {
                                ObjectKind::Array(elems) => elems.len() as i64,
                                _ => {
                                    return Err(RuntimeError::TypeMismatch {
                                        expected: "array",
                                        found: "non-array",
                                    });
                                }
                            }
                        }
                        Value::String(s) => s.len() as i64,
                        _ => {
                            return Err(RuntimeError::TypeMismatch {
                                expected: "array or string",
                                found: frame.registers[b].typeName(),
                            });
                        }
                    }
                };
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Int(len);
            }
            Opcode::Import => {
                let a = inst.a() as usize;
                let frame = self.frames.last_mut().unwrap();
                frame.registers[a] = Value::Nil;
            }
            Opcode::Export => {}
            Opcode::Throw => {
                let a = inst.a() as usize;
                let val = {
                    let frame = self.frames.last().unwrap();
                    frame.registers[a].clone()
                };
                return Err(RuntimeError::Custom(format!("uncaught throw: {val:?}")));
            }
            Opcode::Try => {
                let _catchOffset = inst.imm16() as usize;
            }
            Opcode::EndTry => {}
            Opcode::Wide => {}
            Opcode::Line => {}
        }
        Ok(())
    }

    fn CloseFrameUpvalues(&mut self) {
        let upvalues: Vec<GcRef> = {
            let frame = self.frames.last().unwrap();
            frame.openUpvalues.clone()
        };
        for gcref in &upvalues {
            if let Some(obj) = self.heap.get_mut(*gcref) {
                if let ObjectKind::Upvalue(inner) = &mut obj.kind {
                    if !inner.closed {
                        let frame = self.frames.last().unwrap();
                        inner.value = frame.registers[inner.registerIndex].clone();
                        inner.closed = true;
                    }
                }
            }
        }
    }

    fn CloseUpvaluesGE(&mut self, register: usize) {
        let upvalues: Vec<GcRef> = {
            let frame = self.frames.last().unwrap();
            frame
                .openUpvalues
                .iter()
                .filter(|gcref| {
                    self.heap.get(**gcref).is_some_and(|obj| {
                        matches!(&obj.kind, ObjectKind::Upvalue(inner) if inner.registerIndex >= register)
                    })
                })
                .copied()
                .collect()
        };
        for gcref in &upvalues {
            if let Some(obj) = self.heap.get_mut(*gcref) {
                if let ObjectKind::Upvalue(inner) = &mut obj.kind {
                    if !inner.closed {
                        let frame = self.frames.last().unwrap();
                        inner.value = frame.registers[inner.registerIndex].clone();
                        inner.closed = true;
                    }
                }
            }
        }
    }
}
