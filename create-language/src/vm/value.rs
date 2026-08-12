use super::error::VmResult;

#[derive(Debug, Clone)]
pub struct RuntimeValue {
    pub tag: ValueTag,
    pub payload: ValuePayload,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueTag {
    Nil,
    Bool,
    Int,
    Float,
    Str,
    Func,
    Closure,
    Object,
    Array,
    Upvalue,
    Native,
}

#[derive(Debug, Clone)]
pub enum ValuePayload {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Func(usize),
    Closure(ClosurePayload),
    Object(GcRef),
    Array(GcRef),
    Upvalue(GcRef),
    Native(fn(&[RuntimeValue]) -> VmResult<RuntimeValue>),
}

#[derive(Debug, Clone)]
pub struct ClosurePayload {
    pub funcIndex: usize,
    pub upvalues: Vec<GcRef>,
}

impl RuntimeValue {
    pub const NIL: RuntimeValue = RuntimeValue { tag: ValueTag::Nil, payload: ValuePayload::Nil };

    pub fn Bool(v: bool) -> Self { RuntimeValue { tag: ValueTag::Bool, payload: ValuePayload::Bool(v) } }
    pub fn Int(v: i64) -> Self { RuntimeValue { tag: ValueTag::Int, payload: ValuePayload::Int(v) } }
    pub fn Float(v: f64) -> Self { RuntimeValue { tag: ValueTag::Float, payload: ValuePayload::Float(v) } }
    pub fn Str(v: String) -> Self { RuntimeValue { tag: ValueTag::Str, payload: ValuePayload::Str(v) } }
    pub fn Func(v: usize) -> Self { RuntimeValue { tag: ValueTag::Func, payload: ValuePayload::Func(v) } }
    pub fn Closure(funcIndex: usize, upvalues: Vec<GcRef>) -> Self {
        RuntimeValue {
            tag: ValueTag::Closure,
            payload: ValuePayload::Closure(ClosurePayload { funcIndex, upvalues }),
        }
    }
    pub fn Object(r: GcRef) -> Self { RuntimeValue { tag: ValueTag::Object, payload: ValuePayload::Object(r) } }
    pub fn Array(r: GcRef) -> Self { RuntimeValue { tag: ValueTag::Array, payload: ValuePayload::Array(r) } }
    pub fn Upvalue(r: GcRef) -> Self { RuntimeValue { tag: ValueTag::Upvalue, payload: ValuePayload::Upvalue(r) } }
    pub fn Native(f: fn(&[RuntimeValue]) -> VmResult<RuntimeValue>) -> Self {
        RuntimeValue { tag: ValueTag::Native, payload: ValuePayload::Native(f) }
    }

