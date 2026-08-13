#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    Halt = 0,
    Mov,
    Loadk,
    Loadi,
    Loadbool,
    Loadnil,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,
    I2f,
    F2i,
    Eq,
    Lt,
    Le,
    Not,
    IsType,
    Jmp,
    JmpT,
    JmpF,
    Call,
    Return,
    TailCall,
    Closure,
    LoadUpvalue,
    StoreUpvalue,
    CloseUpvalue,
    NewObject,
    NewArray,
    GetField,
    SetField,
    AGet,
    ASet,
    ALen,
    Import,
    Export,
    Throw,
    Try,
    EndTry,
    Wide,
    Line,

    // Match group (固定32位)
    TableSwitch,
    LookupSwitch,
    Match,

    // SIMD 基础 (固定32位)
    VecAdd,
    VecSub,
    VecMul,
    VecDiv,
    VecCmpEq,
    VecCmpLt,
    VecCmpLe,
    VecLoad,
    VecStore,

    // 协程 (固定32位)
    Suspend,
    Resume,

    // 运算符重载 (固定32位)
    InvokeSpecial,
}

impl Opcode {
    pub fn from_u8(value: u8) -> Self {
        unsafe { std::mem::transmute(value) }
    }
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}
