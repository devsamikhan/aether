use std::collections::HashMap;
use cranelift_codegen::ir::{
    types, AbiParam, Block as ClifBlock, BlockArg, InstBuilder, MemFlagsData, Value as ClifValue,
    condcodes::IntCC,
};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};

use crate::syntax::ast::*;

// =========================================================================
// Native C-ABI Runtime Bridge Functions
// =========================================================================

pub extern "C" fn aether_native_alloc_buffer(size: i64) -> i64 {
    let count = size.max(1) as usize;
    let layout = match std::alloc::Layout::array::<i64>(count) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
    ptr as i64
}

pub extern "C" fn aether_native_free_buffer(ptr: i64, size: i64) {
    if ptr != 0 && size > 0 {
        if let Ok(layout) = std::alloc::Layout::array::<i64>(size as usize) {
            unsafe { std::alloc::dealloc(ptr as *mut u8, layout) };
        }
    }
}

pub extern "C" fn aether_native_print_i64(val: i64) {
    print!("{}", val);
}

pub extern "C" fn aether_native_println_i64(val: i64) {
    println!("{}", val);
}

pub extern "C" fn aether_native_print_str(ptr: *const u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        if let Ok(s) = std::str::from_utf8(slice) {
            print!("{}", s);
        }
    }
}

pub extern "C" fn aether_native_println_str(ptr: *const u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        if let Ok(s) = std::str::from_utf8(slice) {
            println!("{}", s);
        }
    } else {
        println!();
    }
}

pub extern "C" fn aether_native_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub extern "C" fn aether_native_contract_violation(contract_type: i64, line: i64) {
    let name = if contract_type == 0 { "require" } else { "ensure" };
    eprintln!("\n[CONTRACT VIOLATION] Intent '{}' constraint failed at line {}", name, line);
    std::process::exit(101);
}

// =========================================================================
// Cranelift Native Compiler
// =========================================================================

pub struct CraneliftCompiler {
    module: JITModule,
    func_ids: HashMap<String, FuncId>,
    string_data: HashMap<String, (DataId, usize)>,
    str_counter: usize,
    loop_blocks: Vec<(ClifBlock, ClifBlock)>,
}

impl CraneliftCompiler {
    pub fn new() -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").map_err(|e| e.to_string())?;
        flag_builder.set("is_pic", "false").map_err(|e| e.to_string())?;
        flag_builder.set("opt_level", "speed").map_err(|e| e.to_string())?;

