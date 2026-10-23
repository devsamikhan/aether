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

    // Language v3.0 Upgrades
    JumpIfNotNil,
    OptionalGetAttr,
    OptionalIndexGet,
    TryUnwrap,
    BuildRange,
    BuildRangeInclusive,
    Slice,
    UnpackArrayRest,
    ImplMethod,
    MatchEnumVariant,
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

/// Human-readable disassembler for a compiled AETHER function and any nested functions.
pub fn disassemble_function(func: &super::value::CompiledFunction) -> String {
    let mut out = String::new();
    let mut nested_fns = Vec::new();

    out.push_str(&format!(
        "=== Function: {} (arity: {}..{}, code: {} bytes, constants: {}) ===\n",
        func.name,
        func.arity,
        func.max_arity,
        func.chunk.code.len(),
        func.chunk.constants.len()
    ));

    if !func.chunk.constants.is_empty() {
        out.push_str("Constants:\n");
        for (idx, c) in func.chunk.constants.iter().enumerate() {
            out.push_str(&format!("  [{:04}] {:?}\n", idx, c));
            if let super::value::Value::Function(sub) = c {
                nested_fns.push(sub.clone());
            }
        }
    }

    out.push_str("Instructions:\n");
    let code = &func.chunk.code;
    let mut ip = 0usize;
    while ip < code.len() {
        let line = func.chunk.spans.get(ip).map(|s| s.line).unwrap_or(0);
        let op = OpCode::from(code[ip]);
        let op_ip = ip;
        ip += 1;

        match op {
            OpCode::Constant
            | OpCode::DefineGlobal
            | OpCode::GetGlobal
            | OpCode::SetGlobal
            | OpCode::ImportModule
            | OpCode::OptionalGetAttr => {
                if ip + 1 < code.len() {
                    let const_idx = func.chunk.read_u16(ip) as usize;
                    ip += 2;
                    let const_repr = func
                        .chunk
                        .constants
                        .get(const_idx)
                        .map(|v| format!("{:?}", v))
                        .unwrap_or_else(|| "<invalid>".to_string());
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} #{:<4} ({})\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        const_idx,
                        const_repr
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            OpCode::GetLocal
            | OpCode::SetLocal
            | OpCode::BuildClosure
            | OpCode::BuildArray
            | OpCode::BuildTuple
            | OpCode::BuildSet
            | OpCode::BuildMap
            | OpCode::UnpackSequence
            | OpCode::UnpackArrayRest => {
                if ip + 1 < code.len() {
                    let val = func.chunk.read_u16(ip);
                    ip += 2;
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} {}\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        val
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            OpCode::Jump
            | OpCode::JumpIfFalse
            | OpCode::JumpIfNotNil
            | OpCode::PushExceptionHandler => {
                if ip + 1 < code.len() {
                    let offset = func.chunk.read_u16(ip) as usize;
                    ip += 2;
                    let target = ip + offset;
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} +{} -> {:04}\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        offset,
                        target
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            OpCode::Loop => {
                if ip + 1 < code.len() {
                    let offset = func.chunk.read_u16(ip) as usize;
                    ip += 2;
                    let target = ip.saturating_sub(offset);
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} -{} -> {:04}\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        offset,
                        target
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            OpCode::Call | OpCode::Spawn => {
                if ip < code.len() {
                    let argc = code[ip];
                    ip += 1;
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} argc={}\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        argc
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            OpCode::CallKw => {
                if ip + 1 < code.len() {
                    let pos_c = code[ip];
                    let kw_c = code[ip + 1];
                    ip += 2;
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} pos={}, kw={}\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        pos_c,
                        kw_c
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            OpCode::ImportFrom | OpCode::ImplMethod => {
                if ip + 3 < code.len() {
                    let idx1 = func.chunk.read_u16(ip) as usize;
                    let idx2 = func.chunk.read_u16(ip + 2) as usize;
                    ip += 4;
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} #{}, #{}\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        idx1,
                        idx2
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            OpCode::MatchEnumVariant => {
                if ip + 4 < code.len() {
                    let enum_idx = func.chunk.read_u16(ip) as usize;
                    let var_idx = func.chunk.read_u16(ip + 2) as usize;
                    let arity = code[ip + 4];
                    ip += 5;
                    out.push_str(&format!(
                        "  {:04} (L{:03})  {:<22} enum=#{}, var=#{}, arity={}\n",
                        op_ip,
                        line,
                        format!("{:?}", op),
                        enum_idx,
                        var_idx,
                        arity
                    ));
                } else {
                    out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
                }
            }
            _ => {
                out.push_str(&format!("  {:04} (L{:03})  {:?}\n", op_ip, line, op));
            }
        }
    }

    for sub in nested_fns {
        out.push('\n');
        out.push_str(&disassemble_function(&sub));
    }

    out
}

/// Compiles and disassembles an AETHER source string into human-readable bytecode.
pub fn disassemble_source(source: &str) -> Result<String, String> {
    let raw_program = crate::syntax::parse(source)
        .map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    let (program, _) = super::optimizer::optimize_program(&raw_program);
    let compiler = super::compiler::BytecodeCompiler::new("<main>", 0);
    let func = compiler.compile(&program)?;
    Ok(disassemble_function(&func))
}
