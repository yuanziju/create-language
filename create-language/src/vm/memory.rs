use std::collections::HashMap;

// ---- Runtime Value ----

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
        !matches!(self.tag, ValueTag::Nil)
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

// ---- GC Reference ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcRef(pub usize);

// ---- Object Model ----

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

// ---- Generational Heap ----

const YOUNG_GEN_MAX: usize = 1024;
const OLD_GEN_MAX: usize = 4096;
const SURVIVOR_MAX_AGE: u8 = 15;
const _CARD_BITS_PER_OBJECT: usize = 8;

#[derive(Default)]
pub struct Heap {
    youngGen: Vec<GcObject>,
    survivorFrom: Vec<GcObject>,
    survivorTo: Vec<GcObject>,
    oldGen: Vec<GcObject>,
    nextYoungId: usize,
    nextOldId: usize,
    allocCount: usize,
    minorGcCount: u64,
    majorGcCount: u64,
    cardTable: CardTable,
    // Forwarding table for copy GC: maps old GcRef -> new GcRef
    forwardingTable: HashMap<usize, GcRef>,
}

impl Heap {
    pub fn new() -> Self {
        Heap::default()
    }

    pub fn Alloc(&mut self, kind: ObjectKind) -> GcRef {
        if self.youngGen.len() >= YOUNG_GEN_MAX {
            self.MinorGc();
        }
        let obj = match kind {
            k @ ObjectKind::Instance { .. } => GcObject::new_instance_default(k),
            ObjectKind::Array { elements } => GcObject::new_array(elements),
            ObjectKind::Upvalue { value, .. } => GcObject::new_upvalue(value),
            ObjectKind::Str { chars } => GcObject::new_str(chars),
            ObjectKind::Bytes { data } => GcObject::new_bytes(data),
        };
        let id = self.nextYoungId;
        self.nextYoungId += 1;
        self.youngGen.push(obj);
        self.allocCount += 1;
        GcRef(id)
    }

    pub fn AllocObj(&mut self, obj: GcObject) -> GcRef {
        if self.youngGen.len() >= YOUNG_GEN_MAX {
            self.MinorGc();
        }
        let id = self.nextYoungId;
        self.nextYoungId += 1;
        self.youngGen.push(obj);
        self.allocCount += 1;
        GcRef(id)
    }

    pub fn PromoteObject(&mut self, mut obj: GcObject) -> GcRef {
        obj.generation = Generation::Old;
        obj.age = 0;
        let id = self.nextOldId;
        self.nextOldId += 1;
        self.oldGen.push(obj);
        GcRef(id)
    }

    pub fn Get(&self, gcRef: GcRef) -> &GcObject {
        self.find(gcRef)
    }

    pub fn GetMut(&mut self, gcRef: GcRef) -> &mut GcObject {
        self.find_mut(gcRef)
    }

    fn find(&self, gcRef: GcRef) -> &GcObject {
        if gcRef.0 < self.nextYoungId {
            return &self.youngGen[gcRef.0];
        }
        if gcRef.0 < self.nextYoungId + self.survivorFrom.len() {
            return &self.survivorFrom[gcRef.0 - self.nextYoungId];
        }
        let survivorToBase = self.nextYoungId + self.survivorFrom.len();
        if gcRef.0 < survivorToBase + self.survivorTo.len() {
            return &self.survivorTo[gcRef.0 - survivorToBase];
        }
        let oldBase = survivorToBase + self.survivorTo.len();
        if gcRef.0 < oldBase + self.oldGen.len() {
            return &self.oldGen[gcRef.0 - oldBase];
        }
        panic!("GcRef {} out of range", gcRef.0);
    }

    fn find_mut(&mut self, gcRef: GcRef) -> &mut GcObject {
        if gcRef.0 < self.nextYoungId {
            return &mut self.youngGen[gcRef.0];
        }
        if gcRef.0 < self.nextYoungId + self.survivorFrom.len() {
            return &mut self.survivorFrom[gcRef.0 - self.nextYoungId];
        }
        let survivorToBase = self.nextYoungId + self.survivorFrom.len();
        if gcRef.0 < survivorToBase + self.survivorTo.len() {
            return &mut self.survivorTo[gcRef.0 - survivorToBase];
        }
        let oldBase = survivorToBase + self.survivorTo.len();
        if gcRef.0 < oldBase + self.oldGen.len() {
            return &mut self.oldGen[gcRef.0 - oldBase];
        }
        panic!("GcRef {} out of range", gcRef.0);
    }

