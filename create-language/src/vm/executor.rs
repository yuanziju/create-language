use crate::binary::ModuleFile;
use crate::constant_pool::Value as CpValue;
use crate::instruction::Instruction;
use crate::opcode::Opcode;

use super::error::*;
use super::memory::*;

type Handler = fn(&mut Executor, Instruction) -> VmResult<()>;

pub struct DispatchTable {
    handlers: [Handler; 256],
}

impl Default for DispatchTable {
    fn default() -> Self {
        Self::new()
    }
}

impl DispatchTable {
    pub fn new() -> Self {
        let mut table = [handle_unimplemented as Handler; 256];
        table[Opcode::Halt as usize] = handle_halt;
        table[Opcode::Mov as usize] = handle_mov;
        table[Opcode::Loadk as usize] = handle_loadk;
        table[Opcode::Loadi as usize] = handle_loadi;
        table[Opcode::Loadbool as usize] = handle_loadbool;
        table[Opcode::Loadnil as usize] = handle_loadnil;
        table[Opcode::Add as usize] = handle_add;
        table[Opcode::Sub as usize] = handle_sub;
        table[Opcode::Mul as usize] = handle_mul;
        table[Opcode::Div as usize] = handle_div;
        table[Opcode::Mod as usize] = handle_mod;
        table[Opcode::Neg as usize] = handle_neg;
        table[Opcode::BitAnd as usize] = handle_bit_and;
        table[Opcode::BitOr as usize] = handle_bit_or;
        table[Opcode::BitXor as usize] = handle_bit_xor;
        table[Opcode::BitNot as usize] = handle_bit_not;
        table[Opcode::Shl as usize] = handle_shl;
        table[Opcode::Shr as usize] = handle_shr;
        table[Opcode::I2f as usize] = handle_i2f;
        table[Opcode::F2i as usize] = handle_f2i;
        table[Opcode::Eq as usize] = handle_eq;
        table[Opcode::Lt as usize] = handle_lt;
        table[Opcode::Le as usize] = handle_le;
        table[Opcode::Not as usize] = handle_not;
        table[Opcode::IsType as usize] = handle_is_type;
        table[Opcode::Jmp as usize] = handle_jmp;
        table[Opcode::JmpT as usize] = handle_jmp_t;
        table[Opcode::JmpF as usize] = handle_jmp_f;
        table[Opcode::Call as usize] = handle_call;
        table[Opcode::Return as usize] = handle_return;
        table[Opcode::TailCall as usize] = handle_tail_call;
        table[Opcode::Closure as usize] = handle_closure;
        table[Opcode::LoadUpvalue as usize] = handle_load_upvalue;
        table[Opcode::StoreUpvalue as usize] = handle_store_upvalue;
        table[Opcode::CloseUpvalue as usize] = handle_close_upvalue;
        table[Opcode::NewObject as usize] = handle_new_object;
        table[Opcode::NewArray as usize] = handle_new_array;
        table[Opcode::GetField as usize] = handle_get_field;
        table[Opcode::SetField as usize] = handle_set_field;
        table[Opcode::AGet as usize] = handle_aget;
        table[Opcode::ASet as usize] = handle_aset;
        table[Opcode::ALen as usize] = handle_alen;
        table[Opcode::Import as usize] = handle_import;
        table[Opcode::Export as usize] = handle_export;
        table[Opcode::Throw as usize] = handle_throw;
        table[Opcode::Try as usize] = handle_try;
        table[Opcode::EndTry as usize] = handle_end_try;
        table[Opcode::Wide as usize] = handle_wide;
        table[Opcode::Line as usize] = handle_line;
        DispatchTable { handlers: table }
    }

    pub fn Get(&self, opcode: Opcode) -> Handler {
        self.handlers[opcode as usize]
    }

    pub fn GetMut(&mut self) -> &mut [Handler; 256] {
        &mut self.handlers
    }
}