        let isa_builder = cranelift_native::builder().map_err(|e| e.to_string())?;
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).map_err(|e| e.to_string())?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register runtime symbols
        builder.symbol("aether_native_print_i64", aether_native_print_i64 as *const u8);
        builder.symbol("aether_native_println_i64", aether_native_println_i64 as *const u8);
        builder.symbol("aether_native_print_str", aether_native_print_str as *const u8);
        builder.symbol("aether_native_println_str", aether_native_println_str as *const u8);
        builder.symbol("aether_native_time_ms", aether_native_time_ms as *const u8);
        builder.symbol("aether_native_contract_violation", aether_native_contract_violation as *const u8);
        builder.symbol("aether_native_alloc_buffer", aether_native_alloc_buffer as *const u8);
        builder.symbol("aether_native_free_buffer", aether_native_free_buffer as *const u8);

        let module = JITModule::new(builder);

        let mut compiler = Self {
            module,
            func_ids: HashMap::new(),
            string_data: HashMap::new(),
            str_counter: 0,
            loop_blocks: Vec::new(),
        };

        compiler.register_runtime_declarations()?;
        Ok(compiler)
    }

    fn register_runtime_declarations(&mut self) -> Result<(), String> {
        let mut sig_i64 = self.module.make_signature();
        sig_i64.params.push(AbiParam::new(types::I64));
        let fid = self.module.declare_function("aether_native_print_i64", Linkage::Import, &sig_i64)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_print_i64".to_string(), fid);

        let fid2 = self.module.declare_function("aether_native_println_i64", Linkage::Import, &sig_i64)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_println_i64".to_string(), fid2);

        let mut sig_str = self.module.make_signature();
        sig_str.params.push(AbiParam::new(types::I64));
        sig_str.params.push(AbiParam::new(types::I64));
        let fid3 = self.module.declare_function("aether_native_print_str", Linkage::Import, &sig_str)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_print_str".to_string(), fid3);

        let fid4 = self.module.declare_function("aether_native_println_str", Linkage::Import, &sig_str)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_println_str".to_string(), fid4);

        let mut sig_time = self.module.make_signature();
        sig_time.returns.push(AbiParam::new(types::I64));
        let fid5 = self.module.declare_function("aether_native_time_ms", Linkage::Import, &sig_time)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_time_ms".to_string(), fid5);

        let mut sig_viol = self.module.make_signature();
        sig_viol.params.push(AbiParam::new(types::I64));
        sig_viol.params.push(AbiParam::new(types::I64));
        let fid6 = self.module.declare_function("aether_native_contract_violation", Linkage::Import, &sig_viol)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_contract_violation".to_string(), fid6);

        let mut sig_alloc = self.module.make_signature();
        sig_alloc.params.push(AbiParam::new(types::I64));
        sig_alloc.returns.push(AbiParam::new(types::I64));
        let fid7 = self.module.declare_function("aether_native_alloc_buffer", Linkage::Import, &sig_alloc)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_alloc_buffer".to_string(), fid7);

        let mut sig_free = self.module.make_signature();
        sig_free.params.push(AbiParam::new(types::I64));
        sig_free.params.push(AbiParam::new(types::I64));
        let fid8 = self.module.declare_function("aether_native_free_buffer", Linkage::Import, &sig_free)
            .map_err(|e| e.to_string())?;
        self.func_ids.insert("aether_native_free_buffer".to_string(), fid8);

        Ok(())
    }

    pub fn get_or_create_string(&mut self, s: &str) -> Result<(DataId, usize), String> {
        if let Some(entry) = self.string_data.get(s) {
            return Ok(*entry);
        }
        let sym_name = format!("__str_const_{}", self.str_counter);
        self.str_counter += 1;

        let mut desc = DataDescription::new();
        desc.define(s.as_bytes().to_vec().into_boxed_slice());
        let data_id = self.module.declare_data(&sym_name, Linkage::Local, true, false)
            .map_err(|e| e.to_string())?;
        self.module.define_data(data_id, &desc).map_err(|e| e.to_string())?;

        let entry = (data_id, s.len());
        self.string_data.insert(s.to_string(), entry);
        Ok(entry)
    }

    pub fn compile_and_run(&mut self, program: &Program) -> Result<i64, String> {
        for stmt in &program.statements {
            match stmt {
                Statement::FnDef { name, params, .. } => {
                    let mut sig = self.module.make_signature();
                    for _ in params {
                        sig.params.push(AbiParam::new(types::I64));
                    }
                    sig.returns.push(AbiParam::new(types::I64));
                    let func_id = self.module.declare_function(name, Linkage::Local, &sig)
                        .map_err(|e| e.to_string())?;
                    self.func_ids.insert(name.clone(), func_id);
                }
                Statement::IntentDef { name, params, .. } => {
                    let mut sig = self.module.make_signature();
                    for _ in params {
                        sig.params.push(AbiParam::new(types::I64));
                    }
                    sig.returns.push(AbiParam::new(types::I64));
                    let func_id = self.module.declare_function(name, Linkage::Local, &sig)
                        .map_err(|e| e.to_string())?;
                    self.func_ids.insert(name.clone(), func_id);
                }
                _ => {}
            }
        }

        let mut ctx = self.module.make_context();
        let mut fn_builder_ctx = FunctionBuilderContext::new();

        for stmt in &program.statements {
            match stmt {
                Statement::FnDef { name, params, body, .. } => {
                    self.compile_fn(name, params, &[], &[], body, &mut ctx, &mut fn_builder_ctx)?;
                }
                Statement::IntentDef { name, params, require, ensure, body, .. } => {
                    self.compile_fn(name, params, require, ensure, body, &mut ctx, &mut fn_builder_ctx)?;
                }
                _ => {}
            }
        }

        let main_func_id = {
            let mut sig = self.module.make_signature();
            sig.returns.push(AbiParam::new(types::I64));
            self.module.declare_function("aether_main", Linkage::Export, &sig)
                .map_err(|e| e.to_string())?
        };

        ctx.func.signature.returns.push(AbiParam::new(types::I64));
        let target_config = self.module.target_config();
        {
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_builder_ctx);
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            let mut var_scope = VarScope::new();
            let mut last_val = builder.ins().iconst(types::I64, 0);
            let mut returned = false;

            for stmt in &program.statements {
                match stmt {
                    Statement::FnDef { .. } | Statement::IntentDef { .. } => {}
                    _ => {
                        let (val_opt, ret) = self.compile_stmt(stmt, &mut builder, &mut var_scope)?;
                        if let Some(v) = val_opt {
                            last_val = v;
                        }
                        if ret {
                            returned = true;
                            break;
                        }
                    }
                }
            }

            if !returned {
                builder.ins().return_(&[last_val]);
            }
            builder.finalize(target_config);
        }

        self.module.define_function(main_func_id, &mut ctx).map_err(|e| e.to_string())?;
        self.module.clear_context(&mut ctx);

        self.module.finalize_definitions().map_err(|e| e.to_string())?;

        let code_ptr = self.module.get_finalized_function(main_func_id);
        let main_fn: extern "C" fn() -> i64 = unsafe { std::mem::transmute(code_ptr) };
        let exit_code = main_fn();

        Ok(exit_code)
    }

    fn compile_fn(
        &mut self,
        name: &str,
        params: &[Param],
        require: &[Expr],
        ensure: &[Expr],
        body: &Block,
        ctx: &mut cranelift_codegen::Context,
        fn_builder_ctx: &mut FunctionBuilderContext,
    ) -> Result<(), String> {
        let func_id = *self.func_ids.get(name).unwrap();

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

            let mut var_scope = VarScope::new();

            for (i, param) in params.iter().enumerate() {
                let p_val = builder.block_params(entry_block)[i];
                let var = var_scope.declare_var(&param.name, &mut builder);
                builder.def_var(var, p_val);
            }

            let no_args: [BlockArg; 0] = [];

            for req_expr in require {
                let cond_val = self.compile_expr(req_expr, &mut builder, &mut var_scope)?;
                let ok_block = builder.create_block();
                let fail_block = builder.create_block();
                builder.ins().brif(cond_val, ok_block, &no_args, fail_block, &no_args);

                builder.switch_to_block(fail_block);
                builder.seal_block(fail_block);
                let zero = builder.ins().iconst(types::I64, 0);
                let line = builder.ins().iconst(types::I64, req_expr.span().line as i64);
                let viol_fid = *self.func_ids.get("aether_native_contract_violation").unwrap();
                let local_viol = self.module.declare_func_in_func(viol_fid, &mut builder.func);
                builder.ins().call(local_viol, &[zero, line]);
                builder.ins().return_(&[zero]);

                builder.switch_to_block(ok_block);
                builder.seal_block(ok_block);
            }

            let mut last_val = None;
            let mut returned = false;

            for stmt in &body.statements {
                let (val_opt, ret) = self.compile_stmt(stmt, &mut builder, &mut var_scope)?;
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
                    let r = self.compile_expr(res_expr, &mut builder, &mut var_scope)?;
                    last_val = Some(r);
                }

                let final_res = last_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));

                for ens_expr in ensure {
                    let res_var = var_scope.declare_var("result", &mut builder);
                    builder.def_var(res_var, final_res);

                    let cond_val = self.compile_expr(ens_expr, &mut builder, &mut var_scope)?;
                    let ok_block = builder.create_block();
                    let fail_block = builder.create_block();
                    builder.ins().brif(cond_val, ok_block, &no_args, fail_block, &no_args);

                    builder.switch_to_block(fail_block);
                    builder.seal_block(fail_block);
                    let one = builder.ins().iconst(types::I64, 1);
                    let line = builder.ins().iconst(types::I64, ens_expr.span().line as i64);
                    let viol_fid = *self.func_ids.get("aether_native_contract_violation").unwrap();
                    let local_viol = self.module.declare_func_in_func(viol_fid, &mut builder.func);
                    builder.ins().call(local_viol, &[one, line]);
                    builder.ins().return_(&[final_res]);

                    builder.switch_to_block(ok_block);
                    builder.seal_block(ok_block);
                }

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
        scope: &mut VarScope,
    ) -> Result<(Option<ClifValue>, bool), String> {
        let no_args: [BlockArg; 0] = [];

        match stmt {
            Statement::Let { name, initializer, .. } => {
                let init_val = if let Some(init) = initializer {
                    self.compile_expr(init, builder, scope)?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                let var = scope.declare_var(name, builder);
                builder.def_var(var, init_val);
                Ok((None, false))
            }
            Statement::Assign { target, op, value, .. } => {
                let rhs_val = self.compile_expr(value, builder, scope)?;
                match target {
                    Expr::Ident(name, _) => {
                        let var = match scope.get_var(name) {
                            Some(v) => v,
                            None => {
                                if *op == AssignOp::Assign {
                                    scope.declare_var(name, builder)
                                } else {
                                    return Err(format!("Undefined variable '{}'", name));
                                }
                            }
                        };
                        let final_val = match op {
                            AssignOp::Assign => rhs_val,
                            AssignOp::AddAssign => {
                                let cur = builder.use_var(var);
                                builder.ins().iadd(cur, rhs_val)
                            }
                            AssignOp::SubAssign => {
                                let cur = builder.use_var(var);
                                builder.ins().isub(cur, rhs_val)
                            }
                            AssignOp::MulAssign => {
                                let cur = builder.use_var(var);
                                builder.ins().imul(cur, rhs_val)
                            }
                            AssignOp::DivAssign => {
                                let cur = builder.use_var(var);
                                builder.ins().sdiv(cur, rhs_val)
                            }
                            AssignOp::ModAssign => {
                                let cur = builder.use_var(var);
                                builder.ins().srem(cur, rhs_val)
                            }
                        };
                        builder.def_var(var, final_val);
                    }
                    Expr::Index(obj, idx, _) => {
                        let obj_ptr = self.compile_expr(obj, builder, scope)?;
                        let idx_val = self.compile_expr(idx, builder, scope)?;
                        let eight = builder.ins().iconst(types::I64, 8);
                        let offset = builder.ins().imul(idx_val, eight);
                        let addr = builder.ins().iadd(obj_ptr, offset);

                        let final_val = match op {
                            AssignOp::Assign => rhs_val,
                            AssignOp::AddAssign => {
                                let cur = builder.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
                                builder.ins().iadd(cur, rhs_val)
                            }
                            AssignOp::SubAssign => {
                                let cur = builder.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
                                builder.ins().isub(cur, rhs_val)
                            }
                            AssignOp::MulAssign => {
                                let cur = builder.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
                                builder.ins().imul(cur, rhs_val)
                            }
                            AssignOp::DivAssign => {
                                let cur = builder.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
                                builder.ins().sdiv(cur, rhs_val)
                            }
                            AssignOp::ModAssign => {
                                let cur = builder.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
                                builder.ins().srem(cur, rhs_val)
                            }
                        };
                        builder.ins().store(MemFlagsData::trusted(), final_val, addr, 0);
                    }
                    _ => {}
                }
                Ok((None, false))
            }
            Statement::While { condition, body, .. } => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                builder.ins().jump(header_block, &no_args);
                builder.switch_to_block(header_block);

                let cond_val = self.compile_expr(condition, builder, scope)?;
                builder.ins().brif(cond_val, body_block, &no_args, exit_block, &no_args);

                builder.seal_block(body_block);
                builder.switch_to_block(body_block);

                self.loop_blocks.push((header_block, exit_block));
                let mut body_returned = false;
                for s in &body.statements {
                    let (_, ret) = self.compile_stmt(s, builder, scope)?;
                    if ret {
                        body_returned = true;
                        break;
                    }
                }
                self.loop_blocks.pop();

                if !body_returned {
                    builder.ins().jump(header_block, &no_args);
                }

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);

                Ok((None, false))
            }
            Statement::For { item, iter, body, .. } => {
                let (start_val, stop_val, step_val) = match iter {
                    Expr::Call(callee, args, _) if matches!(callee.as_ref(), Expr::Ident(name, _) if name == "range") => {
                        match args.len() {
                            1 => {
                                let start = builder.ins().iconst(types::I64, 0);
                                let stop = self.compile_expr(&args[0], builder, scope)?;
                                let step = builder.ins().iconst(types::I64, 1);
                                (start, stop, step)
                            }
                            2 => {
                                let start = self.compile_expr(&args[0], builder, scope)?;
                                let stop = self.compile_expr(&args[1], builder, scope)?;
                                let step = builder.ins().iconst(types::I64, 1);
                                (start, stop, step)
                            }
                            3 => {
                                let start = self.compile_expr(&args[0], builder, scope)?;
                                let stop = self.compile_expr(&args[1], builder, scope)?;
                                let step = self.compile_expr(&args[2], builder, scope)?;
                                (start, stop, step)
                            }
                            _ => {
                                let zero = builder.ins().iconst(types::I64, 0);
                                (zero, zero, zero)
                            }
                        }
                    }
                    _ => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        (zero, zero, zero)
                    }
                };

                let item_var = scope.declare_var(item, builder);
                builder.def_var(item_var, start_val);

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let step_block = builder.create_block();
                let exit_block = builder.create_block();

                builder.ins().jump(header_block, &no_args);
                builder.switch_to_block(header_block);

                let cur_item = builder.use_var(item_var);
                let cond = builder.ins().icmp(IntCC::SignedLessThan, cur_item, stop_val);
                builder.ins().brif(cond, body_block, &no_args, exit_block, &no_args);

                builder.seal_block(body_block);
                builder.switch_to_block(body_block);

                self.loop_blocks.push((step_block, exit_block));
                let mut body_returned = false;
                for s in &body.statements {
                    let (_, ret) = self.compile_stmt(s, builder, scope)?;
                    if ret {
                        body_returned = true;
                        break;
                    }
                }
                self.loop_blocks.pop();

                if !body_returned {
                    builder.ins().jump(step_block, &no_args);
                }

                builder.seal_block(step_block);
                builder.switch_to_block(step_block);
                let cur = builder.use_var(item_var);
                let nxt = builder.ins().iadd(cur, step_val);
                builder.def_var(item_var, nxt);
                builder.ins().jump(header_block, &no_args);

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);

                Ok((None, false))
            }
            Statement::Break(_) => {
                if let Some(&(_, exit_block)) = self.loop_blocks.last() {
                    builder.ins().jump(exit_block, &no_args);
                    Ok((None, true))
                } else {
                    Err("Cannot use 'break' outside of loop".to_string())
                }
            }
            Statement::Continue(_) => {
                if let Some(&(cont_block, _)) = self.loop_blocks.last() {
                    builder.ins().jump(cont_block, &no_args);
                    Ok((None, true))
                } else {
                    Err("Cannot use 'continue' outside of loop".to_string())
                }
            }
            Statement::Pass(_) => {
                Ok((None, false))
            }
            Statement::Return { value, .. } => {
                let ret_val = if let Some(expr) = value {
                    self.compile_expr(expr, builder, scope)?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                builder.ins().return_(&[ret_val]);
                Ok((None, true))
            }
            Statement::Expr(expr) => {
                let val = self.compile_expr(expr, builder, scope)?;
                Ok((Some(val), false))
            }
            _ => Ok((None, false)),
        }
    }

    fn compile_expr(
        &mut self,
        expr: &Expr,
        builder: &mut FunctionBuilder,
        scope: &mut VarScope,
    ) -> Result<ClifValue, String> {
        let no_args: [BlockArg; 0] = [];

        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(i) => Ok(builder.ins().iconst(types::I64, *i)),
                Literal::Float(f) => Ok(builder.ins().iconst(types::I64, *f as i64)),
                Literal::Bool(b) => Ok(builder.ins().iconst(types::I64, if *b { 1 } else { 0 })),
                Literal::String(s) => {
                    let (data_id, _) = self.get_or_create_string(s)?;
                    let local_data = self.module.declare_data_in_func(data_id, &mut builder.func);
                    let ptr = builder.ins().symbol_value(types::I64, local_data);
                    Ok(ptr)
                }
                Literal::Nil => Ok(builder.ins().iconst(types::I64, 0)),
            },
            Expr::Ident(name, _) => {
                if let Some(var) = scope.get_var(name) {
                    Ok(builder.use_var(var))
                } else {
                    Ok(builder.ins().iconst(types::I64, 0))
                }
            }
            Expr::Binary(left, op, right, _) => {
                let l = self.compile_expr(left, builder, scope)?;
                let r = self.compile_expr(right, builder, scope)?;

                let val = match op {
                    BinaryOp::Add => builder.ins().iadd(l, r),
                    BinaryOp::Sub => builder.ins().isub(l, r),
                    BinaryOp::Mul => builder.ins().imul(l, r),
                    BinaryOp::Div => builder.ins().sdiv(l, r),
                    BinaryOp::Mod => builder.ins().srem(l, r),
                    BinaryOp::Pow => builder.ins().imul(l, r),
                    BinaryOp::Equal => {
                        let cmp = builder.ins().icmp(IntCC::Equal, l, r);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::NotEqual => {
                        let cmp = builder.ins().icmp(IntCC::NotEqual, l, r);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::Less => {
                        let cmp = builder.ins().icmp(IntCC::SignedLessThan, l, r);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::LessEqual => {
                        let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::Greater => {
                        let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::GreaterEqual => {
                        let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::And => builder.ins().band(l, r),
                    BinaryOp::Or => builder.ins().bor(l, r),
                    BinaryOp::In => builder.ins().iconst(types::I64, 0),
                };
                Ok(val)
            }
            Expr::Unary(op, inner, _) => {
                let v = self.compile_expr(inner, builder, scope)?;
                match op {
                    UnaryOp::Negate => Ok(builder.ins().ineg(v)),
                    UnaryOp::Not => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        let cmp = builder.ins().icmp(IntCC::Equal, v, zero);
                        Ok(builder.ins().uextend(types::I64, cmp))
                    }
                }
            }
            Expr::Call(callee, args, _) => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    match name.as_str() {
                        "print" | "println" => {
                            let is_newline = name == "println";
                            if args.is_empty() {
                                if is_newline {
                                    let zero = builder.ins().iconst(types::I64, 0);
                                    let fid = *self.func_ids.get("aether_native_println_str").unwrap();
                                    let local_fid = self.module.declare_func_in_func(fid, &mut builder.func);
                                    builder.ins().call(local_fid, &[zero, zero]);
                                }
                                return Ok(builder.ins().iconst(types::I64, 0));
                            }

                            for (i, arg) in args.iter().enumerate() {
                                if i > 0 {
                                    let (space_id, space_len) = self.get_or_create_string(" ")?;
                                    let local_space = self.module.declare_data_in_func(space_id, &mut builder.func);
                                    let space_ptr = builder.ins().symbol_value(types::I64, local_space);
                                    let space_len_val = builder.ins().iconst(types::I64, space_len as i64);
                                    let fid_print = *self.func_ids.get("aether_native_print_str").unwrap();
                                    let local_print = self.module.declare_func_in_func(fid_print, &mut builder.func);
                                    builder.ins().call(local_print, &[space_ptr, space_len_val]);
                                }

                                let arg_is_last = i == args.len() - 1;
                                let should_nl = is_newline && arg_is_last;

                                if let Expr::Literal(Literal::String(s), _) = arg {
                                    let (data_id, len) = self.get_or_create_string(s)?;
                                    let local_data = self.module.declare_data_in_func(data_id, &mut builder.func);
                                    let ptr = builder.ins().symbol_value(types::I64, local_data);
                                    let len_val = builder.ins().iconst(types::I64, len as i64);

                                    let target_fn = if should_nl { "aether_native_println_str" } else { "aether_native_print_str" };
                                    let fid = *self.func_ids.get(target_fn).unwrap();
                                    let local_fid = self.module.declare_func_in_func(fid, &mut builder.func);
                                    builder.ins().call(local_fid, &[ptr, len_val]);
                                } else {
                                    let val = self.compile_expr(arg, builder, scope)?;
                                    let target_fn = if should_nl { "aether_native_println_i64" } else { "aether_native_print_i64" };
                                    let fid = *self.func_ids.get(target_fn).unwrap();
                                    let local_fid = self.module.declare_func_in_func(fid, &mut builder.func);
                                    builder.ins().call(local_fid, &[val]);
                                }
                            }
                            return Ok(builder.ins().iconst(types::I64, 0));
                        }
                        "clock" | "time_ms" => {
                            let fid = *self.func_ids.get("aether_native_time_ms").unwrap();
                            let local_fid = self.module.declare_func_in_func(fid, &mut builder.func);
                            let call_inst = builder.ins().call(local_fid, &[]);
                            let res = builder.inst_results(call_inst)[0];
                            return Ok(res);
                        }
                        "alloc_buffer" | "buffer" | "NativeBuffer" => {
                            let size_val = if !args.is_empty() {
                                self.compile_expr(&args[0], builder, scope)?
                            } else {
                                builder.ins().iconst(types::I64, 0)
                            };
                            let alloc_fid = *self.func_ids.get("aether_native_alloc_buffer").unwrap();
                            let local_alloc = self.module.declare_func_in_func(alloc_fid, &mut builder.func);
                            let call_inst = builder.ins().call(local_alloc, &[size_val]);
                            let res = builder.inst_results(call_inst)[0];
                            return Ok(res);
                        }
                        "free_buffer" => {
                            let ptr_val = if !args.is_empty() {
                                self.compile_expr(&args[0], builder, scope)?
                            } else {
                                builder.ins().iconst(types::I64, 0)
                            };
                            let size_val = if args.len() > 1 {
                                self.compile_expr(&args[1], builder, scope)?
                            } else {
                                builder.ins().iconst(types::I64, 0)
                            };
                            let free_fid = *self.func_ids.get("aether_native_free_buffer").unwrap();
                            let local_free = self.module.declare_func_in_func(free_fid, &mut builder.func);
                            builder.ins().call(local_free, &[ptr_val, size_val]);
                            return Ok(builder.ins().iconst(types::I64, 0));
                        }
                        "str" => {
                            if !args.is_empty() {
                                return self.compile_expr(&args[0], builder, scope);
                            }
                            return Ok(builder.ins().iconst(types::I64, 0));
                        }
                        _ => {}
                    }
                }

                if let Expr::Member(base, method, _) = callee.as_ref() {
                    if let Expr::Ident(base_name, _) = base.as_ref() {
                        if base_name == "Sys" && method == "time_ms" {
                            let fid = *self.func_ids.get("aether_native_time_ms").unwrap();
                            let local_fid = self.module.declare_func_in_func(fid, &mut builder.func);
                            let call_inst = builder.ins().call(local_fid, &[]);
                            let res = builder.inst_results(call_inst)[0];
                            return Ok(res);
                        }
                    }
                }

                if let Expr::Ident(func_name, _) = callee.as_ref() {
                    if let Some(&fid) = self.func_ids.get(func_name) {
                        let mut arg_vals = Vec::new();
                        for a in args {
                            arg_vals.push(self.compile_expr(a, builder, scope)?);
                        }
                        let local_fid = self.module.declare_func_in_func(fid, &mut builder.func);
                        let call_inst = builder.ins().call(local_fid, &arg_vals);
                        let res = builder.inst_results(call_inst)[0];
                        return Ok(res);
                    }
                }

                Ok(builder.ins().iconst(types::I64, 0))
            }
            Expr::Pipe(left, right, span) => {
                match right.as_ref() {
                    Expr::Call(callee, args, call_span) => {
                        let mut new_args = vec![*left.clone()];
                        new_args.extend(args.clone());
                        let call_expr = Expr::Call(callee.clone(), new_args, *call_span);
                        self.compile_expr(&call_expr, builder, scope)
                    }
                    _ => {
                        let call_expr = Expr::Call(right.clone(), vec![*left.clone()], *span);
                        self.compile_expr(&call_expr, builder, scope)
                    }
                }
            }
            Expr::If { condition, then_branch, else_branch, .. } => {
                let cond_val = self.compile_expr(condition, builder, scope)?;

                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                builder.append_block_param(merge_block, types::I64);

                builder.ins().brif(cond_val, then_block, &no_args, else_block, &no_args);

                // Then
                builder.switch_to_block(then_block);
                builder.seal_block(then_block);
                let mut then_val = None;
                let mut then_returned = false;
                for s in &then_branch.statements {
                    let (v, ret) = self.compile_stmt(s, builder, scope)?;
                    if let Some(val) = v {
                        then_val = Some(val);
                    }
                    if ret {
                        then_returned = true;
                        break;
                    }
                }
                if !then_returned {
                    if let Some(res) = &then_branch.result {
                        then_val = Some(self.compile_expr(res, builder, scope)?);
                    }
                    let tv = then_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                    builder.ins().jump(merge_block, &[BlockArg::Value(tv)]);
                }

                // Else
                builder.switch_to_block(else_block);
                builder.seal_block(else_block);
                let mut else_val = None;
                let mut else_returned = false;
                if let Some(eb) = else_branch {
                    for s in &eb.statements {
                        let (v, ret) = self.compile_stmt(s, builder, scope)?;
                        if let Some(val) = v {
                            else_val = Some(val);
                        }
                        if ret {
                            else_returned = true;
                            break;
                        }
                    }
                    if !else_returned {
                        if let Some(res) = &eb.result {
                            else_val = Some(self.compile_expr(res, builder, scope)?);
                        }
                        let ev = else_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                        builder.ins().jump(merge_block, &[BlockArg::Value(ev)]);
                    }
                } else {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().jump(merge_block, &[BlockArg::Value(zero)]);
                }

                // Merge
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                Ok(builder.block_params(merge_block)[0])
            }
            Expr::Match { target, arms, .. } => {
                let target_val = self.compile_expr(target, builder, scope)?;
                let merge_block = builder.create_block();
                builder.append_block_param(merge_block, types::I64);

                let mut current_arm_block = builder.create_block();
                builder.ins().jump(current_arm_block, &no_args);

                let mut has_wildcard = false;

                for arm in arms {
                    builder.switch_to_block(current_arm_block);
                    builder.seal_block(current_arm_block);

                    match &arm.pattern {
                        MatchPattern::Wildcard => {
                            let arm_val = self.compile_expr(&arm.body, builder, scope)?;
                            builder.ins().jump(merge_block, &[BlockArg::Value(arm_val)]);
                            has_wildcard = true;
                            break;
                        }
                        MatchPattern::Literal(lit) => {
                            let next_arm_block = builder.create_block();
                            let lit_val = match lit {
                                Literal::Int(i) => builder.ins().iconst(types::I64, *i),
                                Literal::Bool(b) => builder.ins().iconst(types::I64, if *b { 1 } else { 0 }),
                                _ => builder.ins().iconst(types::I64, 0),
                            };
                            let cmp = builder.ins().icmp(IntCC::Equal, target_val, lit_val);
                            let arm_body_block = builder.create_block();
                            builder.ins().brif(cmp, arm_body_block, &no_args, next_arm_block, &no_args);

                            builder.switch_to_block(arm_body_block);
                            builder.seal_block(arm_body_block);
                            let arm_val = self.compile_expr(&arm.body, builder, scope)?;
                            builder.ins().jump(merge_block, &[BlockArg::Value(arm_val)]);

                            current_arm_block = next_arm_block;
                        }
                        MatchPattern::Range(start, end) => {
                            let next_arm_block = builder.create_block();
                            let s_int = match start { Literal::Int(i) => *i, _ => 0 };
                            let e_int = match end { Literal::Int(i) => *i, _ => 0 };
                            let s_val = builder.ins().iconst(types::I64, s_int);
                            let e_val = builder.ins().iconst(types::I64, e_int);

                            let cmp_ge = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, target_val, s_val);
                            let cmp_le = builder.ins().icmp(IntCC::SignedLessThanOrEqual, target_val, e_val);
                            let in_range = builder.ins().band(cmp_ge, cmp_le);

                            let arm_body_block = builder.create_block();
                            builder.ins().brif(in_range, arm_body_block, &no_args, next_arm_block, &no_args);

                            builder.switch_to_block(arm_body_block);
                            builder.seal_block(arm_body_block);
                            let arm_val = self.compile_expr(&arm.body, builder, scope)?;
                            builder.ins().jump(merge_block, &[BlockArg::Value(arm_val)]);

                            current_arm_block = next_arm_block;
                        }
                        _ => {
                            let arm_val = self.compile_expr(&arm.body, builder, scope)?;
                            builder.ins().jump(merge_block, &[BlockArg::Value(arm_val)]);
                            has_wildcard = true;
                            break;
                        }
                    }
                }

                if !has_wildcard {
                    builder.switch_to_block(current_arm_block);
                    builder.seal_block(current_arm_block);
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().jump(merge_block, &[BlockArg::Value(zero)]);
                }

                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                Ok(builder.block_params(merge_block)[0])
            }
            Expr::Block(block, ..) => {
                let mut last_val = None;
                let num_stmts = block.statements.len();
                for (i, s) in block.statements.iter().enumerate() {
                    if i == num_stmts - 1 && block.result.is_none() {
                        if let Statement::Expr(expr) = s {
                            last_val = Some(self.compile_expr(expr, builder, scope)?);
                            break;
                        }
                    }
                    let (v, ret) = self.compile_stmt(s, builder, scope)?;
                    if let Some(val) = v { last_val = Some(val); }
                    if ret { break; }
                }
                if let Some(res) = &block.result {
                    last_val = Some(self.compile_expr(res, builder, scope)?);
                }
                Ok(last_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0)))
            }
            Expr::Index(obj, idx, _) => {
                let obj_ptr = self.compile_expr(obj, builder, scope)?;
                let idx_val = self.compile_expr(idx, builder, scope)?;
                let eight = builder.ins().iconst(types::I64, 8);
                let offset = builder.ins().imul(idx_val, eight);
                let addr = builder.ins().iadd(obj_ptr, offset);
                let val = builder.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
                Ok(val)
            }
            Expr::Array(elements, _) => {
                let count = elements.len() as i64;
                let size_val = builder.ins().iconst(types::I64, count);
                let alloc_fid = *self.func_ids.get("aether_native_alloc_buffer").unwrap();
                let local_alloc = self.module.declare_func_in_func(alloc_fid, &mut builder.func);
                let call_inst = builder.ins().call(local_alloc, &[size_val]);
                let buf_ptr = builder.inst_results(call_inst)[0];

                for (i, elem) in elements.iter().enumerate() {
                    let elem_val = self.compile_expr(elem, builder, scope)?;
                    let offset = builder.ins().iconst(types::I64, (i * 8) as i64);
                    let addr = builder.ins().iadd(buf_ptr, offset);
                    builder.ins().store(MemFlagsData::trusted(), elem_val, addr, 0);
                }
                Ok(buf_ptr)
            }
            _ => Ok(builder.ins().iconst(types::I64, 0)),
        }
    }
}

struct VarScope {
    vars: HashMap<String, Variable>,
}

impl VarScope {
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