    pub fn tag(&self) -> ValueTag { self.tag }
    pub fn is_truthy(&self) -> bool {
        match self.tag {
            ValueTag::Nil => false,
            ValueTag::Bool => {
                if let ValuePayload::Bool(v) = &self.payload { *v } else { true }
            }
            _ => true,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let ValuePayload::Bool(v) = &self.payload { Some(*v) } else { None }
    }
    pub fn as_int(&self) -> Option<i64> {
        if let ValuePayload::Int(v) = &self.payload { Some(*v) } else { None }
    }
    pub fn as_float(&self) -> Option<f64> {
        if let ValuePayload::Float(v) = &self.payload { Some(*v) } else { None }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let ValuePayload::Str(v) = &self.payload { Some(v.as_str()) } else { None }
    }
    pub fn as_func_index(&self) -> Option<usize> {
        match &self.payload {
            ValuePayload::Func(idx) => Some(*idx),
            ValuePayload::Closure(c) => Some(c.funcIndex),
            _ => None,
        }
    }
    pub fn as_gc_ref(&self) -> Option<GcRef> {
        match &self.payload {
            ValuePayload::Object(r) | ValuePayload::Array(r) | ValuePayload::Upvalue(r) => Some(*r),
            _ => None,
        }
    }
    pub fn as_closure(&self) -> Option<&ClosurePayload> {
        if let ValuePayload::Closure(c) = &self.payload { Some(c) } else { None }
    }
    pub fn as_closure_mut(&mut self) -> Option<&mut ClosurePayload> {
        if let ValuePayload::Closure(ref mut c) = self.payload { Some(c) } else { None }
    }

    pub fn type_name(&self) -> &'static str {
        match self.tag {
            ValueTag::Nil => "nil",
            ValueTag::Bool => "bool",
            ValueTag::Int => "int",
            ValueTag::Float => "float",
            ValueTag::Str => "string",
            ValueTag::Func | ValueTag::Closure => "function",
            ValueTag::Object => "object",
            ValueTag::Array => "array",
            ValueTag::Upvalue => "upvalue",
            ValueTag::Native => "native",
        }
    }
}

impl PartialEq for RuntimeValue {
    fn eq(&self, other: &Self) -> bool {
        if self.tag != other.tag { return false; }
        match (&self.payload, &other.payload) {
            (ValuePayload::Bool(a), ValuePayload::Bool(b)) => a == b,
            (ValuePayload::Int(a), ValuePayload::Int(b)) => a == b,
            (ValuePayload::Float(a), ValuePayload::Float(b)) => a.to_bits() == b.to_bits(),
            (ValuePayload::Str(a), ValuePayload::Str(b)) => a == b,
            (ValuePayload::Func(a), ValuePayload::Func(b)) => a == b,
            (ValuePayload::Object(a), ValuePayload::Object(b)) => a == b,
            (ValuePayload::Array(a), ValuePayload::Array(b)) => a == b,
            (ValuePayload::Upvalue(a), ValuePayload::Upvalue(b)) => a == b,
            (ValuePayload::Native(_), ValuePayload::Native(_)) => false,
            (ValuePayload::Closure(a), ValuePayload::Closure(b)) => a.funcIndex == b.funcIndex && a.upvalues == b.upvalues,
            _ => true,
        }
    }
}

impl std::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.payload {
            ValuePayload::Nil => write!(f, "nil"),
            ValuePayload::Bool(v) => write!(f, "{v}"),
            ValuePayload::Int(v) => write!(f, "{v}"),
            ValuePayload::Float(v) => write!(f, "{v}"),
            ValuePayload::Str(v) => write!(f, "\"{v}\""),
            ValuePayload::Func(idx) => write!(f, "func({idx})"),
            ValuePayload::Closure(c) => write!(f, "closure({})", c.funcIndex),
            ValuePayload::Object(r) => write!(f, "object({})", r.0),
            ValuePayload::Array(r) => write!(f, "array({})", r.0),
            ValuePayload::Upvalue(r) => write!(f, "upvalue({})", r.0),
            ValuePayload::Native(_) => write!(f, "<native>"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcRef(pub usize);

#[derive(Debug, Clone)]
pub enum ObjectKind {
    Instance { fields: Vec<(String, RuntimeValue)>, class: usize },
    Array { elements: Vec<RuntimeValue> },
    Upvalue { value: RuntimeValue, closed: bool },
    Str { chars: String },
    Bytes { data: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct GcObject {
    pub kind: ObjectKind,
    pub marked: bool,
    pub age: u8,
    pub generation: Generation,
    pub refcount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    Young,
    Survivor,
    Old,
}

impl GcObject {
    pub fn new_instance(fields: Vec<(String, RuntimeValue)>, class: usize) -> Self {
        GcObject { kind: ObjectKind::Instance { fields, class }, marked: false, age: 0, generation: Generation::Young, refcount: 1 }
    }
    pub fn new_array(elements: Vec<RuntimeValue>) -> Self {
        GcObject { kind: ObjectKind::Array { elements }, marked: false, age: 0, generation: Generation::Young, refcount: 1 }
    }
    pub fn new_upvalue(value: RuntimeValue) -> Self {
        GcObject { kind: ObjectKind::Upvalue { value, closed: false }, marked: false, age: 0, generation: Generation::Young, refcount: 1 }
    }
    pub fn new_str(chars: String) -> Self {
        GcObject { kind: ObjectKind::Str { chars }, marked: false, age: 0, generation: Generation::Young, refcount: 1 }
    }
    pub fn new_bytes(data: Vec<u8>) -> Self {
        GcObject { kind: ObjectKind::Bytes { data }, marked: false, age: 0, generation: Generation::Young, refcount: 1 }
    }

    pub fn children(&self) -> Vec<GcRef> {
        match &self.kind {
            ObjectKind::Instance { fields, .. } => fields
                .iter()
                .filter_map(|(_, v)| v.as_gc_ref())
                .collect(),
            ObjectKind::Array { elements } => elements
                .iter()
                .filter_map(|v| v.as_gc_ref())
                .collect(),
            ObjectKind::Upvalue { value, .. } => value.as_gc_ref().into_iter().collect(),
            ObjectKind::Str { .. } | ObjectKind::Bytes { .. } => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub funcIndex: usize,
    pub ip: usize,
    pub registers: Vec<RuntimeValue>,
    pub stackStart: usize,
    pub returnAddr: usize,
    pub upvalues: Vec<GcRef>,
    pub tier: CompilationTier,
    pub registers_bitmap: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationTier {
    Interpreter,
    BaselineJit,
    OptimizingJit,
}

impl Default for CallFrame {
    fn default() -> Self {
        CallFrame {
            funcIndex: 0,
            ip: 0,
            registers: Vec::new(),
            stackStart: 0,
            returnAddr: 0,
            upvalues: Vec::new(),
            tier: CompilationTier::Interpreter,
            registers_bitmap: Vec::new(),
        }
    }
}