pub struct Executor {
    pub dispatchTable: DispatchTable,
    pub ip: usize,
    pub frames: Vec<CallFrame>,
    pub stack: Vec<RuntimeValue>,
    pub heap: Heap,
    pub module: Option<ModuleFile>,
    pub halted: bool,
    pub tryStack: Vec<usize>,
    pub currentFuncIndex: usize,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            dispatchTable: DispatchTable::new(),
            ip: 0,
            frames: Vec::new(),
            stack: Vec::new(),
            heap: Heap::new(),
            module: None,
            halted: false,
            tryStack: Vec::new(),
            currentFuncIndex: 0,
        }
    }

    pub fn Execute(&mut self) -> VmResult<()> {
        if self.module.is_none() {
            return Err(RuntimeError::new(ErrorKind::Custom, "no module loaded"));
        }
        if self.frames.is_empty() {
            return Err(RuntimeError::new(ErrorKind::Custom, "no active frame"));
        }
        self.halted = false;
        while !self.halted {
            let inst = self.fetch();
            let handler = self.dispatchTable.handlers[inst.opcode() as usize];
            handler(self, inst)?;
        }
        Ok(())
    }

    pub fn LoadModule(&mut self, module: ModuleFile) {
        self.module = Some(module);
    }

    pub fn Reset(&mut self) {
        self.ip = 0;
        self.frames.clear();
        self.stack.clear();
        self.halted = false;
        self.tryStack.clear();
        self.currentFuncIndex = 0;
    }

    fn fetch(&mut self) -> Instruction {
        let ip = self.ip;
        self.ip += 1;
        let module = self.module.as_ref().expect("no module loaded");
        let funcIndex = self.frames[self.currentFuncIndex].funcIndex;
        module.functions[funcIndex].instructions[ip]
    }

    fn frame(&self) -> &CallFrame {
        &self.frames[self.currentFuncIndex]
    }

    #[allow(dead_code)]
    fn frameMut(&mut self) -> &mut CallFrame {
        let idx = self.currentFuncIndex;
        &mut self.frames[idx]
    }

    fn reg(&self, idx: u8) -> &RuntimeValue {
        &self.frames[self.currentFuncIndex].registers[idx as usize]
    }

    fn regMut(&mut self, idx: u8) -> &mut RuntimeValue {
        let frameIdx = self.currentFuncIndex;
        &mut self.frames[frameIdx].registers[idx as usize]
    }

    fn constant(&self, idx: u16) -> RuntimeValue {
        let module = self.module.as_ref().expect("no module loaded");
        let funcIndex = self.frames[self.currentFuncIndex].funcIndex;
        let func = &module.functions[funcIndex];
        let cpVal = &func.constants[idx as usize];
        match cpVal {
            CpValue::Nil => RuntimeValue::Nil,
            CpValue::Bool(b) => RuntimeValue::Bool(*b),
            CpValue::Int(i) => RuntimeValue::Int(*i),
            CpValue::Float(f) => RuntimeValue::Float(*f),
            CpValue::String(s) => RuntimeValue::String(s.clone()),
            CpValue::Function(f) => RuntimeValue::Function(*f as usize),
        }
    }

    #[allow(dead_code)]
    fn push(&mut self, v: RuntimeValue) {
        self.stack.push(v);
    }

    fn getArithmetic(a: &RuntimeValue, b: &RuntimeValue) -> VmResult<(i64, i64, bool)> {
        match (a, b) {
            (RuntimeValue::Int(ia), RuntimeValue::Int(ib)) => Ok((*ia, *ib, false)),
            (RuntimeValue::Float(fa), RuntimeValue::Float(fb)) => {
                Ok((fa.to_bits() as i64, fb.to_bits() as i64, true))
            }
            (RuntimeValue::Int(ia), RuntimeValue::Float(fb)) => {
                Ok((*ia, fb.to_bits() as i64, true))
            }
            (RuntimeValue::Float(fa), RuntimeValue::Int(ib)) => {
                Ok((fa.to_bits() as i64, *ib, true))
            }
            _ => Err(RuntimeError::type_error("number", "non-number")),
        }
    }

    fn doArithmetic(
        &mut self,
        a: u8,
        b: u8,
        c: u8,
        op: fn(i64, i64) -> i64,
        floatOp: fn(f64, f64) -> f64,
    ) -> VmResult<()> {
        let (bv, cv, isFloat) = {
            let bVal = self.reg(b).clone();
            let cVal = self.reg(c).clone();
            Self::getArithmetic(&bVal, &cVal)?
        };
        let result = if isFloat {
            let bf = f64::from_bits(bv as u64);
            let cf = f64::from_bits(cv as u64);
            RuntimeValue::Float(floatOp(bf, cf))
        } else {
            RuntimeValue::Int(op(bv, cv))
        };
        *self.regMut(a) = result;
        Ok(())
    }
}

