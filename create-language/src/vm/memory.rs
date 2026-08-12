use crate::vm::value::{GcRef, Value};
use std::collections::HashMap;

pub enum ObjectKind {
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    Upvalue(UpvalueInner),
}

pub struct UpvalueInner {
    pub closed: bool,
    pub value: Value,
    pub registerIndex: usize,
}

pub struct GcObject {
    pub kind: ObjectKind,
    pub marked: bool,
}

impl GcObject {
    pub fn new_object(fields: HashMap<String, Value>) -> Self {
        GcObject {
            kind: ObjectKind::Object(fields),
            marked: false,
        }
    }

    pub fn new_array(elements: Vec<Value>) -> Self {
        GcObject {
            kind: ObjectKind::Array(elements),
            marked: false,
        }
    }

    pub fn new_string(s: String) -> Self {
        GcObject {
            kind: ObjectKind::String(s),
            marked: false,
        }
    }

    pub fn new_upvalue(registerIndex: usize) -> Self {
        GcObject {
            kind: ObjectKind::Upvalue(UpvalueInner {
                closed: false,
                value: Value::Nil,
                registerIndex,
            }),
            marked: false,
        }
    }
}

pub struct Heap {
    objects: Vec<GcObject>,
    freeList: Vec<usize>,
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
            freeList: Vec::new(),
        }
    }

    pub fn alloc(&mut self, obj: GcObject) -> GcRef {
        if let Some(idx) = self.freeList.pop() {
            self.objects[idx] = obj;
            GcRef::new(idx)
        } else {
            let idx = self.objects.len();
            self.objects.push(obj);
            GcRef::new(idx)
        }
    }

    pub fn get(&self, gcRef: GcRef) -> Option<&GcObject> {
        self.objects.get(gcRef.0)
    }

    pub fn get_mut(&mut self, gcRef: GcRef) -> Option<&mut GcObject> {
        self.objects.get_mut(gcRef.0)
    }

    pub fn collect(&mut self, roots: &[GcRef]) {
        for obj in &mut self.objects {
            obj.marked = false;
        }
        for root in roots {
            self.mark(*root);
        }
        let mut live: Vec<GcObject> = Vec::new();
        let mut oldToNew: Vec<usize> = vec![0; self.objects.len()];
        for (i, obj) in self.objects.drain(..).enumerate() {
            if obj.marked {
                oldToNew[i] = live.len();
                live.push(obj);
            } else {
                self.freeList.push(i);
            }
        }
        self.objects = live;
    }

    fn mark(&mut self, gcRef: GcRef) {
        let Some(obj) = self.objects.get_mut(gcRef.0) else {
            return;
        };
        if obj.marked {
            return;
        }
        obj.marked = true;
        let children: Vec<GcRef> = match &obj.kind {
            ObjectKind::Object(fields) => fields.values().filter_map(|v| v.asGcRef()).collect(),
            ObjectKind::Array(elems) => elems.iter().filter_map(|v| v.asGcRef()).collect(),
            ObjectKind::Upvalue(_) => Vec::new(),
            ObjectKind::String(_) => Vec::new(),
        };
        for child in children {
            self.mark(child);
        }
    }
}

pub struct Stack {
    data: Vec<Value>,
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Self {
        Stack { data: Vec::new() }
    }

    pub fn push(&mut self, value: Value) {
        self.data.push(value);
    }

    pub fn pop(&mut self) -> Option<Value> {
        self.data.pop()
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.data.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.data.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn truncate(&mut self, len: usize) {
        self.data.truncate(len);
    }
}

impl Value {
    pub fn asGcRef(&self) -> Option<GcRef> {
        match self {
            Value::Object(gc) | Value::Array(gc) | Value::Upvalue(gc) => Some(*gc),
            _ => None,
        }
    }
}
