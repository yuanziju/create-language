use std::collections::HashMap;

use crate::instruction::Instruction;

use super::executor::DispatchTable;
use super::memory::CompilationTier;

pub const TIER1_THRESHOLD: u64 = 10;
pub const TIER2_THRESHOLD: u64 = 100;

pub struct JitContext {
    pub enabled: bool,
    pub compileCount: usize,
    pub codeCache: CodeCache,
    pub tierInfo: HashMap<usize, CompilationInfo>,
}

impl Default for JitContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct CodeCache {
    pub baselineCount: usize,
    pub optimizedCount: usize,
}

#[derive(Debug, Clone)]
pub struct CompilationInfo {
    pub funcIndex: usize,
    pub tier: CompilationTier,
    pub compileCount: u64,
    pub deoptCount: u64,
}

impl CompilationInfo {
    pub fn new(funcIndex: usize) -> Self {
        CompilationInfo {
            funcIndex,
            tier: CompilationTier::Interpreter,
            compileCount: 0,
            deoptCount: 0,
        }
    }
}

impl JitContext {
    pub fn new() -> Self {
        JitContext {
            enabled: false,
            compileCount: 0,
            codeCache: CodeCache::default(),
            tierInfo: HashMap::new(),
        }
    }

    pub fn InitDispatchTable(&mut self, _table: &mut DispatchTable) {
    }

    pub fn CheckAndCompile(
        &mut self,
        funcIndex: usize,
        callCount: u64,
        _instructions: &[Instruction],
    ) -> Option<CompilationTier> {
        if !self.enabled {
            return None;
        }

        let info = self.tierInfo.entry(funcIndex).or_insert_with(|| CompilationInfo::new(funcIndex));

        let newTier = if callCount >= TIER2_THRESHOLD && info.tier != CompilationTier::OptimizingJit {
            CompilationTier::OptimizingJit
        } else if callCount >= TIER1_THRESHOLD && info.tier == CompilationTier::Interpreter {
            CompilationTier::BaselineJit
        } else {
            return None;
        };

        info.tier = newTier;
        info.compileCount += 1;
        self.compileCount += 1;

        match newTier {
            CompilationTier::BaselineJit => self.codeCache.baselineCount += 1,
            CompilationTier::OptimizingJit => self.codeCache.optimizedCount += 1,
            _ => {}
        }

        Some(newTier)
    }

    pub fn Compile(
        &mut self,
        funcIndex: usize,
        instructions: &[Instruction],
    ) -> CompilationTier {
        self.compileCount += 1;
        self.CheckAndCompile(funcIndex, u64::MAX, instructions)
            .unwrap_or(CompilationTier::Interpreter)
    }

    pub fn Deoptimize(&mut self, funcIndex: usize) {
        if let Some(info) = self.tierInfo.get_mut(&funcIndex) {
            info.deoptCount += 1;
            if info.tier == CompilationTier::OptimizingJit {
                info.tier = CompilationTier::BaselineJit;
            } else {
                info.tier = CompilationTier::Interpreter;
            }
        }
    }

    pub fn GetTier(&self, funcIndex: usize) -> CompilationTier {
        self.tierInfo.get(&funcIndex).map(|i| i.tier).unwrap_or(CompilationTier::Interpreter)
    }

    pub fn TierInfo(&self) -> &HashMap<usize, CompilationInfo> {
        &self.tierInfo
    }

    pub fn CodeCacheStats(&self) -> &CodeCache {
        &self.codeCache
    }
}

pub type _DeoptHandler = fn(&[Instruction], usize) -> VmResult<Instruction>;

use super::error::VmResult;