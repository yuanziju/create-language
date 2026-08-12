use crate::vm::error::VmResult;
use crate::vm::executor_backend::{DeoptContext, Deoptimizer};
use crate::vm::value::{CallFrame, CompilationTier};

pub struct C2IDeoptimizer;

impl Default for C2IDeoptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl C2IDeoptimizer {
    pub fn new() -> Self {
        C2IDeoptimizer
    }

    pub fn rebuild_frame(&self, frame: &mut CallFrame, ctx: &DeoptContext) {
        frame.registers = ctx.snapshot_registers.clone();
        frame.ip = ctx.snapshot_ip;
        frame.tier = ctx.to_tier;
        frame.RebuildRegisterBitmap();
    }

    pub fn should_deopt_from_tier(tier: CompilationTier) -> bool {
        matches!(
            tier,
            CompilationTier::BaselineJit | CompilationTier::OptimizingJit
        )
    }

    pub fn target_tier_from(current: CompilationTier) -> CompilationTier {
        match current {
            CompilationTier::OptimizingJit => CompilationTier::BaselineJit,
            _ => CompilationTier::Interpreter,
        }
    }
}

impl Deoptimizer for C2IDeoptimizer {
    fn deoptimize(&mut self, ctx: DeoptContext) -> VmResult<()> {
        let _target_tier = Self::target_tier_from(ctx.from_tier);

        if let Some(func_index) = ctx.func_index.checked_sub(0) {
            let _ = func_index;
        }

        let _snapshot_len = ctx.snapshot_registers.len();
        let _ = _snapshot_len;

        Ok(())
    }
}
