use crate::binary::ModuleFile;
use crate::constant_pool::Value as CpValue;
use crate::instruction::Instruction;
use crate::opcode::Opcode;

use super::error::*;
use super::memory::*;

use super::heap::Heap;

pub type Handler = fn(&mut Executor, Instruction) -> VmResult<()>;

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
    pub globalRefs: Vec<GcRef>,
    pub inlineCache: InlineCache,
    pub callCounters: Vec<CallCounter>,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct InlineCache {
    pub cachedClass: Option<usize>,
    pub cachedFieldIdx: Option<usize>,
    pub hitCount: u64,
    pub missCount: u64,
}

impl InlineCache {
    pub fn HitRate(&self) -> f64 {
        let total = self.hitCount + self.missCount;
        if total == 0 { 0.0 } else { self.hitCount as f64 / total as f64 }
    }
}

#[derive(Debug)]
pub struct CallCounter {
    pub funcIndex: usize,
    pub count: u64,
    pub triggerTier: CompilationTier,
}

impl CallCounter {
    pub fn new(funcIndex: usize) -> Self {
        CallCounter { funcIndex, count: 0, triggerTier: CompilationTier::Interpreter }
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
            globalRefs: Vec::new(),
            inlineCache: InlineCache::default(),
            callCounters: Vec::new(),
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
        self.globalRefs.clear();
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

    #[allow(dead_code)]
    fn regMut(&mut self, idx: u8) -> &mut RuntimeValue {
        let frameIdx = self.currentFuncIndex;
        &mut self.frames[frameIdx].registers[idx as usize]
    }

    fn set_reg(&mut self, idx: u8, val: RuntimeValue) {
        let frameIdx = self.currentFuncIndex;
        let frame = &mut self.frames[frameIdx];
        if (idx as usize) >= frame.registers.len() {
            frame.registers.resize(idx as usize + 1, RuntimeValue::NIL.clone());
        }
        frame.registers[idx as usize] = val;
        frame.UpdateRegisterBitmap(idx as usize);
    }

    fn constant(&self, idx: u16) -> RuntimeValue {
        let module = self.module.as_ref().expect("no module loaded");
        let funcIndex = self.frames[self.currentFuncIndex].funcIndex;
        let func = &module.functions[funcIndex];
        let cpVal = &func.constants[idx as usize];
        match cpVal {
            CpValue::Nil => RuntimeValue::NIL.clone(),
            CpValue::Bool(b) => RuntimeValue::Bool(*b),
            CpValue::Int(i) => RuntimeValue::Int(*i),
            CpValue::Float(f) => RuntimeValue::Float(*f),
            CpValue::String(s) => RuntimeValue::Str(s.clone()),
            CpValue::Function(f) => RuntimeValue::Func(*f as usize),
        }
    }

    pub fn CollectRoots(&self) -> Vec<GcRef> {
        let mut roots = Vec::new();
        for frame in &self.frames {
            for (i, reg) in frame.registers.iter().enumerate() {
                let word = i / 64;
                let bit = i % 64;
                if word < frame.registers_bitmap.len() && (frame.registers_bitmap[word] & (1 << bit)) != 0 {
                    if let Some(r) = reg.as_gc_ref() {
                        roots.push(r);
                    }
                }
            }
            for u in &frame.upvalues {
                roots.push(*u);
            }
        }
        for val in &self.stack {
            if let Some(r) = val.as_gc_ref() {
                roots.push(r);
            }
        }
        for r in &self.globalRefs {
            roots.push(*r);
        }
        roots
    }

    pub fn CollectGarbage(&mut self) {
        let roots = self.CollectRoots();

        self.heap.MajorGc(&roots);

        let ft = self.heap.GetForwardingTable().clone();
        if !ft.is_empty() {
            for frame in &mut self.frames {
                for reg in &mut frame.registers {
                    Heap::ApplyForwardingToValue(reg, &ft);
                }
                for upv in &mut frame.upvalues {
                    if let Some(newRef) = ft.get(&upv.0) {
                        *upv = *newRef;
                    }
                }
            }
            for val in &mut self.stack {
                Heap::ApplyForwardingToValue(val, &ft);
            }
            for r in &mut self.globalRefs {
                if let Some(newRef) = ft.get(&r.0) {
                    *r = *newRef;
                }
            }
            for frame in &mut self.frames {
                frame.RebuildRegisterBitmap();
            }
        }
    }

    pub fn MinorGcNow(&mut self) {
        let roots = self.CollectRoots();
        self.heap.MinorGc(&roots);

        let ft = self.heap.GetForwardingTable().clone();
        if !ft.is_empty() {
            for frame in &mut self.frames {
                for reg in &mut frame.registers {
                    Heap::ApplyForwardingToValue(reg, &ft);
                }
                for upv in &mut frame.upvalues {
                    if let Some(newRef) = ft.get(&upv.0) {
                        *upv = *newRef;
                    }
                }
            }
            for val in &mut self.stack {
                Heap::ApplyForwardingToValue(val, &ft);
            }
            for r in &mut self.globalRefs {
                if let Some(newRef) = ft.get(&r.0) {
                    *r = *newRef;
                }
            }
            for frame in &mut self.frames {
                frame.RebuildRegisterBitmap();
            }
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
        let (isFloat, bv, cv) = {
            let bVal = self.reg(b).clone();
            let cVal = self.reg(c).clone();
            Self::get_arithmetic(&bVal, &cVal)?
        };
        let result = if isFloat {
            let bf = f64::from_bits(bv as u64);
            let cf = f64::from_bits(cv as u64);
            RuntimeValue::Float(floatOp(bf, cf))
        } else {
            RuntimeValue::Int(op(bv, cv))
        };
        self.set_reg(a, result);
        Ok(())
    }

    fn get_arithmetic(a: &RuntimeValue, b: &RuntimeValue) -> VmResult<(bool, i64, i64)> {
        match (&a.payload, &b.payload) {
            (ValuePayload::Int(ia), ValuePayload::Int(ib)) => Ok((false, *ia, *ib)),
            (ValuePayload::Float(fa), ValuePayload::Float(fb)) => Ok((true, fa.to_bits() as i64, fb.to_bits() as i64)),
            (ValuePayload::Int(ia), ValuePayload::Float(fb)) => Ok((true, *ia, fb.to_bits() as i64)),
            (ValuePayload::Float(fa), ValuePayload::Int(ib)) => Ok((true, fa.to_bits() as i64, *ib)),
            _ => Err(RuntimeError::type_error("number", "non-number")),
        }
    }

    #[allow(dead_code)]
    fn is_number(a: &RuntimeValue) -> bool {
        matches!(a.tag, ValueTag::Int | ValueTag::Float)
    }
}

// ---- Handler implementations ----

fn handle_unimplemented(_exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    Err(RuntimeError::new(ErrorKind::Custom, format!("unimplemented instruction: {:?}", inst.opcode())))
}

fn handle_halt(exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    exec.halted = true;
    Ok(())
}

fn handle_mov(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    exec.set_reg(a, val);
    Ok(())
}

fn handle_loadk(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let k = inst.c() as u16;
    let val = exec.constant(k);
    exec.set_reg(a, val);
    Ok(())
}

fn handle_loadi(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let imm = inst.imm16() as i64;
    exec.set_reg(a, RuntimeValue::Int(imm));
    Ok(())
}

fn handle_loadbool(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.c() != 0;
    exec.set_reg(a, RuntimeValue::Bool(b));
    Ok(())
}

fn handle_loadnil(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    exec.set_reg(a, RuntimeValue::NIL.clone());
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
    match (&bVal.payload, &cVal.payload) {
        (ValuePayload::Int(l), ValuePayload::Int(r)) => {
            if *r == 0 {
                return Err(RuntimeError::new(ErrorKind::DivisionByZero, "division by zero"));
            }
            exec.set_reg(a, RuntimeValue::Int(l / r));
        }
        (ValuePayload::Float(l), ValuePayload::Float(r)) => {
            exec.set_reg(a, RuntimeValue::Float(l / r));
        }
        (ValuePayload::Int(l), ValuePayload::Float(r)) => {
            exec.set_reg(a, RuntimeValue::Float(*l as f64 / r));
        }
        (ValuePayload::Float(l), ValuePayload::Int(r)) => {
            if *r == 0 {
                return Err(RuntimeError::new(ErrorKind::DivisionByZero, "division by zero"));
            }
            exec.set_reg(a, RuntimeValue::Float(l / *r as f64));
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
    match (&bVal.payload, &cVal.payload) {
        (ValuePayload::Int(l), ValuePayload::Int(r)) => {
            if *r == 0 {
                return Err(RuntimeError::new(ErrorKind::DivisionByZero, "modulo by zero"));
            }
            exec.set_reg(a, RuntimeValue::Int(l % r));
            Ok(())
        }
        _ => Err(RuntimeError::type_error("integer", "non-integer")),
    }
}

fn handle_neg(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    match val.payload {
        ValuePayload::Int(i) => exec.set_reg(a, RuntimeValue::Int(-i)),
        ValuePayload::Float(f) => exec.set_reg(a, RuntimeValue::Float(-f)),
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
    match val.payload {
        ValuePayload::Int(i) => exec.set_reg(a, RuntimeValue::Int(!i)),
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
    match val.payload {
        ValuePayload::Int(i) => exec.set_reg(a, RuntimeValue::Float(i as f64)),
        _ => return Err(RuntimeError::type_error("integer", "non-integer")),
    }
    Ok(())
}

fn handle_f2i(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b).clone();
    match val.payload {
        ValuePayload::Float(f) => exec.set_reg(a, RuntimeValue::Int(f as i64)),
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
    exec.set_reg(a, RuntimeValue::Bool(bVal == cVal));
    Ok(())
}

fn handle_lt(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let bVal = exec.reg(b).clone();
    let cVal = exec.reg(c).clone();
    let result = match (&bVal.payload, &cVal.payload) {
        (ValuePayload::Int(l), ValuePayload::Int(r)) => l < r,
        (ValuePayload::Float(l), ValuePayload::Float(r)) => l < r,
        (ValuePayload::Int(l), ValuePayload::Float(r)) => (*l as f64) < *r,
        (ValuePayload::Float(l), ValuePayload::Int(r)) => *l < (*r as f64),
        _ => return Err(RuntimeError::type_error("number", "non-number")),
    };
    exec.set_reg(a, RuntimeValue::Bool(result));
    Ok(())
}

fn handle_le(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let bVal = exec.reg(b).clone();
    let cVal = exec.reg(c).clone();
    let result = match (&bVal.payload, &cVal.payload) {
        (ValuePayload::Int(l), ValuePayload::Int(r)) => l <= r,
        (ValuePayload::Float(l), ValuePayload::Float(r)) => l <= r,
        (ValuePayload::Int(l), ValuePayload::Float(r)) => (*l as f64) <= *r,
        (ValuePayload::Float(l), ValuePayload::Int(r)) => *l <= (*r as f64),
        _ => return Err(RuntimeError::type_error("number", "non-number")),
    };
    exec.set_reg(a, RuntimeValue::Bool(result));
    Ok(())
}

fn handle_not(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let val = exec.reg(b);
    exec.set_reg(a, RuntimeValue::Bool(!val.is_truthy()));
    Ok(())
}

fn handle_is_type(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let typeCode = inst.c();
    let val = exec.reg(b);
    let matches = match typeCode {
        0 => matches!(val.tag, ValueTag::Nil),
        1 => matches!(val.tag, ValueTag::Bool),
        2 => matches!(val.tag, ValueTag::Int),
        3 => matches!(val.tag, ValueTag::Float),
        4 => matches!(val.tag, ValueTag::Str),
        5 => matches!(val.tag, ValueTag::Func | ValueTag::Closure),
        _ => false,
    };
    exec.set_reg(a, RuntimeValue::Bool(matches));
    Ok(())
}

fn handle_jmp(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let offset = if a == 0 { inst.imm16_signed() as i32 } else { inst.imm24_signed() };
    exec.ip = ((exec.ip as i32) + offset - 1) as usize;
    Ok(())
}

fn handle_jmp_t(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let val = exec.reg(a);
    if val.is_truthy() {
        let offset = inst.imm16_signed() as i32;
        exec.ip = ((exec.ip as i32) + offset - 1) as usize;
    }
    Ok(())
}

fn handle_jmp_f(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let val = exec.reg(a);
    if !val.is_truthy() {
        let offset = inst.imm16_signed() as i32;
        exec.ip = ((exec.ip as i32) + offset - 1) as usize;
    }
    Ok(())
}

fn handle_call(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let _a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let funcVal = exec.reg(b).clone();
    let argCount = c as usize;

    let (funcIndex, upvalues): (usize, Vec<GcRef>) = match &funcVal.payload {
        ValuePayload::Func(idx) => (*idx, Vec::new()),
        ValuePayload::Closure(c) => (c.funcIndex, c.upvalues.clone()),
        _ => return Err(RuntimeError::type_error("function", "non-function")),
    };

    let (arity, numRegisters, upvalueDescs) = {
        let module = exec.module.as_ref().expect("no module loaded");
        let func = &module.functions[funcIndex];
        (func.arity, func.numRegisters, func.upvalueDescs.clone())
    };

    if arity != argCount {
        return Err(RuntimeError::new(ErrorKind::ArityMismatch, format!("function expects {arity} arguments, got {argCount}")));
    }

    let mut args = Vec::with_capacity(argCount);
    for i in 0..argCount {
        args.push(exec.reg(b + 1 + i as u8).clone());
    }

    let mut registers = Vec::with_capacity(numRegisters);
    registers.extend(args);
    registers.resize(numRegisters, RuntimeValue::NIL.clone());

    let mut frameUpvalues = Vec::new();
    for upvDesc in &upvalueDescs {
        let idx = upvDesc.index;
        if idx < upvalues.len() {
            frameUpvalues.push(upvalues[idx]);
        } else {
            let obj = GcObject::new_upvalue(RuntimeValue::NIL.clone());
            frameUpvalues.push(exec.heap.AllocObj(obj));
        }
    }

    let returnAddr = exec.ip;
    let stackStart = exec.stack.len();

    let mut frame = CallFrame {
        funcIndex,
        ip: 0,
        registers,
        stackStart,
        returnAddr,
        upvalues: frameUpvalues,
        tier: CompilationTier::Interpreter,
        ..Default::default()
    };
    frame.RebuildRegisterBitmap();

    exec.frames.push(frame);
    exec.currentFuncIndex = exec.frames.len() - 1;
    exec.ip = 0;

    if let Some(counter) = exec.callCounters.iter_mut().find(|c| c.funcIndex == funcIndex) {
        counter.count += 1;
    } else {
        exec.callCounters.push(CallCounter::new(funcIndex));
    }

    Ok(())
}

fn handle_return(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let retVal = exec.reg(a).clone();

    if exec.frames.is_empty() {
        exec.halted = true;
        return Ok(());
    }

    let frame = exec.frames.pop().expect("no frame to return from");
    let returnAddr = frame.returnAddr;

    exec.stack.truncate(frame.stackStart);
    exec.stack.push(retVal);

    if exec.frames.is_empty() {
        exec.halted = true;
        return Ok(());
    }

    exec.currentFuncIndex = exec.frames.len() - 1;
    exec.ip = returnAddr;
    Ok(())
}

fn handle_tail_call(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let funcVal = exec.reg(a).clone();
    let argCount = c as usize;

    let (funcIndex, upvalues): (usize, Vec<GcRef>) = match &funcVal.payload {
        ValuePayload::Func(idx) => (*idx, Vec::new()),
        ValuePayload::Closure(c) => (c.funcIndex, c.upvalues.clone()),
        _ => return Err(RuntimeError::type_error("function", "non-function")),
    };

    let (arity, numRegisters, upvalueDescs) = {
        let module = exec.module.as_ref().expect("no module loaded");
        let func = &module.functions[funcIndex];
        (func.arity, func.numRegisters, func.upvalueDescs.clone())
    };

    if arity != argCount {
        return Err(RuntimeError::new(ErrorKind::ArityMismatch, format!("function expects {arity} arguments, got {argCount}")));
    }

    let mut args = Vec::with_capacity(argCount);
    for i in 0..argCount {
        args.push(exec.reg(b + i as u8).clone());
    }

    let mut registers = Vec::with_capacity(numRegisters);
    registers.extend(args);
    registers.resize(numRegisters, RuntimeValue::NIL.clone());

    let mut frameUpvalues = Vec::new();
    for upvDesc in &upvalueDescs {
        let idx = upvDesc.index;
        if idx < upvalues.len() {
            frameUpvalues.push(upvalues[idx]);
        } else {
            let obj = GcObject::new_upvalue(RuntimeValue::NIL.clone());
            frameUpvalues.push(exec.heap.AllocObj(obj));
        }
    }

    let currentReturnAddr = exec.frames.pop().map(|f| f.returnAddr).unwrap_or(0);
    let stackStart = exec.stack.len();

    let mut frame = CallFrame {
        funcIndex,
        ip: 0,
        registers,
        stackStart,
        returnAddr: currentReturnAddr,
        upvalues: frameUpvalues,
        tier: CompilationTier::Interpreter,
        ..Default::default()
    };
    frame.RebuildRegisterBitmap();

    exec.frames.push(frame);
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

    let mut upvalues = Vec::new();
    let module = exec.module.as_ref().expect("no module loaded");
    let func = &module.functions[funcIndex];
    for upvDesc in &func.upvalueDescs {
        let idx = upvDesc.index;
        let frameIdx = if exec.frames.len() > 1 { exec.frames.len() - 2 } else { 0 };
        if frameIdx < exec.frames.len() {
            let val = exec.frames[frameIdx].registers[idx].clone();
            let obj = GcObject::new_upvalue(val);
            let gcRef = exec.heap.AllocObj(obj);
            upvalues.push(gcRef);
        } else {
            let obj = GcObject::new_upvalue(RuntimeValue::NIL.clone());
            let gcRef = exec.heap.AllocObj(obj);
            upvalues.push(gcRef);
        }
    }
    upvalues.truncate(upvalueCount);

    exec.set_reg(a, RuntimeValue::Closure(funcIndex, upvalues));
    Ok(())
}

fn handle_load_upvalue(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let upvIdx = b as usize;
    let frame = exec.frame();
    if upvIdx < frame.upvalues.len() {
        let gcRef = frame.upvalues[upvIdx];
        let obj = exec.heap.Get(gcRef);
        let val = match &obj.kind {
            ObjectKind::Upvalue { value, .. } => value.clone(),
            _ => RuntimeValue::NIL.clone(),
        };
        exec.set_reg(a, val);
        Ok(())
    } else {
        Err(RuntimeError::new(ErrorKind::IndexOutOfBounds, format!("upvalue index {upvIdx} out of bounds")))
    }
}

fn handle_store_upvalue(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let _a = inst.a();
    let b = inst.b();
    let upvIdx = b as usize;
    let val = exec.reg(inst.a()).clone();
    let frame = exec.frame();
    if upvIdx < frame.upvalues.len() {
        let gcRef = frame.upvalues[upvIdx];
        let obj = exec.heap.GetMut(gcRef);
        if let ObjectKind::Upvalue { ref mut value, ref mut closed } = obj.kind {
            *value = val;
            *closed = true;
        }
        Ok(())
    } else {
        Err(RuntimeError::new(ErrorKind::IndexOutOfBounds, format!("upvalue index {upvIdx} out of bounds")))
    }
}

fn handle_close_upvalue(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let val = exec.reg(a).clone();
    exec.set_reg(a, val);
    Ok(())
}

fn handle_new_object(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let obj = GcObject::new_instance(Vec::new(), 0);
    let gcRef = exec.heap.AllocObj(obj);
    exec.globalRefs.push(gcRef);
    exec.heap.WriteBarrier(gcRef, gcRef);
    exec.set_reg(a, RuntimeValue::Object(gcRef));
    Ok(())
}

fn handle_new_array(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let obj = GcObject::new_array(Vec::new());
    let gcRef = exec.heap.AllocObj(obj);
    exec.globalRefs.push(gcRef);
    exec.heap.WriteBarrier(gcRef, gcRef);
    exec.set_reg(a, RuntimeValue::Array(gcRef));
    Ok(())
}

fn handle_get_field(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let fieldIdx = inst.c() as u16;
    let objVal = exec.reg(b).clone();

    let gcRef = match &objVal.payload {
        ValuePayload::Object(r) => *r,
        _ => return Err(RuntimeError::type_error("object", "non-object")),
    };

    let fieldName = {
        let module = exec.module.as_ref().expect("no module loaded");
        let funcIndex = exec.frames[exec.currentFuncIndex].funcIndex;
        let func = &module.functions[funcIndex];
        match &func.constants[fieldIdx as usize] {
            CpValue::String(s) => s.clone(),
            _ => return Err(RuntimeError::new(ErrorKind::Custom, "field name must be a string constant")),
        }
    };

    let obj = exec.heap.Get(gcRef);
    match &obj.kind {
        ObjectKind::Instance { fields, .. } => {
            let val = fields
                .iter()
                .find(|(name, _)| name == &fieldName)
                .map(|(_, v)| v.clone())
                .unwrap_or(RuntimeValue::NIL.clone());
            exec.set_reg(a, val);
        }
        _ => return Err(RuntimeError::type_error("object", "non-object")),
    }
    Ok(())
}

fn handle_set_field(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let fieldIdx = inst.c() as u16;
    let val = exec.reg(a).clone();
    let objVal = exec.reg(b).clone();

    let fieldName = {
        let module = exec.module.as_ref().expect("no module loaded");
        let funcIndex = exec.frames[exec.currentFuncIndex].funcIndex;
        let func = &module.functions[funcIndex];
        match &func.constants[fieldIdx as usize] {
            CpValue::String(s) => s.clone(),
            _ => return Err(RuntimeError::new(ErrorKind::Custom, "field name must be a string constant")),
        }
    };

    let gcRef = match &objVal.payload {
        ValuePayload::Object(r) => *r,
        _ => return Err(RuntimeError::type_error("object", "non-object")),
    };

    if let Some(newRef) = val.as_gc_ref() {
        exec.heap.WriteBarrier(gcRef, newRef);
    }

    let obj = exec.heap.GetMut(gcRef);
    match &mut obj.kind {
        ObjectKind::Instance { ref mut fields, .. } => {
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
    let idx = match idxVal.payload {
        ValuePayload::Int(i) => i as usize,
        _ => return Err(RuntimeError::type_error("integer", "non-integer index")),
    };
    let gcRef = match &arrVal.payload {
        ValuePayload::Array(r) => *r,
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };
    let obj = exec.heap.Get(gcRef);
    match &obj.kind {
        ObjectKind::Array { elements } => {
            let val = elements.get(idx).cloned().unwrap_or(RuntimeValue::NIL.clone());
            exec.set_reg(a, val);
        }
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    }
    Ok(())
}

fn handle_aset(exec: &mut Executor, inst: Instruction) -> VmResult<()> {
    let a = inst.a();
    let b = inst.b();
    let c = inst.c();
    let arrVal = exec.reg(a).clone();
    let idxVal = exec.reg(b).clone();
    let val = exec.reg(c).clone();
    let idx = match idxVal.payload {
        ValuePayload::Int(i) => i as usize,
        _ => return Err(RuntimeError::type_error("integer", "non-integer index")),
    };
    let gcRef = match &arrVal.payload {
        ValuePayload::Array(r) => *r,
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };

    if let Some(newRef) = val.as_gc_ref() {
        exec.heap.WriteBarrier(gcRef, newRef);
    }

    let obj = exec.heap.GetMut(gcRef);
    match &mut obj.kind {
        ObjectKind::Array { ref mut elements } => {
            if idx >= elements.len() {
                elements.resize(idx + 1, RuntimeValue::NIL.clone());
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
    let gcRef = match &arrVal.payload {
        ValuePayload::Array(r) => *r,
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };
    let obj = exec.heap.Get(gcRef);
    let len = match &obj.kind {
        ObjectKind::Array { elements } => elements.len(),
        _ => return Err(RuntimeError::type_error("array", "non-array")),
    };
    exec.set_reg(a, RuntimeValue::Int(len as i64));
    Ok(())
}

fn handle_import(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    Ok(())
}

fn handle_export(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
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
    Ok(())
}

fn handle_line(_exec: &mut Executor, _inst: Instruction) -> VmResult<()> {
    Ok(())
}