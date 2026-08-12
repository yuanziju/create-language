use super::value::{GcRef, ObjectKind};
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