    pub fn MinorGc(&mut self) {
        self.minorGcCount += 1;
        self.forwardingTable.clear();

        let mut copiedFromYoung: Vec<GcObject> = Vec::new();
        let mut copiedFromSurvivor: Vec<GcObject> = Vec::new();
        let mut promoted: Vec<GcObject> = Vec::new();

        // Copy young generation to survivorTo
        for (id, obj) in self.youngGen.drain(..).enumerate() {
            let objId = id;
            let (isSurvived, obj) = if obj.marked {
                if obj.age + 1 >= SURVIVOR_MAX_AGE {
                    (false, GcObject { ..obj })
                } else {
                    (true, GcObject { age: obj.age + 1, generation: Generation::Survivor, ..obj })
                }
            } else {
                (false, obj)
            };
            if isSurvived {
                let newId = self.nextYoungId + self.survivorFrom.len() + self.survivorTo.len() + copiedFromSurvivor.len();
                self.forwardingTable.insert(objId, GcRef(newId));
                copiedFromYoung.push(obj);
            }
        }

        // Copy survivorFrom to survivorTo (also age them)
        let survivorFromBase = self.nextYoungId;
        for (i, obj) in self.survivorFrom.drain(..).enumerate() {
            let objId = survivorFromBase + i;
            if obj.marked {
                if obj.age + 1 >= SURVIVOR_MAX_AGE {
                    promoted.push(GcObject { age: obj.age + 1, generation: Generation::Old, ..obj });
                } else {
                    let newId = self.nextYoungId + self.survivorTo.len() + copiedFromSurvivor.len() + copiedFromYoung.len();
                    self.forwardingTable.insert(objId, GcRef(newId));
                    copiedFromSurvivor.push(GcObject { age: obj.age + 1, generation: Generation::Survivor, ..obj });
                }
            }
        }

        // Apply forwardee to references in promoted objects
        for obj in &mut promoted {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }
        for obj in &mut copiedFromYoung {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }
        for obj in &mut copiedFromSurvivor {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }

        // Promote aged objects to old gen
        for obj in promoted {
            self.oldGen.push(obj);
        }

        // Swap survivorFrom <-> survivorTo: move copied objects into survivorTo
        self.survivorTo.extend(copiedFromYoung);
        self.survivorTo.extend(copiedFromSurvivor);

        // Swap survivorFrom and survivorTo roles
        std::mem::swap(&mut self.survivorFrom, &mut self.survivorTo);
        self.survivorTo.clear();

        self.cardTable.Clear();

        if self.oldGen.len() >= OLD_GEN_MAX {
            self.MajorGc();
        }

        self.allocCount = 0;
    }

    fn apply_forwarding(obj: &mut GcObject, table: &HashMap<usize, GcRef>) {
        match &mut obj.kind {
            ObjectKind::Instance { fields, .. } => {
                for (_, v) in fields.iter_mut() {
                    Self::forward_value(v, table);
                }
            }
            ObjectKind::Array { elements } => {
                for v in elements.iter_mut() {
                    Self::forward_value(v, table);
                }
            }
            ObjectKind::Upvalue { value, .. } => {
                Self::forward_value(value, table);
            }
            _ => {}
        }
    }

    fn forward_value(v: &mut RuntimeValue, table: &HashMap<usize, GcRef>) {
        match &mut v.payload {
            ValuePayload::Object(r) | ValuePayload::Array(r) | ValuePayload::Upvalue(r) => {
                if let Some(newRef) = table.get(&r.0) {
                    *r = *newRef;
                }
            }
            ValuePayload::Closure(c) => {
                for r in c.upvalues.iter_mut() {
                    if let Some(newRef) = table.get(&r.0) {
                        *r = *newRef;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn MajorGc(&mut self) {
        self.majorGcCount += 1;
        for obj in &mut self.oldGen {
            obj.marked = false;
        }
        self.SweepOldGen();
    }

    fn SweepOldGen(&mut self) {
        self.oldGen.retain(|obj| obj.marked);
    }

    pub fn WriteBarrier(&mut self, oldRef: GcRef, newRef: GcRef) {
        if self.is_in_old_gen(oldRef) && self.is_in_young_gen(newRef) {
            self.cardTable.MarkCard(newRef);
        }
    }

    pub fn ScanRememberedSet(&mut self, _markFn: impl FnMut(GcRef)) {
        for card in self.cardTable.DirtyCards() {
            // In this simplified version, we scan the card directly
            let _ = card;
        }
        self.cardTable.Clear();
    }

    fn is_in_old_gen(&self, r: GcRef) -> bool {
        let survivorToBase = self.nextYoungId + self.survivorFrom.len();
        let oldBase = survivorToBase + self.survivorTo.len();
        r.0 >= oldBase
    }

    fn is_in_young_gen(&self, r: GcRef) -> bool {
        r.0 < self.nextYoungId + self.survivorFrom.len() + self.survivorTo.len()
    }

    pub fn YoungGenSize(&self) -> usize {
        self.youngGen.len() + self.survivorFrom.len() + self.survivorTo.len()
    }
    pub fn OldGenSize(&self) -> usize {
        self.oldGen.len()
    }
    pub fn MinorGcCount(&self) -> u64 { self.minorGcCount }
    pub fn MajorGcCount(&self) -> u64 { self.majorGcCount }

    pub fn GetCardTable(&self) -> &CardTable {
        &self.cardTable
    }
    pub fn GetCardTableMut(&mut self) -> &mut CardTable {
        &mut self.cardTable
    }
}

impl GcObject {
    fn new_instance_default(kind: ObjectKind) -> Self {
        match kind {
            ObjectKind::Instance { fields, class } => GcObject::new_instance(fields, class),
            _ => unreachable!(),
        }
    }
}

// ---- Card Table (Remembered Set) ----

pub struct CardTable {
    cards: Vec<u8>,
    dirtyCount: usize,
}

impl Default for CardTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CardTable {
    pub fn new() -> Self {
        CardTable {
            cards: vec![0u8; 256],
            dirtyCount: 0,
        }
    }

    pub fn MarkCard(&mut self, _ref: GcRef) {
        let cardIdx = self.dirtyCount % self.cards.len();
        if self.cards[cardIdx] == 0 {
            self.dirtyCount += 1;
        }
        self.cards[cardIdx] = 1;
    }

    pub fn DirtyCards(&self) -> Vec<usize> {
        self.cards
            .iter()
            .enumerate()
            .filter(|(_, &v)| v != 0)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn Clear(&mut self) {
        self.cards = vec![0u8; 256];
        self.dirtyCount = 0;
    }
}

// ---- CallFrame ----

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub funcIndex: usize,
    pub ip: usize,
    pub registers: Vec<RuntimeValue>,
    pub stackStart: usize,
    pub returnAddr: usize,
    pub upvalues: Vec<GcRef>,
    pub tier: CompilationTier,
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
        }
    }
}

use super::error::VmResult;