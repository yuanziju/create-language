use crate::instruction::Instruction;
use crate::opcode::Opcode;
use crate::vm::error::VmResult;
use crate::vm::executor::Executor;
use crate::vm::executor_backend::ExecutionBackend;
use crate::vm::value::CallFrame;

use super::cache::InlineCache;

pub struct InterpreterBackend {
    pub inline_cache: InlineCache,
    pub instruction_count: u64,
}

impl Default for InterpreterBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl InterpreterBackend {
    pub fn new() -> Self {
        InterpreterBackend {
            inline_cache: InlineCache::new(),
            instruction_count: 0,
        }
    }

    fn check_inline_cache(&mut self, executor: &mut Executor, inst: Instruction) -> Option<bool> {
        if inst.opcode() != Opcode::Call {
            return None;
        }

        let func_val = executor.reg(inst.b()).clone();
        let actual_target = match &func_val.payload {
            crate::vm::value::ValuePayload::Func(idx) => Some(*idx),
            crate::vm::value::ValuePayload::Closure(c) => Some(c.funcIndex),
            _ => None,
        };

        let actual_tag = func_val.tag;

        match &self.inline_cache.kind {
            super::cache::InlineCacheKind::Monomorphic { target, tag } => {
                if Some(*target) == actual_target && *tag == actual_tag {
                    self.inline_cache.hit_count += 1;
                    Some(true)
                } else {
                    self.inline_cache
                        .update(actual_target.unwrap_or(0), actual_tag);
                    Some(false)
                }
            }
            _ => {
                if let Some(target) = actual_target {
                    self.inline_cache.update(target, actual_tag);
                }
                None
            }
        }
    }
}

impl ExecutionBackend for InterpreterBackend {
    fn ExecuteFrame(
        &mut self,
        executor: &mut Executor,
        frame: &mut CallFrame,
        instructions: &[Instruction],
    ) -> VmResult<()> {
        let mut ip = frame.ip;

        while ip < instructions.len() && !executor.halted {
            let inst = instructions[ip];
            self.instruction_count += 1;

            if inst.opcode() == Opcode::Call {
                let ic_result = self.check_inline_cache(executor, inst);
                if ic_result == Some(true) {}
            }

            let handler = executor.dispatchTable.handlers[inst.opcode() as usize];
            handler(executor, inst)?;

            ip = executor.ip;
        }

        Ok(())
    }
}
