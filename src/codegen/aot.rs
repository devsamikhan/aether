// ==============================================================================
// AetherAOT — Ahead-of-Time Native Machine Code Compiler
// Direct Native Machine Code Emission (.o / .obj / Binary) via Cranelift ObjectModule
// Zero Third-Party Crates (Pure Cranelift Pipeline)
// ==============================================================================

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use cranelift_codegen::ir::{
    types, AbiParam, Block as ClifBlock, BlockArg, InstBuilder, Value as ClifValue,
    condcodes::IntCC,
};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::syntax::ast::*;
use crate::syntax::parse;

// ==============================================================================
// 1. Variable Scope Tracker
// ==============================================================================

struct AotVarScope {
    vars: HashMap<String, Variable>,
}

impl AotVarScope {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    fn declare_var(&mut self, name: &str, builder: &mut FunctionBuilder) -> Variable {
        if let Some(&var) = self.vars.get(name) {
            return var;
        }
        let var = builder.declare_var(types::I64);
        self.vars.insert(name.to_string(), var);
        var
    }

    fn get_var(&self, name: &str) -> Option<Variable> {
        self.vars.get(name).copied()
    }
}

// ==============================================================================
// 2. Aether AOT Native Machine Code Compiler
// ==============================================================================

pub struct AotCompiler {
    module: ObjectModule,
    func_ids: HashMap<String, FuncId>,
    loop_blocks: Vec<(ClifBlock, ClifBlock)>,
}

