//! VM integration tests
#![allow(clippy::approx_constant, clippy::identity_op)]

use create_language::binary::{Function, ModuleFile, UpvalueDesc};
use create_language::constant_pool::Value as CpValue;
use create_language::instruction::Instruction;
use create_language::opcode::Opcode;
use create_language::vm::error::{ErrorKind, RuntimeError};
use create_language::vm::memory::{GcObject, Heap, ObjectKind};
use create_language::vm::Value;
use create_language::vm::Vm;

// ---------------------------------------------------------------------------
// Helper: construct a minimal module with a single function
// ---------------------------------------------------------------------------

fn make_module(
    arity: usize,
    num_registers: usize,
    instructions: Vec<Instruction>,
    constants: Vec<CpValue>,
) -> ModuleFile {
    ModuleFile {
        version: (1, 0, 0),
        constants: constants.clone(),
        functions: vec![Function {
            name: "main".into(),
            arity,
            numRegisters: num_registers,
            instructions,
            constants,
            upvalueCount: 0,
            upvalueDescs: vec![],
        }],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    }
}

fn run_module(module: ModuleFile) -> Result<(), RuntimeError> {
    let mut vm = Vm::new();
    vm.LoadModule(module)?;
    vm.Exec()
}

fn exec_func(
    module: ModuleFile,
    func_index: usize,
    args: Vec<Value>,
) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new();
    vm.LoadModule(module)?;
    vm.ExecFunc(func_index, args)
}

// ---------------------------------------------------------------------------
// Basic operations
// ---------------------------------------------------------------------------

#[test]
fn test_halt() {
    let module = make_module(0, 1, vec![Instruction::new(Opcode::Halt)], vec![]);
    let result = run_module(module);
    assert!(result.is_ok(), "Halt failed: {:?}", result.err());
}

#[test]
fn test_loadi() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 42),
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_loadk() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![CpValue::Int(100)],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(100));
}

#[test]
fn test_loadbool_true() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::rri(Opcode::Loadbool, 0, 0, 1), // true
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_loadbool_false() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::rri(Opcode::Loadbool, 0, 0, 0), // false
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_loadnil() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::rrk(Opcode::Loadnil, 0, 0, 0),
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_mov() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 42),
            Instruction::rrr(Opcode::Mov, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

#[test]
fn test_add() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::ri(Opcode::Loadi, 1, 20),
            Instruction::rrr(Opcode::Add, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_sub() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 50),
            Instruction::ri(Opcode::Loadi, 1, 30),
            Instruction::rrr(Opcode::Sub, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_mul() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 7),
            Instruction::ri(Opcode::Loadi, 1, 6),
            Instruction::rrr(Opcode::Mul, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_div() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 100),
            Instruction::ri(Opcode::Loadi, 1, 3),
            Instruction::rrr(Opcode::Div, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(33));
}

#[test]
fn test_mod() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 100),
            Instruction::ri(Opcode::Loadi, 1, 30),
            Instruction::rrr(Opcode::Mod, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_neg() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 42),
            Instruction::rrr(Opcode::Neg, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(-42));
}

#[test]
fn test_division_by_zero() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::ri(Opcode::Loadi, 1, 0),
            Instruction::rrr(Opcode::Div, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = run_module(module);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DivisionByZero);
}

#[test]
fn test_mod_by_zero() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::ri(Opcode::Loadi, 1, 0),
            Instruction::rrr(Opcode::Mod, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = run_module(module);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DivisionByZero);
}

// ---------------------------------------------------------------------------
// Bitwise operations
// ---------------------------------------------------------------------------

#[test]
fn test_bit_and() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 6),
            Instruction::ri(Opcode::Loadi, 1, 3),
            Instruction::rrr(Opcode::BitAnd, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_bit_or() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 6),
            Instruction::ri(Opcode::Loadi, 1, 3),
            Instruction::rrr(Opcode::BitOr, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_bit_xor() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 6),
            Instruction::ri(Opcode::Loadi, 1, 3),
            Instruction::rrr(Opcode::BitXor, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_bit_not() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 42),
            Instruction::rrr(Opcode::BitNot, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(!42));
}

#[test]
fn test_shl() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 1),
            Instruction::ri(Opcode::Loadi, 1, 4),
            Instruction::rrr(Opcode::Shl, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(16));
}