// ---- Handler implementations ----

fn handle_unimplemented(_exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    Err(RuntimeError::new(
        ErrorKind::Custom,
        format!("unimplemented instruction: {:?}", inst.opcode()),
    ))
}

fn handle_halt(exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    exec.halted = true;
    Ok(())
}

fn handle_mov(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    *exec.regMut(a) = val;
    Ok(())
}

fn handle_loadk(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let k = inst.c() as u16;
    let val = exec.constant(k);
    *exec.regMut(a) = val;
    Ok(())
}

fn handle_loadi(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let imm = inst.imm16() as i64;
    *exec.regMut(a) = RuntimeValue::Int(imm);
    Ok(())
}

fn handle_loadbool(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.c() != 0;
    *exec.regMut(a) = RuntimeValue::Bool(b);
    Ok(())
}

fn handle_loadnil(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    *exec.regMut(a) = RuntimeValue::Nil;
    Ok(())
}

fn handle_add(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x + y, |x, y| x + y)
}

fn handle_sub(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x - y, |x, y| x - y)
}

fn handle_mul(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x * y, |x, y| x * y)
}

fn handle_div(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let bVal = exec.reg(b).clone();
    let cVal = exec.reg(c).clone();
    match (&bVal, &cVal) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
            if *r == 0 {
                return Err(RuntimeError::new(
                    ErrorKind::DivisionByZero,
                    "division by zero",
                ));
            }
            *exec.regMut(a) = RuntimeValue::Int(l / r);
        }
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
            *exec.regMut(a) = RuntimeValue::Float(l / r);
        }
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => {
            *exec.regMut(a) = RuntimeValue::Float(*l as f64 / r);
        }
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => {
            if *r == 0 {
                return Err(RuntimeError::new(
                    ErrorKind::DivisionByZero,
                    "division by zero",
                ));
            }
            *exec.regMut(a) = RuntimeValue::Float(l / *r as f64);
        }
        _ => return Err(RuntimeError::type_error("number", "non-number")),
    }
    Ok(())
}

fn handle_mod(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let bVal = exec.reg(b).clone();
    let cVal = exec.reg(c).clone();
    match (&bVal, &cVal) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
            if *r == 0 {
                return Err(RuntimeError::new(
                    ErrorKind::DivisionByZero,
                    "modulo by zero",
                ));
            }
            *exec.regMut(a) = RuntimeValue::Int(l % r);
            Ok(())
        }
        _ => Err(RuntimeError::type_error("integer", "non-integer")),
    }
}

fn handle_neg(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    match val {
        RuntimeValue::Int(i) => *exec.regMut(a) = RuntimeValue::Int(-i),
        RuntimeValue::Float(f) => *exec.regMut(a) = RuntimeValue::Float(-f),
        _ => return Err(RuntimeError::type_error("number", "non-number")),
    }
    Ok(())
}

fn handle_bit_and(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x & y, |_, _| 0.0)
}

fn handle_bit_or(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x | y, |_, _| 0.0)
}

fn handle_bit_xor(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x ^ y, |_, _| 0.0)
}

fn handle_bit_not(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    match val {
        RuntimeValue::Int(i) => *exec.regMut(a) = RuntimeValue::Int(!i),
        _ => return Err(RuntimeError::type_error("integer", "non-integer")),
    }
    Ok(())
}

fn handle_shl(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x << y, |_, _| 0.0)
}

fn handle_shr(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    exec.doArithmetic(inst.a(), inst.b(), inst.c(), |x, y| x >> y, |_, _| 0.0)
}

