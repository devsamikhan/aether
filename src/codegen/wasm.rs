// ==============================================================================
// AetherWasm — Universal W3C WebAssembly Binary Compiler (Zero External Crates)
// ==============================================================================

use crate::syntax::ast::*;
use std::collections::HashMap;

// W3C WebAssembly Section IDs
pub const SEC_TYPE: u8 = 1;
pub const SEC_IMPORT: u8 = 2;
pub const SEC_FUNCTION: u8 = 3;
pub const SEC_TABLE: u8 = 4;
pub const SEC_MEMORY: u8 = 5;
pub const SEC_GLOBAL: u8 = 6;
pub const SEC_EXPORT: u8 = 7;
pub const SEC_START: u8 = 8;
pub const SEC_ELEMENT: u8 = 9;
pub const SEC_CODE: u8 = 10;
pub const SEC_DATA: u8 = 11;

// W3C WebAssembly Value Types
pub const TYPE_I32: u8 = 0x7F;
pub const TYPE_I64: u8 = 0x7E;
pub const TYPE_F32: u8 = 0x7D;
pub const TYPE_F64: u8 = 0x7C;
pub const TYPE_FUNC: u8 = 0x60;
pub const TYPE_EMPTY: u8 = 0x40;

// W3C WebAssembly Opcodes
pub const OP_UNREACHABLE: u8 = 0x00;
pub const OP_NOP: u8 = 0x01;
pub const OP_BLOCK: u8 = 0x02;
pub const OP_LOOP: u8 = 0x03;
pub const OP_IF: u8 = 0x04;
pub const OP_ELSE: u8 = 0x05;
pub const OP_END: u8 = 0x0B;
pub const OP_BR: u8 = 0x0C;
pub const OP_BR_IF: u8 = 0x0D;
pub const OP_RETURN: u8 = 0x0F;
pub const OP_CALL: u8 = 0x10;
pub const OP_DROP: u8 = 0x1A;

pub const OP_LOCAL_GET: u8 = 0x20;
pub const OP_LOCAL_SET: u8 = 0x21;
pub const OP_LOCAL_TEE: u8 = 0x22;

pub const OP_I32_LOAD: u8 = 0x28;
pub const OP_I64_LOAD: u8 = 0x29;
pub const OP_F64_LOAD: u8 = 0x2B;
pub const OP_I32_STORE: u8 = 0x36;
pub const OP_I64_STORE: u8 = 0x37;
pub const OP_F64_STORE: u8 = 0x39;
pub const OP_MEMORY_SIZE: u8 = 0x3F;
pub const OP_MEMORY_GROW: u8 = 0x40;

pub const OP_I32_CONST: u8 = 0x41;
pub const OP_I64_CONST: u8 = 0x42;
pub const OP_F64_CONST: u8 = 0x44;

pub const OP_I32_EQZ: u8 = 0x45;
pub const OP_I32_EQ: u8 = 0x46;
pub const OP_I32_NE: u8 = 0x47;
pub const OP_I32_LT_S: u8 = 0x48;
pub const OP_I32_GT_S: u8 = 0x4A;
pub const OP_I32_LE_S: u8 = 0x4C;
pub const OP_I32_GE_S: u8 = 0x4E;

pub const OP_I64_EQZ: u8 = 0x50;
pub const OP_I64_EQ: u8 = 0x51;
pub const OP_I64_NE: u8 = 0x52;
pub const OP_I64_LT_S: u8 = 0x53;
pub const OP_I64_GT_S: u8 = 0x54;
pub const OP_I64_LE_S: u8 = 0x55;
pub const OP_I64_GE_S: u8 = 0x56;

pub const OP_F64_EQ: u8 = 0x61;
pub const OP_F64_NE: u8 = 0x62;
pub const OP_F64_LT: u8 = 0x63;
pub const OP_F64_GT: u8 = 0x64;
pub const OP_F64_LE: u8 = 0x65;
pub const OP_F64_GE: u8 = 0x66;