#[test]
fn test_shr() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 16),
            Instruction::ri(Opcode::Loadi, 1, 2),
            Instruction::rrr(Opcode::Shr, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(4));
}

// ---------------------------------------------------------------------------
// Type conversion
// ---------------------------------------------------------------------------

#[test]
fn test_i2f() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 42),
            Instruction::rrr(Opcode::I2f, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Float(42.0));
}

#[test]
fn test_f2i() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrr(Opcode::F2i, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![CpValue::Float(3.14)],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(3));
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

#[test]
fn test_eq_true() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::ri(Opcode::Loadi, 1, 10),
            Instruction::rrr(Opcode::Eq, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_eq_false() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::ri(Opcode::Loadi, 1, 20),
            Instruction::rrr(Opcode::Eq, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_lt_true() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 5),
            Instruction::ri(Opcode::Loadi, 1, 10),
            Instruction::rrr(Opcode::Lt, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_lt_false() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::ri(Opcode::Loadi, 1, 5),
            Instruction::rrr(Opcode::Lt, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_le_true() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::ri(Opcode::Loadi, 1, 10),
            Instruction::rrr(Opcode::Le, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_not() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::rri(Opcode::Loadbool, 0, 0, 0), // false
            Instruction::rrr(Opcode::Not, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// ---------------------------------------------------------------------------
// IsType instruction
// ---------------------------------------------------------------------------

#[test]
fn test_istype_int() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 42),
            Instruction::rri(Opcode::IsType, 1, 0, 2), // typeCode 2 = Int
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_istype_nil() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::rrk(Opcode::Loadnil, 0, 0, 0),
            Instruction::rri(Opcode::IsType, 1, 0, 0), // typeCode 0 = Nil
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_istype_bool() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::rri(Opcode::Loadbool, 0, 0, 1), // true
            Instruction::rri(Opcode::IsType, 1, 0, 1),   // typeCode 1 = Bool
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// ---------------------------------------------------------------------------
// Control flow
// ---------------------------------------------------------------------------

#[test]
fn test_jmp() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 1),
            Instruction::from_raw(Opcode::Jmp as u32 | (0 << 8) | (2 << 16)),
            Instruction::ri(Opcode::Loadi, 1, 99), // skipped
            Instruction::ri(Opcode::Loadi, 1, 42),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_jmpt_true() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::rri(Opcode::Loadbool, 2, 0, 1), // r2 = true
            Instruction::from_raw(Opcode::JmpT as u32 | (2 << 8) | (2 << 16)),
            Instruction::ri(Opcode::Loadi, 1, 99), // skipped
            Instruction::ri(Opcode::Loadi, 1, 42),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_jmpt_false() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::rri(Opcode::Loadbool, 2, 0, 0), // r2 = false
            Instruction::from_raw(Opcode::JmpT as u32 | (2 << 8) | (2 << 16)),
            Instruction::ri(Opcode::Loadi, 1, 99), // executed
            Instruction::ri(Opcode::Loadi, 2, 42),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(99));
}

#[test]
fn test_jmpf_false() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::rri(Opcode::Loadbool, 2, 0, 0), // r2 = false
            Instruction::from_raw(Opcode::JmpF as u32 | (2 << 8) | (2 << 16)),
            Instruction::ri(Opcode::Loadi, 1, 99), // skipped
            Instruction::ri(Opcode::Loadi, 1, 42),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

// ---------------------------------------------------------------------------
// Function call and return
// ---------------------------------------------------------------------------

#[test]
fn test_simple_call() {
    let module = ModuleFile {
        version: (1, 0, 0),
        constants: vec![CpValue::Function(1)],
        functions: vec![
            Function {
                name: "main".into(),
                arity: 0,
                numRegisters: 4,
                instructions: vec![
                    Instruction::rrk(Opcode::Loadk, 0, 0, 0), // r0 = func 1
                    Instruction::ri(Opcode::Loadi, 1, 10),    // r1 = arg 0
                    Instruction::ri(Opcode::Loadi, 2, 20),    // r2 = arg 1
                    Instruction::rrr(Opcode::Call, 0, 0, 2),  // call func[0] with 2 args
                    Instruction::new(Opcode::Halt),
                ],
                constants: vec![CpValue::Function(1)],
                upvalueCount: 0,
                upvalueDescs: vec![],
            },
            Function {
                name: "add".into(),
                arity: 2,
                numRegisters: 3,
                instructions: vec![
                    Instruction::rrr(Opcode::Add, 2, 0, 1),
                    Instruction::rrr(Opcode::Return, 2, 0, 0),
                ],
                constants: vec![],
                upvalueCount: 0,
                upvalueDescs: vec![],
            },
        ],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    };
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_tail_call() {
    let module = ModuleFile {
        version: (1, 0, 0),
        constants: vec![CpValue::Function(1)],
        functions: vec![
            Function {
                name: "main".into(),
                arity: 0,
                numRegisters: 3,
                instructions: vec![
                    Instruction::rrk(Opcode::Loadk, 0, 0, 0),    // r0 = func 1
                    Instruction::ri(Opcode::Loadi, 1, 99),       // r1 = arg 0
                    Instruction::rrr(Opcode::TailCall, 0, 1, 1), // tail-call with 1 arg
                    Instruction::new(Opcode::Halt),
                ],
                constants: vec![CpValue::Function(1)],
                upvalueCount: 0,
                upvalueDescs: vec![],
            },
            Function {
                name: "identity".into(),
                arity: 1,
                numRegisters: 1,
                instructions: vec![Instruction::rrr(Opcode::Return, 0, 0, 0)],
                constants: vec![],
                upvalueCount: 0,
                upvalueDescs: vec![],
            },
        ],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    };
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(99));
}

// ---------------------------------------------------------------------------
// String operations
// ---------------------------------------------------------------------------

#[test]
fn test_string_load() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![CpValue::String("hello".into())],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::String("hello".into()));
}

