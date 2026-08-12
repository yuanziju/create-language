use crate::instruction::Instruction;
use crate::vm::error::VmResult;
use crate::vm::executor::Executor;
use crate::vm::memory::{CallFrame, CompilationTier, RuntimeValue};

pub trait ExecutionBackend {
    fn ExecuteFrame(
        &mut self,
        executor: &mut Executor,
        frame: &mut CallFrame,
        instructions: &[Instruction],
    ) -> VmResult<()>;
}

pub struct DeoptContext {
    pub from_tier: CompilationTier,
    pub to_tier: CompilationTier,
    pub snapshot_registers: Vec<RuntimeValue>,
    pub snapshot_ip: usize,
    pub func_index: usize,
}

pub trait Deoptimizer {
    fn deoptimize(&mut self, ctx: DeoptContext) -> VmResult<()>;
}
