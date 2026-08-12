use crate::vm::value::{GcRef, Value};

pub struct CallFrame {
    pub ip: usize,
    pub registers: Vec<Value>,
    pub stackStart: usize,
    pub funcIndex: usize,
    pub returnAddr: usize,
    pub openUpvalues: Vec<GcRef>,
}

impl CallFrame {
    pub fn new(
        funcIndex: usize,
        numRegisters: usize,
        stackStart: usize,
        returnAddr: usize,
    ) -> Self {
        CallFrame {
            ip: 0,
            registers: vec![Value::Nil; numRegisters],
            stackStart,
            funcIndex,
            returnAddr,
            openUpvalues: Vec::new(),
        }
    }
}
