#![allow(non_snake_case)]

pub mod frontend;
pub mod ir;
pub mod memory;
pub mod vm;

pub use frontend::ast;
pub use frontend::lexer;
pub use frontend::parser;
pub use frontend::token;

pub use ir::binary;
pub use ir::compiler;
pub use ir::constant_pool;
pub use ir::instruction;
pub use ir::opcode;