pub const OP_I64_ADD: u8 = 0x7C;
pub const OP_I64_SUB: u8 = 0x7D;
pub const OP_I64_MUL: u8 = 0x7E;
pub const OP_I64_DIV_S: u8 = 0x7F;
pub const OP_I64_REM_S: u8 = 0x81;
pub const OP_I64_AND: u8 = 0x83;
pub const OP_I64_OR: u8 = 0x84;
pub const OP_I64_XOR: u8 = 0x85;
pub const OP_I64_SHL: u8 = 0x86;
pub const OP_I64_SHR_S: u8 = 0x87;

pub const OP_F64_ABS: u8 = 0x99;
pub const OP_F64_NEG: u8 = 0x9A;
pub const OP_F64_CEIL: u8 = 0x9B;
pub const OP_F64_FLOOR: u8 = 0x9C;
pub const OP_F64_SQRT: u8 = 0x9F;
pub const OP_F64_ADD: u8 = 0xA0;
pub const OP_F64_SUB: u8 = 0xA1;
pub const OP_F64_MUL: u8 = 0xA2;
pub const OP_F64_DIV: u8 = 0xA3;

pub const OP_I32_WRAP_I64: u8 = 0xA7;
pub const OP_I64_EXTEND_I32_S: u8 = 0xAC;
pub const OP_I64_EXTEND_I32_U: u8 = 0xAD;
pub const OP_F64_CONVERT_I64_S: u8 = 0xB9;
pub const OP_I64_TRUNC_F64_S: u8 = 0xB0;

// ==============================================================================
// LEB128 Encoding Utilities
// ==============================================================================

