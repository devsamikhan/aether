use super::bytecode::OpCode;
use super::value::{CompiledFunction, Value};
use crate::syntax::ast::*;
use crate::syntax::span::Span;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Local {
    name: String,
    depth: usize,
    #[allow(dead_code)]
    is_captured: bool,
}

#[derive(Debug, Clone)]
struct LoopContext {
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
    local_count: usize,
}

pub struct BytecodeCompiler {
    function: CompiledFunction,
    locals: Vec<Local>,
    scope_depth: usize,
    parent_locals: Option<Vec<Local>>,
    captured: Vec<String>,
    loop_stack: Vec<LoopContext>,
    global_vars: std::collections::HashSet<String>,
    current_class_name: Option<String>,
}

impl BytecodeCompiler {
    pub fn new(name: &str, arity: usize) -> Self {
        Self {
            function: CompiledFunction::new(name, arity),
            locals: vec![Local {
                name: "".to_string(),
                depth: 0,
                is_captured: false,
            }],
            scope_depth: 0,
            parent_locals: None,
            captured: Vec::new(),
            loop_stack: Vec::new(),
            global_vars: std::collections::HashSet::new(),
            current_class_name: None,
        }
    }

    pub fn compile(mut self, program: &Program) -> Result<CompiledFunction, String> {
        let total = program.statements.len();
        for (i, stmt) in program.statements.iter().enumerate() {
            if i == total - 1 {
                if let Statement::Expr(expr) = stmt {
                    self.compile_expression(expr)?;
                    self.emit_op(OpCode::Return, expr.span());
                    return Ok(self.function);
                }
            }
            self.compile_statement(stmt)?;
        }

        // Return nil at the end of the script/function
        self.emit_op(OpCode::Nil, program.span);
        self.emit_op(OpCode::Return, program.span);

        Ok(self.function)
    }

