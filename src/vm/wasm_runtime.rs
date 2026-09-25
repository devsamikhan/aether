// ==============================================================================
// AetherEdge — Sandboxed In-Memory WebAssembly Virtual Machine (Zero Crates)
// ==============================================================================

use super::value::Value;
use crate::codegen::wasm::*;
use crate::syntax::parse;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ==============================================================================
// LEB128 Reader Utilities
// ==============================================================================

pub struct ByteReader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn has_more(&self) -> bool {
        self.pos < self.data.len()
    }

    pub fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            Ok(b)
        } else {
            Err("Unexpected EOF in WASM byte stream".to_string())
        }
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        if self.pos + len <= self.data.len() {
            let slice = &self.data[self.pos..self.pos + len];
            self.pos += len;
            Ok(slice)
        } else {
            Err("Unexpected EOF while reading WASM byte slice".to_string())
        }
    }

    pub fn read_u32_leb128(&mut self) -> Result<u32, String> {
        let mut result = 0u32;
        let mut shift = 0;
        loop {
            let byte = self.read_u8()?;
            result |= ((byte & 0x7F) as u32) << shift;
            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift >= 35 {
                return Err("LEB128 integer overflow".to_string());
            }
        }
        Ok(result)
    }

    pub fn read_i64_leb128(&mut self) -> Result<i64, String> {
        let mut result = 0i64;
        let mut shift = 0;
        let mut byte;
        loop {
            byte = self.read_u8()?;
            result |= ((byte & 0x7F) as i64) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                break;
            }
            if shift >= 70 {
                return Err("LEB128 integer overflow".to_string());
            }
        }
        if shift < 64 && (byte & 0x40) != 0 {
            result |= !0i64 << shift;
        }
        Ok(result)
    }

    pub fn read_i32_leb128(&mut self) -> Result<i32, String> {
        let v = self.read_i64_leb128()?;
        Ok(v as i32)
    }

    pub fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_u32_leb128()? as usize;
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())
    }
}

// ==============================================================================
// WASM Module Definition
// ==============================================================================