impl AotCompiler {
    pub fn new(module_name: &str) -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "true").map_err(|e| e.to_string())?;
        flag_builder.set("opt_level", "speed").map_err(|e| e.to_string())?;

        let isa_builder = cranelift_native::builder().map_err(|e| e.to_string())?;
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).map_err(|e| e.to_string())?;

        let builder = ObjectBuilder::new(isa, module_name, cranelift_module::default_libcall_names())
            .map_err(|e| e.to_string())?;
        let module = ObjectModule::new(builder);

        Ok(Self {
            module,
            func_ids: HashMap::new(),
            loop_blocks: Vec::new(),
        })
    }

    /// Compiles an entire AST program into the native object module
    pub fn compile_program(&mut self, program: &Program) -> Result<(), String> {
        // 1. Forward-declare all top-level functions
        for stmt in &program.statements {
            match stmt {
                Statement::FnDef { name, params, .. } => {
                    let mut sig = self.module.make_signature();
                    for _ in params {
                        sig.params.push(AbiParam::new(types::I64));
                    }
                    sig.returns.push(AbiParam::new(types::I64));
                    let func_id = self.module.declare_function(name, Linkage::Export, &sig)
                        .map_err(|e| e.to_string())?;
                    self.func_ids.insert(name.clone(), func_id);
                }
                Statement::IntentDef { name, params, .. } => {
                    let mut sig = self.module.make_signature();
                    for _ in params {
                        sig.params.push(AbiParam::new(types::I64));
                    }
                    sig.returns.push(AbiParam::new(types::I64));
                    let func_id = self.module.declare_function(name, Linkage::Export, &sig)
                        .map_err(|e| e.to_string())?;
                    self.func_ids.insert(name.clone(), func_id);
                }
                _ => {}
            }
        }

        let mut ctx = self.module.make_context();
        let mut fn_builder_ctx = FunctionBuilderContext::new();

        // 2. Compile each defined function
        for stmt in &program.statements {
            match stmt {
                Statement::FnDef { name, params, body, .. } => {
                    self.compile_fn(name, params, body, &mut ctx, &mut fn_builder_ctx)?;
                }
                Statement::IntentDef { name, params, body, .. } => {
                    self.compile_fn(name, params, body, &mut ctx, &mut fn_builder_ctx)?;
                }
                _ => {}
            }
        }

        // 3. If there are top-level statements that aren't functions, synthesize an "aether_main" entry function
        let top_level_stmts: Vec<&Statement> = program.statements.iter().filter(|s| {
            !matches!(s, Statement::FnDef { .. } | Statement::IntentDef { .. } | Statement::ClassDef { .. } | Statement::StructDef { .. })
        }).collect();

        if !top_level_stmts.is_empty() {
            let mut sig = self.module.make_signature();
            sig.returns.push(AbiParam::new(types::I64));
            let main_id = self.module.declare_function("aether_main", Linkage::Export, &sig)
                .map_err(|e| e.to_string())?;
            self.func_ids.insert("aether_main".to_string(), main_id);

            ctx.func.signature = sig;
            let target_config = self.module.target_config();
            {
                let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_builder_ctx);
                let entry_block = builder.create_block();
                builder.append_block_params_for_function_params(entry_block);
                builder.switch_to_block(entry_block);
                builder.seal_block(entry_block);

                let mut scope = AotVarScope::new();
                let mut last_val = builder.ins().iconst(types::I64, 0);
                let mut returned = false;

                for s in top_level_stmts {
                    let (val_opt, ret) = self.compile_stmt(s, &mut builder, &mut scope)?;
                    if let Some(v) = val_opt {
                        last_val = v;
                    }
                    if ret {
                        returned = true;
                        break;
                    }
                }

                if !returned {
                    builder.ins().return_(&[last_val]);
                }
                builder.finalize(target_config);
            }

            self.module.define_function(main_id, &mut ctx).map_err(|e| e.to_string())?;
            self.module.clear_context(&mut ctx);
        }

        Ok(())
    }

    fn compile_fn(
        &mut self,
        name: &str,
        params: &[Param],
        body: &Block,
        ctx: &mut cranelift_codegen::Context,
        fn_builder_ctx: &mut FunctionBuilderContext,
    ) -> Result<(), String> {
        let func_id = *self.func_ids.get(name).ok_or_else(|| format!("Function '{}' not declared", name))?;

        for _ in params {
            ctx.func.signature.params.push(AbiParam::new(types::I64));
        }
        ctx.func.signature.returns.push(AbiParam::new(types::I64));

        let target_config = self.module.target_config();
        {
            let mut builder = FunctionBuilder::new(&mut ctx.func, fn_builder_ctx);
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            let mut scope = AotVarScope::new();

            for (i, param) in params.iter().enumerate() {
                let p_val = builder.block_params(entry_block)[i];
                let var = scope.declare_var(&param.name, &mut builder);
                builder.def_var(var, p_val);
            }

            let mut last_val = None;
            let mut returned = false;

            for stmt in &body.statements {
                let (val_opt, ret) = self.compile_stmt(stmt, &mut builder, &mut scope)?;
                if let Some(v) = val_opt {
                    last_val = Some(v);
                }
                if ret {
                    returned = true;
                    break;
                }
            }

            if !returned {
                if let Some(res_expr) = &body.result {
                    let r = self.compile_expr(res_expr, &mut builder, &mut scope)?;
                    last_val = Some(r);
                }

                let final_res = last_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                builder.ins().return_(&[final_res]);
            }

            builder.finalize(target_config);
        }

        self.module.define_function(func_id, ctx).map_err(|e| e.to_string())?;
        self.module.clear_context(ctx);

        Ok(())
    }

    fn compile_stmt(
        &mut self,
        stmt: &Statement,
        builder: &mut FunctionBuilder,
        scope: &mut AotVarScope,
    ) -> Result<(Option<ClifValue>, bool), String> {
        match stmt {
            Statement::Let { name, initializer, .. } => {
                let init_val = if let Some(init) = initializer {
                    self.compile_expr(init, builder, scope)?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };

                let var = scope.declare_var(name, builder);
                builder.def_var(var, init_val);
                Ok((Some(init_val), false))
            }
            Statement::Assign { target, op, value, .. } => {
                let rhs = self.compile_expr(value, builder, scope)?;
                match target {
                    Expr::Ident(name, _) => {
                        let var = scope.get_var(name).ok_or_else(|| format!("AOT: Variable '{}' not declared", name))?;
                        let new_val = match op {
                            AssignOp::Assign => rhs,
                            AssignOp::AddAssign => {
                                let curr = builder.use_var(var);
                                builder.ins().iadd(curr, rhs)
                            }
                            AssignOp::SubAssign => {
                                let curr = builder.use_var(var);
                                builder.ins().isub(curr, rhs)
                            }
                            AssignOp::MulAssign => {
                                let curr = builder.use_var(var);
                                builder.ins().imul(curr, rhs)
                            }
                            AssignOp::DivAssign => {
                                let curr = builder.use_var(var);
                                builder.ins().sdiv(curr, rhs)
                            }
                            AssignOp::ModAssign => {
                                let curr = builder.use_var(var);
                                builder.ins().srem(curr, rhs)
                            }
                        };
                        builder.def_var(var, new_val);
                        Ok((Some(new_val), false))
                    }
                    _ => Err("Complex assignment targets not yet supported in AOT".to_string()),
                }
            }
            Statement::Expr(expr) => {
                let v = self.compile_expr(expr, builder, scope)?;
                Ok((Some(v), false))
            }
            Statement::Return { value, .. } => {
                let ret_val = if let Some(v_expr) = value {
                    self.compile_expr(v_expr, builder, scope)?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                builder.ins().return_(&[ret_val]);
                Ok((Some(ret_val), true))
            }
            Statement::While { condition, body, .. } => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                let no_args: [BlockArg; 0] = [];
                builder.ins().jump(header_block, &no_args);
                builder.switch_to_block(header_block);

                let cond_val = self.compile_expr(condition, builder, scope)?;
                builder.ins().brif(cond_val, body_block, &no_args, exit_block, &no_args);

                self.loop_blocks.push((header_block, exit_block));
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);

                for s in &body.statements {
                    let (_, ret) = self.compile_stmt(s, builder, scope)?;
                    if ret { break; }
                }

                builder.ins().jump(header_block, &no_args);
                self.loop_blocks.pop();

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);

                Ok((None, false))
            }
            Statement::Break(_) => {
                if let Some((_, exit_block)) = self.loop_blocks.last() {
                    let no_args: [BlockArg; 0] = [];
                    builder.ins().jump(*exit_block, &no_args);
                    Ok((None, true))
                } else {
                    Err("AOT: break statement outside of loop".to_string())
                }
            }
            Statement::Continue(_) => {
                if let Some((header_block, _)) = self.loop_blocks.last() {
                    let no_args: [BlockArg; 0] = [];
                    builder.ins().jump(*header_block, &no_args);
                    Ok((None, true))
                } else {
                    Err("AOT: continue statement outside of loop".to_string())
                }
            }
            _ => Ok((None, false)),
        }
    }

    fn compile_expr(
        &mut self,
        expr: &Expr,
        builder: &mut FunctionBuilder,
        scope: &mut AotVarScope,
    ) -> Result<ClifValue, String> {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(n) => Ok(builder.ins().iconst(types::I64, *n)),
                Literal::Float(f) => Ok(builder.ins().iconst(types::I64, *f as i64)),
                Literal::Bool(b) => Ok(builder.ins().iconst(types::I64, if *b { 1 } else { 0 })),
                Literal::Nil => Ok(builder.ins().iconst(types::I64, 0)),
                Literal::String(_) => Ok(builder.ins().iconst(types::I64, 0)),
            },
            Expr::Ident(name, _) => {
                if let Some(var) = scope.get_var(name) {
                    Ok(builder.use_var(var))
                } else {
                    Err(format!("AOT: Variable '{}' not declared in scope", name))
                }
            }
            Expr::Binary(left, op, right, _) => {
                let l_val = self.compile_expr(left, builder, scope)?;
                let r_val = self.compile_expr(right, builder, scope)?;

                match op {
                    BinaryOp::Add => Ok(builder.ins().iadd(l_val, r_val)),
                    BinaryOp::Sub => Ok(builder.ins().isub(l_val, r_val)),
                    BinaryOp::Mul => Ok(builder.ins().imul(l_val, r_val)),
                    BinaryOp::Div => Ok(builder.ins().sdiv(l_val, r_val)),
                    BinaryOp::Mod => Ok(builder.ins().srem(l_val, r_val)),
                    BinaryOp::Equal => {
                        let cmp = builder.ins().icmp(IntCC::Equal, l_val, r_val);
                        Ok(builder.ins().uextend(types::I64, cmp))
                    }
                    BinaryOp::NotEqual => {
                        let cmp = builder.ins().icmp(IntCC::NotEqual, l_val, r_val);
                        Ok(builder.ins().uextend(types::I64, cmp))
                    }
                    BinaryOp::Less => {
                        let cmp = builder.ins().icmp(IntCC::SignedLessThan, l_val, r_val);
                        Ok(builder.ins().uextend(types::I64, cmp))
                    }
                    BinaryOp::LessEqual => {
                        let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, l_val, r_val);
                        Ok(builder.ins().uextend(types::I64, cmp))
                    }
                    BinaryOp::Greater => {
                        let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, l_val, r_val);
                        Ok(builder.ins().uextend(types::I64, cmp))
                    }
                    BinaryOp::GreaterEqual => {
                        let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l_val, r_val);
                        Ok(builder.ins().uextend(types::I64, cmp))
                    }
                    BinaryOp::And => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        let l_bool = builder.ins().icmp(IntCC::NotEqual, l_val, zero);
                        let r_bool = builder.ins().icmp(IntCC::NotEqual, r_val, zero);
                        let and_res = builder.ins().band(l_bool, r_bool);
                        Ok(builder.ins().uextend(types::I64, and_res))
                    }
                    BinaryOp::Or => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        let l_bool = builder.ins().icmp(IntCC::NotEqual, l_val, zero);
                        let r_bool = builder.ins().icmp(IntCC::NotEqual, r_val, zero);
                        let or_res = builder.ins().bor(l_bool, r_bool);
                        Ok(builder.ins().uextend(types::I64, or_res))
                    }
                    _ => Ok(builder.ins().iadd(l_val, r_val)),
                }
            }
            Expr::Call(callee, args, _) => {
                let fn_name = match callee.as_ref() {
                    Expr::Ident(s, _) => s.as_str(),
                    _ => return Err("AOT: Computed function pointers not yet supported".to_string()),
                };

                let func_id = *self.func_ids.get(fn_name)
                    .ok_or_else(|| format!("AOT: Function '{}' not defined", fn_name))?;

                let local_func = self.module.declare_func_in_func(func_id, builder.func);
                let mut arg_vals = Vec::new();
                for arg in args {
                    let av = self.compile_expr(arg, builder, scope)?;
                    arg_vals.push(av);
                }

                let call_inst = builder.ins().call(local_func, &arg_vals);
                let res = builder.inst_results(call_inst)[0];
                Ok(res)
            }
            _ => Ok(builder.ins().iconst(types::I64, 0)),
        }
    }

    /// Emits native object file bytes (.o / .obj) directly from Cranelift
    pub fn emit_object(self) -> Result<Vec<u8>, String> {
        let product = self.module.finish();
        let bytes = product.emit().map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}

// ==============================================================================
// 3. High-Level AOT Compilation Helper Functions
// ==============================================================================

/// Compiles Aether source code into native machine code object bytes (`Vec<u8>`)
pub fn compile_source_to_object(source: &str, module_name: &str) -> Result<Vec<u8>, String> {
    let program = parse(source).map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    let mut compiler = AotCompiler::new(module_name)?;
    compiler.compile_program(&program)?;
    compiler.emit_object()
}

/// Compiles an Aether source file into a native object file on disk (.obj / .o)
pub fn compile_file_to_object(
    source_path: &Path,
    output_path: &Path,
) -> Result<usize, String> {
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read source file '{}': {}", source_path.display(), e))?;

    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("aether_module");

    let bytes = compile_source_to_object(&source, stem)?;
    fs::write(output_path, &bytes)
        .map_err(|e| format!("Failed to write object file '{}': {}", output_path.display(), e))?;

    Ok(bytes.len())
}