fn handle_i2f(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    match val {
        RuntimeValue::Int(i) => *exec.regMut(a) = RuntimeValue::Float(i as f64),
        _ => return Err(RuntimeError::type_error("integer", "non-integer")),
    }
    Ok(())
}

fn handle_f2i(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    match val {
        RuntimeValue::Float(f) => *exec.regMut(a) = RuntimeValue::Int(f as i64),
        _ => return Err(RuntimeError::type_error("float", "non-float")),
    }
    Ok(())
}

fn handle_eq(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let bVal = exec.reg(b).clone();
    let cVal = exec.reg(c).clone();
    *exec.regMut(a) = RuntimeValue::Bool(bVal == cVal);
    Ok(())
}

fn handle_lt(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let bVal = exec.reg(b).clone();
    let cVal = exec.reg(c).clone();
    let result = match (&bVal, &cVal) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => l < r,
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => l < r,
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => (*l as f64) < *r,
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => *l < (*r as f64),
        _ => return Err(RuntimeError::type_error("number", "non-number")),
    };
    *exec.regMut(a) = RuntimeValue::Bool(result);
    Ok(())
}

fn handle_le(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let bVal = exec.reg(b).clone();
    let cVal = exec.reg(c).clone();
    let result = match (&bVal, &cVal) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => l <= r,
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => l <= r,
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => (*l as f64) <= *r,
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => *l <= (*r as f64),
        _ => return Err(RuntimeError::type_error("number", "non-number")),
    };
    *exec.regMut(a) = RuntimeValue::Bool(result);
    Ok(())
}

fn handle_not(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b);
    *exec.regMut(a) = RuntimeValue::Bool(!val.is_truthy());
    Ok(())
}

fn handle_is_type(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let typeCode = inst.c();
    let val = exec.reg(b);
    let matches = match typeCode {
        0 => matches!(val, RuntimeValue::Nil),
        1 => matches!(val, RuntimeValue::Bool(_)),
        2 => matches!(val, RuntimeValue::Int(_)),
        3 => matches!(val, RuntimeValue::Float(_)),
        4 => matches!(val, RuntimeValue::String(_)),
        5 => matches!(val, RuntimeValue::Function(_) | RuntimeValue::Closure(_, _)),
        _ => false,
    };
    *exec.regMut(a) = RuntimeValue::Bool(matches);
    Ok(())
}

fn handle_jmp(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let offset = if a == 0 {
        inst.imm16_signed() as i32
    } else {
        inst.imm24_signed()
    };
    // offset is relative to the start of the jump instruction
    exec.ip = ((exec.ip as i32) + offset - 1) as usize;
    Ok(())
}

fn handle_jmp_t(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let val = exec.reg(a);
    if val.is_truthy() {
        let offset = inst.imm16_signed() as i32;
        // offset is relative to the start of the jump instruction
        exec.ip = ((exec.ip as i32) + offset - 1) as usize;
    }
    Ok(())
}

fn handle_jmp_f(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let val = exec.reg(a);
    if !val.is_truthy() {
        let offset = inst.imm16_signed() as i32;
        // offset is relative to the start of the jump instruction
        exec.ip = ((exec.ip as i32) + offset - 1) as usize;
    }
    Ok(())
}

