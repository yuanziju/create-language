use std::collections::HashMap;

use crate::instruction::Instruction;
use crate::vm::executor::DispatchTable;
use crate::vm::executor_backend::DeoptContext;
use crate::vm::value::CompilationTier;

pub use self::c0::C0Stub;
pub use self::cache::{CallCounter, CodeCache, InlineCache};
pub use self::deopt::C2IDeoptimizer;
pub use self::interpreter::InterpreterBackend;

pub const TIER1_THRESHOLD: u64 = 10;
pub const TIER2_THRESHOLD: u64 = 100;

pub mod c0;
pub mod cache;
pub mod deopt;
pub mod interpreter;

#[derive(Debug, Clone)]
pub struct CompilationInfo {
    pub func_index: usize,
    pub tier: CompilationTier,
    pub compile_count: u64,
    pub deopt_count: u64,
}

impl CompilationInfo {
    pub fn new(func_index: usize) -> Self {
        CompilationInfo {
            func_index,
            tier: CompilationTier::Interpreter,
            compile_count: 0,
            deopt_count: 0,
        }
    }
}

pub struct JitContext {
    pub enabled: bool,
    pub compile_count: usize,
    pub code_cache: CodeCache,
    pub tier_info: HashMap<usize, CompilationInfo>,
    pub c0_stubs: HashMap<usize, C0Stub>,
    pub call_counters: HashMap<usize, CallCounter>,
    pub interpreter_backend: InterpreterBackend,
    pub deoptimizer: C2IDeoptimizer,
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
            compile_count: 0,
            code_cache: CodeCache::default(),
            tier_info: HashMap::new(),
            c0_stubs: HashMap::new(),
            call_counters: HashMap::new(),
            interpreter_backend: InterpreterBackend::new(),
            deoptimizer: C2IDeoptimizer::new(),
        }
    }

    pub fn InitDispatchTable(&mut self, _table: &mut DispatchTable) {}

    pub fn CheckAndCompile(
        &mut self,
        func_index: usize,
        call_count: u64,
        _instructions: &[Instruction],
    ) -> Option<CompilationTier> {
        if !self.enabled {
            return None;
        }

        let info = self
            .tier_info
            .entry(func_index)
            .or_insert_with(|| CompilationInfo::new(func_index));

        let new_tier =
            if call_count >= TIER2_THRESHOLD && info.tier != CompilationTier::OptimizingJit {
                CompilationTier::OptimizingJit
            } else if call_count >= TIER1_THRESHOLD && info.tier == CompilationTier::Interpreter {
                CompilationTier::BaselineJit
            } else {
                return None;
            };

        info.tier = new_tier;
        info.compile_count += 1;
        self.compile_count += 1;

        match new_tier {
            CompilationTier::BaselineJit => self.code_cache.baseline_count += 1,
            CompilationTier::OptimizingJit => self.code_cache.optimized_count += 1,
            _ => {}
        }

        Some(new_tier)
    }

    pub fn Compile(&mut self, func_index: usize, instructions: &[Instruction]) -> CompilationTier {
        self.CheckAndCompile(func_index, u64::MAX, instructions)
            .unwrap_or(CompilationTier::Interpreter)
    }

    pub fn CompileToC0(
        &mut self,
        func_index: usize,
        instructions: &[Instruction],
        dispatch_table: &DispatchTable,
    ) -> bool {
        if self.c0_stubs.contains_key(&func_index) {
            return false;
        }

        let stub = C0Stub::new(func_index, instructions.to_vec(), dispatch_table);
        self.c0_stubs.insert(func_index, stub);
        true
    }

    pub fn GetC0Stub(&mut self, func_index: usize) -> Option<&mut C0Stub> {
        self.c0_stubs.get_mut(&func_index)
    }

    pub fn HasC0Stub(&self, func_index: usize) -> bool {
        self.c0_stubs.contains_key(&func_index)
    }

    pub fn Deoptimize(&mut self, func_index: usize) {
        if let Some(info) = self.tier_info.get_mut(&func_index) {
            info.deopt_count += 1;
            if info.tier == CompilationTier::OptimizingJit {
                info.tier = CompilationTier::BaselineJit;
            } else {
                info.tier = CompilationTier::Interpreter;
            }
        }

        if let Some(stub) = self.c0_stubs.get_mut(&func_index) {
            stub.inline_cache.invalidate();
            stub.deopt_pending = false;
        }
    }

    pub fn HandleDeoptContext(&mut self, ctx: DeoptContext) {
        let func_index = ctx.func_index;
        self.Deoptimize(func_index);
    }

    pub fn IncrementCallCounter(&mut self, func_index: usize) -> u64 {
        let counter = self
            .call_counters
            .entry(func_index)
            .or_insert_with(|| CallCounter::new(func_index));
        counter.count += 1;
        counter.count
    }

    pub fn GetCallCount(&self, func_index: usize) -> u64 {
        self.call_counters
            .get(&func_index)
            .map(|c| c.count)
            .unwrap_or(0)
    }

    pub fn ShouldCompile(&self, func_index: usize) -> bool {
        if !self.enabled {
            return false;
        }
        let count = self.GetCallCount(func_index);
        count >= TIER1_THRESHOLD
    }

    pub fn GetTier(&self, func_index: usize) -> CompilationTier {
        self.tier_info
            .get(&func_index)
            .map(|i| i.tier)
            .unwrap_or(CompilationTier::Interpreter)
    }

    pub fn GetBackend(&self, func_index: usize) -> CompilationTier {
        self.GetTier(func_index)
    }

    pub fn TierInfo(&self) -> &HashMap<usize, CompilationInfo> {
        &self.tier_info
    }

    pub fn CodeCacheStats(&self) -> &CodeCache {
        &self.code_cache
    }

    pub fn SetEnabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
