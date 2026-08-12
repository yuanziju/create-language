use crate::vm::error::RuntimeError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GcRef(pub usize);

impl GcRef {
    pub fn new(index: usize) -> Self {
        GcRef(index)
    }
}

#[derive(Debug, Clone)]
pub struct UpvalueCell {
    pub closed: bool,
    pub value: Value,
    pub registerIndex: usize,
}

#[derive(Debug, Clone)]
pub struct ClosureData {
    pub funcIndex: usize,
    pub upvalues: Vec<GcRef>,
}

#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Function(usize),
    Object(GcRef),
    Array(GcRef),
    Closure(ClosureData),
    NativeFn(fn(&[Value]) -> std::result::Result<Value, RuntimeError>),
    Upvalue(GcRef),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "Nil"),
            Value::Bool(b) => write!(f, "Bool({b})"),
            Value::Int(i) => write!(f, "Int({i})"),
            Value::Float(fl) => write!(f, "Float({fl})"),
            Value::String(s) => write!(f, "String({s:?})"),
            Value::Function(idx) => write!(f, "Function({idx})"),
            Value::Object(gc) => write!(f, "Object({:?})", gc),
            Value::Array(gc) => write!(f, "Array({:?})", gc),
            Value::Closure(cd) => write!(f, "Closure(func={})", cd.funcIndex),
            Value::NativeFn(_) => write!(f, "NativeFn"),
            Value::Upvalue(gc) => write!(f, "Upvalue({:?})", gc),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Closure(a), Value::Closure(b)) => a.funcIndex == b.funcIndex,
            (Value::NativeFn(a), Value::NativeFn(b)) => *a as usize == *b as usize,
            (Value::Upvalue(a), Value::Upvalue(b)) => a == b,
            _ => false,
        }
    }
}

impl Value {
    pub fn isTruthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            _ => true,
        }
    }

    pub fn typeName(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Function(_) => "function",
            Value::Object(_) => "object",
            Value::Array(_) => "array",
            Value::Closure(_) => "closure",
            Value::NativeFn(_) => "native",
            Value::Upvalue(_) => "upvalue",
        }
    }
}

impl From<&crate::constant_pool::Value> for Value {
    fn from(v: &crate::constant_pool::Value) -> Self {
        match v {
            crate::constant_pool::Value::Nil => Value::Nil,
            crate::constant_pool::Value::Bool(b) => Value::Bool(*b),
            crate::constant_pool::Value::Int(i) => Value::Int(*i),
            crate::constant_pool::Value::Float(f) => Value::Float(*f),
            crate::constant_pool::Value::String(s) => Value::String(s.clone()),
            crate::constant_pool::Value::Function(idx) => Value::Function(*idx as usize),
        }
    }
}
