use super::value::{GcRef, ObjectKind, CallFrame, RuntimeValue};
use super::heap::CardTable;

pub trait GcStrategy {
    fn Alloc(&mut self, kind: ObjectKind) -> GcRef;
    fn MinorGc(&mut self);
    fn MajorGc(&mut self);
    fn WriteBarrier(&mut self, oldRef: GcRef, newRef: GcRef);
    fn CollectGarbage(&mut self);
    fn GetCardTable(&self) -> &CardTable;
    fn GetCardTableMut(&mut self) -> &mut CardTable;
    fn YoungGenSize(&self) -> usize;
    fn OldGenSize(&self) -> usize;
    fn MinorGcCount(&self) -> u64;
    fn MajorGcCount(&self) -> u64;
}

pub trait RootVisitor {
    fn visit_ref(&mut self, r: GcRef);
}

pub trait RootSet {
    fn scan(&self, visitor: &mut impl RootVisitor);
}

pub struct StackRootSet<'a> {
    pub frames: &'a [CallFrame],
    pub stack: &'a [RuntimeValue],
    pub globalRefs: &'a [GcRef],
}

impl RootSet for StackRootSet<'_> {
    fn scan(&self, visitor: &mut impl RootVisitor) {
        for frame in self.frames {
            frame.ScanRoots(visitor);
        }
        for v in self.stack {
            if let Some(r) = v.as_gc_ref() {
                visitor.visit_ref(r);
            }
        }
        for r in self.globalRefs {
            visitor.visit_ref(*r);
        }
    }
}