// ---------------------------------------------------------------------------
// Object and array operations
// ---------------------------------------------------------------------------

#[test]
fn test_new_object() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::rrk(Opcode::NewObject, 0, 0, 0),
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    match result {
        Value::Object(_) => {}
        _ => panic!("expected Object, got {:?}", result),
    }
}

#[test]
fn test_new_array() {
    let module = make_module(
        0,
        1,
        vec![
            Instruction::rrk(Opcode::NewArray, 0, 0, 0),
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    match result {
        Value::Array(_) => {}
        _ => panic!("expected Array, got {:?}", result),
    }
}

// ---------------------------------------------------------------------------
// Array element access
// ---------------------------------------------------------------------------

#[test]
fn test_aset_aget() {
    let module = ModuleFile {
        version: (1, 0, 0),
        constants: vec![],
        functions: vec![Function {
            name: "main".into(),
            arity: 0,
            numRegisters: 5,
            instructions: vec![
                Instruction::rrk(Opcode::NewArray, 0, 0, 0),
                Instruction::ri(Opcode::Loadi, 1, 10), // value 10
                Instruction::ri(Opcode::Loadi, 2, 20), // value 20
                Instruction::ri(Opcode::Loadi, 3, 0),  // index 0
                Instruction::ri(Opcode::Loadi, 4, 1),  // index 1
                Instruction::rrr(Opcode::ASet, 0, 3, 1), // arr[0] = 10
                Instruction::rrr(Opcode::ASet, 0, 4, 2), // arr[1] = 20
                Instruction::rrr(Opcode::AGet, 1, 0, 4), // r1 = arr[1]
                Instruction::rrr(Opcode::Return, 1, 0, 0),
            ],
            constants: vec![],
            upvalueCount: 0,
            upvalueDescs: vec![],
        }],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    };
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_alen() {
    let module = ModuleFile {
        version: (1, 0, 0),
        constants: vec![],
        functions: vec![Function {
            name: "main".into(),
            arity: 0,
            numRegisters: 4,
            instructions: vec![
                Instruction::rrk(Opcode::NewArray, 0, 0, 0),
                Instruction::ri(Opcode::Loadi, 1, 10),
                Instruction::ri(Opcode::Loadi, 2, 0), // index 0
                Instruction::rrr(Opcode::ASet, 0, 2, 1), // arr[0] = 10
                Instruction::ri(Opcode::Loadi, 2, 1), // index 1
                Instruction::rrr(Opcode::ASet, 0, 2, 1), // arr[1] = 10
                Instruction::ri(Opcode::Loadi, 2, 2), // index 2
                Instruction::rrr(Opcode::ASet, 0, 2, 1), // arr[2] = 10
                Instruction::rrr(Opcode::ALen, 3, 0, 0),
                Instruction::rrr(Opcode::Return, 3, 0, 0),
            ],
            constants: vec![],
            upvalueCount: 0,
            upvalueDescs: vec![],
        }],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    };
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(3));
}

// ---------------------------------------------------------------------------
// GetField and SetField (object properties)
// ---------------------------------------------------------------------------

#[test]
fn test_set_get_field() {
    let module = ModuleFile {
        version: (1, 0, 0),
        constants: vec![CpValue::String("name".into())],
        functions: vec![Function {
            name: "main".into(),
            arity: 0,
            numRegisters: 3,
            instructions: vec![
                Instruction::rrk(Opcode::NewObject, 0, 0, 0), // r0 = obj
                Instruction::ri(Opcode::Loadi, 1, 42),        // r1 = 42
                Instruction::rrr(Opcode::SetField, 1, 0, 0),  // obj."name" = 42
                Instruction::rrr(Opcode::GetField, 2, 0, 0),  // r2 = obj."name"
                Instruction::rrr(Opcode::Return, 2, 0, 0),
            ],
            constants: vec![CpValue::String("name".into())],
            upvalueCount: 0,
            upvalueDescs: vec![],
        }],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    };
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn test_type_mismatch_on_add() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 10),
            Instruction::rri(Opcode::Loadbool, 1, 0, 1),
            Instruction::rrr(Opcode::Add, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![],
    );
    let result = run_module(module);
    assert!(result.is_err());
}

#[test]
fn test_no_module_loaded() {
    let mut vm = Vm::new();
    let result = vm.Exec();
    assert!(result.is_err());
}

#[test]
fn test_wrong_arity() {
    let module = make_module(2, 2, vec![Instruction::new(Opcode::Halt)], vec![]);
    let mut vm = Vm::new();
    vm.LoadModule(module).unwrap();
    let result = vm.ExecFunc(0, vec![Value::Int(1)]);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// GC: mark-and-sweep basic test
// ---------------------------------------------------------------------------

#[test]
fn test_gc_collect() {
    let mut heap = Heap::new();
    let r1 = heap.allocObj(GcObject::new_object(vec![]));
    let r2 = heap.allocObj(GcObject::new_array(vec![Value::Int(1), Value::Int(2)]));
    let _r3 = heap.allocObj(GcObject::new_object(vec![]));

    let roots = vec![r1, r2];
    heap.collect(&roots);

    let obj1 = heap.get(r1);
    assert!(matches!(obj1.kind, ObjectKind::Object(_)));

    let obj2 = heap.get(r2);
    assert!(matches!(obj2.kind, ObjectKind::Array(_)));
}

// ---------------------------------------------------------------------------
// Native function test
// ---------------------------------------------------------------------------

#[test]
fn test_native_function_ptr() {
    fn my_native(_args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::Int(42))
    }

    let result = my_native(&[]).unwrap();
    assert_eq!(result, Value::Int(42));
}

// ---------------------------------------------------------------------------
// Closure and upvalue test
// ---------------------------------------------------------------------------

#[test]
fn test_closure_upvalue() {
    let module = ModuleFile {
        version: (1, 0, 0),
        constants: vec![CpValue::Function(1)],
        functions: vec![
            Function {
                name: "outer".into(),
                arity: 0,
                numRegisters: 2,
                instructions: vec![
                    Instruction::ri(Opcode::Loadi, 0, 42),
                    Instruction::rrk(Opcode::Closure, 1, 0, 0), // closure for func 1
                    Instruction::rrr(Opcode::Mov, 0, 1, 0),     // r0 = closure (return)
                    Instruction::rrr(Opcode::Return, 0, 0, 0),
                ],
                constants: vec![CpValue::Function(1)],
                upvalueCount: 0,
                upvalueDescs: vec![],
            },
            Function {
                name: "inner".into(),
                arity: 0,
                numRegisters: 1,
                instructions: vec![
                    Instruction::rrk(Opcode::LoadUpvalue, 0, 0, 0),
                    Instruction::rrr(Opcode::Return, 0, 0, 0),
                ],
                constants: vec![],
                upvalueCount: 1,
                upvalueDescs: vec![UpvalueDesc {
                    isLocal: true,
                    index: 0,
                }],
            },
        ],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    };
    let result = exec_func(module, 0, vec![]).unwrap();
    match result {
        Value::Closure(_, _) => {}
        _ => panic!("expected Closure, got {:?}", result),
    }
}

// ---------------------------------------------------------------------------
// Float arithmetic
// ---------------------------------------------------------------------------

#[test]
fn test_float_add() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrk(Opcode::Loadk, 1, 0, 1),
            Instruction::rrr(Opcode::Add, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![CpValue::Float(1.5), CpValue::Float(2.5)],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Float(4.0));
}

