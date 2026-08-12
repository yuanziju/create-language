use super::executor::DispatchTable;
use crate::instruction::Instruction;

pub struct JitContext {
    pub enabled: bool,
    pub compileCount: usize,
}

impl Default for JitContext {
    fn default() -> Self {
        Self::new()
    }
}

impl JitContext {
    pub fn new() -> Self {
        JitContext {
            enabled: false,
            compileCount: 0,
        }
    }

    pub fn InitDispatchTable(&mut self, _table: &mut DispatchTable) {
        // Phase 1: no JIT, just stub registration
        // Future: replace table handlers with compiled machine code stubs
    }

    pub fn Compile(&mut self, _funcIndex: usize, _instructions: &[Instruction]) {
        // Phase 1: stub, record compile count
        self.compileCount += 1;
    }
}
