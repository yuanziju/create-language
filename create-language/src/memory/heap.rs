use std::collections::HashMap;

use super::gc_strategy::GcStrategy;
use super::value::*;

const YOUNG_GEN_MAX: usize = 1024;
const OLD_GEN_MAX: usize = 4096;
const SURVIVOR_MAX_AGE: u8 = 15;
const CARD_TABLE_SIZE: usize = 4096;

#[derive(Default)]
pub struct Heap {
    youngGen: Vec<GcObject>,
    survivorFrom: Vec<GcObject>,
    survivorTo: Vec<GcObject>,
    oldGen: Vec<GcObject>,
    nextId: usize,
    allocCount: usize,
    minorGcCount: u64,
    majorGcCount: u64,
    cardTable: CardTable,
    forwardingTable: HashMap<usize, GcRef>,
    rememberedSet: Vec<GcRef>,
}

impl Heap {
    pub fn new() -> Self {
        Heap::default()
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.nextId;
        self.nextId += 1;
        id
    }

    pub fn Alloc(&mut self, kind: ObjectKind) -> GcRef {
        if self.youngGen.len() >= YOUNG_GEN_MAX {
            self.MinorGc(&[]);
        }
        let id = self.alloc_id();
        let mut obj = match kind {
            k @ ObjectKind::Instance { .. } => GcObject::new_instance_default(k),
            ObjectKind::Array { elements } => GcObject::new_array(elements),
            ObjectKind::Upvalue { value, .. } => GcObject::new_upvalue(value),
            ObjectKind::Str { chars } => GcObject::new_str(chars),
            ObjectKind::Bytes { data } => GcObject::new_bytes(data),
        };
        obj.id = id;
        self.youngGen.push(obj);
        self.allocCount += 1;
        GcRef(id)
    }

    pub fn AllocObj(&mut self, mut obj: GcObject) -> GcRef {
        if self.youngGen.len() >= YOUNG_GEN_MAX {
            self.MinorGc(&[]);
        }
        let id = self.alloc_id();
        obj.id = id;
        self.youngGen.push(obj);
        self.allocCount += 1;
        GcRef(id)
    }