#[derive(Debug, Clone)]
pub struct ParsedFuncType {
    pub params: Vec<u8>,
    pub results: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ParsedFunction {
    pub type_index: u32,
    pub locals: Vec<(u32, u8)>, // (count, type)
    pub code: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WasmModule {
    pub types: Vec<ParsedFuncType>,
    pub functions: Vec<ParsedFunction>,
    pub exports: HashMap<String, (u8, u32)>, // name -> (kind, index)
    pub initial_memory_pages: u32,
    pub data_segments: Vec<(u32, Vec<u8>)>,
}

impl WasmModule {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = ByteReader::new(bytes);

        // 1. Verify Magic Header: \0asm
        let magic = reader.read_bytes(4)?;
        if magic != [0x00, 0x61, 0x73, 0x6D] {
            return Err("Invalid WebAssembly magic header (expected '\\0asm')".to_string());
        }

        // 2. Verify Version: 1
        let version = reader.read_bytes(4)?;
        if version != [0x01, 0x00, 0x00, 0x00] {
            return Err("Unsupported WebAssembly version (expected 1)".to_string());
        }

        let mut types = Vec::new();
        let mut func_type_indices = Vec::new();
        let mut exports = HashMap::new();
        let mut code_bodies = Vec::new();
        let mut initial_memory_pages = 1u32;
        let mut data_segments = Vec::new();

        while reader.has_more() {
            let sec_id = reader.read_u8()?;
            let sec_len = reader.read_u32_leb128()? as usize;
            let sec_bytes = reader.read_bytes(sec_len)?;
            let mut sec_reader = ByteReader::new(sec_bytes);

            match sec_id {
                SEC_TYPE => {
                    let count = sec_reader.read_u32_leb128()?;
                    for _ in 0..count {
                        let form = sec_reader.read_u8()?;
                        if form != TYPE_FUNC {
                            return Err(format!("Expected func type 0x60, got {:#X}", form));
                        }
                        let param_count = sec_reader.read_u32_leb128()?;
                        let mut params = Vec::new();
                        for _ in 0..param_count {
                            params.push(sec_reader.read_u8()?);
                        }
                        let res_count = sec_reader.read_u32_leb128()?;
                        let mut results = Vec::new();
                        for _ in 0..res_count {
                            results.push(sec_reader.read_u8()?);
                        }
                        types.push(ParsedFuncType { params, results });
                    }
                }
                SEC_FUNCTION => {
                    let count = sec_reader.read_u32_leb128()?;
                    for _ in 0..count {
                        func_type_indices.push(sec_reader.read_u32_leb128()?);
                    }
                }
                SEC_MEMORY => {
                    let count = sec_reader.read_u32_leb128()?;
                    if count > 0 {
                        let flags = sec_reader.read_u8()?;
                        initial_memory_pages = sec_reader.read_u32_leb128()?;
                        if (flags & 0x01) != 0 {
                            let _max_pages = sec_reader.read_u32_leb128()?;
                        }
                    }
                }
                SEC_EXPORT => {
                    let count = sec_reader.read_u32_leb128()?;
                    for _ in 0..count {
                        let name = sec_reader.read_string()?;
                        let kind = sec_reader.read_u8()?;
                        let idx = sec_reader.read_u32_leb128()?;
                        exports.insert(name, (kind, idx));
                    }
                }
                SEC_CODE => {
                    let count = sec_reader.read_u32_leb128()?;
                    for _ in 0..count {
                        let body_size = sec_reader.read_u32_leb128()? as usize;
                        let body_bytes = sec_reader.read_bytes(body_size)?;
                        let mut body_reader = ByteReader::new(body_bytes);

                        let local_vec_count = body_reader.read_u32_leb128()?;
                        let mut locals = Vec::new();
                        for _ in 0..local_vec_count {
                            let count = body_reader.read_u32_leb128()?;
                            let l_type = body_reader.read_u8()?;
                            locals.push((count, l_type));
                        }
                        let code = body_reader.data[body_reader.pos..].to_vec();
                        code_bodies.push((locals, code));
                    }
                }
                SEC_DATA => {
                    let count = sec_reader.read_u32_leb128()?;
                    for _ in 0..count {
                        let _mem_idx = sec_reader.read_u8()?;
                        let mut offset = 0u32;
                        // Read offset expr
                        let op = sec_reader.read_u8()?;
                        if op == OP_I32_CONST {
                            offset = sec_reader.read_i32_leb128()? as u32;
                            let end = sec_reader.read_u8()?;
                            if end != OP_END {
                                return Err("Expected OP_END in data offset expr".to_string());
                            }
                        }
                        let data_len = sec_reader.read_u32_leb128()? as usize;
                        let bytes = sec_reader.read_bytes(data_len)?.to_vec();
                        data_segments.push((offset, bytes));
                    }
                }
                _ => {} // Ignore unknown or optional sections
            }
        }

        let mut functions = Vec::new();
        for (idx, type_idx) in func_type_indices.into_iter().enumerate() {
            if idx < code_bodies.len() {
                let (locals, code) = code_bodies[idx].clone();
                functions.push(ParsedFunction {
                    type_index: type_idx,
                    locals,
                    code,
                });
            }
        }

        Ok(Self {
            types,
            functions,
            exports,
            initial_memory_pages,
            data_segments,
        })
    }
}

// ==============================================================================
// AetherEdge In-Memory Sandboxed Execution Instance
// ==============================================================================

#[derive(Debug, Clone)]
struct ControlFrame {
    block_type: u8, // OP_BLOCK, OP_LOOP, OP_IF
    #[allow(dead_code)]
    stack_depth: usize,
    pc: usize,
}

pub struct WasmInstance {
    pub module: WasmModule,
    pub memory: Vec<u8>,
}

impl WasmInstance {
    pub fn new(module: WasmModule) -> Self {
        let mem_size = (module.initial_memory_pages as usize) * 65536;
        let mut memory = vec![0u8; mem_size];

        // Initialize Data segments into linear memory
        for (offset, bytes) in &module.data_segments {
            let start = *offset as usize;
            let end = start + bytes.len();
            if end <= memory.len() {
                memory[start..end].copy_from_slice(bytes);
            }
        }

        Self { module, memory }
    }

