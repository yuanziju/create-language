use super::error::*;

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Function(usize),
    Closure(usize, Vec<GcRef>),
    Object(GcRef),
    Array(GcRef),
    NativeFn(fn(&[RuntimeValue]) -> VmResult<RuntimeValue>),
    Upvalue(GcRef),
}

impl PartialEq for RuntimeValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeValue::Nil, RuntimeValue::Nil) => true,
            (RuntimeValue::Bool(a), RuntimeValue::Bool(b)) => a == b,
            (RuntimeValue::Int(a), RuntimeValue::Int(b)) => a == b,
            (RuntimeValue::Float(a), RuntimeValue::Float(b)) => a == b,
            (RuntimeValue::String(a), RuntimeValue::String(b)) => a == b,
            (RuntimeValue::Function(a), RuntimeValue::Function(b)) => a == b,
            (RuntimeValue::Closure(a, b), RuntimeValue::Closure(c, d)) => a == c && b == d,
            (RuntimeValue::Object(a), RuntimeValue::Object(b)) => a == b,
            (RuntimeValue::Array(a), RuntimeValue::Array(b)) => a == b,
            (RuntimeValue::NativeFn(_), RuntimeValue::NativeFn(_)) => false,
            (RuntimeValue::Upvalue(a), RuntimeValue::Upvalue(b)) => a == b,
            _ => false,
        }
    }
}

impl RuntimeValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            RuntimeValue::Nil => false,
            RuntimeValue::Bool(b) => *b,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcRef(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
    Object(Vec<(String, RuntimeValue)>),
    Array(Vec<RuntimeValue>),
    UpvalueData(RuntimeValue, bool),
}

#[derive(Debug, Clone)]
pub struct GcObject {
    pub kind: ObjectKind,
    pub marked: bool,
}

impl GcObject {
    pub fn new_object(fields: Vec<(String, RuntimeValue)>) -> Self {
        GcObject {
            kind: ObjectKind::Object(fields),
            marked: false,
        }
    }

    pub fn new_array(elements: Vec<RuntimeValue>) -> Self {
        GcObject {
            kind: ObjectKind::Array(elements),
            marked: false,
        }
    }

    pub fn new_upvalue(value: RuntimeValue, closed: bool) -> Self {
        GcObject {
            kind: ObjectKind::UpvalueData(value, closed),
            marked: false,
        }
    }
}

pub struct Heap {
    objects: Vec<GcObject>,
    allocCount: usize,
    threshold: usize,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            objects: Vec::new(),
            allocCount: 0,
            threshold: 1024,
        }
    }

    pub fn alloc(&mut self, kind: ObjectKind) -> GcRef {
        let idx = self.objects.len();
        self.objects.push(GcObject {
            kind,
            marked: false,
        });
        self.allocCount += 1;
        if self.allocCount >= self.threshold {
            let roots = Vec::new();
            self.collect(&roots);
        }
        GcRef(idx)
    }

    pub fn allocObj(&mut self, obj: GcObject) -> GcRef {
        let idx = self.objects.len();
        self.objects.push(obj);
        self.allocCount += 1;
        if self.allocCount >= self.threshold {
            let roots = Vec::new();
            self.collect(&roots);
        }
        GcRef(idx)
    }

    pub fn get(&self, gcRef: GcRef) -> &GcObject {
        &self.objects[gcRef.0]
    }

    pub fn getMut(&mut self, gcRef: GcRef) -> &mut GcObject {
        &mut self.objects[gcRef.0]
    }

    pub fn collect(&mut self, roots: &[GcRef]) {
        for obj in &mut self.objects {
            obj.marked = false;
        }
        for &root in roots {
            self.mark(root);
        }
        self.sweep();
        self.allocCount = 0;
    }

    fn mark(&mut self, gcRef: GcRef) {
        if gcRef.0 >= self.objects.len() {
            return;
        }
        if self.objects[gcRef.0].marked {
            return;
        }
        self.objects[gcRef.0].marked = true;

        let kind = self.objects[gcRef.0].kind.clone();
        let children = match kind {
            ObjectKind::Object(fields) => fields
                .iter()
                .filter_map(|(_, v)| {
                    if let RuntimeValue::Object(r)
                    | RuntimeValue::Array(r)
                    | RuntimeValue::Upvalue(r) = v
                    {
                        Some(*r)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
            ObjectKind::Array(elements) => elements
                .iter()
                .filter_map(|v| {
                    if let RuntimeValue::Object(r)
                    | RuntimeValue::Array(r)
                    | RuntimeValue::Upvalue(r) = v
                    {
                        Some(*r)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
            ObjectKind::UpvalueData(val, _) => {
                if let RuntimeValue::Object(r) | RuntimeValue::Array(r) | RuntimeValue::Upvalue(r) =
                    &val
                {
                    vec![*r]
                } else {
                    vec![]
                }
            }
        };
        for child in children {
            self.mark(child);
        }
    }

    fn sweep(&mut self) {
        self.objects.retain(|obj| obj.marked);
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
}