fn handle_call(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let _a = inst.a(); // return value register
    let b = inst.b(); // function register
    let c = inst.c(); // argument count
    let funcVal = exec.reg(b).clone();
    let argCount = c as usize;

    let (funcIndex, upvalues) = match &funcVal {
        RuntimeValue::Function(idx) => (*idx, Vec::new()),
        RuntimeValue::Closure(idx, upvs) => (*idx, upvs.clone()),
        _ => return Err(RuntimeError::type_error("function", "non-function")),
    };

    // Extract function info before mutable operations on exec
    let (arity, numRegisters, upvalueDescs) = {
        let module = exec.module.as_ref().expect("no module loaded");
        let func = &module.functions[funcIndex];
        (func.arity, func.numRegisters, func.upvalueDescs.clone())
    };

    if arity != argCount {
        return Err(RuntimeError::new(
            ErrorKind::ArityMismatch,
            format!("function expects {arity} arguments, got {argCount}"),
        ));
    }

    // Read arguments from registers starting at b+1
    let mut args = Vec::with_capacity(argCount);
    for i in 0..argCount {
        args.push(exec.reg(b + 1 + i as u8).clone());
    }

    // Create registers
    let mut registers = Vec::with_capacity(numRegisters);
    registers.extend(args);
    registers.resize(numRegisters, RuntimeValue::Nil);

    // Set up upvalues
    let mut frameUpvalues = Vec::new();
    for upvDesc in &upvalueDescs {
        let idx = upvDesc.index;
        if idx < upvalues.len() {
            frameUpvalues.push(upvalues[idx]);
        } else {
            frameUpvalues.push(
                exec.heap
                    .allocObj(GcObject::new_upvalue(RuntimeValue::Nil, false)),
            );
        }
    }

    let returnAddr = exec.ip;
    let stackStart = exec.stack.len();

    exec.frames.push(CallFrame {
        funcIndex,
        ip: 0,
        registers,
        stackStart,
        returnAddr,
        upvalues: frameUpvalues,
    });

    exec.currentFuncIndex = exec.frames.len() - 1;
    exec.ip = 0;
    Ok(())
}

fn handle_return(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let retVal = exec.reg(a).clone();

    // Pop the current frame
    if exec.frames.is_empty() {
        exec.halted = true;
        return Ok(());
    }

    let frame = exec.frames.pop().expect("no frame to return from");
    let returnAddr = frame.returnAddr;

    // Restore stack and push return value
    exec.stack.truncate(frame.stackStart);
    exec.stack.push(retVal);

    if exec.frames.is_empty() {
        // Returned from main function
        exec.halted = true;
        return Ok(());
    }

    exec.currentFuncIndex = exec.frames.len() - 1;
    exec.ip = returnAddr;
    Ok(())
}

fn handle_tail_call(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a(); // function register
    let b = inst.b(); // argument register start
    let c = inst.c(); // argument count
    let funcVal = exec.reg(a).clone();
    let argCount = c as usize;

    let (funcIndex, upvalues) = match &funcVal {
        RuntimeValue::Function(idx) => (*idx, Vec::new()),
        RuntimeValue::Closure(idx, upvs) => (*idx, upvs.clone()),
        _ => return Err(RuntimeError::type_error("function", "non-function")),
    };

    // Extract function info before mutable operations
    let (arity, numRegisters, upvalueDescs) = {
        let module = exec.module.as_ref().expect("no module loaded");
        let func = &module.functions[funcIndex];
        (func.arity, func.numRegisters, func.upvalueDescs.clone())
    };

    if arity != argCount {
        return Err(RuntimeError::new(
            ErrorKind::ArityMismatch,
            format!("function expects {arity} arguments, got {argCount}"),
        ));
    }

    // Read arguments from registers starting at b
    let mut args = Vec::with_capacity(argCount);
    for i in 0..argCount {
        args.push(exec.reg(b + i as u8).clone());
    }

    // Reuse current frame
    let mut registers = Vec::with_capacity(numRegisters);
    registers.extend(args);
    registers.resize(numRegisters, RuntimeValue::Nil);

    let mut frameUpvalues = Vec::new();
    for upvDesc in &upvalueDescs {
        let idx = upvDesc.index;
        if idx < upvalues.len() {
            frameUpvalues.push(upvalues[idx]);
        } else {
            frameUpvalues.push(
                exec.heap
                    .allocObj(GcObject::new_upvalue(RuntimeValue::Nil, false)),
            );
        }
    }

    // Pop current frame and reuse its return address
    let currentReturnAddr = exec.frames.pop().map(|f| f.returnAddr).unwrap_or(0);
    let stackStart = exec.stack.len();

    exec.frames.push(CallFrame {
        funcIndex,
        ip: 0,
        registers,
        stackStart,
        returnAddr: currentReturnAddr,
        upvalues: frameUpvalues,
    });

    exec.currentFuncIndex = exec.frames.len() - 1;
    exec.ip = 0;
    Ok(())
}