    /// Invokes an exported WASM function by name with sandboxed execution
    pub fn invoke(
        &mut self,
        export_name: &str,
        args: &[i64],
        mut fuel: u64,
    ) -> Result<i64, String> {
        let (kind, func_idx) = self
            .module
            .exports
            .get(export_name)
            .cloned()
            .ok_or_else(|| format!("WASM Export '{}' not found", export_name))?;

        if kind != 0x00 {
            return Err(format!("Export '{}' is not a function", export_name));
        }

        let func = self
            .module
            .functions
            .get(func_idx as usize)
            .ok_or_else(|| format!("Function index {} out of bounds", func_idx))?
            .clone();

        let sig = &self.module.types[func.type_index as usize];
        if args.len() != sig.params.len() {
            return Err(format!(
                "Function '{}' expects {} arguments, got {}",
                export_name,
                sig.params.len(),
                args.len()
            ));
        }

        // Initialize local variables: arguments followed by function-declared locals
        let mut locals = Vec::new();
        locals.extend_from_slice(args);
        for (count, _l_type) in &func.locals {
            for _ in 0..*count {
                locals.push(0i64);
            }
        }

        let res = self.execute_bytecode(&func.code, &mut locals, &mut fuel)?;
        Ok(res)
    }

    fn execute_bytecode(
        &mut self,
        code: &[u8],
        locals: &mut Vec<i64>,
        fuel: &mut u64,
    ) -> Result<i64, String> {
        let mut stack: Vec<i64> = Vec::new();
        let mut control_stack: Vec<ControlFrame> = Vec::new();
        let mut pc = 0;

        while pc < code.len() {
            if *fuel == 0 {
                return Err("AetherEdge sandbox execution fuel exhausted (infinite loop prevented)".to_string());
            }
            *fuel -= 1;

            let op = code[pc];
            pc += 1;

            match op {
                OP_UNREACHABLE => return Err("WASM unreachable executed".to_string()),
                OP_NOP => {}
                OP_BLOCK => {
                    let _block_type = code[pc];
                    pc += 1;
                    control_stack.push(ControlFrame {
                        block_type: OP_BLOCK,
                        stack_depth: stack.len(),
                        pc,
                    });
                }
                OP_LOOP => {
                    let _block_type = code[pc];
                    pc += 1;
                    control_stack.push(ControlFrame {
                        block_type: OP_LOOP,
                        stack_depth: stack.len(),
                        pc, // start of loop body
                    });
                }
                OP_IF => {
                    let _block_type = code[pc];
                    pc += 1;
                    let cond = stack.pop().unwrap_or(0);
                    control_stack.push(ControlFrame {
                        block_type: OP_IF,
                        stack_depth: stack.len(),
                        pc,
                    });

                    if cond == 0 {
                        // Skip to matching OP_ELSE or OP_END at same depth
                        let mut depth = 1;
                        let mut found_else = false;
                        while pc < code.len() && depth > 0 {
                            let next_op = code[pc];
                            pc += 1;
                            if next_op == OP_BLOCK || next_op == OP_LOOP || next_op == OP_IF {
                                depth += 1;
                                pc += 1; // skip block type
                            } else if next_op == OP_ELSE && depth == 1 {
                                found_else = true;
                                break;
                            } else if next_op == OP_END {
                                depth -= 1;
                            } else {
                                Self::skip_immediate(next_op, code, &mut pc)?;
                            }
                        }
                        if !found_else {
                            control_stack.pop();
                        }
                    }
                }
                OP_ELSE => {
                    // Reached end of 'then' branch, skip to matching OP_END
                    let mut depth = 1;
                    while pc < code.len() && depth > 0 {
                        let next_op = code[pc];
                        pc += 1;
                        if next_op == OP_BLOCK || next_op == OP_LOOP || next_op == OP_IF {
                            depth += 1;
                            pc += 1;
                        } else if next_op == OP_END {
                            depth -= 1;
                        } else {
                            Self::skip_immediate(next_op, code, &mut pc)?;
                        }
                    }
                    control_stack.pop();
                }
                OP_END => {
                    if let Some(_frame) = control_stack.pop() {
                        // normal end of block/loop/if
                    } else {
                        // End of function
                        break;
                    }
                }
                OP_BR => {
                    let depth = Self::read_u32_at(code, &mut pc)? as usize;
                    if depth < control_stack.len() {
                        let target_idx = control_stack.len() - 1 - depth;
                        let frame = control_stack[target_idx].clone();
                        if frame.block_type == OP_LOOP {
                            pc = frame.pc; // jump back to start of loop
                        } else {
                            // Break out of block: skip to matching end
                            control_stack.truncate(target_idx);
                            let mut d = depth + 1;
                            while pc < code.len() && d > 0 {
                                let next_op = code[pc];
                                pc += 1;
                                if next_op == OP_BLOCK || next_op == OP_LOOP || next_op == OP_IF {
                                    d += 1;
                                    pc += 1;
                                } else if next_op == OP_END {
                                    d -= 1;
                                } else {
                                    Self::skip_immediate(next_op, code, &mut pc)?;
                                }
                            }
                        }
                    } else {
                        return Err("Branch target depth out of bounds".to_string());
                    }
                }
                OP_BR_IF => {
                    let depth = Self::read_u32_at(code, &mut pc)? as usize;
                    let cond = stack.pop().unwrap_or(0);
                    if cond != 0 {
                        if depth < control_stack.len() {
                            let target_idx = control_stack.len() - 1 - depth;
                            let frame = control_stack[target_idx].clone();
                            if frame.block_type == OP_LOOP {
                                pc = frame.pc;
                            } else {
                                control_stack.truncate(target_idx);
                                let mut d = depth + 1;
                                while pc < code.len() && d > 0 {
                                    let next_op = code[pc];
                                    pc += 1;
                                    if next_op == OP_BLOCK || next_op == OP_LOOP || next_op == OP_IF {
                                        d += 1;
                                        pc += 1;
                                    } else if next_op == OP_END {
                                        d -= 1;
                                    } else {
                                        Self::skip_immediate(next_op, code, &mut pc)?;
                                    }
                                }
                            }
                        }
                    }
                }
                OP_RETURN => {
                    break;
                }
                OP_CALL => {
                    let target_func_idx = Self::read_u32_at(code, &mut pc)? as usize;
                    let target_func = self
                        .module
                        .functions
                        .get(target_func_idx)
                        .ok_or_else(|| format!("Target func index {} invalid", target_func_idx))?
                        .clone();

                    let sig = &self.module.types[target_func.type_index as usize];
                    let mut call_locals = Vec::new();
                    for _ in 0..sig.params.len() {
                        call_locals.push(stack.pop().unwrap_or(0));
                    }
                    call_locals.reverse();
                    for (count, _) in &target_func.locals {
                        for _ in 0..*count {
                            call_locals.push(0i64);
                        }
                    }

                    let res = self.execute_bytecode(&target_func.code, &mut call_locals, fuel)?;
                    stack.push(res);
                }
                OP_DROP => {
                    stack.pop();
                }
                OP_LOCAL_GET => {
                    let idx = Self::read_u32_at(code, &mut pc)? as usize;
                    let val = locals.get(idx).cloned().unwrap_or(0);
                    stack.push(val);
                }
                OP_LOCAL_SET => {
                    let idx = Self::read_u32_at(code, &mut pc)? as usize;
                    let val = stack.pop().unwrap_or(0);
                    if idx >= locals.len() {
                        locals.resize(idx + 1, 0);
                    }
                    locals[idx] = val;
                }
                OP_LOCAL_TEE => {
                    let idx = Self::read_u32_at(code, &mut pc)? as usize;
                    let val = stack.last().cloned().unwrap_or(0);
                    if idx >= locals.len() {
                        locals.resize(idx + 1, 0);
                    }
                    locals[idx] = val;
                }
                OP_I32_CONST => {
                    let val = Self::read_i32_at(code, &mut pc)?;
                    stack.push(val as i64);
                }
                OP_I64_CONST => {
                    let val = Self::read_i64_at(code, &mut pc)?;
                    stack.push(val);
                }
                OP_F64_CONST => {
                    if pc + 8 <= code.len() {
                        let bytes: [u8; 8] = code[pc..pc + 8].try_into().unwrap();
                        pc += 8;
                        let val = f64::from_le_bytes(bytes);
                        stack.push(val.to_bits() as i64);
                    }
                }
                OP_I64_ADD => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a.wrapping_add(b));
                }
                OP_I64_SUB => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a.wrapping_sub(b));
                }
                OP_I64_MUL => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a.wrapping_mul(b));
                }
                OP_I64_DIV_S => {
                    let b = stack.pop().unwrap_or(1);
                    let a = stack.pop().unwrap_or(0);
                    if b == 0 {
                        return Err("Integer division by zero in WASM".to_string());
                    }
                    stack.push(a.wrapping_div(b));
                }
                OP_I64_REM_S => {
                    let b = stack.pop().unwrap_or(1);
                    let a = stack.pop().unwrap_or(0);
                    if b == 0 {
                        return Err("Integer remainder by zero in WASM".to_string());
                    }
                    stack.push(a.wrapping_rem(b));
                }
                OP_I64_AND => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a & b);
                }
                OP_I64_OR => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a | b);
                }
                OP_I64_XOR => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a ^ b);
                }
                OP_I64_EQZ => {
                    let a = stack.pop().unwrap_or(0);
                    stack.push(if a == 0 { 1 } else { 0 });
                }
                OP_I64_EQ => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(if a == b { 1 } else { 0 });
                }
                OP_I64_NE => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(if a != b { 1 } else { 0 });
                }
                OP_I64_LT_S => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(if a < b { 1 } else { 0 });
                }
                OP_I64_GT_S => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(if a > b { 1 } else { 0 });
                }
                OP_I64_LE_S => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(if a <= b { 1 } else { 0 });
                }
                OP_I64_GE_S => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(if a >= b { 1 } else { 0 });
                }
                OP_I64_EXTEND_I32_U => {
                    let a = stack.pop().unwrap_or(0);
                    stack.push((a as u32) as i64);
                }
                OP_I64_TRUNC_F64_S => {
                    let bits = stack.pop().unwrap_or(0) as u64;
                    let f = f64::from_bits(bits);
                    stack.push(f as i64);
                }
                OP_I32_LOAD | OP_I64_LOAD => {
                    let _align = Self::read_u32_at(code, &mut pc)?;
                    let offset = Self::read_u32_at(code, &mut pc)? as usize;
                    let base = stack.pop().unwrap_or(0) as usize;
                    let addr = base + offset;
                    if addr + 8 <= self.memory.len() {
                        let bytes: [u8; 8] = self.memory[addr..addr + 8].try_into().unwrap();
                        stack.push(i64::from_le_bytes(bytes));
                    } else {
                        return Err(format!("Memory load out of bounds: {} (len {})", addr, self.memory.len()));
                    }
                }
                OP_I32_STORE | OP_I64_STORE => {
                    let _align = Self::read_u32_at(code, &mut pc)?;
                    let offset = Self::read_u32_at(code, &mut pc)? as usize;
                    let val = stack.pop().unwrap_or(0);
                    let base = stack.pop().unwrap_or(0) as usize;
                    let addr = base + offset;
                    if addr + 8 <= self.memory.len() {
                        self.memory[addr..addr + 8].copy_from_slice(&val.to_le_bytes());
                    } else {
                        return Err(format!("Memory store out of bounds: {} (len {})", addr, self.memory.len()));
                    }
                }
                OP_MEMORY_SIZE => {
                    let _reserved = code[pc];
                    pc += 1;
                    let pages = (self.memory.len() / 65536) as i64;
                    stack.push(pages);
                }
                OP_MEMORY_GROW => {
                    let _reserved = code[pc];
                    pc += 1;
                    let add_pages = stack.pop().unwrap_or(0) as usize;
                    let old_pages = self.memory.len() / 65536;
                    let new_size = (old_pages + add_pages) * 65536;
                    if new_size <= 16 * 1024 * 1024 {
                        self.memory.resize(new_size, 0);
                        stack.push(old_pages as i64);
                    } else {
                        stack.push(-1); // failed to grow
                    }
                }
                _ => {}
            }
        }

        Ok(stack.pop().unwrap_or(0))
    }

    fn read_u32_at(code: &[u8], pc: &mut usize) -> Result<u32, String> {
        let mut result = 0u32;
        let mut shift = 0;
        while *pc < code.len() {
            let byte = code[*pc];
            *pc += 1;
            result |= ((byte & 0x7F) as u32) << shift;
            if (byte & 0x80) == 0 {
                return Ok(result);
            }
            shift += 7;
        }
        Err("Unexpected EOF in LEB128 u32".to_string())
    }

    fn read_i64_at(code: &[u8], pc: &mut usize) -> Result<i64, String> {
        let mut result = 0i64;
        let mut shift = 0;
        let mut byte;
        while *pc < code.len() {
            byte = code[*pc];
            *pc += 1;
            result |= ((byte & 0x7F) as i64) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                if shift < 64 && (byte & 0x40) != 0 {
                    result |= !0i64 << shift;
                }
                return Ok(result);
            }
        }
        Err("Unexpected EOF in LEB128 i64".to_string())
    }

    fn read_i32_at(code: &[u8], pc: &mut usize) -> Result<i32, String> {
        let v = Self::read_i64_at(code, pc)?;
        Ok(v as i32)
    }

    fn skip_immediate(op: u8, code: &[u8], pc: &mut usize) -> Result<(), String> {
        match op {
            OP_BR | OP_BR_IF | OP_CALL | OP_LOCAL_GET | OP_LOCAL_SET | OP_LOCAL_TEE => {
                Self::read_u32_at(code, pc)?;
            }
            OP_I32_CONST => {
                Self::read_i32_at(code, pc)?;
            }
            OP_I64_CONST => {
                Self::read_i64_at(code, pc)?;
            }
            OP_F64_CONST => {
                if *pc + 8 <= code.len() {
                    *pc += 8;
                }
            }
            OP_I32_LOAD | OP_I64_LOAD | OP_I32_STORE | OP_I64_STORE => {
                Self::read_u32_at(code, pc)?;
                Self::read_u32_at(code, pc)?;
            }
            OP_MEMORY_SIZE | OP_MEMORY_GROW => {
                if *pc < code.len() {
                    *pc += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// ==============================================================================
// Global WASM Instance Registry & Native VM Module Registration
// ==============================================================================

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);
static WASM_INSTANCES: OnceLock<Mutex<HashMap<u64, WasmInstance>>> = OnceLock::new();

fn get_instances() -> &'static Mutex<HashMap<u64, WasmInstance>> {
    WASM_INSTANCES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_wasm_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Wasm.compile(source_code) -> byte array
    mod_map.insert(
        "compile".to_string(),
        Value::Native("Wasm.compile".into(), |args| {
            if args.is_empty() {
                return Err("Wasm.compile(source_code) expects source code string".into());
            }
            let src = args[0].to_string();
            let program = parse(&src)
                .map_err(|(e, span)| format!("{}:{}: Syntax error: {}", span.line, span.col, e))?;

            let mut compiler = WasmCompiler::new();
            let bytes = compiler.compile_program(&program)?;

            let val_bytes: Vec<Value> = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
            Ok(Value::array(val_bytes))
        }),
    );

    // 2. Wasm.load(wasm_bytes) -> instance_id
    mod_map.insert(
        "load".to_string(),
        Value::Native("Wasm.load".into(), |args| {
            if args.is_empty() {
                return Err("Wasm.load(bytes) expects WASM byte array".into());
            }
            let raw_bytes: Vec<u8> = match &args[0] {
                Value::Array(arr) => {
                    let items = arr.lock();
                    items.iter().map(|v| match v {
                        Value::Int(i) => *i as u8,
                        _ => 0,
                    }).collect()
                }
                Value::String(s) => s.as_bytes().to_vec(),
                _ => return Err("Wasm.load expects array of bytes".into()),
            };

            let module = WasmModule::parse(&raw_bytes)?;
            let instance = WasmInstance::new(module);
            let id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::SeqCst);
            get_instances().lock().unwrap().insert(id, instance);

            Ok(Value::Int(id as i64))
        }),
    );

    // 3. Wasm.invoke(instance_id, export_name, [args], [fuel]) -> result
    mod_map.insert(
        "invoke".to_string(),
        Value::Native("Wasm.invoke".into(), |args| {
            if args.len() < 2 {
                return Err("Wasm.invoke(instance_id, export_name, [args], [fuel]) expects at least 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("instance_id must be integer".into()),
            };
            let func_name = args[1].to_string();

            let call_args: Vec<i64> = if args.len() > 2 {
                match &args[2] {
                    Value::Array(arr) => {
                        arr.lock().iter().map(|v| match v {
                            Value::Int(i) => *i,
                            Value::Float(f) => *f as i64,
                            Value::Bool(b) => if *b { 1 } else { 0 },
                            _ => 0,
                        }).collect()
                    }
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };

            let fuel: u64 = if args.len() > 3 {
                match args[3] {
                    Value::Int(f) if f > 0 => f as u64,
                    _ => 1_000_000,
                }
            } else {
                1_000_000 // 1M instructions default budget
            };

            let mut instances = get_instances().lock().unwrap();
            let instance = instances
                .get_mut(&id)
                .ok_or_else(|| format!("WASM Instance {} not found", id))?;

            let res = instance.invoke(&func_name, &call_args, fuel)?;
            Ok(Value::Int(res))
        }),
    );

    // 4. Wasm.read_memory(instance_id, offset, len) -> string
    mod_map.insert(
        "read_memory".to_string(),
        Value::Native("Wasm.read_memory".into(), |args| {
            if args.len() < 3 {
                return Err("read_memory(instance_id, offset, len) requires 3 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("instance_id must be integer".into()),
            };
            let offset = match args[1] {
                Value::Int(o) => o as usize,
                _ => 0,
            };
            let len = match args[2] {
                Value::Int(l) => l as usize,
                _ => 0,
            };

            let instances = get_instances().lock().unwrap();
            let instance = instances
                .get(&id)
                .ok_or_else(|| format!("WASM Instance {} not found", id))?;

            if offset + len <= instance.memory.len() {
                let slice = &instance.memory[offset..offset + len];
                let s = String::from_utf8_lossy(slice).to_string();
                Ok(Value::string(s))
            } else {
                Err("Memory read out of bounds".into())
            }
        }),
    );

    // 5. Wasm.write_memory(instance_id, offset, text_or_bytes) -> bool
    mod_map.insert(
        "write_memory".to_string(),
        Value::Native("Wasm.write_memory".into(), |args| {
            if args.len() < 3 {
                return Err("write_memory(instance_id, offset, data) requires 3 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("instance_id must be integer".into()),
            };
            let offset = match args[1] {
                Value::Int(o) => o as usize,
                _ => 0,
            };
            let bytes = match &args[2] {
                Value::String(s) => s.as_bytes().to_vec(),
                Value::Array(arr) => arr.lock().iter().map(|v| match v {
                    Value::Int(i) => *i as u8,
                    _ => 0,
                }).collect(),
                _ => return Err("Data must be string or byte array".into()),
            };

            let mut instances = get_instances().lock().unwrap();
            let instance = instances
                .get_mut(&id)
                .ok_or_else(|| format!("WASM Instance {} not found", id))?;

            if offset + bytes.len() <= instance.memory.len() {
                instance.memory[offset..offset + bytes.len()].copy_from_slice(&bytes);
                Ok(Value::Bool(true))
            } else {
                Err("Memory write out of bounds".into())
            }
        }),
    );

    // 6. Wasm.memory_size(instance_id) -> bytes count
    mod_map.insert(
        "memory_size".to_string(),
        Value::Native("Wasm.memory_size".into(), |args| {
            if args.is_empty() {
                return Err("memory_size(instance_id) requires instance_id".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("instance_id must be integer".into()),
            };
            let instances = get_instances().lock().unwrap();
            let instance = instances
                .get(&id)
                .ok_or_else(|| format!("WASM Instance {} not found", id))?;
            Ok(Value::Int(instance.memory.len() as i64))
        }),
    );

    globals.insert("Wasm".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_wasm".to_string(), Value::map(mod_map));
}
