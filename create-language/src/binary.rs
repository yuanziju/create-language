use crate::constant_pool::Value;
use crate::instruction::Instruction;

pub const MAGIC: [u8; 4] = *b"CLBC";
pub const VERSION_MAJOR: u16 = 1;
pub const VERSION_MINOR: u16 = 0;
pub const VERSION_PATCH: u16 = 0;

pub struct ModuleFile {
    pub version: (u16, u16, u16),
    pub constants: Vec<Value>,
    pub functions: Vec<Function>,
    pub exports: Vec<Export>,
    pub imports: Vec<Import>,
    pub entryPoint: u32,
}

#[derive(Debug, Clone)]
pub struct UpvalueDesc {
    pub isLocal: bool,
    pub index: usize,
}

pub struct Function {
    pub name: String,
    pub arity: usize,
    pub numRegisters: usize,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub upvalueCount: usize,
    pub upvalueDescs: Vec<UpvalueDesc>,
}

pub struct Export {
    pub name: String,
    pub kind: ExportKind,
    pub functionIndex: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum ExportKind {
    Function = 0,
    Variable = 1,
    Class = 2,
    Struct = 3,
    Enum = 4,
}

pub struct Import {
    pub modulePath: String,
    pub alias: Option<String>,
    pub importedNames: Vec<String>,
}

#[derive(Default)]
pub struct BinaryWriter {
    data: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn writeU8(&mut self, v: u8) {
        self.data.push(v);
    }
    pub fn writeU16(&mut self, v: u16) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    pub fn writeU32(&mut self, v: u32) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    pub fn writeU64(&mut self, v: u64) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    pub fn writeI64(&mut self, v: i64) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    pub fn writeF64(&mut self, v: f64) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    pub fn writeBool(&mut self, v: bool) {
        self.writeU8(v as u8);
    }
    pub fn writeBytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }
    pub fn writeString(&mut self, s: &str) {
        self.writeU32(s.len() as u32);
        self.writeBytes(s.as_bytes());
    }
    pub fn writeValue(&mut self, value: &Value) {
        match value {
            Value::Nil => self.writeU8(0),
            Value::Bool(b) => {
                self.writeU8(1);
                self.writeBool(*b);
            }
            Value::Int(i) => {
                self.writeU8(2);
                self.writeI64(*i);
            }
            Value::Float(f) => {
                self.writeU8(3);
                self.writeF64(*f);
            }
            Value::String(s) => {
                self.writeU8(4);
                self.writeString(s);
            }
            Value::Function(idx) => {
                self.writeU8(5);
                self.writeU32(*idx as u32);
            }
        }
    }
    pub fn writeInstruction(&mut self, inst: &Instruction) {
        self.writeU32(inst.raw());
    }
    pub fn writeFunction(&mut self, func: &Function) {
        self.writeString(&func.name);
        self.writeU32(func.arity as u32);
        self.writeU32(func.numRegisters as u32);
        self.writeU32(func.upvalueCount as u32);
        self.writeU32(func.instructions.len() as u32);
        for inst in &func.instructions {
            self.writeInstruction(inst);
        }
        self.writeU32(func.constants.len() as u32);
        for c in &func.constants {
            self.writeValue(c);
        }
    }
    pub fn writeModule(&mut self, module: &ModuleFile) -> Vec<u8> {
        self.writeBytes(&MAGIC);
        self.writeU16(module.version.0);
        self.writeU16(module.version.1);
        self.writeU16(module.version.2);
        self.writeU32(0); // flags
        self.writeU32(module.constants.len() as u32);
        for c in &module.constants {
            self.writeValue(c);
        }
        self.writeU32(module.functions.len() as u32);
        for f in &module.functions {
            self.writeFunction(f);
        }
        self.writeU32(module.exports.len() as u32);
        for e in &module.exports {
            self.writeString(&e.name);
            self.writeU8(e.kind as u8);
            self.writeU32(e.functionIndex);
        }
        self.writeU32(module.imports.len() as u32);
        for imp in &module.imports {
            self.writeString(&imp.modulePath);
            self.writeBool(imp.alias.is_some());
            if let Some(ref a) = imp.alias {
                self.writeString(a);
            }
            self.writeU32(imp.importedNames.len() as u32);
            for n in &imp.importedNames {
                self.writeString(n);
            }
        }
        self.writeU32(module.entryPoint);
        self.data.clone()
    }
    pub fn intoBytes(self) -> Vec<u8> {
        self.data
    }
}

pub fn serializeModule(module: &ModuleFile) -> Vec<u8> {
    BinaryWriter::new().writeModule(module)
}

pub fn validateHeader(data: &[u8]) -> Result<(), String> {
    if data.len() < 12 {
        return Err("file too short".into());
    }
    if data[0..4] != MAGIC {
        return Err("invalid magic".into());
    }
    Ok(())
}