fn handle_closure(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let funcIndex = b as usize;
    let upvalueCount = c as usize;

    // For now, collect upvalues from the current frame's upvalues
    let mut upvalues = Vec::new();
    let module = exec.module.as_ref().expect("no module loaded");
    let func = &module.functions[funcIndex];
    for upvDesc in &func.upvalueDescs {
        let idx = upvDesc.index;
        let frameIdx = if exec.frames.len() > 1 {
            exec.frames.len() - 2
        } else {
            0
        };
        if frameIdx < exec.frames.len() {
            let val = exec.frames[frameIdx].registers[idx].clone();
            let gcRef = exec.heap.allocObj(GcObject::new_upvalue(val, false));
            upvalues.push(gcRef);
        } else {
            let gcRef = exec
                .heap
                .allocObj(GcObject::new_upvalue(RuntimeValue::Nil, false));
            upvalues.push(gcRef);
        }
    }
    // Ensure we have the right count
    upvalues.truncate(upvalueCount);

    *exec.regMut(a) = RuntimeValue::Closure(funcIndex, upvalues);
    Ok(())
}

fn handle_load_upvalue(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let upvIdx = b as usize;
    let frame = exec.frame();
    if upvIdx < frame.upvalues.len() {
        let gcRef = frame.upvalues[upvIdx];
        let obj = exec.heap.get(gcRef);
        let val = match &obj.kind {
            ObjectKind::UpvalueData(v, _) => v.clone(),
            _ => RuntimeValue::Nil,
        };
        *exec.regMut(a) = val;
        Ok(())
    } else {
        Err(RuntimeError::new(
            ErrorKind::IndexOutOfBounds,
            format!("upvalue index {upvIdx} out of bounds"),
        ))
    }
}

fn handle_store_upvalue(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let upvIdx = b as usize;
    let val = exec.reg(a).clone();
    let frame = exec.frame();
    if upvIdx < frame.upvalues.len() {
        let gcRef = frame.upvalues[upvIdx];
        let obj = exec.heap.getMut(gcRef);
        if let ObjectKind::UpvalueData(ref mut v, ref mut closed) = obj.kind {
            *v = val;
            *closed = true;
        }
        Ok(())
    } else {
        Err(RuntimeError::new(
            ErrorKind::IndexOutOfBounds,
            format!("upvalue index {upvIdx} out of bounds"),
        ))
    }
}

fn handle_close_upvalue(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let val = exec.reg(a).clone();
    *exec.regMut(a) = val;
    Ok(())
}

fn handle_new_object(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let gcRef = exec.heap.alloc(ObjectKind::Object(Vec::new()));
    *exec.regMut(a) = RuntimeValue::Object(gcRef);
    Ok(())
}

fn handle_new_array(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let gcRef = exec.heap.alloc(ObjectKind::Array(Vec::new()));
    *exec.regMut(a) = RuntimeValue::Array(gcRef);
    Ok(())
}

fn handle_get_field(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let fieldIdx = inst.c() as u16;
    let objVal = exec.reg(b).clone();
    let fieldName = match &objVal {
        RuntimeValue::Object(_gcRef) => {
            let module = exec.module.as_ref().expect("no module loaded");
            let funcIndex = exec.frames[exec.currentFuncIndex].funcIndex;
            let func = &module.functions[funcIndex];
            match &func.constants[fieldIdx as usize] {
                CpValue::String(s) => s.clone(),
                _ => {
                    return Err(RuntimeError::new(
                        ErrorKind::Custom,
                        "field name must be a string constant",
                    ))
                }
            }
        }
        _ => return Err(RuntimeError::type_error("object", "non-object")),
    };
    let gcRef = match &objVal {
        RuntimeValue::Object(r) => *r,
        _ => unreachable!(),
    };
    let obj = exec.heap.get(gcRef);
    match &obj.kind {
        ObjectKind::Object(fields) => {
            let val = fields
                .iter()
                .find(|(name, _)| name == &fieldName)
                .map(|(_, v)| v.clone())
                .unwrap_or(RuntimeValue::Nil);
            *exec.regMut(a) = val;
        }
        _ => return Err(RuntimeError::type_error("object", "non-object")),
    }
    Ok(())
}

