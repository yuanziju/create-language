use crate::opcode::Opcode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction(u32);

impl Instruction {
    pub fn new(opcode: Opcode) -> Self {
        Instruction(opcode.to_u8() as u32)
    }

    pub fn rrr(opcode: Opcode, a: u8, b: u8, c: u8) -> Self {
        Instruction(
            (opcode.to_u8() as u32) | ((a as u32) << 8) | ((b as u32) << 16) | ((c as u32) << 24),
        )
    }

    pub fn rri(opcode: Opcode, a: u8, b: u8, imm: u8) -> Self {
        Instruction(
            (opcode.to_u8() as u32) | ((a as u32) << 8) | ((b as u32) << 16) | ((imm as u32) << 24),
        )
    }

    pub fn ri(opcode: Opcode, a: u8, imm: u16) -> Self {
        Instruction((opcode.to_u8() as u32) | ((a as u32) << 8) | ((imm as u32) << 16))
    }

    pub fn i(opcode: Opcode, imm: u32) -> Self {
        Instruction((opcode.to_u8() as u32) | ((imm & 0xFFFFFF) << 8))
    }

    pub fn rrk(opcode: Opcode, a: u8, b: u8, k: u8) -> Self {
        Instruction(
            (opcode.to_u8() as u32) | ((a as u32) << 8) | ((b as u32) << 16) | ((k as u32) << 24),
        )
    }

    pub fn opcode(&self) -> Opcode {
        Opcode::from_u8((self.0 & 0xFF) as u8)
    }

    pub fn a(&self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }
    pub fn b(&self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }
    pub fn c(&self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }
    pub fn imm8(&self) -> u8 {
        self.c()
    }
    pub fn imm16(&self) -> u16 {
        ((self.0 >> 16) & 0xFFFF) as u16
    }
    pub fn imm24(&self) -> u32 {
        (self.0 >> 8) & 0xFFFFFF
    }
    pub fn imm16_signed(&self) -> i16 {
        self.imm16() as i16
    }
    pub fn imm24_signed(&self) -> i32 {
        (self.imm24() as i32) << 8 >> 8
    }
    pub fn raw(&self) -> u32 {
        self.0
    }
    pub fn from_raw(raw: u32) -> Self {
        Instruction(raw)
    }
}
