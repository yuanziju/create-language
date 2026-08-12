use crate::instruction::Instruction;
use crate::opcode::Opcode;
use crate::vm::error::VmResult;
use crate::vm::executor::{DispatchTable, Executor, Handler};
use crate::vm::executor_backend::ExecutionBackend;
use crate::vm::value::CallFrame;

use super::cache::InlineCache;

pub const C0_DEFAULT_THRESHOLD: u64 = 10;

pub struct C0Stub {
    pub func_index: usize,
    pub instructions: Vec<Instruction>,
    pub handlers: Vec<Handler>,
    pub call_site_threshold: u64,
    pub inline_cache: InlineCache,
    pub deopt_pending: bool,
}

impl C0Stub {
    pub fn new(
        func_index: usize,
        instructions: Vec<Instruction>,
        dispatch_table: &DispatchTable,
    ) -> Self {
        let handlers: Vec<Handler> = instructions
            .iter()
            .map(|inst| dispatch_table.Get(inst.opcode()))
            .collect();

        C0Stub {
            func_index,
            instructions,
            handlers,
            call_site_threshold: C0_DEFAULT_THRESHOLD,
            inline_cache: InlineCache::new(),
            deopt_pending: false,
        }
    }

    pub fn execute_with_inline_cache(&mut self, executor: &mut Executor) -> VmResult<()> {
        let frame = &executor.frames[executor.currentFuncIndex];
        let frame_ip = frame.ip;
        let len = self.instructions.len();

        let mut ip = frame_ip;

        while ip < len && !executor.halted {
            let inst = self.instructions[ip];

            if inst.opcode() == Opcode::Call {
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
                        } else {
                            self.inline_cache
                                .update(actual_target.unwrap_or(0), actual_tag);
                            self.deopt_pending = true;
                        }
                    }
                    _ => {
                        if let Some(target) = actual_target {
                            self.inline_cache.update(target, actual_tag);
                        }
                    }
                }
            }

            let handler = self.handlers[ip];
            handler(executor, inst)?;

            ip = executor.ip;
        }

        Ok(())
    }
}

impl ExecutionBackend for C0Stub {
    fn ExecuteFrame(
        &mut self,
        executor: &mut Executor,
        _frame: &mut CallFrame,
        _instructions: &[Instruction],
    ) -> VmResult<()> {
        self.execute_with_inline_cache(executor)
    }
}
