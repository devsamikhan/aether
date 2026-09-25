use super::value::Value;
use crate::syntax::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Constant,
    Nil,
    True,
    False,
    Pop,
    Dup,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Negate,

    // Logic & Comparison
    Not,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Variables & Scope
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,

    // Jumps & Loops
    Jump,
    JumpIfFalse,
    Loop,

    // Functions & Calls
    Call,
    CallKw,
    Return,
    BuildClosure,
    Super,

    // Data Structures
    BuildArray,
    BuildMap,
    BuildTuple,
    BuildSet,
    UnpackSequence,
    IndexGet,
    IndexSet,
    SetAttr,

    // Concurrency
    Spawn,
    ChannelCreate,
    ChannelSend,
    ChannelRecv,

    // IO / Builtins
    Print,
    Println,

    // Exception Handling
    PushExceptionHandler,
    PopExceptionHandler,
    Raise,

    // Modules
    ImportModule,
    ImportFrom,

    // Comprehensions & Collections
    ArrayPush,
    MapInsert,
}

impl From<u8> for OpCode {
    fn from(byte: u8) -> Self {
        unsafe { std::mem::transmute(byte) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub spans: Vec<Span>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            spans: Vec::new(),
        }
    }

    pub fn write_byte(&mut self, byte: u8, span: Span) {
        self.code.push(byte);
        self.spans.push(span);
    }

    pub fn write_op(&mut self, op: OpCode, span: Span) {
        self.write_byte(op as u8, span);
    }

    pub fn write_u16(&mut self, val: u16, span: Span) {
        self.write_byte((val >> 8) as u8, span);
        self.write_byte((val & 0xFF) as u8, span);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn read_u16(&self, offset: usize) -> u16 {
        ((self.code[offset] as u16) << 8) | (self.code[offset + 1] as u16)
    }
}
