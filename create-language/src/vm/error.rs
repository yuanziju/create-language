use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    StackOverflow,
    TypeMismatch,
    DivisionByZero,
    IndexOutOfBounds,
    ArityMismatch,
    UndefinedFunction,
    UndefinedVariable,
    NullReference,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError {
    pub kind: ErrorKind,
    pub message: String,
    pub trace: Vec<TraceFrame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceFrame {
    pub funcName: String,
    pub ip: usize,
}

pub type VmResult<T> = std::result::Result<T, RuntimeError>;

impl RuntimeError {
    pub fn new(kind: ErrorKind, msg: impl Into<String>) -> Self {
        RuntimeError {
            kind,
            message: msg.into(),
            trace: Vec::new(),
        }
    }

    pub fn type_error(expected: &str, found: &str) -> Self {
        RuntimeError::new(
            ErrorKind::TypeMismatch,
            format!("type mismatch: expected {expected}, found {found}"),
        )
    }

    pub fn push_trace(&mut self, funcName: String, ip: usize) {
        self.trace.push(TraceFrame { funcName, ip });
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)?;
        for frame in &self.trace {
            write!(f, "\n  at {}:{}", frame.funcName, frame.ip)?;
        }
        Ok(())
    }
}

impl std::error::Error for RuntimeError {}