fn handle_set_field(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a(); // value register
    let b = inst.b(); // object register
    let fieldIdx = inst.c() as u16;
    let val = exec.reg(a).clone();
    let objVal = exec.reg(b).clone();
    let fieldName = match &objVal {
        RuntimeValue::Object(_) => {
            let module = exec.module.as_ref().expect("no module loaded");
            let funcIndex = exec.frames[exec.currentFuncIndex].funcIndex;
            let func = &module.functions[funcIndex];
            match &func.constants[fieldIdx as usize] {
                CpValue::String(s) => s.clone(),
                _ => {
                    return Err(RuntimeError::new(
                        ErrorKind::Custom,
                        "field name must be a string constant",
                    ))
                }
            }
        }
        _ => return Err(RuntimeError::type_error("object", "non-object")),
    };
    let gcRef = match &objVal {
        RuntimeValue::Object(r) => *r,
        _ => unreachable!(),
    };
    let obj = exec.heap.getMut(gcRef);
    match &mut obj.kind {
        ObjectKind::Object(fields) => {
            if let Some((_, existing)) = fields.iter_mut().find(|(name, _)| name == &fieldName) {
                *existing = val;
            } else {
                fields.push((fieldName, val));
            }
        }
        _ => return Err(RuntimeError::type_error("object", "non-object")),
    }
    Ok(())
}

fn handle_aget(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let arrVal = exec.reg(b).clone();
    let idxVal = exec.reg(c).clone();
    let idx = match idxVal {
        RuntimeValue::Int(i) => i as usize,
        _ => return Err(RuntimeError::type_error("integer", "non-integer index")),
    };
    let gcRef = match &arrVal {
        RuntimeValue::Array(r) => *r,
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };
    let obj = exec.heap.get(gcRef);
    match &obj.kind {
        ObjectKind::Array(elements) => {
            let val = elements.get(idx).cloned().unwrap_or(RuntimeValue::Nil);
            *exec.regMut(a) = val;
        }
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    }
    Ok(())
}

fn handle_aset(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a(); // array ref register
    let b = inst.b(); // index register
    let c = inst.c(); // value register
    let arrVal = exec.reg(a).clone();
    let idxVal = exec.reg(b).clone();
    let val = exec.reg(c).clone();
    let idx = match idxVal {
        RuntimeValue::Int(i) => i as usize,
        _ => return Err(RuntimeError::type_error("integer", "non-integer index")),
    };
    let gcRef = match &arrVal {
        RuntimeValue::Array(r) => *r,
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };
    let obj = exec.heap.getMut(gcRef);
    match &mut obj.kind {
        ObjectKind::Array(elements) => {
            if idx >= elements.len() {
                elements.resize(idx + 1, RuntimeValue::Nil);
            }
            elements[idx] = val;
        }
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    }
    Ok(())
}

fn handle_alen(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let arrVal = exec.reg(b).clone();
    let gcRef = match &arrVal {
        RuntimeValue::Array(r) => *r,
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };
    let obj = exec.heap.get(gcRef);
    let len = match &obj.kind {
        ObjectKind::Array(elements) => elements.len(),
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };
    *exec.regMut(a) = RuntimeValue::Int(len as i64);
    Ok(())
}

fn handle_import(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    // Stub: import not yet implemented
    Ok(())
}

fn handle_export(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    // Stub: export not yet implemented
    Ok(())
}

fn handle_throw(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    Err(RuntimeError::new(ErrorKind::Custom, "uncaught throw"))
}

fn handle_try(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let _a = inst.a();
    let offset = inst.imm16_signed() as i32;
    let catchIp = ((exec.ip as i32) + offset - 1) as usize;
    exec.tryStack.push(catchIp);
    Ok(())
}

fn handle_end_try(exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    exec.tryStack.pop();
    Ok(())
}

fn handle_wide(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    // Wide prefix: ignore for now
    Ok(())
}

fn handle_line(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    // Debug info: ignore
    Ok(())
}