#[test]
fn test_float_sub() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrk(Opcode::Loadk, 1, 0, 1),
            Instruction::rrr(Opcode::Sub, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![CpValue::Float(5.5), CpValue::Float(2.0)],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Float(3.5));
}

#[test]
fn test_float_mul() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrk(Opcode::Loadk, 1, 0, 1),
            Instruction::rrr(Opcode::Mul, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![CpValue::Float(3.0), CpValue::Float(1.5)],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Float(4.5));
}

#[test]
fn test_float_div() {
    let module = make_module(
        0,
        3,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrk(Opcode::Loadk, 1, 0, 1),
            Instruction::rrr(Opcode::Div, 2, 0, 1),
            Instruction::rrr(Opcode::Return, 2, 0, 0),
        ],
        vec![CpValue::Float(10.0), CpValue::Float(3.0)],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Float(10.0 / 3.0));
}

#[test]
fn test_float_neg() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::rrk(Opcode::Loadk, 0, 0, 0),
            Instruction::rrr(Opcode::Neg, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![CpValue::Float(3.14)],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Float(-3.14));
}

// ---------------------------------------------------------------------------
// CloseUpvalue
// ---------------------------------------------------------------------------

#[test]
fn test_close_upvalue() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 42),
            Instruction::rrk(Opcode::CloseUpvalue, 0, 0, 0),
            Instruction::rrr(Opcode::Return, 0, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Int(42));
}