    pub fn PromoteObject(&mut self, mut obj: GcObject) -> GcRef {
        obj.generation = Generation::Old;
        obj.age = 0;
        let id = self.alloc_id();
        obj.id = id;
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
        for obj in &self.youngGen {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        for obj in &self.survivorFrom {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        for obj in &self.survivorTo {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        for obj in &self.oldGen {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        panic!("GcRef({}) not found in any generation", gcRef.0);
    }

    fn find_mut(&mut self, gcRef: GcRef) -> &mut GcObject {
        for obj in &mut self.youngGen {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        for obj in &mut self.survivorFrom {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        for obj in &mut self.survivorTo {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        for obj in &mut self.oldGen {
            if obj.id == gcRef.0 {
                return obj;
            }
        }
        panic!("GcRef({}) not found in any generation", gcRef.0);
    }

    #[allow(dead_code, clippy::manual_find)]
    fn try_find(&self, gcRef: GcRef) -> Option<&GcObject> {
        for obj in &self.youngGen {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        for obj in &self.survivorFrom {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        for obj in &self.survivorTo {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        for obj in &self.oldGen {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        None
    }

    #[allow(clippy::manual_find)]
    fn try_find_mut(&mut self, gcRef: GcRef) -> Option<&mut GcObject> {
        for obj in &mut self.youngGen {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        for obj in &mut self.survivorFrom {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        for obj in &mut self.survivorTo {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        for obj in &mut self.oldGen {
            if obj.id == gcRef.0 {
                return Some(obj);
            }
        }
        None
    }

    pub fn MinorGc(&mut self, roots: &[GcRef]) {
        self.minorGcCount += 1;
        self.forwardingTable.clear();

        let mut worklist: Vec<GcRef> = roots.to_vec();
        for r in self.rememberedSet.drain(..) {
            worklist.push(r);
        }

        while let Some(r) = worklist.pop() {
            if let Some(obj) = self.try_find_mut(r) {
                if obj.marked {
                    continue;
                }
                obj.marked = true;
                if obj.generation != Generation::Old {
                    worklist.extend(obj.children());
                }
            }
        }

        let young_snapshot: Vec<GcObject> = self.youngGen.drain(..).collect();
        let survivor_snapshot: Vec<GcObject> = self.survivorFrom.drain(..).collect();

        let mut copied_from_young: Vec<GcObject> = Vec::new();
        let mut copied_from_survivor: Vec<GcObject> = Vec::new();
        let mut promoted: Vec<GcObject> = Vec::new();

        for obj in young_snapshot {
            if obj.marked {
                if obj.age + 1 >= SURVIVOR_MAX_AGE {
                    let new_id = self.alloc_id();
                    self.forwardingTable.insert(obj.id, GcRef(new_id));
                    promoted.push(GcObject {
                        id: new_id,
                        age: 0,
                        generation: Generation::Old,
                        marked: false,
                        ..obj
                    });
                } else {
                    let new_id = self.alloc_id();
                    self.forwardingTable.insert(obj.id, GcRef(new_id));
                    copied_from_young.push(GcObject {
                        id: new_id,
                        age: obj.age + 1,
                        generation: Generation::Survivor,
                        marked: false,
                        ..obj
                    });
                }
            }
        }

        for obj in survivor_snapshot {
            if obj.marked {
                if obj.age + 1 >= SURVIVOR_MAX_AGE {
                    let new_id = self.alloc_id();
                    self.forwardingTable.insert(obj.id, GcRef(new_id));
                    promoted.push(GcObject {
                        id: new_id,
                        age: 0,
                        generation: Generation::Old,
                        marked: false,
                        ..obj
                    });
                } else {
                    let new_id = self.alloc_id();
                    self.forwardingTable.insert(obj.id, GcRef(new_id));
                    copied_from_survivor.push(GcObject {
                        id: new_id,
                        age: obj.age + 1,
                        generation: Generation::Survivor,
                        marked: false,
                        ..obj
                    });
                }
            }
        }

        for obj in &mut promoted {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }
        for obj in &mut copied_from_young {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }
        for obj in &mut copied_from_survivor {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }

        self.oldGen.extend(promoted);
        self.survivorTo.extend(copied_from_young);
        self.survivorTo.extend(copied_from_survivor);

        std::mem::swap(&mut self.survivorFrom, &mut self.survivorTo);
        self.survivorTo.clear();

        self.cardTable.Clear();
        self.rememberedSet.clear();

        if self.oldGen.len() >= OLD_GEN_MAX {
            self.MajorGc(roots);
        }
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

    pub fn MajorGc(&mut self, roots: &[GcRef]) {
        self.majorGcCount += 1;

        for obj in &mut self.oldGen {
            obj.marked = false;
        }

        let mut worklist: Vec<GcRef> = roots.to_vec();

        for obj in &self.survivorFrom {
            worklist.push(GcRef(obj.id));
        }
        for obj in &self.survivorTo {
            worklist.push(GcRef(obj.id));
        }
        for obj in &self.youngGen {
            worklist.push(GcRef(obj.id));
        }

        while let Some(r) = worklist.pop() {
            if let Some(obj) = self.try_find_mut(r) {
                if obj.marked {
                    continue;
                }
                obj.marked = true;
                worklist.extend(obj.children());
            }
        }

        let total_old = self.oldGen.len();
        let survived_old = self.oldGen.iter().filter(|o| o.marked).count();

        if total_old > 0 {
            let frag_rate = (total_old - survived_old) as f64 / total_old as f64;
            if frag_rate > 0.35 {
                self.Lisp2Compact(roots);
            } else {
                self.SweepOldGen();
            }
        }
    }

    fn SweepOldGen(&mut self) {
        self.oldGen.retain(|obj| obj.marked);
    }

    pub fn Lisp2Compact(&mut self, _roots: &[GcRef]) {
        if self.oldGen.is_empty() {
            return;
        }

        self.forwardingTable.clear();

        let mut old_gen = std::mem::take(&mut self.oldGen);
        let mut write_idx: usize = 0;

        for read_idx in 0..old_gen.len() {
            if old_gen[read_idx].marked {
                if write_idx != read_idx {
                    let (left, right) = old_gen.split_at_mut(read_idx);
                    let moved =
                        std::mem::replace(&mut left[write_idx], std::mem::take(&mut right[0]));
                    self.forwardingTable
                        .insert(moved.id, GcRef(left[write_idx].id));
                }
                write_idx += 1;
            }
        }

        old_gen.truncate(write_idx);
        self.oldGen = old_gen;

        {
            let old_snapshot: Vec<GcObject> = self.oldGen.drain(..).collect();
            for mut obj in old_snapshot {
                Self::apply_forwarding(&mut obj, &self.forwardingTable);
                self.oldGen.push(obj);
            }
        }

        {
            let young_snapshot: Vec<GcObject> = self.youngGen.drain(..).collect();
            for mut obj in young_snapshot {
                Self::apply_forwarding(&mut obj, &self.forwardingTable);
                self.youngGen.push(obj);
            }
        }

        {
            let sf_snapshot: Vec<GcObject> = self.survivorFrom.drain(..).collect();
            for mut obj in sf_snapshot {
                Self::apply_forwarding(&mut obj, &self.forwardingTable);
                self.survivorFrom.push(obj);
            }
        }

        {
            let st_snapshot: Vec<GcObject> = self.survivorTo.drain(..).collect();
            for mut obj in st_snapshot {
                Self::apply_forwarding(&mut obj, &self.forwardingTable);
                self.survivorTo.push(obj);
            }
        }
    }

    pub fn WriteBarrier(&mut self, oldRef: GcRef, newRef: GcRef) {
        if self.is_in_old_gen(oldRef) && self.is_in_young_gen(newRef) {
            self.cardTable.MarkCard(newRef);
            self.rememberedSet.push(newRef);
        }
    }

    pub fn is_in_old_gen(&self, r: GcRef) -> bool {
        self.oldGen.iter().any(|o| o.id == r.0)
    }

    pub fn is_in_young_gen(&self, r: GcRef) -> bool {
        self.youngGen.iter().any(|o| o.id == r.0)
            || self.survivorFrom.iter().any(|o| o.id == r.0)
            || self.survivorTo.iter().any(|o| o.id == r.0)
    }

    pub fn YoungGenSize(&self) -> usize {
        self.youngGen.len() + self.survivorFrom.len() + self.survivorTo.len()
    }

    pub fn OldGenSize(&self) -> usize {
        self.oldGen.len()
    }

    pub fn MinorGcCount(&self) -> u64 {
        self.minorGcCount
    }
    pub fn MajorGcCount(&self) -> u64 {
        self.majorGcCount
    }
    pub fn AllocCount(&self) -> usize {
        self.allocCount
    }

    pub fn GetCardTable(&self) -> &CardTable {
        &self.cardTable
    }
    pub fn GetCardTableMut(&mut self) -> &mut CardTable {
        &mut self.cardTable
    }

    pub fn ApplyForwardingToValue(v: &mut RuntimeValue, table: &HashMap<usize, GcRef>) {
        Self::forward_value(v, table);
    }

    pub fn GetForwardingTable(&self) -> &HashMap<usize, GcRef> {
        &self.forwardingTable
    }
}

impl GcStrategy for Heap {
    fn Alloc(&mut self, kind: ObjectKind) -> GcRef {
        Heap::Alloc(self, kind)
    }

    fn MinorGc(&mut self) {
        Heap::MinorGc(self, &[])
    }

    fn MajorGc(&mut self) {
        Heap::MajorGc(self, &[])
    }

    fn WriteBarrier(&mut self, oldRef: GcRef, newRef: GcRef) {
        Heap::WriteBarrier(self, oldRef, newRef)
    }

    fn CollectGarbage(&mut self) {
        self.MajorGc(&[]);
    }

    fn GetCardTable(&self) -> &CardTable {
        Heap::GetCardTable(self)
    }

    fn GetCardTableMut(&mut self) -> &mut CardTable {
        Heap::GetCardTableMut(self)
    }

    fn YoungGenSize(&self) -> usize {
        Heap::YoungGenSize(self)
    }

    fn OldGenSize(&self) -> usize {
        Heap::OldGenSize(self)
    }

    fn MinorGcCount(&self) -> u64 {
        Heap::MinorGcCount(self)
    }

    fn MajorGcCount(&self) -> u64 {
        Heap::MajorGcCount(self)
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

pub struct CardTable {
    cards: Vec<u8>,
}

impl Default for CardTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CardTable {
    pub fn new() -> Self {
        CardTable {
            cards: vec![0u8; CARD_TABLE_SIZE],
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        CardTable {
            cards: vec![0u8; cap],
        }
    }

    pub fn MarkCard(&mut self, r: GcRef) {
        let idx = r.0 % self.cards.len();
        self.cards[idx] = 1;
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
        let cap = self.cards.len();
        self.cards = vec![0u8; cap];
    }

    pub fn Len(&self) -> usize {
        self.cards.len()
    }
}