    fn compile_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Global(names, _) => {
                for name in names {
                    self.global_vars.insert(name.clone());
                }
            }
            Statement::Let {
                name,
                initializer,
                span,
                ..
            } => {
                if let Some(init) = initializer {
                    self.compile_expression(init)?;
                } else {
                    self.emit_op(OpCode::Nil, *span);
                }

                if self.function.name != "<main>" && self.function.name != "<repl>" && self.scope_depth > 0 {
                    self.add_local(name.clone(), *span)?;
                } else {
                    let const_idx = self.add_constant(Value::string(name.clone()));
                    self.emit_op(OpCode::DefineGlobal, *span);
                    self.emit_u16(const_idx as u16, *span);
                }
            }
            Statement::Assign {
                target,
                op,
                value,
                span,
            } => {
                match target {
                    Expr::Ident(name, id_span) => {
                        match op {
                            AssignOp::Assign => {
                                self.compile_expression(value)?;
                            }
                            AssignOp::AddAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Add, *span);
                            }
                            AssignOp::SubAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Sub, *span);
                            }
                            AssignOp::MulAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Mul, *span);
                            }
                            AssignOp::DivAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Div, *span);
                            }
                            AssignOp::ModAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Mod, *span);
                            }
                        }
                        if self.global_vars.contains(name) {
                            let const_idx = self.add_constant(Value::string(name.clone()));
                            self.emit_op(OpCode::SetGlobal, *id_span);
                            self.emit_u16(const_idx as u16, *id_span);
                            self.emit_op(OpCode::Pop, *span);
                        } else if let Some(local_idx) = self.resolve_local(name) {
                            self.emit_op(OpCode::SetLocal, *id_span);
                            self.emit_u16(local_idx as u16, *id_span);
                            self.emit_op(OpCode::Pop, *span);
                        } else if self.function.name != "<main>" && self.function.name != "<repl>" && *op == AssignOp::Assign {
                            self.add_local(name.clone(), *id_span)?;
                        } else {
                            let const_idx = self.add_constant(Value::string(name.clone()));
                            self.emit_op(OpCode::SetGlobal, *id_span);
                            self.emit_u16(const_idx as u16, *id_span);
                            self.emit_op(OpCode::Pop, *span);
                        }
                    }
                    Expr::Tuple(targets, _) => {
                        // Sequence unpacking: a, b = ...
                        // Pre-allocate local variable slots if inside a function
                        if self.function.name != "<main>" && self.function.name != "<repl>" {
                            for t in targets {
                                if let Expr::Ident(name, id_span) = t {
                                    if !self.global_vars.contains(name) && self.resolve_local(name).is_none() {
                                        self.emit_op(OpCode::Nil, *id_span);
                                        self.add_local(name.clone(), *id_span)?;
                                    }
                                }
                            }
                        }

                        self.compile_expression(value)?;
                        self.emit_op(OpCode::UnpackSequence, *span);
                        self.emit_u16(targets.len() as u16, *span);

                        for t in targets {
                            match t {
                                Expr::Ident(name, id_span) => {
                                    if self.global_vars.contains(name) {
                                        let const_idx = self.add_constant(Value::string(name.clone()));
                                        self.emit_op(OpCode::SetGlobal, *id_span);
                                        self.emit_u16(const_idx as u16, *id_span);
                                    } else if let Some(local_idx) = self.resolve_local(name) {
                                        self.emit_op(OpCode::SetLocal, *id_span);
                                        self.emit_u16(local_idx as u16, *id_span);
                                    } else {
                                        let const_idx = self.add_constant(Value::string(name.clone()));
                                        self.emit_op(OpCode::SetGlobal, *id_span);
                                        self.emit_u16(const_idx as u16, *id_span);
                                    }
                                    self.emit_op(OpCode::Pop, *span);
                                }
                                _ => return Err("Invalid assignment target in unpacking".to_string()),
                            }
                        }
                    }
                    Expr::Index(obj, idx, _) => {
                        self.compile_expression(obj)?;
                        self.compile_expression(idx)?;
                        match op {
                            AssignOp::Assign => {
                                self.compile_expression(value)?;
                            }
                            AssignOp::AddAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Add, *span);
                            }
                            AssignOp::SubAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Sub, *span);
                            }
                            AssignOp::MulAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Mul, *span);
                            }
                            AssignOp::DivAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Div, *span);
                            }
                            AssignOp::ModAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Mod, *span);
                            }
                        }
                        self.emit_op(OpCode::IndexSet, *span);
                        self.emit_op(OpCode::Pop, *span);
                    }
                    Expr::Member(obj, member, _) => {
                        self.compile_expression(obj)?;
                        let const_idx = self.add_constant(Value::string(member.clone()));
                        self.emit_op(OpCode::Constant, *span);
                        self.emit_u16(const_idx as u16, *span);
                        match op {
                            AssignOp::Assign => {
                                self.compile_expression(value)?;
                            }
                            AssignOp::AddAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Add, *span);
                            }
                            AssignOp::SubAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Sub, *span);
                            }
                            AssignOp::MulAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Mul, *span);
                            }
                            AssignOp::DivAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Div, *span);
                            }
                            AssignOp::ModAssign => {
                                self.compile_expression(target)?;
                                self.compile_expression(value)?;
                                self.emit_op(OpCode::Mod, *span);
                            }
                        }
                        self.emit_op(OpCode::SetAttr, *span);
                        self.emit_op(OpCode::Pop, *span);
                    }
                    _ => return Err("Invalid assignment target".to_string()),
                }
            }
            Statement::FnDef {
                name,
                params,
                body,
                span,
                ..
            } => {
                let mut regular_params = Vec::new();
                let mut varargs_param = None;
                let mut kwargs_param = None;
                let mut min_arity = 0;

                for p in params {
                    if p.is_vararg {
                        varargs_param = Some(p.name.clone());
                    } else if p.is_kwarg {
                        kwargs_param = Some(p.name.clone());
                    } else {
                        if p.default_val.is_none() && varargs_param.is_none() {
                            min_arity += 1;
                        }
                        regular_params.push(p.clone());
                    }
                }

                let mut sub_compiler = BytecodeCompiler::new(name, min_arity);
                sub_compiler.function.max_arity = regular_params.len();
                sub_compiler.function.param_names = regular_params.iter().map(|p| p.name.clone()).collect();
                sub_compiler.function.has_varargs = varargs_param.is_some();
                sub_compiler.function.varargs_param = varargs_param.clone();
                sub_compiler.function.has_kwargs = kwargs_param.is_some();
                sub_compiler.function.kwargs_param = kwargs_param.clone();
                sub_compiler.scope_depth = 1;
                sub_compiler.parent_locals = Some(self.locals.clone());

                // Add regular parameters as locals
                for param in &regular_params {
                    sub_compiler.add_local(param.name.clone(), *span)?;
                }
                if let Some(ref v_name) = varargs_param {
                    sub_compiler.add_local(v_name.clone(), *span)?;
                }
                if let Some(ref k_name) = kwargs_param {
                    sub_compiler.add_local(k_name.clone(), *span)?;
                }

                // Default argument initialization preamble
                for (i, p) in regular_params.iter().enumerate() {
                    if let Some(ref def_expr) = p.default_val {
                        let slot = i + 1;
                        sub_compiler.emit_op(OpCode::GetLocal, *span);
                        sub_compiler.emit_u16(slot as u16, *span);
                        let sent_idx = sub_compiler.add_constant(Value::DefaultSentinel);
                        sub_compiler.emit_op(OpCode::Constant, *span);
                        sub_compiler.emit_u16(sent_idx as u16, *span);
                        sub_compiler.emit_op(OpCode::Equal, *span);
                        let skip_jump = sub_compiler.emit_jump(OpCode::JumpIfFalse, *span);
                        sub_compiler.emit_op(OpCode::Pop, *span);
                        sub_compiler.compile_expression(def_expr)?;
                        sub_compiler.emit_op(OpCode::SetLocal, *span);
                        sub_compiler.emit_u16(slot as u16, *span);
                        sub_compiler.emit_op(OpCode::Pop, *span);
                        let end_jump = sub_compiler.emit_jump(OpCode::Jump, *span);
                        sub_compiler.patch_jump(skip_jump);
                        sub_compiler.emit_op(OpCode::Pop, *span);
                        sub_compiler.patch_jump(end_jump);
                    }
                }

                // Compile function body
                let num_stmts = body.statements.len();
                for (i, st) in body.statements.iter().enumerate() {
                    if i == num_stmts - 1 && body.result.is_none() {
                        if let Statement::Expr(expr) = st {
                            sub_compiler.compile_expression(expr)?;
                            sub_compiler.emit_op(OpCode::Return, *span);
                            continue;
                        }
                    }
                    sub_compiler.compile_statement(st)?;
                }

                if let Some(ref res) = body.result {
                    sub_compiler.compile_expression(res)?;
                    sub_compiler.emit_op(OpCode::Return, *span);
                } else if !sub_compiler.function.chunk.code.ends_with(&[OpCode::Return as u8]) {
                    sub_compiler.emit_op(OpCode::Nil, *span);
                    sub_compiler.emit_op(OpCode::Return, *span);
                }

                let captured_vars = sub_compiler.captured.clone();
                let compiled_fn = sub_compiler.function;
                let const_idx = self.add_constant(Value::Function(Arc::new(compiled_fn)));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);

                if !captured_vars.is_empty() {
                    for var_name in &captured_vars {
                        if let Some(slot) = self.resolve_local(var_name) {
                            self.emit_op(OpCode::GetLocal, *span);
                            self.emit_u16(slot as u16, *span);
                        } else {
                            return Err(format!("Failed to resolve captured local '{}'", var_name));
                        }
                    }
                    self.emit_op(OpCode::BuildClosure, *span);
                    self.emit_u16(captured_vars.len() as u16, *span);
                }

                if self.scope_depth > 0 {
                    self.add_local(name.clone(), *span)?;
                } else {
                    let glob_idx = self.add_constant(Value::string(name.clone()));
                    self.emit_op(OpCode::DefineGlobal, *span);
                    self.emit_u16(glob_idx as u16, *span);
                }
            }
            Statement::IntentDef {
                name,
                params,
                require,
                ensure,
                body,
                span,
            } => {
                // Intent compiles into a validated contract function!
                let mut sub_compiler = BytecodeCompiler::new(name, params.len());
                sub_compiler.scope_depth = 1;

                for param in params {
                    sub_compiler.add_local(param.name.clone(), *span)?;
                }

                // Emit pre-conditions (require assertions)
                for req in require {
                    sub_compiler.compile_expression(req)?;
                    sub_compiler.emit_op(OpCode::Pop, req.span());
                }

                // Body statements
                let num_stmts = body.statements.len();
                for (i, st) in body.statements.iter().enumerate() {
                    if i == num_stmts - 1 && body.result.is_none() {
                        if let Statement::Expr(expr) = st {
                            sub_compiler.compile_expression(expr)?;
                            sub_compiler.emit_op(OpCode::Return, *span);
                            continue;
                        }
                    }
                    sub_compiler.compile_statement(st)?;
                }

                // Emit post-conditions (ensure assertions)
                for ens in ensure {
                    sub_compiler.compile_expression(ens)?;
                    sub_compiler.emit_op(OpCode::Pop, ens.span());
                }

                if let Some(ref res) = body.result {
                    sub_compiler.compile_expression(res)?;
                    sub_compiler.emit_op(OpCode::Return, *span);
                } else if !sub_compiler.function.chunk.code.ends_with(&[OpCode::Return as u8]) {
                    sub_compiler.emit_op(OpCode::Nil, *span);
                    sub_compiler.emit_op(OpCode::Return, *span);
                }

                let compiled_fn = sub_compiler.function;
                let const_idx = self.add_constant(Value::Function(Arc::new(compiled_fn)));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);

                let glob_idx = self.add_constant(Value::string(name.clone()));
                self.emit_op(OpCode::DefineGlobal, *span);
                self.emit_u16(glob_idx as u16, *span);
            }
            Statement::While {
                condition,
                body,
                span,
            } => {
                let loop_start = self.function.chunk.code.len();
                self.compile_expression(condition)?;

                let exit_jump = self.emit_jump(OpCode::JumpIfFalse, *span);
                self.emit_op(OpCode::Pop, *span); // pop condition

                self.begin_scope();
                let loop_ctx = LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                    local_count: self.locals.len(),
                };
                self.loop_stack.push(loop_ctx);

                for st in &body.statements {
                    self.compile_statement(st)?;
                }
                self.end_scope(*span);

                let ctx = self.loop_stack.pop().unwrap();

                // Any continues jump to loop_start
                for cont_jump in ctx.continue_jumps {
                    self.patch_jump(cont_jump);
                }

                self.emit_loop(loop_start, *span);
                self.patch_jump(exit_jump);
                self.emit_op(OpCode::Pop, *span); // pop condition on false exit

                // Any breaks jump here
                for brk_jump in ctx.break_jumps {
                    self.patch_jump(brk_jump);
                }
            }
            Statement::For {
                item,
                iter,
                body,
                span,
            } => {
                // 1. Evaluate iterator expression (e.g. range or array)
                self.compile_expression(iter)?;

                // 2. Begin scope and create internal locals
                self.begin_scope();
                let iter_slot = self.locals.len();
                self.add_local("__iter".to_string(), *span)?;

                // Push 0 as initial index
                let zero_idx = self.add_constant(Value::Int(0));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(zero_idx as u16, *span);
                let idx_slot = self.locals.len();
                self.add_local("__idx".to_string(), *span)?;

                // Push nil as initial item slot
                self.emit_op(OpCode::Nil, *span);
                let item_slot = self.locals.len();
                self.add_local(item.clone(), *span)?;

                // Loop start
                let loop_start = self.function.chunk.code.len();

                // Condition: __idx < len(__iter)
                self.emit_op(OpCode::GetLocal, *span);
                self.emit_u16(idx_slot as u16, *span);

                let len_const = self.add_constant(Value::string("len"));
                self.emit_op(OpCode::GetGlobal, *span);
                self.emit_u16(len_const as u16, *span);
                self.emit_op(OpCode::GetLocal, *span);
                self.emit_u16(iter_slot as u16, *span);
                self.emit_op(OpCode::Call, *span);
                self.emit_byte(1, *span); // 1 arg

                self.emit_op(OpCode::Less, *span);

                // Exit if false
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse, *span);
                self.emit_op(OpCode::Pop, *span); // Pop condition

                // item = __iter[__idx]
                self.emit_op(OpCode::GetLocal, *span);
                self.emit_u16(iter_slot as u16, *span);
                self.emit_op(OpCode::GetLocal, *span);
                self.emit_u16(idx_slot as u16, *span);
                self.emit_op(OpCode::IndexGet, *span);
                self.emit_op(OpCode::SetLocal, *span);
                self.emit_u16(item_slot as u16, *span);
                self.emit_op(OpCode::Pop, *span); // Pop result of SetLocal

                // Execute body inside iteration scope
                self.begin_scope();
                let loop_ctx = LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                    local_count: self.locals.len(),
                };
                self.loop_stack.push(loop_ctx);

                for st in &body.statements {
                    self.compile_statement(st)?;
                }
                self.end_scope(*span);

                let ctx = self.loop_stack.pop().unwrap();

                // Patch continue jumps to jump right here (before incrementing __idx)
                for cont_jump in ctx.continue_jumps {
                    self.patch_jump(cont_jump);
                }

                // Increment __idx: __idx = __idx + 1
                self.emit_op(OpCode::GetLocal, *span);
                self.emit_u16(idx_slot as u16, *span);
                let one_const = self.add_constant(Value::Int(1));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(one_const as u16, *span);
                self.emit_op(OpCode::Add, *span);
                self.emit_op(OpCode::SetLocal, *span);
                self.emit_u16(idx_slot as u16, *span);
                self.emit_op(OpCode::Pop, *span);

                // Loop back
                self.emit_loop(loop_start, *span);

                // Patch exit jump
                self.patch_jump(exit_jump);
                self.emit_op(OpCode::Pop, *span); // Pop condition on false

                // Patch break jumps to jump right here
                for brk_jump in ctx.break_jumps {
                    self.patch_jump(brk_jump);
                }

                // End scope (pops __iter, __idx, item)
                self.end_scope(*span);
            }
            Statement::Break(span) => {
                let to_pop = if let Some(ctx) = self.loop_stack.last() {
                    self.locals.len().saturating_sub(ctx.local_count)
                } else {
                    return Err("Cannot use 'break' outside of a loop".to_string());
                };
                for _ in 0..to_pop {
                    self.emit_op(OpCode::Pop, *span);
                }
                let jump = self.emit_jump(OpCode::Jump, *span);
                self.loop_stack.last_mut().unwrap().break_jumps.push(jump);
            }
            Statement::Continue(span) => {
                let to_pop = if let Some(ctx) = self.loop_stack.last() {
                    self.locals.len().saturating_sub(ctx.local_count)
                } else {
                    return Err("Cannot use 'continue' outside of a loop".to_string());
                };
                for _ in 0..to_pop {
                    self.emit_op(OpCode::Pop, *span);
                }
                let jump = self.emit_jump(OpCode::Jump, *span);
                self.loop_stack.last_mut().unwrap().continue_jumps.push(jump);
            }
            Statement::Pass(_) => {
                // No-operation
            }
            Statement::Return { value, span } => {
                if let Some(val) = value {
                    self.compile_expression(val)?;
                } else {
                    self.emit_op(OpCode::Nil, *span);
                }
                self.emit_op(OpCode::Return, *span);
            }
            Statement::Spawn { body, span } => {
                let mut sub_compiler = BytecodeCompiler::new("<fiber>", 0);
                sub_compiler.scope_depth = 1;
                sub_compiler.parent_locals = Some(self.locals.clone());

                let num_stmts = body.statements.len();
                for (i, st) in body.statements.iter().enumerate() {
                    if i == num_stmts - 1 && body.result.is_none() {
                        if let Statement::Expr(expr) = st {
                            sub_compiler.compile_expression(expr)?;
                            sub_compiler.emit_op(OpCode::Return, *span);
                            continue;
                        }
                    }
                    sub_compiler.compile_statement(st)?;
                }
                if let Some(ref res) = body.result {
                    sub_compiler.compile_expression(res)?;
                    sub_compiler.emit_op(OpCode::Return, *span);
                } else if !sub_compiler.function.chunk.code.ends_with(&[OpCode::Return as u8]) {
                    sub_compiler.emit_op(OpCode::Nil, *span);
                    sub_compiler.emit_op(OpCode::Return, *span);
                }

                let captured_vars = sub_compiler.captured.clone();
                sub_compiler.function.arity = captured_vars.len();

                let compiled_fn = sub_compiler.function;
                let const_idx = self.add_constant(Value::Function(Arc::new(compiled_fn)));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);

                for var_name in &captured_vars {
                    if let Some(slot) = self.resolve_local(var_name) {
                        self.emit_op(OpCode::GetLocal, *span);
                        self.emit_u16(slot as u16, *span);
                    } else {
                        return Err(format!("Failed to resolve captured local '{}'", var_name));
                    }
                }

                self.emit_op(OpCode::Spawn, *span);
                self.emit_byte(captured_vars.len() as u8, *span);
            }
            Statement::StructDef { name, fields, span } => {
                let struct_def = Arc::new(super::value::StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                });
                let const_idx = self.add_constant(Value::StructDef(struct_def));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);

                if self.scope_depth > 0 {
                    self.add_local(name.clone(), *span)?;
                } else {
                    let glob_idx = self.add_constant(Value::string(name.clone()));
                    self.emit_op(OpCode::DefineGlobal, *span);
                    self.emit_u16(glob_idx as u16, *span);
                }
            }
            Statement::ClassDef { name, bases, body, span } => {
                let mut methods = HashMap::new();
                let mut class_vars = HashMap::new();

                for stmt in body {
                    match stmt {
                        Statement::FnDef {
                            name: m_name,
                            params,
                            body: m_body,
                            span: m_span,
                            ..
                        } => {
                            let compiled = self.compile_method(name, m_name, params, m_body, *m_span)?;
                            methods.insert(m_name.clone(), Arc::new(compiled));
                        }
                        Statement::Assign {
                            target,
                            op: AssignOp::Assign,
                            value,
                            ..
                        } => {
                            if let Expr::Ident(var_name, _) = target {
                                if let Expr::Literal(lit, _) = value {
                                    let val = match lit {
                                        Literal::Int(i) => Value::Int(*i),
                                        Literal::Float(f) => Value::Float(*f),
                                        Literal::String(s) => Value::string(s.clone()),
                                        Literal::Bool(b) => Value::Bool(*b),
                                        Literal::Nil => Value::Nil,
                                    };
                                    class_vars.insert(var_name.clone(), val);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                let class_def = Arc::new(super::value::ClassDef {
                    name: name.clone(),
                    bases: bases.clone(),
                    methods: Arc::new(parking_lot::Mutex::new(methods)),
                    class_vars: Arc::new(parking_lot::Mutex::new(class_vars)),
                });

                let const_idx = self.add_constant(Value::ClassDef(class_def));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);

                if self.scope_depth > 0 {
                    self.add_local(name.clone(), *span)?;
                } else {
                    let glob_idx = self.add_constant(Value::string(name.clone()));
                    self.emit_op(OpCode::DefineGlobal, *span);
                    self.emit_u16(glob_idx as u16, *span);
                }
            }
            Statement::TryCatch {
                try_block,
                handlers,
                finally_block,
                span,
            } => {
                // Emit push exception handler with jump to handler
                self.emit_op(OpCode::PushExceptionHandler, *span);
                self.emit_byte(0xFF, *span);
                self.emit_byte(0xFF, *span);
                let handler_offset = self.function.chunk.code.len() - 2;

                // Compile try block
                self.begin_scope();
                for st in &try_block.statements {
                    self.compile_statement(st)?;
                }
                self.end_scope(*span);

                // Pop exception handler when try completes without error
                self.emit_op(OpCode::PopExceptionHandler, *span);

                // Execute finally on success path if present
                if let Some(ref fb) = finally_block {
                    self.begin_scope();
                    for st in &fb.statements {
                        self.compile_statement(st)?;
                    }
                    self.end_scope(fb.span);
                }

                let skip_handlers = self.emit_jump(OpCode::Jump, *span);

                // Patch handler_offset: when exception occurs, VM jumps here with exception value on stack
                let jump = self.function.chunk.code.len() - handler_offset - 2;
                self.function.chunk.code[handler_offset] = (jump >> 8) as u8;
                self.function.chunk.code[handler_offset + 1] = (jump & 0xFF) as u8;

                for handler in handlers {
                    self.begin_scope();
                    if let Some(ref name) = handler.as_name {
                        if self.scope_depth > 0 {
                            // exc_val is already on top of stack at local slot
                            self.add_local(name.clone(), handler.span)?;
                        } else {
                            let const_name = self.add_constant(Value::string(name.clone()));
                            self.emit_op(OpCode::DefineGlobal, handler.span);
                            self.emit_u16(const_name as u16, handler.span);
                        }
                    } else {
                        // Pop exception value from stack
                        self.emit_op(OpCode::Pop, handler.span);
                    }

                    for st in &handler.body.statements {
                        self.compile_statement(st)?;
                    }
                    self.end_scope(handler.span);
                }

                // Execute finally on error path if present
                if let Some(ref fb) = finally_block {
                    self.begin_scope();
                    for st in &fb.statements {
                        self.compile_statement(st)?;
                    }
                    self.end_scope(fb.span);
                }

                self.patch_jump(skip_handlers);
            }
            Statement::Raise { expr, span } => {
                if let Some(ref e) = expr {
                    self.compile_expression(e)?;
                } else {
                    let err_const = self.add_constant(Value::string("RuntimeError: Exception raised"));
                    self.emit_op(OpCode::Constant, *span);
                    self.emit_u16(err_const as u16, *span);
                }
                self.emit_op(OpCode::Raise, *span);
            }
            Statement::Import { module, alias, span } => {
                let mod_const = self.add_constant(Value::string(module.clone()));
                self.emit_op(OpCode::ImportModule, *span);
                self.emit_u16(mod_const as u16, *span);

                let bind_name = alias.clone().unwrap_or_else(|| {
                    module.split('.').last().unwrap_or(module).to_string()
                });

                if self.scope_depth > 0 {
                    self.add_local(bind_name, *span)?;
                } else {
                    let glob_idx = self.add_constant(Value::string(bind_name));
                    self.emit_op(OpCode::DefineGlobal, *span);
                    self.emit_u16(glob_idx as u16, *span);
                }
            }
            Statement::FromImport { module, symbols, span } => {
                for (sym, alias) in symbols {
                    let mod_const = self.add_constant(Value::string(module.clone()));
                    let sym_const = self.add_constant(Value::string(sym.clone()));
                    self.emit_op(OpCode::ImportFrom, *span);
                    self.emit_u16(mod_const as u16, *span);
                    self.emit_u16(sym_const as u16, *span);

                    let bind_name = alias.clone().unwrap_or_else(|| sym.clone());
                    if self.scope_depth > 0 {
                        self.add_local(bind_name, *span)?;
                    } else {
                        let glob_idx = self.add_constant(Value::string(bind_name));
                        self.emit_op(OpCode::DefineGlobal, *span);
                        self.emit_u16(glob_idx as u16, *span);
                    }
                }
            }
            Statement::Expr(expr) => {
                self.compile_expression(expr)?;
                self.emit_op(OpCode::Pop, expr.span());
            }
            _ => {}
        }
        Ok(())
    }

    pub fn compile_expression(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Literal(lit, span) => match lit {
                Literal::Int(v) => {
                    let idx = self.add_constant(Value::Int(*v));
                    self.emit_op(OpCode::Constant, *span);
                    self.emit_u16(idx as u16, *span);
                }
                Literal::Float(v) => {
                    let idx = self.add_constant(Value::Float(*v));
                    self.emit_op(OpCode::Constant, *span);
                    self.emit_u16(idx as u16, *span);
                }
                Literal::String(s) => {
                    let idx = self.add_constant(Value::string(s.clone()));
                    self.emit_op(OpCode::Constant, *span);
                    self.emit_u16(idx as u16, *span);
                }
                Literal::Bool(true) => self.emit_op(OpCode::True, *span),
                Literal::Bool(false) => self.emit_op(OpCode::False, *span),
                Literal::Nil => self.emit_op(OpCode::Nil, *span),
            },
            Expr::Ident(name, span) => {
                if self.global_vars.contains(name) {
                    let const_idx = self.add_constant(Value::string(name.clone()));
                    self.emit_op(OpCode::GetGlobal, *span);
                    self.emit_u16(const_idx as u16, *span);
                } else if let Some(local_idx) = self.resolve_local(name) {
                    self.emit_op(OpCode::GetLocal, *span);
                    self.emit_u16(local_idx as u16, *span);
                } else {
                    let const_idx = self.add_constant(Value::string(name.clone()));
                    self.emit_op(OpCode::GetGlobal, *span);
                    self.emit_u16(const_idx as u16, *span);
                }
            }
            Expr::Binary(left, op, right, span) => {
                match op {
                    BinaryOp::And => {
                        self.compile_expression(left)?;
                        let end_jump = self.emit_jump(OpCode::JumpIfFalse, *span);
                        self.emit_op(OpCode::Pop, *span);
                        self.compile_expression(right)?;
                        self.patch_jump(end_jump);
                    }
                    BinaryOp::Or => {
                        self.compile_expression(left)?;
                        let else_jump = self.emit_jump(OpCode::JumpIfFalse, *span);
                        let end_jump = self.emit_jump(OpCode::Jump, *span);
                        self.patch_jump(else_jump);
                        self.emit_op(OpCode::Pop, *span);
                        self.compile_expression(right)?;
                        self.patch_jump(end_jump);
                    }
                    BinaryOp::In => {
                        let fn_idx = self.add_constant(Value::string("__in"));
                        self.emit_op(OpCode::GetGlobal, *span);
                        self.emit_u16(fn_idx as u16, *span);
                        self.compile_expression(left)?;
                        self.compile_expression(right)?;
                        self.emit_op(OpCode::Call, *span);
                        self.emit_byte(2, *span);
                    }
                    _ => {
                        self.compile_expression(left)?;
                        self.compile_expression(right)?;

                        match op {
                            BinaryOp::Add => self.emit_op(OpCode::Add, *span),
                            BinaryOp::Sub => self.emit_op(OpCode::Sub, *span),
                            BinaryOp::Mul => self.emit_op(OpCode::Mul, *span),
                            BinaryOp::Div => self.emit_op(OpCode::Div, *span),
                            BinaryOp::Mod => self.emit_op(OpCode::Mod, *span),
                            BinaryOp::Pow => self.emit_op(OpCode::Pow, *span),
                            BinaryOp::Equal => self.emit_op(OpCode::Equal, *span),
                            BinaryOp::NotEqual => self.emit_op(OpCode::NotEqual, *span),
                            BinaryOp::Less => self.emit_op(OpCode::Less, *span),
                            BinaryOp::LessEqual => self.emit_op(OpCode::LessEqual, *span),
                            BinaryOp::Greater => self.emit_op(OpCode::Greater, *span),
                            BinaryOp::GreaterEqual => self.emit_op(OpCode::GreaterEqual, *span),
                            _ => unreachable!(),
                        }
                    }
                }
            }
            Expr::Unary(op, inner, span) => {
                self.compile_expression(inner)?;
                match op {
                    UnaryOp::Negate => self.emit_op(OpCode::Negate, *span),
                    UnaryOp::Not => self.emit_op(OpCode::Not, *span),
                }
            }
            Expr::Call(callee, args, span) => {
                let has_named_or_star = args.iter().any(|a| matches!(a, Expr::NamedArg(..) | Expr::StarArg(..) | Expr::DoubleStarArg(..)));
                if !has_named_or_star {
                    self.compile_expression(callee)?;
                    for arg in args {
                        self.compile_expression(arg)?;
                    }
                    self.emit_op(OpCode::Call, *span);
                    self.emit_byte(args.len() as u8, *span);
                } else {
                    self.compile_expression(callee)?;
                    let mut pos_count = 0;
                    let mut kw_count = 0;
                    for arg in args {
                        match arg {
                            Expr::NamedArg(name, val, _) => {
                                let const_idx = self.add_constant(Value::string(name.clone()));
                                self.emit_op(OpCode::Constant, *span);
                                self.emit_u16(const_idx as u16, *span);
                                self.compile_expression(val)?;
                                kw_count += 1;
                            }
                            Expr::StarArg(inner, _) => {
                                let const_idx = self.add_constant(Value::string("*"));
                                self.emit_op(OpCode::Constant, *span);
                                self.emit_u16(const_idx as u16, *span);
                                self.compile_expression(inner)?;
                                kw_count += 1;
                            }
                            Expr::DoubleStarArg(inner, _) => {
                                let const_idx = self.add_constant(Value::string("**"));
                                self.emit_op(OpCode::Constant, *span);
                                self.emit_u16(const_idx as u16, *span);
                                self.compile_expression(inner)?;
                                kw_count += 1;
                            }
                            _ => {
                                self.compile_expression(arg)?;
                                pos_count += 1;
                            }
                        }
                    }
                    self.emit_op(OpCode::CallKw, *span);
                    self.emit_byte(pos_count as u8, *span);
                    self.emit_byte(kw_count as u8, *span);
                }
            }
            Expr::Pipe(left, right, span) => {
                // Pipeline operator: left |> right
                // If right is Call(callee, args), transform into callee(left, args...)
                // If right is Ident/Expression, transform into right(left)
                match right.as_ref() {
                    Expr::Call(callee, args, _call_span) => {
                        self.compile_expression(callee)?;
                        // Pass left as the first argument
                        self.compile_expression(left)?;
                        for arg in args {
                            self.compile_expression(arg)?;
                        }
                        self.emit_op(OpCode::Call, *span);
                        self.emit_byte((args.len() + 1) as u8, *span);
                    }
                    _ => {
                        self.compile_expression(right)?;
                        self.compile_expression(left)?;
                        self.emit_op(OpCode::Call, *span);
                        self.emit_byte(1, *span);
                    }
                }
            }
            Expr::Array(elements, span) => {
                for el in elements {
                    self.compile_expression(el)?;
                }
                self.emit_op(OpCode::BuildArray, *span);
                self.emit_u16(elements.len() as u16, *span);
            }
            Expr::Tuple(elements, span) => {
                for el in elements {
                    self.compile_expression(el)?;
                }
                self.emit_op(OpCode::BuildTuple, *span);
                self.emit_u16(elements.len() as u16, *span);
            }
            Expr::Set(elements, span) => {
                for el in elements {
                    self.compile_expression(el)?;
                }
                self.emit_op(OpCode::BuildSet, *span);
                self.emit_u16(elements.len() as u16, *span);
            }
            Expr::Map(entries, span) => {
                for (k, v) in entries {
                    self.compile_expression(k)?;
                    self.compile_expression(v)?;
                }
                self.emit_op(OpCode::BuildMap, *span);
                self.emit_u16(entries.len() as u16, *span);
            }
            Expr::Index(obj, idx, span) => {
                self.compile_expression(obj)?;
                self.compile_expression(idx)?;
                self.emit_op(OpCode::IndexGet, *span);
            }
            Expr::Member(obj, member, span) => {
                self.compile_expression(obj)?;
                let const_idx = self.add_constant(Value::string(member.clone()));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);
                self.emit_op(OpCode::IndexGet, *span);
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                self.compile_expression(condition)?;
                let then_jump = self.emit_jump(OpCode::JumpIfFalse, *span);
                self.emit_op(OpCode::Pop, *span); // pop condition

                for st in &then_branch.statements {
                    self.compile_statement(st)?;
                }
                if let Some(res) = &then_branch.result {
                    self.compile_expression(res)?;
                } else {
                    self.emit_op(OpCode::Nil, *span);
                }

                let else_jump = self.emit_jump(OpCode::Jump, *span);
                self.patch_jump(then_jump);
                self.emit_op(OpCode::Pop, *span); // pop condition on false

                if let Some(eb) = else_branch {
                    for st in &eb.statements {
                        self.compile_statement(st)?;
                    }
                    if let Some(res) = &eb.result {
                        self.compile_expression(res)?;
                    } else {
                        self.emit_op(OpCode::Nil, *span);
                    }
                } else {
                    self.emit_op(OpCode::Nil, *span);
                }

                self.patch_jump(else_jump);
            }
            Expr::Match { target, arms, span } => {
                self.compile_expression(target)?;

                let mut end_jumps = Vec::new();
                let has_arms = !arms.is_empty();

                for (i, arm) in arms.iter().enumerate() {
                    let is_last = i == arms.len() - 1;

                    match &arm.pattern {
                        MatchPattern::Wildcard => {
                            self.emit_op(OpCode::Pop, arm.span);
                            self.compile_expression(&arm.body)?;
                            break;
                        }
                        MatchPattern::Ident(_) => {
                            self.emit_op(OpCode::Pop, arm.span);
                            self.compile_expression(&arm.body)?;
                            break;
                        }
                        MatchPattern::Literal(lit) => {
                            self.emit_op(OpCode::Dup, arm.span);
                            self.compile_literal(lit, arm.span);
                            self.emit_op(OpCode::Equal, arm.span);

                            let next_arm_jump = self.emit_jump(OpCode::JumpIfFalse, arm.span);
                            self.emit_op(OpCode::Pop, arm.span);
                            self.emit_op(OpCode::Pop, arm.span);

                            self.compile_expression(&arm.body)?;
                            end_jumps.push(self.emit_jump(OpCode::Jump, arm.span));

                            self.patch_jump(next_arm_jump);
                            self.emit_op(OpCode::Pop, arm.span);
                        }
                        MatchPattern::Range(start_lit, end_lit) => {
                            self.emit_op(OpCode::Dup, arm.span);
                            self.compile_literal(start_lit, arm.span);
                            self.emit_op(OpCode::GreaterEqual, arm.span);

                            let fail_start_jump = self.emit_jump(OpCode::JumpIfFalse, arm.span);
                            self.emit_op(OpCode::Pop, arm.span);

                            self.emit_op(OpCode::Dup, arm.span);
                            self.compile_literal(end_lit, arm.span);
                            self.emit_op(OpCode::LessEqual, arm.span);

                            let fail_end_jump = self.emit_jump(OpCode::JumpIfFalse, arm.span);
                            self.emit_op(OpCode::Pop, arm.span);
                            self.emit_op(OpCode::Pop, arm.span);

                            self.compile_expression(&arm.body)?;
                            end_jumps.push(self.emit_jump(OpCode::Jump, arm.span));

                            self.patch_jump(fail_start_jump);
                            self.patch_jump(fail_end_jump);
                            self.emit_op(OpCode::Pop, arm.span);
                        }
                    }

                    if is_last {
                        self.emit_op(OpCode::Pop, *span);
                        self.emit_op(OpCode::Nil, *span);
                    }
                }

                if !has_arms {
                    self.emit_op(OpCode::Pop, *span);
                    self.emit_op(OpCode::Nil, *span);
                }

                for jmp in end_jumps {
                    self.patch_jump(jmp);
                }
            }
            Expr::Block(block, span) => {
                self.begin_scope();
                let num_stmts = block.statements.len();
                let mut has_res = false;
                for (i, st) in block.statements.iter().enumerate() {
                    if i == num_stmts - 1 && block.result.is_none() {
                        if let Statement::Expr(expr) = st {
                            self.compile_expression(expr)?;
                            has_res = true;
                            break;
                        }
                    }
                    self.compile_statement(st)?;
                }
                if let Some(res) = &block.result {
                    self.compile_expression(res)?;
                } else if !has_res {
                    self.emit_op(OpCode::Nil, *span);
                }
                self.end_scope(*span);
            }
            Expr::NamedArg(_, val, _) => self.compile_expression(val)?,
            Expr::StarArg(inner, _) => self.compile_expression(inner)?,
            Expr::DoubleStarArg(inner, _) => self.compile_expression(inner)?,
            Expr::Lambda(params, body, span) => {
                let mut regular_params = Vec::new();
                let mut varargs_param = None;
                let mut kwargs_param = None;
                let mut min_arity = 0;

                for p in params {
                    if p.is_vararg {
                        varargs_param = Some(p.name.clone());
                    } else if p.is_kwarg {
                        kwargs_param = Some(p.name.clone());
                    } else {
                        if p.default_val.is_none() && varargs_param.is_none() {
                            min_arity += 1;
                        }
                        regular_params.push(p.clone());
                    }
                }

                let mut sub_compiler = BytecodeCompiler::new("<lambda>", min_arity);
                sub_compiler.function.max_arity = regular_params.len();
                sub_compiler.function.param_names = regular_params.iter().map(|p| p.name.clone()).collect();
                sub_compiler.function.has_varargs = varargs_param.is_some();
                sub_compiler.function.varargs_param = varargs_param.clone();
                sub_compiler.function.has_kwargs = kwargs_param.is_some();
                sub_compiler.function.kwargs_param = kwargs_param.clone();
                sub_compiler.scope_depth = 1;
                sub_compiler.parent_locals = Some(self.locals.clone());

                for param in &regular_params {
                    sub_compiler.add_local(param.name.clone(), *span)?;
                }
                if let Some(ref v_name) = varargs_param {
                    sub_compiler.add_local(v_name.clone(), *span)?;
                }
                if let Some(ref k_name) = kwargs_param {
                    sub_compiler.add_local(k_name.clone(), *span)?;
                }

                // Default argument initialization preamble
                for (i, p) in regular_params.iter().enumerate() {
                    if let Some(ref def_expr) = p.default_val {
                        let slot = i + 1;
                        sub_compiler.emit_op(OpCode::GetLocal, *span);
                        sub_compiler.emit_u16(slot as u16, *span);
                        let sent_idx = sub_compiler.add_constant(Value::DefaultSentinel);
                        sub_compiler.emit_op(OpCode::Constant, *span);
                        sub_compiler.emit_u16(sent_idx as u16, *span);
                        sub_compiler.emit_op(OpCode::Equal, *span);
                        let skip_jump = sub_compiler.emit_jump(OpCode::JumpIfFalse, *span);
                        sub_compiler.emit_op(OpCode::Pop, *span);
                        sub_compiler.compile_expression(def_expr)?;
                        sub_compiler.emit_op(OpCode::SetLocal, *span);
                        sub_compiler.emit_u16(slot as u16, *span);
                        sub_compiler.emit_op(OpCode::Pop, *span);
                        let end_jump = sub_compiler.emit_jump(OpCode::Jump, *span);
                        sub_compiler.patch_jump(skip_jump);
                        sub_compiler.emit_op(OpCode::Pop, *span);
                        sub_compiler.patch_jump(end_jump);
                    }
                }

                let num_stmts = body.statements.len();
                for (i, st) in body.statements.iter().enumerate() {
                    if i == num_stmts - 1 && body.result.is_none() {
                        if let Statement::Expr(expr) = st {
                            sub_compiler.compile_expression(expr)?;
                            sub_compiler.emit_op(OpCode::Return, *span);
                            continue;
                        }
                    }
                    sub_compiler.compile_statement(st)?;
                }

                if let Some(ref res) = body.result {
                    sub_compiler.compile_expression(res)?;
                    sub_compiler.emit_op(OpCode::Return, *span);
                } else if !sub_compiler.function.chunk.code.ends_with(&[OpCode::Return as u8]) {
                    sub_compiler.emit_op(OpCode::Nil, *span);
                    sub_compiler.emit_op(OpCode::Return, *span);
                }

                let captured_vars = sub_compiler.captured.clone();
                let compiled_fn = sub_compiler.function;
                let const_idx = self.add_constant(Value::Function(Arc::new(compiled_fn)));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);

                if !captured_vars.is_empty() {
                    for var_name in &captured_vars {
                        if let Some(slot) = self.resolve_local(var_name) {
                            self.emit_op(OpCode::GetLocal, *span);
                            self.emit_u16(slot as u16, *span);
                        } else {
                            return Err(format!("Failed to resolve captured local '{}'", var_name));
                        }
                    }
                    self.emit_op(OpCode::BuildClosure, *span);
                    self.emit_u16(captured_vars.len() as u16, *span);
                }
            }
            Expr::Super(span) => {
                let class_name = self.current_class_name.clone().ok_or_else(|| {
                    "super() can only be used inside a class method".to_string()
                })?;
                let self_slot = self.resolve_local("self").ok_or_else(|| {
                    "super() requires 'self' parameter in enclosing method".to_string()
                })?;
                self.emit_op(OpCode::GetLocal, *span);
                self.emit_u16(self_slot as u16, *span);

                let const_idx = self.add_constant(Value::string(class_name));
                self.emit_op(OpCode::Constant, *span);
                self.emit_u16(const_idx as u16, *span);

                self.emit_op(OpCode::Super, *span);
            }
            Expr::ListComp { element, item, iter, condition, span } => {
                let append_call = Expr::Call(
                    Box::new(Expr::Ident("append".to_string(), *span)),
                    vec![
                        Expr::Ident("__comp_arr".to_string(), *span),
                        *element.clone(),
                    ],
                    *span,
                );

                let loop_body_stmt = if let Some(cond) = condition {
                    Statement::Expr(Expr::If {
                        condition: Box::new(*cond.clone()),
                        then_branch: Block {
                            statements: vec![Statement::Expr(append_call)],
                            result: None,
                            span: *span,
                        },
                        else_branch: None,
                        span: *span,
                    })
                } else {
                    Statement::Expr(append_call)
                };

                let for_stmt = Statement::For {
                    item: item.clone(),
                    iter: Expr::Ident("__comp_iter".to_string(), *span),
                    body: Block {
                        statements: vec![loop_body_stmt],
                        result: None,
                        span: *span,
                    },
                    span: *span,
                };

                let init_stmt = Statement::Let {
                    name: "__comp_arr".to_string(),
                    is_mut: true,
                    type_annotation: None,
                    initializer: Some(Expr::Array(Vec::new(), *span)),
                    span: *span,
                };

                let lambda = Expr::Lambda(
                    vec![Param::new("__comp_iter")],
                    Box::new(Block {
                        statements: vec![init_stmt, for_stmt],
                        result: Some(Box::new(Expr::Ident("__comp_arr".to_string(), *span))),
                        span: *span,
                    }),
                    *span,
                );

                let call = Expr::Call(Box::new(lambda), vec![*iter.clone()], *span);
                self.compile_expression(&call)?;
            }
            Expr::DictComp { key, value, item, iter, condition, span } => {
                let assign_stmt = Statement::Assign {
                    target: Expr::Index(
                        Box::new(Expr::Ident("__comp_map".to_string(), *span)),
                        key.clone(),
                        *span,
                    ),
                    op: AssignOp::Assign,
                    value: *value.clone(),
                    span: *span,
                };

                let loop_body_stmt = if let Some(cond) = condition {
                    Statement::Expr(Expr::If {
                        condition: Box::new(*cond.clone()),
                        then_branch: Block {
                            statements: vec![assign_stmt],
                            result: None,
                            span: *span,
                        },
                        else_branch: None,
                        span: *span,
                    })
                } else {
                    assign_stmt
                };

                let for_stmt = Statement::For {
                    item: item.clone(),
                    iter: Expr::Ident("__comp_iter".to_string(), *span),
                    body: Block {
                        statements: vec![loop_body_stmt],
                        result: None,
                        span: *span,
                    },
                    span: *span,
                };

                let init_stmt = Statement::Let {
                    name: "__comp_map".to_string(),
                    is_mut: true,
                    type_annotation: None,
                    initializer: Some(Expr::Map(Vec::new(), *span)),
                    span: *span,
                };

                let lambda = Expr::Lambda(
                    vec![Param::new("__comp_iter")],
                    Box::new(Block {
                        statements: vec![init_stmt, for_stmt],
                        result: Some(Box::new(Expr::Ident("__comp_map".to_string(), *span))),
                        span: *span,
                    }),
                    *span,
                );

                let call = Expr::Call(Box::new(lambda), vec![*iter.clone()], *span);
                self.compile_expression(&call)?;
            }
        }
        Ok(())
    }

    fn compile_method(
        &mut self,
        class_name: &str,
        m_name: &str,
        params: &[Param],
        body: &Block,
        span: Span,
    ) -> Result<CompiledFunction, String> {
        let mut regular_params = Vec::new();
        let mut varargs_param = None;
        let mut kwargs_param = None;
        let mut min_arity = 0;

        for p in params {
            if p.is_vararg {
                varargs_param = Some(p.name.clone());
            } else if p.is_kwarg {
                kwargs_param = Some(p.name.clone());
            } else {
                if p.default_val.is_none() && varargs_param.is_none() {
                    min_arity += 1;
                }
                regular_params.push(p.clone());
            }
        }

        let full_name = format!("{}.{}", class_name, m_name);
        let mut sub_compiler = BytecodeCompiler::new(&full_name, min_arity);
        sub_compiler.current_class_name = Some(class_name.to_string());
        sub_compiler.function.max_arity = regular_params.len();
        sub_compiler.function.param_names = regular_params.iter().map(|p| p.name.clone()).collect();
        sub_compiler.function.has_varargs = varargs_param.is_some();
        sub_compiler.function.varargs_param = varargs_param.clone();
        sub_compiler.function.has_kwargs = kwargs_param.is_some();
        sub_compiler.function.kwargs_param = kwargs_param.clone();
        sub_compiler.scope_depth = 1;
        sub_compiler.parent_locals = Some(self.locals.clone());

        for param in &regular_params {
            sub_compiler.add_local(param.name.clone(), span)?;
        }
        if let Some(ref v_name) = varargs_param {
            sub_compiler.add_local(v_name.clone(), span)?;
        }
        if let Some(ref k_name) = kwargs_param {
            sub_compiler.add_local(k_name.clone(), span)?;
        }

        for (i, p) in regular_params.iter().enumerate() {
            if let Some(ref def_expr) = p.default_val {
                let slot = i + 1;
                sub_compiler.emit_op(OpCode::GetLocal, span);
                sub_compiler.emit_u16(slot as u16, span);
                let sent_idx = sub_compiler.add_constant(Value::DefaultSentinel);
                sub_compiler.emit_op(OpCode::Constant, span);
                sub_compiler.emit_u16(sent_idx as u16, span);
                sub_compiler.emit_op(OpCode::Equal, span);
                let skip_jump = sub_compiler.emit_jump(OpCode::JumpIfFalse, span);
                sub_compiler.emit_op(OpCode::Pop, span);

                sub_compiler.compile_expression(def_expr)?;
                sub_compiler.emit_op(OpCode::SetLocal, span);
                sub_compiler.emit_u16(slot as u16, span);
                sub_compiler.emit_op(OpCode::Pop, span);

                let done_jump = sub_compiler.emit_jump(OpCode::Jump, span);
                sub_compiler.patch_jump(skip_jump);
                sub_compiler.emit_op(OpCode::Pop, span);
                sub_compiler.patch_jump(done_jump);
            }
        }

        let num_stmts = body.statements.len();
        for (i, st) in body.statements.iter().enumerate() {
            if i == num_stmts - 1 && body.result.is_none() {
                if let Statement::Expr(expr) = st {
                    sub_compiler.compile_expression(expr)?;
                    sub_compiler.emit_op(OpCode::Return, span);
                    continue;
                }
            }
            sub_compiler.compile_statement(st)?;
        }

        if let Some(ref res) = body.result {
            sub_compiler.compile_expression(res)?;
            sub_compiler.emit_op(OpCode::Return, span);
        } else if !sub_compiler.function.chunk.code.ends_with(&[OpCode::Return as u8]) {
            sub_compiler.emit_op(OpCode::Nil, span);
            sub_compiler.emit_op(OpCode::Return, span);
        }

        Ok(sub_compiler.function)
    }

    fn compile_literal(&mut self, lit: &Literal, span: Span) {
        match lit {
            Literal::Int(v) => {
                let idx = self.add_constant(Value::Int(*v));
                self.emit_op(OpCode::Constant, span);
                self.emit_u16(idx as u16, span);
            }
            Literal::Float(v) => {
                let idx = self.add_constant(Value::Float(*v));
                self.emit_op(OpCode::Constant, span);
                self.emit_u16(idx as u16, span);
            }
            Literal::String(s) => {
                let idx = self.add_constant(Value::string(s.clone()));
                self.emit_op(OpCode::Constant, span);
                self.emit_u16(idx as u16, span);
            }
            Literal::Bool(true) => self.emit_op(OpCode::True, span),
            Literal::Bool(false) => self.emit_op(OpCode::False, span),
            Literal::Nil => self.emit_op(OpCode::Nil, span),
        }
    }

    // =========================================================================
    // Emission & Jump Helpers
    // =========================================================================

    fn emit_byte(&mut self, byte: u8, span: Span) {
        self.function.chunk.write_byte(byte, span);
    }

    fn emit_op(&mut self, op: OpCode, span: Span) {
        self.function.chunk.write_op(op, span);
    }

    fn emit_u16(&mut self, val: u16, span: Span) {
        self.function.chunk.write_u16(val, span);
    }

    fn add_constant(&mut self, val: Value) -> usize {
        self.function.chunk.add_constant(val)
    }

    fn emit_jump(&mut self, op: OpCode, span: Span) -> usize {
        self.emit_op(op, span);
        self.emit_byte(0xFF, span);
        self.emit_byte(0xFF, span);
        self.function.chunk.code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.function.chunk.code.len() - offset - 2;
        self.function.chunk.code[offset] = (jump >> 8) as u8;
        self.function.chunk.code[offset + 1] = (jump & 0xFF) as u8;
    }

    fn emit_loop(&mut self, loop_start: usize, span: Span) {
        self.emit_op(OpCode::Loop, span);
        let offset = self.function.chunk.code.len() - loop_start + 2;
        self.emit_u16(offset as u16, span);
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, span: Span) {
        self.scope_depth -= 1;
        while let Some(last) = self.locals.last() {
            if last.depth > self.scope_depth {
                self.emit_op(OpCode::Pop, span);
                self.locals.pop();
            } else {
                break;
            }
        }
    }

    fn add_local(&mut self, name: String, _span: Span) -> Result<(), String> {
        self.locals.push(Local {
            name,
            depth: self.scope_depth,
            is_captured: false,
        });
        Ok(())
    }

    fn resolve_local(&mut self, name: &str) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i);
            }
        }

        if let Some(ref parent_locals) = self.parent_locals {
            let in_parent = parent_locals.iter().any(|l| l.name == name && !l.name.is_empty());
            if in_parent {
                if let Some(pos) = self.captured.iter().position(|c| c == name) {
                    return Some(pos + 1);
                }
                self.captured.push(name.to_string());
                let slot = self.locals.len();
                self.locals.push(Local {
                    name: name.to_string(),
                    depth: self.scope_depth,
                    is_captured: true,
                });
                return Some(slot);
            }
        }

        None
    }
}
