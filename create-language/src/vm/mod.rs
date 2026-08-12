pub mod error;
pub mod executor;
pub mod jit;
pub mod memory;

use self::error::*;
use self::executor::Executor;
use self::jit::JitContext;
use self::memory::RuntimeValue;
use crate::binary::ModuleFile;

pub use self::memory::RuntimeValue as Value;

pub struct Vm {
    pub executor: Executor,
    pub jit: JitContext,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        let mut vm = Vm {
            executor: Executor::new(),
            jit: JitContext::new(),
        };
        vm.jit.InitDispatchTable(&mut vm.executor.dispatchTable);
        vm
    }

    pub fn LoadModule(&mut self, module: ModuleFile) -> VmResult<()> {
        let entryPoint = module.entryPoint as usize;
        let func = &module.functions[entryPoint];

        let mut registers = Vec::with_capacity(func.numRegisters);
        registers.resize(func.numRegisters, RuntimeValue::Nil);

        let frame = memory::CallFrame {
            funcIndex: entryPoint,
            ip: 0,
            registers,
            stackStart: 0,
            returnAddr: 0,
            upvalues: Vec::new(),
        };

        self.executor.frames.push(frame);
        self.executor.LoadModule(module);
        self.executor.currentFuncIndex = 0;
        self.executor.ip = 0;
        Ok(())
    }

    pub fn Exec(&mut self) -> VmResult<()> {
        self.executor.Execute()
    }

    pub fn ExecFunc(
        &mut self,
        funcIndex: usize,
        args: Vec<RuntimeValue>,
    ) -> VmResult<RuntimeValue> {
        let module = self
            .executor
            .module
            .as_ref()
            .ok_or_else(|| RuntimeError::new(ErrorKind::Custom, "no module loaded"))?;
        let func = &module.functions[funcIndex];

        if func.arity != args.len() {
            return Err(RuntimeError::new(
                ErrorKind::ArityMismatch,
                format!(
                    "function '{}' expects {} arguments, got {}",
                    func.name,
                    func.arity,
                    args.len()
                ),
            ));
        }

        let mut registers = Vec::with_capacity(func.numRegisters);
        registers.extend(args);
        registers.resize(func.numRegisters, RuntimeValue::Nil);

        let mut upvalues = Vec::new();
        for _upvDesc in &func.upvalueDescs {
            let gcRef = self
                .executor
                .heap
                .allocObj(memory::GcObject::new_upvalue(RuntimeValue::Nil, false));
            upvalues.push(gcRef);
        }

        let frame = memory::CallFrame {
            funcIndex,
            ip: 0,
            registers,
            stackStart: 0,
            returnAddr: 0,
            upvalues,
        };

        self.executor.Reset();
        self.executor.frames.push(frame);
        self.executor.currentFuncIndex = 0;
        self.executor.ip = 0;
        self.executor.halted = false;

        self.executor.Execute()?;

        // Return value is on top of the stack
        if let Some(val) = self.executor.stack.pop() {
            Ok(val)
        } else {
            Ok(RuntimeValue::Nil)
        }
    }
}