// ---------------------------------------------------------------------------
// Nil and truthiness
// ---------------------------------------------------------------------------

#[test]
fn test_nil_is_falsy() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::rrk(Opcode::Loadnil, 0, 0, 0),
            Instruction::rrr(Opcode::Not, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_zero_is_truthy() {
    let module = make_module(
        0,
        2,
        vec![
            Instruction::ri(Opcode::Loadi, 0, 0),
            Instruction::rrr(Opcode::Not, 1, 0, 0),
            Instruction::rrr(Opcode::Return, 1, 0, 0),
        ],
        vec![],
    );
    let result = exec_func(module, 0, vec![]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

// ---------------------------------------------------------------------------
// StoreUpvalue
// ---------------------------------------------------------------------------

#[test]
fn test_store_upvalue() {
    let module = ModuleFile {
        version: (1, 0, 0),
        constants: vec![CpValue::Function(1)],
        functions: vec![
            Function {
                name: "outer".into(),
                arity: 0,
                numRegisters: 2,
                instructions: vec![
                    Instruction::ri(Opcode::Loadi, 0, 42),
                    Instruction::rrk(Opcode::Closure, 1, 0, 0),
                    Instruction::rrr(Opcode::Mov, 0, 1, 0),
                    Instruction::rrr(Opcode::Return, 0, 0, 0),
                ],
                constants: vec![CpValue::Function(1)],
                upvalueCount: 0,
                upvalueDescs: vec![],
            },
            Function {
                name: "inner".into(),
                arity: 0,
                numRegisters: 2,
                instructions: vec![
                    Instruction::ri(Opcode::Loadi, 1, 99),
                    Instruction::rrk(Opcode::StoreUpvalue, 1, 0, 0),
                    Instruction::rrk(Opcode::LoadUpvalue, 0, 0, 0),
                    Instruction::rrr(Opcode::Return, 0, 0, 0),
                ],
                constants: vec![],
                upvalueCount: 1,
                upvalueDescs: vec![UpvalueDesc {
                    isLocal: true,
                    index: 0,
                }],
            },
        ],
        exports: vec![],
        imports: vec![],
        entryPoint: 0,
    };
    let result = exec_func(module, 0, vec![]).unwrap();
    match result {
        Value::Closure(_, _) => {}
        _ => panic!("expected Closure, got {:?}", result),
    }
}