pub fn encode_u32_leb128(mut value: u32, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

pub fn encode_i32_leb128(mut value: i32, buf: &mut Vec<u8>) {
    let mut more = true;
    while more {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        let sign_bit = (byte & 0x40) != 0;
        if (value == 0 && !sign_bit) || (value == -1 && sign_bit) {
            more = false;
        } else {
            byte |= 0x80;
        }
        buf.push(byte);
    }
}

pub fn encode_i64_leb128(mut value: i64, buf: &mut Vec<u8>) {
    let mut more = true;
    while more {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        let sign_bit = (byte & 0x40) != 0;
        if (value == 0 && !sign_bit) || (value == -1 && sign_bit) {
            more = false;
        } else {
            byte |= 0x80;
        }
        buf.push(byte);
    }
}

pub fn encode_f64(value: f64, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn encode_string(s: &str, buf: &mut Vec<u8>) {
    encode_u32_leb128(s.len() as u32, buf);
    buf.extend_from_slice(s.as_bytes());
}

// ==============================================================================
// Function Signature & Code Builder
// ==============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WasmFuncType {
    pub params: Vec<u8>,
    pub results: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WasmFunction {
    pub name: String,
    pub type_index: u32,
    pub locals: Vec<(u32, u8)>, // (count, type)
    pub code: Vec<u8>,
}

pub struct WasmCompiler {
    types: Vec<WasmFuncType>,
    functions: Vec<WasmFunction>,
    func_indices: HashMap<String, u32>,
    data_segments: Vec<(u32, Vec<u8>)>,
    current_data_offset: u32,
}

impl WasmCompiler {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            functions: Vec::new(),
            func_indices: HashMap::new(),
            data_segments: Vec::new(),
            current_data_offset: 1024, // Reserve first 1KB for system/stack
        }
    }

    pub fn get_or_add_type(&mut self, ft: WasmFuncType) -> u32 {
        if let Some(idx) = self.types.iter().position(|t| t == &ft) {
            idx as u32
        } else {
            let idx = self.types.len() as u32;
            self.types.push(ft);
            idx
        }
    }

    /// Compiles an AETHER AST Program into standard W3C WebAssembly binary
    pub fn compile_program(&mut self, program: &Program) -> Result<Vec<u8>, String> {
        // Pass 1: Collect and declare function signatures
        for stmt in &program.statements {
            if let Statement::FnDef { name, params, .. } = stmt {
                let param_types = vec![TYPE_I64; params.len()];
                let return_types = vec![TYPE_I64]; // default all functions return i64
                let type_idx = self.get_or_add_type(WasmFuncType {
                    params: param_types,
                    results: return_types,
                });
                let func_idx = self.functions.len() as u32;
                self.func_indices.insert(name.clone(), func_idx);
                self.functions.push(WasmFunction {
                    name: name.clone(),
                    type_index: type_idx,
                    locals: Vec::new(),
                    code: Vec::new(),
                });
            }
        }

        // Pass 2: Compile function bodies
        for stmt in &program.statements {
            if let Statement::FnDef { name, params, body, .. } = stmt {
                let func_idx = *self.func_indices.get(name).unwrap() as usize;
                self.compile_function(func_idx, params, body)?;
            }
        }

        // If top-level non-function statements exist, wrap them in a `main` or `_start` export
        let top_level_stmts: Vec<&Statement> = program
            .statements
            .iter()
            .filter(|s| !matches!(s, Statement::FnDef { .. }))
            .collect();

        if !top_level_stmts.is_empty() && !self.func_indices.contains_key("main") {
            let type_idx = self.get_or_add_type(WasmFuncType {
                params: Vec::new(),
                results: vec![TYPE_I64],
            });
            let main_idx = self.functions.len() as u32;
            self.func_indices.insert("main".to_string(), main_idx);
            self.functions.push(WasmFunction {
                name: "main".to_string(),
                type_index: type_idx,
                locals: Vec::new(),
                code: Vec::new(),
            });

            self.compile_top_level(main_idx as usize, &top_level_stmts)?;
        }

        self.emit_wasm_binary()
    }

    fn compile_function(
        &mut self,
        func_idx: usize,
        params: &[Param],
        body: &Block,
    ) -> Result<(), String> {
        let mut local_map = HashMap::new();
        // Parameters get local indices 0..params.len()
        for (i, p) in params.iter().enumerate() {
            local_map.insert(p.name.clone(), i as u32);
        }

        let mut code = Vec::new();
        let mut local_count = params.len() as u32;

        for st in &body.statements {
            self.compile_statement(st, &mut local_map, &mut local_count, &mut code)?;
        }

        if let Some(ref res_expr) = body.result {
            self.compile_expression(res_expr, &mut local_map, &mut local_count, &mut code)?;
        } else if code.is_empty() || *code.last().unwrap() != OP_RETURN {
            // Default return 0 if no explicit return
            code.push(OP_I64_CONST);
            encode_i64_leb128(0, &mut code);
        }

        code.push(OP_END);

        let extra_locals_count = local_count.saturating_sub(params.len() as u32);
        if extra_locals_count > 0 {
            self.functions[func_idx].locals = vec![(extra_locals_count, TYPE_I64)];
        }
        self.functions[func_idx].code = code;

        Ok(())
    }

    fn compile_top_level(
        &mut self,
        func_idx: usize,
        stmts: &[&Statement],
    ) -> Result<(), String> {
        let mut local_map = HashMap::new();
        let mut local_count = 0u32;
        let mut code = Vec::new();

        for st in stmts {
            self.compile_statement(st, &mut local_map, &mut local_count, &mut code)?;
        }

        code.push(OP_I64_CONST);
        encode_i64_leb128(0, &mut code);
        code.push(OP_END);

        if local_count > 0 {
            self.functions[func_idx].locals = vec![(local_count, TYPE_I64)];
        }
        self.functions[func_idx].code = code;

        Ok(())
    }

    fn compile_statement(
        &mut self,
        stmt: &Statement,
        locals: &mut HashMap<String, u32>,
        local_count: &mut u32,
        code: &mut Vec<u8>,
    ) -> Result<(), String> {
        match stmt {
            Statement::Let { name, initializer, .. } => {
                let slot = if let Some(s) = locals.get(name) {
                    *s
                } else {
                    let s = *local_count;
                    *local_count += 1;
                    locals.insert(name.clone(), s);
                    s
                };

                if let Some(init) = initializer {
                    self.compile_expression(init, locals, local_count, code)?;
                } else {
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(0, code);
                }

                code.push(OP_LOCAL_SET);
                encode_u32_leb128(slot, code);
            }
            Statement::Assign { target, op, value, .. } => {
                if let Expr::Ident(name, _) = target {
                    let slot = if let Some(s) = locals.get(name) {
                        *s
                    } else {
                        let s = *local_count;
                        *local_count += 1;
                        locals.insert(name.clone(), s);
                        s
                    };

                    match op {
                        AssignOp::Assign => {
                            self.compile_expression(value, locals, local_count, code)?;
                        }
                        AssignOp::AddAssign => {
                            code.push(OP_LOCAL_GET);
                            encode_u32_leb128(slot, code);
                            self.compile_expression(value, locals, local_count, code)?;
                            code.push(OP_I64_ADD);
                        }
                        AssignOp::SubAssign => {
                            code.push(OP_LOCAL_GET);
                            encode_u32_leb128(slot, code);
                            self.compile_expression(value, locals, local_count, code)?;
                            code.push(OP_I64_SUB);
                        }
                        AssignOp::MulAssign => {
                            code.push(OP_LOCAL_GET);
                            encode_u32_leb128(slot, code);
                            self.compile_expression(value, locals, local_count, code)?;
                            code.push(OP_I64_MUL);
                        }
                        AssignOp::DivAssign => {
                            code.push(OP_LOCAL_GET);
                            encode_u32_leb128(slot, code);
                            self.compile_expression(value, locals, local_count, code)?;
                            code.push(OP_I64_DIV_S);
                        }
                        AssignOp::ModAssign => {
                            code.push(OP_LOCAL_GET);
                            encode_u32_leb128(slot, code);
                            self.compile_expression(value, locals, local_count, code)?;
                            code.push(OP_I64_REM_S);
                        }
                    }

                    code.push(OP_LOCAL_SET);
                    encode_u32_leb128(slot, code);
                }
            }
            Statement::Return { value, .. } => {
                if let Some(val) = value {
                    self.compile_expression(val, locals, local_count, code)?;
                } else {
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(0, code);
                }
                code.push(OP_RETURN);
            }
            Statement::While { condition, body, .. } => {
                // WASM structured loop:
                // block $exit {
                //   loop $cont {
                //     br_if $exit (!condition)
                //     body...
                //     br $cont
                //   }
                // }
                code.push(OP_BLOCK);
                code.push(TYPE_EMPTY);

                code.push(OP_LOOP);
                code.push(TYPE_EMPTY);

                // Invert condition: condition == 0 -> br $exit (depth 1)
                self.compile_expression(condition, locals, local_count, code)?;
                code.push(OP_I64_EQZ);
                code.push(OP_BR_IF);
                encode_u32_leb128(1, code); // break to outer block

                for st in &body.statements {
                    self.compile_statement(st, locals, local_count, code)?;
                }

                code.push(OP_BR);
                encode_u32_leb128(0, code); // continue loop

                code.push(OP_END); // end loop
                code.push(OP_END); // end block
            }
            Statement::Expr(expr) => {
                self.compile_expression(expr, locals, local_count, code)?;
                code.push(OP_DROP);
            }
            _ => {}
        }
        Ok(())
    }

    fn compile_expression(
        &mut self,
        expr: &Expr,
        locals: &mut HashMap<String, u32>,
        local_count: &mut u32,
        code: &mut Vec<u8>,
    ) -> Result<(), String> {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(v) => {
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(*v, code);
                }
                Literal::Float(v) => {
                    code.push(OP_F64_CONST);
                    encode_f64(*v, code);
                    code.push(OP_I64_TRUNC_F64_S);
                }
                Literal::Bool(true) => {
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(1, code);
                }
                Literal::Bool(false) | Literal::Nil => {
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(0, code);
                }
                Literal::String(s) => {
                    let offset = self.current_data_offset;
                    let bytes = s.as_bytes().to_vec();
                    self.current_data_offset += bytes.len() as u32 + 1; // null terminated
                    self.data_segments.push((offset, bytes));
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(offset as i64, code);
                }
            },
            Expr::Ident(name, span) => {
                if let Some(slot) = locals.get(name) {
                    code.push(OP_LOCAL_GET);
                    encode_u32_leb128(*slot, code);
                } else {
                    return Err(format!("{}:{}: Undefined local variable in WASM: {}", span.line, span.col, name));
                }
            }
            Expr::Binary(left, op, right, _) => {
                self.compile_expression(left, locals, local_count, code)?;
                self.compile_expression(right, locals, local_count, code)?;

                match op {
                    BinaryOp::Add => code.push(OP_I64_ADD),
                    BinaryOp::Sub => code.push(OP_I64_SUB),
                    BinaryOp::Mul => code.push(OP_I64_MUL),
                    BinaryOp::Div => code.push(OP_I64_DIV_S),
                    BinaryOp::Mod => code.push(OP_I64_REM_S),
                    BinaryOp::Equal => {
                        code.push(OP_I64_EQ);
                        code.push(OP_I64_EXTEND_I32_U);
                    }
                    BinaryOp::NotEqual => {
                        code.push(OP_I64_NE);
                        code.push(OP_I64_EXTEND_I32_U);
                    }
                    BinaryOp::Less => {
                        code.push(OP_I64_LT_S);
                        code.push(OP_I64_EXTEND_I32_U);
                    }
                    BinaryOp::LessEqual => {
                        code.push(OP_I64_LE_S);
                        code.push(OP_I64_EXTEND_I32_U);
                    }
                    BinaryOp::Greater => {
                        code.push(OP_I64_GT_S);
                        code.push(OP_I64_EXTEND_I32_U);
                    }
                    BinaryOp::GreaterEqual => {
                        code.push(OP_I64_GE_S);
                        code.push(OP_I64_EXTEND_I32_U);
                    }
                    BinaryOp::And => code.push(OP_I64_AND),
                    BinaryOp::Or => code.push(OP_I64_OR),
                    _ => code.push(OP_I64_ADD),
                }
            }
            Expr::Call(callee, args, span) => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    for arg in args {
                        self.compile_expression(arg, locals, local_count, code)?;
                    }

                    if let Some(target_idx) = self.func_indices.get(name) {
                        code.push(OP_CALL);
                        encode_u32_leb128(*target_idx, code);
                    } else {
                        return Err(format!("{}:{}: Unknown WASM function '{}'", span.line, span.col, name));
                    }
                } else {
                    return Err("WASM call currently supports direct identifier calls".to_string());
                }
            }
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.compile_expression(condition, locals, local_count, code)?;
                code.push(OP_IF);
                code.push(TYPE_I64); // returns i64

                // 1. Then branch
                for st in &then_branch.statements {
                    self.compile_statement(st, locals, local_count, code)?;
                }
                if let Some(res) = &then_branch.result {
                    self.compile_expression(res, locals, local_count, code)?;
                } else {
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(0, code);
                }

                // 2. Else branch
                code.push(OP_ELSE);
                if let Some(eb) = else_branch {
                    for st in &eb.statements {
                        self.compile_statement(st, locals, local_count, code)?;
                    }
                    if let Some(res) = &eb.result {
                        self.compile_expression(res, locals, local_count, code)?;
                    } else {
                        code.push(OP_I64_CONST);
                        encode_i64_leb128(0, code);
                    }
                } else {
                    code.push(OP_I64_CONST);
                    encode_i64_leb128(0, code);
                }

                code.push(OP_END);
            }
            _ => {
                code.push(OP_I64_CONST);
                encode_i64_leb128(0, code);
            }
        }
        Ok(())
    }

    // ==========================================================================
    // Emit Full W3C .wasm Binary
    // ==========================================================================

    fn emit_wasm_binary(&self) -> Result<Vec<u8>, String> {
        let mut wasm = Vec::new();

        // 1. Magic Header & Version (W3C Standard)
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // \0asm
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

        // 2. Type Section (ID 1)
        if !self.types.is_empty() {
            let mut sec_buf = Vec::new();
            encode_u32_leb128(self.types.len() as u32, &mut sec_buf);
            for t in &self.types {
                sec_buf.push(TYPE_FUNC);
                encode_u32_leb128(t.params.len() as u32, &mut sec_buf);
                sec_buf.extend_from_slice(&t.params);
                encode_u32_leb128(t.results.len() as u32, &mut sec_buf);
                sec_buf.extend_from_slice(&t.results);
            }
            self.write_section(SEC_TYPE, &sec_buf, &mut wasm);
        }

        // 3. Function Section (ID 3)
        if !self.functions.is_empty() {
            let mut sec_buf = Vec::new();
            encode_u32_leb128(self.functions.len() as u32, &mut sec_buf);
            for f in &self.functions {
                encode_u32_leb128(f.type_index, &mut sec_buf);
            }
            self.write_section(SEC_FUNCTION, &sec_buf, &mut wasm);
        }

        // 4. Memory Section (ID 5) - 1 initial page (64KB), max 16 pages (1MB)
        {
            let mut sec_buf = Vec::new();
            encode_u32_leb128(1, &mut sec_buf); // 1 memory declaration
            sec_buf.push(0x01); // flags: has maximum
            encode_u32_leb128(1, &mut sec_buf);  // min 1 page
            encode_u32_leb128(16, &mut sec_buf); // max 16 pages
            self.write_section(SEC_MEMORY, &sec_buf, &mut wasm);
        }

        // 5. Export Section (ID 7)
        if !self.functions.is_empty() {
            let mut sec_buf = Vec::new();
            // Export functions + 1 memory export
            encode_u32_leb128((self.functions.len() + 1) as u32, &mut sec_buf);

            // Export functions
            for (idx, f) in self.functions.iter().enumerate() {
                encode_string(&f.name, &mut sec_buf);
                sec_buf.push(0x00); // 0x00 = function export
                encode_u32_leb128(idx as u32, &mut sec_buf);
            }

            // Export linear memory
            encode_string("memory", &mut sec_buf);
            sec_buf.push(0x02); // 0x02 = memory export
            encode_u32_leb128(0, &mut sec_buf); // memory index 0

            self.write_section(SEC_EXPORT, &sec_buf, &mut wasm);
        }

        // 6. Code Section (ID 10)
        if !self.functions.is_empty() {
            let mut sec_buf = Vec::new();
            encode_u32_leb128(self.functions.len() as u32, &mut sec_buf);

            for f in &self.functions {
                let mut body_buf = Vec::new();
                // Local declarations vector
                encode_u32_leb128(f.locals.len() as u32, &mut body_buf);
                for (count, l_type) in &f.locals {
                    encode_u32_leb128(*count, &mut body_buf);
                    body_buf.push(*l_type);
                }
                // Bytecode
                body_buf.extend_from_slice(&f.code);

                // Prefix with body size
                encode_u32_leb128(body_buf.len() as u32, &mut sec_buf);
                sec_buf.extend_from_slice(&body_buf);
            }

            self.write_section(SEC_CODE, &sec_buf, &mut wasm);
        }

        // 7. Data Section (ID 11)
        if !self.data_segments.is_empty() {
            let mut sec_buf = Vec::new();
            encode_u32_leb128(self.data_segments.len() as u32, &mut sec_buf);
            for (offset, bytes) in &self.data_segments {
                sec_buf.push(0x00); // segment flags: active, memory index 0
                // Offset expr: i32.const <offset> end
                sec_buf.push(OP_I32_CONST);
                encode_i32_leb128(*offset as i32, &mut sec_buf);
                sec_buf.push(OP_END);

                // Bytes
                encode_u32_leb128(bytes.len() as u32, &mut sec_buf);
                sec_buf.extend_from_slice(bytes);
            }
            self.write_section(SEC_DATA, &sec_buf, &mut wasm);
        }

        Ok(wasm)
    }

    fn write_section(&self, sec_id: u8, content: &[u8], wasm: &mut Vec<u8>) {
        wasm.push(sec_id);
        encode_u32_leb128(content.len() as u32, wasm);
        wasm.extend_from_slice(content);
    }
}
