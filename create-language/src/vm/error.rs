use std::fmt;

#[derive(Debug, Clone)]
pub enum RuntimeError {
    StackOverflow,
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    UndefinedVariable(String),
    UndefinedFunction(String),
    DivisionByZero,
    IndexOutOfBounds {
        len: usize,
        index: usize,
    },
    NullReference,
    Custom(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackOverflow => write!(f, "stack overflow"),
            RuntimeError::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            RuntimeError::UndefinedVariable(name) => {
                write!(f, "undefined variable: {name}")
            }
            RuntimeError::UndefinedFunction(name) => {
                write!(f, "undefined function: {name}")
            }
            RuntimeError::DivisionByZero => write!(f, "division by zero"),
            RuntimeError::IndexOutOfBounds { len, index } => {
                write!(f, "index out of bounds: len={len}, index={index}")
            }
            RuntimeError::NullReference => write!(f, "null reference"),
            RuntimeError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
