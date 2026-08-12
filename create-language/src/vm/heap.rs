use std::collections::HashMap;

use super::value::*;
use super::gc_strategy::GcStrategy;

const YOUNG_GEN_MAX: usize = 1024;
const OLD_GEN_MAX: usize = 4096;
const SURVIVOR_MAX_AGE: u8 = 15;

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

        for obj in &mut promoted {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }
        for obj in &mut copiedFromYoung {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }
        for obj in &mut copiedFromSurvivor {
            Self::apply_forwarding(obj, &self.forwardingTable);
        }

        for obj in promoted {
            self.oldGen.push(obj);
        }

        self.survivorTo.extend(copiedFromYoung);
        self.survivorTo.extend(copiedFromSurvivor);

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

impl GcStrategy for Heap {
    fn Alloc(&mut self, kind: ObjectKind) -> GcRef {
        Heap::Alloc(self, kind)
    }

    fn MinorGc(&mut self) {
        Heap::MinorGc(self)
    }

    fn MajorGc(&mut self) {
        Heap::MajorGc(self)
    }

    fn WriteBarrier(&mut self, oldRef: GcRef, newRef: GcRef) {
        Heap::WriteBarrier(self, oldRef, newRef)
    }

    fn CollectGarbage(&mut self) {
        self.MinorGc();
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