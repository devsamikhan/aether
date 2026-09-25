use super::bytecode::OpCode;
use super::value::{ChannelHandle, CompiledFunction, NativeFn, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct CallFrame {
    pub function: Arc<CompiledFunction>,
    pub ip: usize,
    pub stack_offset: usize,
    pub constructor_instance: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ExceptionHandler {
    pub frame_idx: usize,
    pub stack_depth: usize,
    pub handler_ip: usize,
}

pub struct VM {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    pub globals: HashMap<String, Value>,
    pub modules_cache: HashMap<String, Value>,
    pub exception_handlers: Vec<ExceptionHandler>,
}

thread_local! {
    static CURRENT_VM_PTR: std::cell::Cell<*mut VM> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

pub struct VmScopeGuard {
    prev: *mut VM,
}

impl VmScopeGuard {
    pub fn enter(vm: &mut VM) -> Self {
        let prev = CURRENT_VM_PTR.with(|c| c.replace(vm as *mut VM));
        Self { prev }
    }
}

impl Drop for VmScopeGuard {
    fn drop(&mut self) {
        CURRENT_VM_PTR.with(|c| c.set(self.prev));
    }
}

pub fn with_current_vm<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut VM) -> Result<R, String>,
{
    let ptr = CURRENT_VM_PTR.with(|c| c.get());
    if ptr.is_null() {
        return Err("No active VM in current thread".to_string());
    }
    unsafe { f(&mut *ptr) }
}

impl VM {
    pub fn new() -> Self {
        let mut vm = Self {
            frames: Vec::new(),
            stack: Vec::with_capacity(512),
            globals: HashMap::new(),
            modules_cache: HashMap::new(),
            exception_handlers: Vec::new(),
        };
        vm.register_builtins();
        super::stdlib::register_stdlib(&mut vm.globals);
        super::net::register_net_module(&mut vm.globals);
        super::crypto::register_crypto_module(&mut vm.globals);
        super::db::register_db_module(&mut vm.globals);
        super::tensor::register_tensor_module(&mut vm.globals);
        super::vector::register_vector_module(&mut vm.globals);
        super::actor::register_actor_module(&mut vm.globals);
        super::wasm_runtime::register_wasm_module(&mut vm.globals);
        super::rpc::register_rpc_module(&mut vm.globals);
        super::dataframe::register_dataframe_module(&mut vm.globals);
        super::compute::register_compute_module(&mut vm.globals);
        super::timetravel::register_timetravel_module(&mut vm.globals);
        super::sql::register_sql_module(&mut vm.globals);
        super::timetravel_tui::register_timetravel_tui_module(&mut vm.globals);
        super::flow::register_flow_module(&mut vm.globals);
        super::proof::register_proof_module(&mut vm.globals);
        super::graph::register_graph_module(&mut vm.globals);
        super::hotreload::register_hotreload_module(&mut vm.globals);
        vm
    }

    pub fn with_globals(globals: HashMap<String, Value>) -> Self {
        Self {
            frames: Vec::new(),
            stack: Vec::with_capacity(512),
            globals,
            modules_cache: HashMap::new(),
            exception_handlers: Vec::new(),
        }
    }

    fn register_builtins(&mut self) {
        self.define_native("print", |args| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 { print!(" "); }
                print!("{}", arg);
            }
            println!();
            Ok(Value::Nil)
        });

        self.define_native("println", |args| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 { print!(" "); }
                print!("{}", arg);
            }
            println!();
            Ok(Value::Nil)
        });

        self.define_native("input", |args| {
            use std::io::{self, Write};
            if !args.is_empty() {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { print!(" "); }
                    print!("{}", arg);
                }
                let _ = io::stdout().flush();
            }
            let mut line = String::new();
            io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Ok(Value::string(line))
        });

        self.define_native("len", |args| {
            if args.is_empty() {
                return Err("len() expects at least 1 argument".to_string());
            }
            match &args[0] {
                Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                Value::Array(a) => Ok(Value::Int(a.lock().len() as i64)),
                Value::Tuple(t) => Ok(Value::Int(t.len() as i64)),
                Value::Set(s) => Ok(Value::Int(s.lock().len() as i64)),
                Value::Map(m) => Ok(Value::Int(m.lock().len() as i64)),
                other => Err(format!("len() not supported on type '{}'", other.type_name())),
            }
        });

        self.define_native("push", |args| {
            if args.len() < 2 {
                return Err("push(array, value) expects 2 arguments".to_string());
            }
            match &args[0] {
                Value::Array(a) => {
                    a.lock().push(args[1].clone());
                    Ok(args[0].clone())
                }
                _ => Err("First argument to push must be an array".to_string()),
            }
        });

        self.define_native("append", |args| {
            if args.len() < 2 {
                return Err("append(array, value) expects 2 arguments".to_string());
            }
            match &args[0] {
                Value::Array(a) => {
                    a.lock().push(args[1].clone());
                    Ok(args[0].clone())
                }
                _ => Err("First argument to append must be an array".to_string()),
            }
        });

        self.define_native("pop", |args| {
            if args.is_empty() {
                return Err("pop(array) expects at least 1 argument".to_string());
            }
            match &args[0] {
                Value::Array(a) => {
                    let mut items = a.lock();
                    if args.len() > 1 {
                        if let Value::Int(i) = &args[1] {
                            let idx = if *i < 0 { (items.len() as i64 + *i) as usize } else { *i as usize };
                            if idx < items.len() {
                                Ok(items.remove(idx))
                            } else {
                                Err("pop() index out of bounds".to_string())
                            }
                        } else {
                            Err("pop() index must be integer".to_string())
                        }
                    } else {
                        Ok(items.pop().unwrap_or(Value::Nil))
                    }
                }
                _ => Err("Argument to pop must be an array".to_string()),
            }
        });

        self.define_native("tuple", |args| {
            if args.is_empty() {
                return Ok(Value::tuple(Vec::new()));
            }
            match &args[0] {
                Value::Array(a) => Ok(Value::tuple(a.lock().clone())),
                Value::Tuple(t) => Ok(Value::Tuple(t.clone())),
                Value::Set(s) => Ok(Value::tuple(s.lock().clone())),
                Value::String(s) => {
                    let chars = s.chars().map(|c| Value::string(c.to_string())).collect();
                    Ok(Value::tuple(chars))
                }
                other => Ok(Value::tuple(vec![other.clone()])),
            }
        });

        self.define_native("list", |args| {
            if args.is_empty() {
                return Ok(Value::array(Vec::new()));
            }
            match &args[0] {
                Value::Array(a) => Ok(Value::array(a.lock().clone())),
                Value::Tuple(t) => Ok(Value::array((**t).clone())),
                Value::Set(s) => Ok(Value::array(s.lock().clone())),
                Value::String(s) => {
                    let chars = s.chars().map(|c| Value::string(c.to_string())).collect();
                    Ok(Value::array(chars))
                }
                other => Ok(Value::array(vec![other.clone()])),
            }
        });

        self.define_native("set", |args| {
            if args.is_empty() {
                return Ok(Value::set(Vec::new()));
            }
            match &args[0] {
                Value::Array(a) => Ok(Value::set(a.lock().clone())),
                Value::Tuple(t) => Ok(Value::set((**t).clone())),
                Value::Set(s) => Ok(Value::set(s.lock().clone())),
                Value::String(s) => {
                    let chars = s.chars().map(|c| Value::string(c.to_string())).collect();
                    Ok(Value::set(chars))
                }
                other => Ok(Value::set(vec![other.clone()])),
            }
        });

        self.define_native("dict", |args| {
            if args.is_empty() {
                return Ok(Value::map(HashMap::new()));
            }
            match &args[0] {
                Value::Map(m) => Ok(Value::map(m.lock().clone())),
                _ => Err("dict() expects a dictionary or no arguments".to_string()),
            }
        });

        self.define_native("keys", |args| {
            if args.is_empty() { return Err("keys(dict) expects 1 argument".to_string()); }
            match &args[0] {
                Value::Map(m) => {
                    let mut keys: Vec<String> = m.lock().keys().cloned().collect();
                    keys.sort();
                    let v_keys: Vec<Value> = keys.into_iter().map(Value::string).collect();
                    Ok(Value::array(v_keys))
                }
                other => Err(format!("keys() expects dictionary, got '{}'", other.type_name())),
            }
        });

        self.define_native("values", |args| {
            if args.is_empty() { return Err("values(dict) expects 1 argument".to_string()); }
            match &args[0] {
                Value::Map(m) => {
                    let map = m.lock();
                    let mut keys: Vec<String> = map.keys().cloned().collect();
                    keys.sort();
                    let vals: Vec<Value> = keys.into_iter().map(|k| map.get(&k).cloned().unwrap_or(Value::Nil)).collect();
                    Ok(Value::array(vals))
                }
                other => Err(format!("values() expects dictionary, got '{}'", other.type_name())),
            }
        });

        self.define_native("items", |args| {
            if args.is_empty() { return Err("items(dict) expects 1 argument".to_string()); }
            match &args[0] {
                Value::Map(m) => {
                    let map = m.lock();
                    let mut keys: Vec<String> = map.keys().cloned().collect();
                    keys.sort();
                    let items: Vec<Value> = keys.into_iter().map(|k| {
                        let v = map.get(&k).cloned().unwrap_or(Value::Nil);
                        Value::tuple(vec![Value::string(k), v])
                    }).collect();
                    Ok(Value::array(items))
                }
                other => Err(format!("items() expects dictionary, got '{}'", other.type_name())),
            }
        });

        self.define_native("get", |args| {
            if args.len() < 2 { return Err("get(dict, key, default?) expects at least 2 arguments".to_string()); }
            match &args[0] {
                Value::Map(m) => {
                    let key = format!("{}", args[1]);
                    let default_val = if args.len() > 2 { args[2].clone() } else { Value::Nil };
                    Ok(m.lock().get(&key).cloned().unwrap_or(default_val))
                }
                other => Err(format!("get() expects dictionary, got '{}'", other.type_name())),
            }
        });

        self.define_native("add", |args| {
            if args.len() < 2 { return Err("add(set, value) expects 2 arguments".to_string()); }
            match &args[0] {
                Value::Set(s) => {
                    let mut items = s.lock();
                    if !items.contains(&args[1]) {
                        items.push(args[1].clone());
                    }
                    Ok(Value::Nil)
                }
                other => Err(format!("add() expects set, got '{}'", other.type_name())),
            }
        });

        self.define_native("remove", |args| {
            if args.len() < 2 { return Err("remove(collection, value) expects 2 arguments".to_string()); }
            match &args[0] {
                Value::Set(s) => {
                    let mut items = s.lock();
                    if let Some(pos) = items.iter().position(|x| x == &args[1]) {
                        items.remove(pos);
                    }
                    Ok(Value::Nil)
                }
                Value::Array(a) => {
                    let mut items = a.lock();
                    if let Some(pos) = items.iter().position(|x| x == &args[1]) {
                        items.remove(pos);
                    }
                    Ok(Value::Nil)
                }
                Value::Map(m) => {
                    let key = format!("{}", args[1]);
                    let val = m.lock().remove(&key).unwrap_or(Value::Nil);
                    Ok(val)
                }
                other => Err(format!("remove() not supported on '{}'", other.type_name())),
            }
        });

        let contains_fn: super::value::NativeFn = |args| {
            if args.len() < 2 { return Err("contains(collection, value) expects 2 arguments".to_string()); }
            let coll = &args[0];
            let val = &args[1];
            match coll {
                Value::Array(a) => Ok(Value::Bool(a.lock().contains(val))),
                Value::Tuple(t) => Ok(Value::Bool(t.contains(val))),
                Value::Set(s) => Ok(Value::Bool(s.lock().contains(val))),
                Value::Map(m) => {
                    let k = format!("{}", val);
                    Ok(Value::Bool(m.lock().contains_key(&k)))
                }
                Value::String(s) => {
                    let sub = format!("{}", val);
                    Ok(Value::Bool(s.contains(&sub)))
                }
                other => Err(format!("contains() not supported on '{}'", other.type_name())),
            }
        };

        self.define_native("contains", contains_fn);

        // __in(val, collection) for Pythonic `val in collection`
        self.define_native("__in", |args| {
            if args.len() < 2 { return Err("__in(val, collection) expects 2 arguments".to_string()); }
            let val = &args[0];
            let coll = &args[1];
            match coll {
                Value::Array(a) => Ok(Value::Bool(a.lock().contains(val))),
                Value::Tuple(t) => Ok(Value::Bool(t.contains(val))),
                Value::Set(s) => Ok(Value::Bool(s.lock().contains(val))),
                Value::Map(m) => {
                    let k = format!("{}", val);
                    Ok(Value::Bool(m.lock().contains_key(&k)))
                }
                Value::String(s) => {
                    let sub = format!("{}", val);
                    Ok(Value::Bool(s.contains(&sub)))
                }
                other => Err(format!("'in' operator not supported on '{}'", other.type_name())),
            }
        });

        self.define_native("clock", |_| {
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap_or_default();
            Ok(Value::Float(since_the_epoch.as_secs_f64()))
        });

        self.define_native("time_ms", |_| {
            let start = SystemTime::now();
            let since = start.duration_since(UNIX_EPOCH).unwrap_or_default();
            Ok(Value::Int(since.as_millis() as i64))
        });

        self.define_native("range", |args| {
            if args.is_empty() {
                return Err("range(end) or range(start, end, step?) expects arguments".to_string());
            }
            let (start, end, step) = if args.len() == 1 {
                let end = match args[0] {
                    Value::Int(i) => i,
                    _ => return Err("range() arguments must be integers".to_string()),
                };
                (0, end, 1)
            } else if args.len() == 2 {
                let start = match args[0] { Value::Int(i) => i, _ => return Err("range() start must be int".to_string()) };
                let end = match args[1] { Value::Int(i) => i, _ => return Err("range() end must be int".to_string()) };
                (start, end, 1)
            } else {
                let start = match args[0] { Value::Int(i) => i, _ => return Err("range() start must be int".to_string()) };
                let end = match args[1] { Value::Int(i) => i, _ => return Err("range() end must be int".to_string()) };
                let step = match args[2] { Value::Int(i) => i, _ => return Err("range() step must be int".to_string()) };
                (start, end, step)
            };

            let mut list = Vec::new();
            if step > 0 {
                let mut curr = start;
                while curr < end {
                    list.push(Value::Int(curr));
                    curr += step;
                }
            } else if step < 0 {
                let mut curr = start;
                while curr > end {
                    list.push(Value::Int(curr));
                    curr += step;
                }
            }
            Ok(Value::array(list))
        });

        self.define_native("type_of", |args| {
            if args.is_empty() { return Ok(Value::string("nil")); }
            Ok(Value::string(args[0].type_name()))
        });

        self.define_native("type", |args| {
            if args.is_empty() { return Ok(Value::string("nil")); }
            Ok(Value::string(args[0].type_name()))
        });

        self.define_native("bool", |args| {
            if args.is_empty() { return Ok(Value::Bool(false)); }
            Ok(Value::Bool(args[0].is_truthy()))
        });

        self.define_native("str", |args| {
            if args.is_empty() { return Ok(Value::string("")); }
            Ok(Value::string(format!("{}", args[0])))
        });

        self.define_native("int", |args| {
            if args.is_empty() { return Ok(Value::Int(0)); }
            match &args[0] {
                Value::Int(i) => Ok(Value::Int(*i)),
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                Value::String(s) => s.trim().parse::<i64>().map(Value::Int).map_err(|e| e.to_string()),
                _ => Err("Cannot cast to int".to_string()),
            }
        });

        self.define_native("float", |args| {
            if args.is_empty() { return Ok(Value::Float(0.0)); }
            match &args[0] {
                Value::Int(i) => Ok(Value::Float(*i as f64)),
                Value::Float(f) => Ok(Value::Float(*f)),
                Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
                Value::String(s) => s.trim().parse::<f64>().map(Value::Float).map_err(|e| e.to_string()),
                _ => Err("Cannot cast to float".to_string()),
            }
        });

        self.define_native("assert", |args| {
            if args.is_empty() {
                return Err("assert expects condition".to_string());
            }
            if !args[0].is_truthy() {
                let msg = if args.len() > 1 {
                    format!("{}", args[1])
                } else {
                    "Assertion failed".to_string()
                };
                return Err(msg);
            }
            Ok(Value::Bool(true))
        });

        self.define_native("channel", |_| {
            Ok(Value::Channel(ChannelHandle::new()))
        });

        self.define_native("send", |args| {
            if args.len() < 2 { return Err("send(channel, val) expects 2 arguments".to_string()); }
            match &args[0] {
                Value::Channel(ch) => {
                    let tx = ch.sender.lock();
                    tx.send(args[1].clone()).map_err(|e| e.to_string())?;
                    Ok(Value::Nil)
                }
                _ => Err("send requires channel as first argument".to_string()),
            }
        });

        self.define_native("recv", |args| {
            if args.is_empty() { return Err("recv(channel) expects 1 argument".to_string()); }
            match &args[0] {
                Value::Channel(ch) => {
                    let rx = ch.receiver.lock();
                    let val = rx.recv().map_err(|e| e.to_string())?;
                    Ok(val)
                }
                _ => Err("recv requires channel as first argument".to_string()),
            }
        });

        self.define_native("try_recv", |args| {
            if args.is_empty() { return Err("try_recv(channel) expects 1 argument".to_string()); }
            match &args[0] {
                Value::Channel(ch) => {
                    let rx = ch.receiver.lock();
                    match rx.try_recv() {
                        Ok(val) => Ok(val),
                        Err(_) => Ok(Value::Nil),
                    }
                }
                _ => Err("try_recv requires channel as first argument".to_string()),
            }
        });

        self.define_native("sleep_ms", |args| {
            if args.is_empty() { return Err("sleep_ms(ms) expects 1 argument".to_string()); }
            match &args[0] {
                Value::Int(ms) => {
                    if *ms > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                    }
                    Ok(Value::Nil)
                }
                _ => Err("sleep_ms expects integer milliseconds".to_string()),
            }
        });

        self.define_native("clock", |_| {
            let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
            Ok(Value::Float(duration.as_secs_f64()))
        });

        self.define_native("time", |_| {
            let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
            Ok(Value::Float(duration.as_secs_f64()))
        });
    }

    pub fn define_native(&mut self, name: &str, func: NativeFn) {
        self.globals.insert(name.to_string(), Value::Native(name.to_string(), func));
    }

    pub fn interpret(&mut self, function: CompiledFunction) -> Result<Value, String> {
        let _guard = VmScopeGuard::enter(self);
        let func_rc = Arc::new(function);
        self.stack.push(Value::Function(func_rc.clone()));
        self.interpret_with_stack(func_rc)
    }

    pub fn interpret_with_stack(&mut self, func_rc: Arc<CompiledFunction>) -> Result<Value, String> {
        let _guard = VmScopeGuard::enter(self);
        self.frames.push(CallFrame {
            function: func_rc,
            ip: 0,
            stack_offset: 0,
            constructor_instance: None,
        });

        self.run()
    }

    pub fn call_global(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let func_val = self.globals.get(name).cloned().ok_or_else(|| format!("Global '{}' not found", name))?;
        match func_val {
            Value::Function(f) => self.call_function(f, args),
            Value::Closure { function, .. } => self.call_function(function, args),
            _ => Err(format!("'{}' is not a callable function", name)),
        }
    }

    fn run(&mut self) -> Result<Value, String> {
        loop {
            let mut frame = self.frames.pop().unwrap();

            while frame.ip < frame.function.chunk.code.len() {
                let op = OpCode::from(frame.function.chunk.code[frame.ip]);
                frame.ip += 1;

                match op {
                    OpCode::Constant => {
                        let idx = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let val = frame.function.chunk.constants[idx].clone();
                        self.stack.push(val);
                    }
                    OpCode::Nil => self.stack.push(Value::Nil),
                    OpCode::True => self.stack.push(Value::Bool(true)),
                    OpCode::False => self.stack.push(Value::Bool(false)),
                    OpCode::Pop => {
                        self.stack.pop();
                    }
                    OpCode::Dup => {
                        if let Some(top) = self.stack.last() {
                            self.stack.push(top.clone());
                        }
                    }
                    OpCode::Add => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match (a, b) {
                            (Value::ClassInstance(inst), other) => {
                                if let Some(m) = Self::find_method(&inst.class, "__add__", &self.globals) {
                                    let res = self.call_function(m, vec![Value::ClassInstance(inst), other])?;
                                    self.stack.push(res);
                                } else {
                                    return Err(format!("'{}' object does not support + operator", inst.class.name));
                                }
                            }
                            (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x + y)),
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x + y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(x as f64 + y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x + y as f64)),
                            (Value::String(x), Value::String(y)) => {
                                self.stack.push(Value::string(format!("{}{}", x, y)));
                            }
                            (Value::Array(a), Value::Array(b)) => {
                                let mut items = a.lock().clone();
                                items.extend(b.lock().iter().cloned());
                                self.stack.push(Value::array(items));
                            }
                            (Value::Tuple(a), Value::Tuple(b)) => {
                                let mut items = (*a).clone();
                                items.extend((*b).iter().cloned());
                                self.stack.push(Value::tuple(items));
                            }
                            (Value::String(x), other) => {
                                self.stack.push(Value::string(format!("{}{}", x, other)));
                            }
                            (other, Value::String(y)) => {
                                self.stack.push(Value::string(format!("{}{}", other, y)));
                            }
                            (a, b) => return Err(format!("Cannot add '{}' and '{}'", a.type_name(), b.type_name())),
                        }
                    }
                    OpCode::Sub => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match (a, b) {
                            (Value::ClassInstance(inst), other) => {
                                if let Some(m) = Self::find_method(&inst.class, "__sub__", &self.globals) {
                                    let res = self.call_function(m, vec![Value::ClassInstance(inst), other])?;
                                    self.stack.push(res);
                                } else {
                                    return Err(format!("'{}' object does not support - operator", inst.class.name));
                                }
                            }
                            (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x - y)),
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x - y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(x as f64 - y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x - y as f64)),
                            (a, b) => return Err(format!("Cannot subtract '{}' and '{}'", a.type_name(), b.type_name())),
                        }
                    }
                    OpCode::Mul => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match (a, b) {
                            (Value::ClassInstance(inst), other) => {
                                if let Some(m) = Self::find_method(&inst.class, "__mul__", &self.globals) {
                                    let res = self.call_function(m, vec![Value::ClassInstance(inst), other])?;
                                    self.stack.push(res);
                                } else {
                                    return Err(format!("'{}' object does not support * operator", inst.class.name));
                                }
                            }
                            (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x * y)),
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x * y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(x as f64 * y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x * y as f64)),
                            (Value::String(s), Value::Int(n)) => {
                                let count = if n < 0 { 0 } else { n as usize };
                                self.stack.push(Value::string(s.repeat(count)));
                            }
                            (Value::Int(n), Value::String(s)) => {
                                let count = if n < 0 { 0 } else { n as usize };
                                self.stack.push(Value::string(s.repeat(count)));
                            }
                            (Value::Array(arr), Value::Int(n)) => {
                                let count = if n < 0 { 0 } else { n as usize };
                                let items = arr.lock();
                                let mut repeated = Vec::with_capacity(items.len() * count);
                                for _ in 0..count {
                                    repeated.extend(items.iter().cloned());
                                }
                                self.stack.push(Value::array(repeated));
                            }
                            (Value::Int(n), Value::Array(arr)) => {
                                let count = if n < 0 { 0 } else { n as usize };
                                let items = arr.lock();
                                let mut repeated = Vec::with_capacity(items.len() * count);
                                for _ in 0..count {
                                    repeated.extend(items.iter().cloned());
                                }
                                self.stack.push(Value::array(repeated));
                            }
                            (a, b) => return Err(format!("Cannot multiply '{}' and '{}'", a.type_name(), b.type_name())),
                        }
                    }
                    OpCode::Div => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match (a, b) {
                            (Value::ClassInstance(inst), other) => {
                                if let Some(m) = Self::find_method(&inst.class, "__div__", &self.globals)
                                    .or_else(|| Self::find_method(&inst.class, "__truediv__", &self.globals))
                                {
                                    let res = self.call_function(m, vec![Value::ClassInstance(inst), other])?;
                                    self.stack.push(res);
                                } else {
                                    return Err(format!("'{}' object does not support / operator", inst.class.name));
                                }
                            }
                            (Value::Int(x), Value::Int(y)) => {
                                if y == 0 {
                                    self.handle_exception(&mut frame, Value::string("ZeroDivisionError: division by zero"))?;
                                    continue;
                                }
                                self.stack.push(Value::Int(x / y));
                            }
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x / y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(x as f64 / y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x / y as f64)),
                            (a, b) => return Err(format!("Cannot divide '{}' by '{}'", a.type_name(), b.type_name())),
                        }
                    }
                    OpCode::Mod => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => {
                                if y == 0 { return Err("Modulo by zero".to_string()); }
                                self.stack.push(Value::Int(x % y));
                            }
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x % y)),
                            (a, b) => return Err(format!("Cannot calculate modulo of '{}' and '{}'", a.type_name(), b.type_name())),
                        }
                    }
                    OpCode::Pow => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => {
                                if y >= 0 {
                                    self.stack.push(Value::Int(x.pow(y as u32)));
                                } else {
                                    self.stack.push(Value::Float((x as f64).powf(y as f64)));
                                }
                            }
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x.powf(y))),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float((x as f64).powf(y))),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x.powi(y as i32))),
                            (a, b) => return Err(format!("Cannot raise '{}' to power of '{}'", a.type_name(), b.type_name())),
                        }
                    }
                    OpCode::Negate => {
                        let val = self.stack.pop().unwrap();
                        match val {
                            Value::Int(i) => self.stack.push(Value::Int(-i)),
                            Value::Float(f) => self.stack.push(Value::Float(-f)),
                            _ => return Err("Operand must be a number to negate".to_string()),
                        }
                    }
                    OpCode::Not => {
                        let val = self.stack.pop().unwrap();
                        self.stack.push(Value::Bool(!val.is_truthy()));
                    }
                    OpCode::Equal => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        if let Value::ClassInstance(ref inst) = a {
                            if let Some(eq_fn) = Self::find_method(&inst.class, "__eq__", &self.globals) {
                                let res = self.call_function(eq_fn, vec![a.clone(), b])?;
                                self.stack.push(res);
                                continue;
                            }
                        }
                        self.stack.push(Value::Bool(a == b));
                    }
                    OpCode::NotEqual => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        if let Value::ClassInstance(ref inst) = a {
                            if let Some(eq_fn) = Self::find_method(&inst.class, "__eq__", &self.globals) {
                                let res = self.call_function(eq_fn, vec![a.clone(), b])?;
                                self.stack.push(Value::Bool(!res.is_truthy()));
                                continue;
                            }
                        }
                        self.stack.push(Value::Bool(a != b));
                    }
                    OpCode::Less => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        if let Value::ClassInstance(ref inst) = a {
                            if let Some(m) = Self::find_method(&inst.class, "__lt__", &self.globals) {
                                let res = self.call_function(m, vec![a.clone(), b])?;
                                self.stack.push(res);
                                continue;
                            }
                        }
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Bool(x < y)),
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Bool(x < y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Bool((x as f64) < y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Bool(x < (y as f64))),
                            (Value::String(x), Value::String(y)) => self.stack.push(Value::Bool(x < y)),
                            (a, b) => return Err(format!("Cannot compare non-numbers (<): '{}' ({}) and '{}' ({})", a, a.type_name(), b, b.type_name())),
                        }
                    }
                    OpCode::LessEqual => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        if let Value::ClassInstance(ref inst) = a {
                            if let Some(m) = Self::find_method(&inst.class, "__le__", &self.globals) {
                                let res = self.call_function(m, vec![a.clone(), b])?;
                                self.stack.push(res);
                                continue;
                            }
                        }
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Bool(x <= y)),
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Bool(x <= y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Bool((x as f64) <= y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Bool(x <= (y as f64))),
                            (Value::String(x), Value::String(y)) => self.stack.push(Value::Bool(x <= y)),
                            (a, b) => return Err(format!("Cannot compare non-numbers (<=): '{}' ({}) and '{}' ({})", a, a.type_name(), b, b.type_name())),
                        }
                    }
                    OpCode::Greater => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        if let Value::ClassInstance(ref inst) = a {
                            if let Some(m) = Self::find_method(&inst.class, "__gt__", &self.globals) {
                                let res = self.call_function(m, vec![a.clone(), b])?;
                                self.stack.push(res);
                                continue;
                            }
                        }
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Bool(x > y)),
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Bool(x > y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Bool((x as f64) > y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Bool(x > (y as f64))),
                            (Value::String(x), Value::String(y)) => self.stack.push(Value::Bool(x > y)),
                            (a, b) => return Err(format!("Cannot compare non-numbers (>): '{}' ({}) and '{}' ({})", a, a.type_name(), b, b.type_name())),
                        }
                    }
                    OpCode::GreaterEqual => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        if let Value::ClassInstance(ref inst) = a {
                            if let Some(m) = Self::find_method(&inst.class, "__ge__", &self.globals) {
                                let res = self.call_function(m, vec![a.clone(), b])?;
                                self.stack.push(res);
                                continue;
                            }
                        }
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Bool(x >= y)),
                            (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Bool(x >= y)),
                            (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Bool((x as f64) >= y)),
                            (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Bool(x >= (y as f64))),
                            (Value::String(x), Value::String(y)) => self.stack.push(Value::Bool(x >= y)),
                            (a, b) => return Err(format!("Cannot compare non-numbers (>=): '{}' ({}) and '{}' ({})", a, a.type_name(), b, b.type_name())),
                        }
                    }
                    OpCode::DefineGlobal => {
                        let idx = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        if let Value::String(name) = &frame.function.chunk.constants[idx] {
                            let val = self.stack.pop().unwrap_or(Value::Nil);
                            self.globals.insert((**name).clone(), val);
                        }
                    }
                    OpCode::GetGlobal => {
                        let idx = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        if let Value::String(name) = &frame.function.chunk.constants[idx] {
                            if let Some(val) = self.globals.get(&**name) {
                                self.stack.push(val.clone());
                            } else {
                                return Err(format!("Undefined variable '{}'", name));
                            }
                        }
                    }
                    OpCode::SetGlobal => {
                        let idx = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        if let Value::String(name) = &frame.function.chunk.constants[idx] {
                            let val = self.stack.last().cloned().unwrap_or(Value::Nil);
                            self.globals.insert((**name).clone(), val);
                        }
                    }
                    OpCode::GetLocal => {
                        let slot = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let idx = frame.stack_offset + slot;
                        if idx >= self.stack.len() {
                            return Err(format!(
                                "GetLocal out of bounds in '{}': slot={}, stack_offset={}, idx={}, stack_len={}",
                                frame.function.name, slot, frame.stack_offset, idx, self.stack.len()
                            ));
                        }
                        let val = self.stack[idx].clone();
                        self.stack.push(val);
                    }
                    OpCode::SetLocal => {
                        let slot = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let idx = frame.stack_offset + slot;
                        if idx >= self.stack.len() {
                            return Err(format!(
                                "SetLocal out of bounds in '{}': slot={}, stack_offset={}, idx={}, stack_len={}",
                                frame.function.name, slot, frame.stack_offset, idx, self.stack.len()
                            ));
                        }
                        let val = self.stack.last().cloned().unwrap_or(Value::Nil);
                        self.stack[idx] = val;
                    }
                    OpCode::Jump => {
                        let offset = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2 + offset;
                    }
                    OpCode::JumpIfFalse => {
                        let offset = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        if let Some(top) = self.stack.last() {
                            if !top.is_truthy() {
                                frame.ip += offset;
                            }
                        }
                    }
                    OpCode::Loop => {
                        let offset = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        frame.ip -= offset;
                    }
                    OpCode::Call => {
                        let arg_count = frame.function.chunk.code[frame.ip] as usize;
                        frame.ip += 1;

                        let callee_idx = self.stack.len() - 1 - arg_count;
                        let callee = self.stack[callee_idx].clone();

                        match callee {
                            Value::Function(func) => {
                                let mut current_args = arg_count;
                                if current_args < func.arity {
                                    return Err(format!("Function '{}' expects at least {} arguments but got {}", func.name, func.arity, current_args));
                                }
                                if current_args > func.max_arity && !func.has_varargs {
                                    return Err(format!("Function '{}' expects at most {} arguments but got {}", func.name, func.max_arity, current_args));
                                }
                                while current_args < func.max_arity {
                                    self.stack.push(Value::DefaultSentinel);
                                    current_args += 1;
                                }
                                if func.has_varargs {
                                    if arg_count > func.max_arity {
                                        let vararg_count = arg_count - func.max_arity;
                                        let start = self.stack.len() - vararg_count;
                                        let varargs: Vec<Value> = self.stack.drain(start..).collect();
                                        self.stack.push(Value::tuple(varargs));
                                    } else {
                                        self.stack.push(Value::tuple(Vec::new()));
                                    }
                                }
                                if func.has_kwargs {
                                    self.stack.push(Value::map(HashMap::new()));
                                }
                                self.frames.push(frame);
                                frame = CallFrame {
                                    function: func,
                                    ip: 0,
                                    stack_offset: callee_idx,
                                    constructor_instance: None,
                                };
                            }
                            Value::Closure { function: func, upvalues } => {
                                let mut current_args = arg_count;
                                if current_args < func.arity {
                                    return Err(format!("Function '{}' expects at least {} arguments but got {}", func.name, func.arity, current_args));
                                }
                                if current_args > func.max_arity && !func.has_varargs {
                                    return Err(format!("Function '{}' expects at most {} arguments but got {}", func.name, func.max_arity, current_args));
                                }
                                while current_args < func.max_arity {
                                    self.stack.push(Value::DefaultSentinel);
                                    current_args += 1;
                                }
                                if func.has_varargs {
                                    if arg_count > func.max_arity {
                                        let vararg_count = arg_count - func.max_arity;
                                        let start = self.stack.len() - vararg_count;
                                        let varargs: Vec<Value> = self.stack.drain(start..).collect();
                                        self.stack.push(Value::tuple(varargs));
                                    } else {
                                        self.stack.push(Value::tuple(Vec::new()));
                                    }
                                }
                                if func.has_kwargs {
                                    self.stack.push(Value::map(HashMap::new()));
                                }
                                for uv in upvalues.iter() {
                                    self.stack.push(uv.clone());
                                }
                                self.frames.push(frame);
                                frame = CallFrame {
                                    function: func,
                                    ip: 0,
                                    stack_offset: callee_idx,
                                    constructor_instance: None,
                                };
                            }
                            Value::BoundUserMethod { receiver, function } => {
                                self.stack.insert(callee_idx + 1, Value::ClassInstance((*receiver).clone()));
                                self.stack[callee_idx] = Value::Function(function.clone());
                                let new_arg_count = arg_count + 1;
                                let func = function;
                                let mut current_args = new_arg_count;
                                if current_args < func.arity {
                                    return Err(format!("Method '{}' expects at least {} arguments but got {}", func.name, func.arity, current_args));
                                }
                                if current_args > func.max_arity && !func.has_varargs {
                                    return Err(format!("Method '{}' expects at most {} arguments but got {}", func.name, func.max_arity, current_args));
                                }
                                while current_args < func.max_arity {
                                    self.stack.push(Value::DefaultSentinel);
                                    current_args += 1;
                                }
                                if func.has_varargs {
                                    if new_arg_count > func.max_arity {
                                        let vararg_count = new_arg_count - func.max_arity;
                                        let start = self.stack.len() - vararg_count;
                                        let varargs: Vec<Value> = self.stack.drain(start..).collect();
                                        self.stack.push(Value::tuple(varargs));
                                    } else {
                                        self.stack.push(Value::tuple(Vec::new()));
                                    }
                                }
                                if func.has_kwargs {
                                    self.stack.push(Value::map(HashMap::new()));
                                }
                                self.frames.push(frame);
                                frame = CallFrame {
                                    function: func,
                                    ip: 0,
                                    stack_offset: callee_idx,
                                    constructor_instance: None,
                                };
                            }
                            Value::ClassDef(def) => {
                                let instance = Value::ClassInstance(super::value::ClassInstance {
                                    class: def.clone(),
                                    fields: Arc::new(parking_lot::Mutex::new(HashMap::new())),
                                });
                                let ctor = Self::find_method(&def, "__init__", &self.globals)
                                    .or_else(|| Self::find_method(&def, "init", &self.globals));
                                if let Some(init_fn) = ctor {
                                    self.stack.insert(callee_idx + 1, instance.clone());
                                    self.stack[callee_idx] = Value::Function(init_fn.clone());
                                    let new_arg_count = arg_count + 1;
                                    let func = init_fn;
                                    let mut current_args = new_arg_count;
                                    if current_args < func.arity {
                                        return Err(format!("Constructor '{}' expects at least {} arguments but got {}", func.name, func.arity, current_args));
                                    }
                                    if current_args > func.max_arity && !func.has_varargs {
                                        return Err(format!("Constructor '{}' expects at most {} arguments but got {}", func.name, func.max_arity, current_args));
                                    }
                                    while current_args < func.max_arity {
                                        self.stack.push(Value::DefaultSentinel);
                                        current_args += 1;
                                    }
                                    if func.has_varargs {
                                        if new_arg_count > func.max_arity {
                                            let vararg_count = new_arg_count - func.max_arity;
                                            let start = self.stack.len() - vararg_count;
                                            let varargs: Vec<Value> = self.stack.drain(start..).collect();
                                            self.stack.push(Value::tuple(varargs));
                                        } else {
                                            self.stack.push(Value::tuple(Vec::new()));
                                        }
                                    }
                                    if func.has_kwargs {
                                        self.stack.push(Value::map(HashMap::new()));
                                    }
                                    self.frames.push(frame);
                                    frame = CallFrame {
                                        function: func,
                                        ip: 0,
                                        stack_offset: callee_idx,
                                        constructor_instance: Some(instance),
                                    };
                                } else {
                                    if arg_count > 0 {
                                        return Err(format!("Class '{}' takes no arguments", def.name));
                                    }
                                    self.stack.truncate(callee_idx);
                                    self.stack.push(instance);
                                }
                            }
                            Value::Native(name, native_fn) => {
                                let args: Vec<Value> = self.stack[callee_idx + 1..].to_vec();
                                if name == "len" && !args.is_empty() {
                                    if let Value::ClassInstance(inst) = &args[0] {
                                        if let Some(m) = Self::find_method(&inst.class, "__len__", &self.globals) {
                                            let inst_val = args[0].clone();
                                            let res = self.call_function(m, vec![inst_val])?;
                                            self.stack.truncate(callee_idx);
                                            self.stack.push(res);
                                            continue;
                                        }
                                    }
                                }
                                if name == "str" && !args.is_empty() {
                                    if let Value::ClassInstance(inst) = &args[0] {
                                        if let Some(m) = Self::find_method(&inst.class, "__str__", &self.globals).or_else(|| Self::find_method(&inst.class, "__repr__", &self.globals)) {
                                            let inst_val = args[0].clone();
                                            let res = self.call_function(m, vec![inst_val])?;
                                            self.stack.truncate(callee_idx);
                                            self.stack.push(res);
                                            continue;
                                        }
                                    }
                                }
                                if (name == "print" || name == "println") && !args.is_empty() {
                                    let mut has_custom_str = false;
                                    for a in args.iter() {
                                        if let Value::ClassInstance(inst) = a {
                                            if Self::find_method(&inst.class, "__str__", &self.globals).is_some() || Self::find_method(&inst.class, "__repr__", &self.globals).is_some() {
                                                has_custom_str = true;
                                                break;
                                            }
                                        }
                                    }
                                    if has_custom_str {
                                        let mut formatted_args = Vec::new();
                                        for a in args.iter() {
                                            if let Value::ClassInstance(inst) = a {
                                                if let Some(m) = Self::find_method(&inst.class, "__str__", &self.globals).or_else(|| Self::find_method(&inst.class, "__repr__", &self.globals)) {
                                                    let res = self.call_function(m, vec![a.clone()])?;
                                                    formatted_args.push(res);
                                                    continue;
                                                }
                                            }
                                            formatted_args.push(a.clone());
                                        }
                                        for (i, arg) in formatted_args.iter().enumerate() {
                                            if i > 0 { print!(" "); }
                                            print!("{}", arg);
                                        }
                                        println!();
                                        self.stack.truncate(callee_idx);
                                        self.stack.push(Value::Nil);
                                        continue;
                                    }
                                }
                                let result = match native_fn(&args) {
                                    Ok(res) => res,
                                    Err(err) => {
                                        if !self.exception_handlers.is_empty() {
                                            self.handle_exception(&mut frame, Value::string(err))?;
                                            continue;
                                        } else {
                                            return Err(err);
                                        }
                                    }
                                };
                                self.stack.truncate(callee_idx);
                                self.stack.push(result);
                            }
                            Value::BoundMethod { receiver, method } => {
                                let args = &self.stack[callee_idx + 1..];
                                let result = match Self::invoke_bound_method(&receiver, &method, args) {
                                    Ok(res) => res,
                                    Err(err) => {
                                        if !self.exception_handlers.is_empty() {
                                            self.handle_exception(&mut frame, Value::string(err))?;
                                            continue;
                                        } else {
                                            return Err(err);
                                        }
                                    }
                                };
                                self.stack.truncate(callee_idx);
                                self.stack.push(result);
                            }
                            Value::StructDef(def) => {
                                if arg_count != def.fields.len() {
                                    return Err(format!("Struct '{}' expects {} fields but got {}", def.name, def.fields.len(), arg_count));
                                }
                                let args = &self.stack[callee_idx + 1..];
                                let mut fields_map = HashMap::new();
                                for (i, f_name) in def.fields.iter().enumerate() {
                                    fields_map.insert(f_name.clone(), args[i].clone());
                                }
                                let instance = Value::StructInstance(super::value::StructInstance {
                                    name: def.name.clone(),
                                    fields: Arc::new(parking_lot::Mutex::new(fields_map)),
                                });
                                self.stack.truncate(callee_idx);
                                self.stack.push(instance);
                            }
                            other => return Err(format!("Cannot call non-callable type '{}'", other.type_name())),
                        }
                    }
                    OpCode::CallKw => {
                        let pos_count = frame.function.chunk.code[frame.ip] as usize;
                        let kw_count = frame.function.chunk.code[frame.ip + 1] as usize;
                        frame.ip += 2;

                        let mut raw_kwargs = Vec::new();
                        for _ in 0..kw_count {
                            let val = self.stack.pop().unwrap();
                            let name_val = self.stack.pop().unwrap();
                            let name = match name_val {
                                Value::String(s) => (*s).clone(),
                                _ => format!("{}", name_val),
                            };
                            raw_kwargs.push((name, val));
                        }
                        raw_kwargs.reverse();

                        let callee_idx = self.stack.len() - 1 - pos_count;
                        let callee = self.stack[callee_idx].clone();

                        let mut pos_args: Vec<Value> = self.stack.drain(callee_idx + 1..).collect();
                        let mut kw_map = HashMap::new();

                        for (name, val) in raw_kwargs {
                            if name == "*" {
                                match val {
                                    Value::Array(arr) => pos_args.extend(arr.lock().clone()),
                                    Value::Tuple(tup) => pos_args.extend(tup.as_ref().clone()),
                                    Value::Set(set) => pos_args.extend(set.lock().clone()),
                                    other => return Err(format!("Cannot unpack non-iterable type '{}' with *", other.type_name())),
                                }
                            } else if name == "**" {
                                match val {
                                    Value::Map(m) => {
                                        for (k, v) in m.lock().iter() {
                                            kw_map.insert(k.clone(), v.clone());
                                        }
                                    }
                                    other => return Err(format!("Cannot unpack non-mapping type '{}' with **", other.type_name())),
                                }
                            } else {
                                kw_map.insert(name, val);
                            }
                        }

                        let mut constructor_instance_opt = None;
                        let (func_opt, upvalues_opt) = match &callee {
                            Value::Function(func) => (Some(func.clone()), None),
                            Value::Closure { function, upvalues } => (Some(function.clone()), Some(upvalues.clone())),
                            Value::BoundUserMethod { receiver, function } => {
                                pos_args.insert(0, Value::ClassInstance((**receiver).clone()));
                                (Some(function.clone()), None)
                            }
                            Value::ClassDef(class) => {
                                let instance = Value::ClassInstance(super::value::ClassInstance {
                                    class: class.clone(),
                                    fields: Arc::new(parking_lot::Mutex::new(HashMap::new())),
                                });
                                let ctor = Self::find_method(class, "__init__", &self.globals)
                                    .or_else(|| Self::find_method(class, "init", &self.globals));
                                if let Some(init_fn) = ctor {
                                    pos_args.insert(0, instance.clone());
                                    constructor_instance_opt = Some(instance);
                                    (Some(init_fn), None)
                                } else {
                                    if !pos_args.is_empty() || !kw_map.is_empty() {
                                        return Err(format!("Class '{}' takes no arguments", class.name));
                                    }
                                    self.stack.truncate(callee_idx);
                                    self.stack.push(instance);
                                    continue;
                                }
                            }
                            _ => (None, None),
                        };

                        if let Some(func) = func_opt {
                            let mut final_args = Vec::new();
                            let mut varargs = Vec::new();

                            for (i, p_name) in func.param_names.iter().enumerate() {
                                if i < pos_args.len() {
                                    final_args.push(pos_args[i].clone());
                                    if kw_map.contains_key(p_name) {
                                        return Err(format!("Argument '{}' given by both positional and keyword argument", p_name));
                                    }
                                } else if let Some(kw_val) = kw_map.remove(p_name) {
                                    final_args.push(kw_val);
                                } else {
                                    final_args.push(Value::DefaultSentinel);
                                }
                            }

                            if pos_args.len() > func.param_names.len() {
                                if func.has_varargs {
                                    for arg in &pos_args[func.param_names.len()..] {
                                        varargs.push(arg.clone());
                                    }
                                } else {
                                    return Err(format!("Function '{}' takes {} positional arguments but {} were given", func.name, func.param_names.len(), pos_args.len()));
                                }
                            }

                            if func.has_varargs {
                                final_args.push(Value::tuple(varargs));
                            }

                            if func.has_kwargs {
                                final_args.push(Value::map(kw_map));
                            } else if !kw_map.is_empty() {
                                let first_key = kw_map.keys().next().unwrap();
                                return Err(format!("Function '{}' got an unexpected keyword argument '{}'", func.name, first_key));
                            }

                            for arg in final_args {
                                self.stack.push(arg);
                            }
                            if let Some(uv) = upvalues_opt {
                                for u in uv.iter() {
                                    self.stack.push(u.clone());
                                }
                            }

                            self.frames.push(frame);
                            frame = CallFrame {
                                function: func,
                                ip: 0,
                                stack_offset: callee_idx,
                                constructor_instance: constructor_instance_opt,
                            };
                        } else {
                            match callee {
                            Value::StructDef(def) => {
                                let mut fields_map = HashMap::new();
                                for (i, f_name) in def.fields.iter().enumerate() {
                                    if i < pos_args.len() {
                                        fields_map.insert(f_name.clone(), pos_args[i].clone());
                                    } else if let Some(kw_val) = kw_map.remove(f_name) {
                                        fields_map.insert(f_name.clone(), kw_val);
                                    } else {
                                        return Err(format!("Missing field '{}' for struct '{}'", f_name, def.name));
                                    }
                                }
                                let instance = Value::StructInstance(super::value::StructInstance {
                                    name: def.name.clone(),
                                    fields: Arc::new(parking_lot::Mutex::new(fields_map)),
                                });
                                self.stack.truncate(callee_idx);
                                self.stack.push(instance);
                            }
                            Value::BoundMethod { receiver, method } => {
                                let result = Self::invoke_bound_method(&receiver, &method, &pos_args)?;
                                self.stack.truncate(callee_idx);
                                self.stack.push(result);
                            }
                            Value::Native(_, native_fn) => {
                                let result = native_fn(&pos_args)?;
                                self.stack.truncate(callee_idx);
                                self.stack.push(result);
                            }
                            other => return Err(format!("Cannot call non-callable type '{}'", other.type_name())),
                            }
                        }
                    }
                    OpCode::BuildClosure => {
                        let count = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let start = self.stack.len() - count;
                        let upvalues: Vec<Value> = self.stack.drain(start..).collect();
                        let fn_val = self.stack.pop().unwrap();
                        match fn_val {
                            Value::Function(func) => {
                                self.stack.push(Value::Closure {
                                    function: func,
                                    upvalues: Arc::new(upvalues),
                                });
                            }
                            _ => return Err("BuildClosure expected function on stack".to_string()),
                        }
                    }
                    OpCode::Super => {
                        let _class_name_val = self.stack.pop().unwrap_or(Value::Nil);
                        let receiver_val = self.stack.pop().unwrap_or(Value::Nil);
                        if let Value::ClassInstance(inst) = receiver_val {
                            self.stack.push(Value::SuperInstance {
                                receiver: Arc::new(inst.clone()),
                                class: inst.class.clone(),
                            });
                        } else {
                            return Err("super() must be called on a class instance".to_string());
                        }
                    }
                    OpCode::Return => {
                        let result = if let Some(ref inst) = frame.constructor_instance {
                            self.stack.pop();
                            inst.clone()
                        } else {
                            self.stack.pop().unwrap_or(Value::Nil)
                        };
                        self.stack.truncate(frame.stack_offset);
                        self.stack.push(result);

                        if self.frames.is_empty() {
                            return Ok(self.stack.pop().unwrap_or(Value::Nil));
                        } else {
                            frame = self.frames.pop().unwrap();
                        }
                    }
                    OpCode::BuildArray => {
                        let count = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let start = self.stack.len() - count;
                        let elements: Vec<Value> = self.stack.drain(start..).collect();
                        self.stack.push(Value::array(elements));
                    }
                    OpCode::BuildTuple => {
                        let count = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let start = self.stack.len() - count;
                        let elements: Vec<Value> = self.stack.drain(start..).collect();
                        self.stack.push(Value::tuple(elements));
                    }
                    OpCode::BuildSet => {
                        let count = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let start = self.stack.len() - count;
                        let elements: Vec<Value> = self.stack.drain(start..).collect();
                        self.stack.push(Value::set(elements));
                    }
                    OpCode::UnpackSequence => {
                        let count = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let val = self.stack.pop().unwrap_or(Value::Nil);
                        let elements: Vec<Value> = match val {
                            Value::Tuple(tup) => tup.as_ref().clone(),
                            Value::Array(arr) => arr.lock().clone(),
                            other => return Err(format!("Cannot unpack non-sequence type '{}'", other.type_name())),
                        };
                        if elements.len() < count {
                            return Err(format!("Not enough values to unpack (expected {}, got {})", count, elements.len()));
                        }
                        if elements.len() > count {
                            return Err(format!("Too many values to unpack (expected {}, got {})", count, elements.len()));
                        }
                        for el in elements.into_iter().rev() {
                            self.stack.push(el);
                        }
                    }
                    OpCode::BuildMap => {
                        let count = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let mut map = HashMap::with_capacity(count);
                        let start = self.stack.len() - (count * 2);
                        for i in 0..count {
                            let k_val = &self.stack[start + i * 2];
                            let v_val = &self.stack[start + i * 2 + 1];
                            let k = match k_val {
                                Value::String(s) => (**s).clone(),
                                other => format!("{}", other),
                            };
                            map.insert(k, v_val.clone());
                        }
                        self.stack.truncate(start);
                        self.stack.push(Value::map(map));
                    }
                    OpCode::IndexGet => {
                        let idx_val = self.stack.pop().unwrap();
                        let target = self.stack.pop().unwrap();
                        match (target, idx_val) {
                            (Value::Array(arr), Value::Int(i)) => {
                                let items = arr.lock();
                                let actual_idx = if i < 0 {
                                    (items.len() as i64 + i) as usize
                                } else {
                                    i as usize
                                };
                                if actual_idx < items.len() {
                                    self.stack.push(items[actual_idx].clone());
                                } else {
                                    self.handle_exception(&mut frame, Value::string(format!("IndexError: Array index out of bounds: {}", i)))?;
                                    continue;
                                }
                            }
                            (Value::Array(arr), Value::String(method)) => {
                                self.stack.push(Value::BoundMethod {
                                    receiver: Arc::new(Value::Array(arr)),
                                    method: method.to_string(),
                                });
                            }
                            (Value::Tuple(tup), Value::Int(i)) => {
                                let actual_idx = if i < 0 {
                                    (tup.len() as i64 + i) as usize
                                } else {
                                    i as usize
                                };
                                if actual_idx < tup.len() {
                                    self.stack.push(tup[actual_idx].clone());
                                } else {
                                    return Err(format!("Tuple index out of bounds: {}", i));
                                }
                            }
                            (Value::Tuple(tup), Value::String(method)) => {
                                self.stack.push(Value::BoundMethod {
                                    receiver: Arc::new(Value::Tuple(tup)),
                                    method: method.to_string(),
                                });
                            }
                            (Value::Set(s), Value::Int(i)) => {
                                let items = s.lock();
                                let actual_idx = if i < 0 {
                                    (items.len() as i64 + i) as usize
                                } else {
                                    i as usize
                                };
                                if actual_idx < items.len() {
                                    self.stack.push(items[actual_idx].clone());
                                } else {
                                    return Err(format!("Set index out of bounds: {}", i));
                                }
                            }
                            (Value::Set(s), Value::String(method)) => {
                                self.stack.push(Value::BoundMethod {
                                    receiver: Arc::new(Value::Set(s)),
                                    method: method.to_string(),
                                });
                            }
                            (Value::Map(m), Value::Int(i)) => {
                                let map = m.lock();
                                let mut keys: Vec<String> = map.keys().cloned().collect();
                                keys.sort();
                                let actual_idx = if i < 0 {
                                    (keys.len() as i64 + i) as usize
                                } else {
                                    i as usize
                                };
                                if actual_idx < keys.len() {
                                    self.stack.push(Value::string(keys[actual_idx].clone()));
                                } else {
                                    return Err(format!("Dictionary index out of bounds: {}", i));
                                }
                            }
                            (Value::Map(m), key) => {
                                let map = m.lock();
                                match &key {
                                    Value::String(s) => {
                                        if let Some(val) = map.get(s.as_str()) {
                                            self.stack.push(val.clone());
                                        } else if matches!(s.as_str(), "keys" | "values" | "items" | "entries" | "get" | "contains" | "has" | "remove" | "pop" | "clear" | "update" | "len") {
                                            self.stack.push(Value::BoundMethod {
                                                receiver: Arc::new(Value::Map(m.clone())),
                                                method: (**s).clone(),
                                            });
                                        } else {
                                            self.stack.push(Value::Nil);
                                        }
                                    }
                                    other => {
                                        let k = format!("{}", other);
                                        if let Some(val) = map.get(&k) {
                                            self.stack.push(val.clone());
                                        } else if matches!(k.as_str(), "keys" | "values" | "items" | "entries" | "get" | "contains" | "has" | "remove" | "pop" | "clear" | "update" | "len") {
                                            self.stack.push(Value::BoundMethod {
                                                receiver: Arc::new(Value::Map(m.clone())),
                                                method: k,
                                            });
                                        } else {
                                            self.stack.push(Value::Nil);
                                        }
                                    }
                                }
                            }
                            (Value::StructInstance(inst), key) => {
                                let k = format!("{}", key);
                                let fields = inst.fields.lock();
                                let val = fields.get(&k).cloned().unwrap_or(Value::Nil);
                                self.stack.push(val);
                            }
                            (Value::ClassInstance(inst), key) => {
                                let k = format!("{}", key);
                                let field_val = inst.fields.lock().get(&k).cloned();
                                if let Some(val) = field_val {
                                    self.stack.push(val);
                                } else if let Some(m) = Self::find_method(&inst.class, &k, &self.globals) {
                                    self.stack.push(Value::BoundUserMethod {
                                        receiver: Arc::new(inst.clone()),
                                        function: m,
                                    });
                                } else if let Some(v) = Self::find_class_var(&inst.class, &k, &self.globals) {
                                    self.stack.push(v);
                                } else if let Some(getitem) = Self::find_method(&inst.class, "__getitem__", &self.globals) {
                                    let res = self.call_function(getitem, vec![Value::ClassInstance(inst.clone()), key])?;
                                    self.stack.push(res);
                                } else {
                                    return Err(format!("'{}' object has no attribute or item '{}'", inst.class.name, k));
                                }
                            }
                            (Value::ClassDef(class), key) => {
                                let k = format!("{}", key);
                                if let Some(v) = Self::find_class_var(&class, &k, &self.globals) {
                                    self.stack.push(v);
                                } else if let Some(m) = Self::find_method(&class, &k, &self.globals) {
                                    self.stack.push(Value::Function(m));
                                } else {
                                    return Err(format!("Class '{}' has no attribute '{}'", class.name, k));
                                }
                            }
                            (Value::SuperInstance { receiver, class }, key) => {
                                let k = format!("{}", key);
                                if let Some(m) = Self::find_method_in_bases(&class, &k, &self.globals) {
                                    self.stack.push(Value::BoundUserMethod {
                                        receiver: receiver.clone(),
                                        function: m,
                                    });
                                } else {
                                    return Err(format!("super object has no attribute '{}'", k));
                                }
                            }
                            (Value::String(s), Value::Int(i)) => {
                                let chars: Vec<char> = s.chars().collect();
                                let idx = if i < 0 { (chars.len() as i64 + i) as usize } else { i as usize };
                                if idx < chars.len() {
                                    self.stack.push(Value::string(chars[idx].to_string()));
                                } else {
                                    return Err("String index out of bounds".to_string());
                                }
                            }
                            (Value::String(s), Value::String(method)) => {
                                self.stack.push(Value::BoundMethod {
                                    receiver: Arc::new(Value::String(s)),
                                    method: method.to_string(),
                                });
                            }
                            (t, key) => return Err(format!("Cannot index into type '{}' with key '{}'", t.type_name(), key)),
                        }
                    }
                    OpCode::IndexSet => {
                        let val = self.stack.pop().unwrap();
                        let idx_val = self.stack.pop().unwrap();
                        let target = self.stack.pop().unwrap();
                        match (target, idx_val) {
                            (Value::Array(arr), Value::Int(i)) => {
                                let mut items = arr.lock();
                                let actual_idx = if i < 0 { (items.len() as i64 + i) as usize } else { i as usize };
                                if actual_idx < items.len() {
                                    items[actual_idx] = val.clone();
                                    self.stack.push(val);
                                } else {
                                    return Err(format!("Array index out of bounds: {}", i));
                                }
                            }
                            (Value::Tuple(_), _) => return Err("Tuples are immutable and cannot be modified".to_string()),
                            (Value::Map(m), key) => {
                                let k = format!("{}", key);
                                m.lock().insert(k, val.clone());
                                self.stack.push(val);
                            }
                            (Value::StructInstance(inst), key) => {
                                let k = format!("{}", key);
                                inst.fields.lock().insert(k, val.clone());
                                self.stack.push(val);
                            }
                            (Value::ClassInstance(inst), key) => {
                                if let Some(setitem) = Self::find_method(&inst.class, "__setitem__", &self.globals) {
                                    let _ = self.call_function(setitem, vec![Value::ClassInstance(inst.clone()), key, val.clone()])?;
                                    self.stack.push(val);
                                } else {
                                    let k = format!("{}", key);
                                    inst.fields.lock().insert(k, val.clone());
                                    self.stack.push(val);
                                }
                            }
                            _ => return Err("Invalid index assignment target".to_string()),
                        }
                    }
                    OpCode::SetAttr => {
                        let val = self.stack.pop().unwrap();
                        let attr_name = self.stack.pop().unwrap();
                        let target = self.stack.pop().unwrap();
                        let k = match attr_name {
                            Value::String(s) => (*s).clone(),
                            _ => format!("{}", attr_name),
                        };
                        match target {
                            Value::ClassInstance(inst) => {
                                inst.fields.lock().insert(k, val.clone());
                                self.stack.push(val);
                            }
                            Value::StructInstance(inst) => {
                                inst.fields.lock().insert(k, val.clone());
                                self.stack.push(val);
                            }
                            Value::ClassDef(class) => {
                                class.class_vars.lock().insert(k, val.clone());
                                self.stack.push(val);
                            }
                            other => return Err(format!("Cannot set attribute '{}' on type '{}'", k, other.type_name())),
                        }
                    }
                    OpCode::Spawn => {
                        let arg_count = frame.function.chunk.code[frame.ip] as usize;
                        frame.ip += 1;

                        let mut args = Vec::with_capacity(arg_count);
                        for _ in 0..arg_count {
                            args.push(self.stack.pop().unwrap());
                        }
                        args.reverse();

                        let fn_val = self.stack.pop().ok_or_else(|| "Stack underflow on spawn".to_string())?;
                        match fn_val {
                            Value::Function(func) => {
                                let globals_clone = self.globals.clone();
                                crate::vm::fiber::global_scheduler().spawn(move || {
                                    let mut fiber_vm = VM::with_globals(globals_clone);
                                    fiber_vm.stack.push(Value::Function(func.clone()));
                                    for arg in args {
                                        fiber_vm.stack.push(arg);
                                    }
                                    if let Err(e) = fiber_vm.interpret_with_stack(func) {
                                        eprintln!("[AETHER Fiber Error] {}", e);
                                    }
                                });
                            }
                            _ => return Err("Spawn requires a function or block".to_string()),
                        }
                    }
                    OpCode::PushExceptionHandler => {
                        let offset = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let handler_ip = frame.ip + offset;
                        self.exception_handlers.push(ExceptionHandler {
                            frame_idx: self.frames.len(),
                            stack_depth: self.stack.len(),
                            handler_ip,
                        });
                    }
                    OpCode::PopExceptionHandler => {
                        self.exception_handlers.pop();
                    }
                    OpCode::Raise => {
                        let exc_val = self.stack.pop().unwrap_or_else(|| Value::string("RuntimeError: exception raised"));
                        self.handle_exception(&mut frame, exc_val)?;
                        continue;
                    }
                    OpCode::ImportModule => {
                        let const_idx = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let mod_name = frame.function.chunk.constants[const_idx].to_string();
                        let module_val = self.load_module(&mod_name)?;
                        self.stack.push(module_val);
                    }
                    OpCode::ImportFrom => {
                        let mod_idx = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;
                        let sym_idx = frame.function.chunk.read_u16(frame.ip) as usize;
                        frame.ip += 2;

                        let mod_name = frame.function.chunk.constants[mod_idx].to_string();
                        let sym_name = frame.function.chunk.constants[sym_idx].to_string();

                        let module_val = self.load_module(&mod_name)?;
                        let sym_val = match module_val {
                            Value::Map(ref m) => {
                                m.lock().get(&sym_name).cloned().unwrap_or_else(|| {
                                    self.globals.get(&sym_name).cloned().unwrap_or(Value::Nil)
                                })
                            }
                            Value::ClassInstance(ref inst) => {
                                inst.fields.lock().get(&sym_name).cloned().unwrap_or(Value::Nil)
                            }
                            _ => self.globals.get(&sym_name).cloned().unwrap_or(Value::Nil),
                        };
                        self.stack.push(sym_val);
                    }
                    OpCode::ArrayPush => {
                        let elem = self.stack.pop().unwrap();
                        let arr_val = self.stack.pop().unwrap();
                        match arr_val {
                            Value::Array(ref a) => {
                                a.lock().push(elem);
                            }
                            _ => return Err("ArrayPush expected array on stack".to_string()),
                        }
                    }
                    OpCode::MapInsert => {
                        let val = self.stack.pop().unwrap();
                        let key = self.stack.pop().unwrap();
                        let map_val = self.stack.pop().unwrap();
                        match map_val {
                            Value::Map(ref m) => {
                                m.lock().insert(key.to_string(), val);
                            }
                            _ => return Err("MapInsert expected map on stack".to_string()),
                        }
                    }
                    _ => {}
                }
            }

            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap_or(Value::Nil));
            }
        }
    }

    fn handle_exception(&mut self, frame: &mut CallFrame, exc_val: Value) -> Result<(), String> {
        if let Some(handler) = self.exception_handlers.pop() {
            while self.frames.len() > handler.frame_idx {
                *frame = self.frames.pop().unwrap();
            }
            frame.ip = handler.handler_ip;
            self.stack.truncate(handler.stack_depth);
            self.stack.push(exc_val);
            Ok(())
        } else {
            Err(format!("Runtime Error: {}", exc_val))
        }
    }

    pub fn load_module(&mut self, mod_name: &str) -> Result<Value, String> {
        if let Some(cached) = self.modules_cache.get(mod_name) {
            return Ok(cached.clone());
        }

        let clean_name = mod_name.to_lowercase();
        let stdlib_module = match clean_name.as_str() {
            "math" => self.globals.get("Math").cloned(),
            "strings" | "string" => self.globals.get("Strings").cloned(),
            "file" | "io" => self.globals.get("File").cloned(),
            "json" => self.globals.get("Json").cloned(),
            "sys" | "system" | "os" => self.globals.get("Sys").cloned(),
            "http" | "net" => self.globals.get("Http").cloned(),
            "crypto" => self.globals.get("Crypto").cloned(),
            "db" | "database" => self.globals.get("Database").cloned(),
            "tensor" | "matrix" => self.globals.get("Tensor").cloned(),
            "ws" | "websocket" => self.globals.get("WebSocket").cloned(),
            "vector" | "vec" => self.globals.get("Vector").cloned(),
            "actor" | "actors" => self.globals.get("Actor").cloned(),
            "wasm" | "edge" => self.globals.get("Wasm").cloned(),
            "rpc" | "pack" | "aetherpack" | "aetherrpc" => self.globals.get("Rpc").cloned(),
            "dataframe" | "df" | "polars" | "arrow" => self.globals.get("DataFrame").cloned(),
            "compute" | "gpu" | "simd" => self.globals.get("Compute").cloned(),
            "timetravel" | "replay" | "omniscient" => self.globals.get("TimeTravel").cloned(),
            "sql" | "aethersql" | "query" => self.globals.get("Sql").cloned(),
            "tui" | "timetraveltui" => self.globals.get("TimeTravelTUI").or_else(|| self.globals.get("__native_tui")).cloned(),
            "flow" | "aetherflow" | "stream" => self.globals.get("Flow").cloned(),
            "proof" | "aetherproof" | "verify" => self.globals.get("Proof").cloned(),
            "graph" | "aethergraph" | "knowledge" => self.globals.get("Graph").cloned(),
            "hotreload" | "live" | "aetherlive" => self.globals.get("HotReload").or_else(|| self.globals.get("__native_hotreload")).cloned(),
            _ => None,
        };

        if let Some(m) = stdlib_module {
            self.modules_cache.insert(mod_name.to_string(), m.clone());
            return Ok(m);
        }

        let mut candidates = vec![
            std::path::PathBuf::from(format!("{}.ae", mod_name)),
            std::path::PathBuf::from(format!("{}.aether", mod_name)),
            std::path::PathBuf::from(format!("libraries/{}.ae", mod_name)),
            std::path::PathBuf::from(format!("libraries/{}.aether", mod_name)),
        ];
        if clean_name.starts_with("aether") && !clean_name.starts_with("aether_") {
            let suffix = &clean_name["aether".len()..];
            candidates.push(std::path::PathBuf::from(format!("libraries/aether_{}.ae", suffix)));
            candidates.push(std::path::PathBuf::from(format!("libraries/aether_{}.aether", suffix)));
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join(format!("{}.ae", mod_name)));
            candidates.push(cwd.join(format!("{}.aether", mod_name)));
            candidates.push(cwd.join(format!("libraries/{}.ae", mod_name)));
            candidates.push(cwd.join(format!("libraries/{}.aether", mod_name)));
        }

        for path in candidates {
            if path.exists() {
                if let Ok(source) = std::fs::read_to_string(&path) {
                    let mut lexer = crate::syntax::lexer::Lexer::new(&source);
                    let tokens = lexer.tokenize().map_err(|(e, _)| format!("Import Error in {}: {}", mod_name, e))?;
                    let mut parser = crate::syntax::parser::Parser::new(tokens);
                    let program = parser.parse_program().map_err(|(e, _)| format!("Import Error in {}: {}", mod_name, e))?;
                    let compiler = crate::vm::compiler::BytecodeCompiler::new("<main>", 0);
                    let compiled_fn = compiler.compile(&program)?;

                    let mut sub_vm = VM::new();
                    sub_vm.modules_cache = self.modules_cache.clone();
                    let _ = sub_vm.interpret(compiled_fn)?;

                    let mut module_exports = HashMap::new();
                    for (k, v) in sub_vm.globals {
                        if !k.starts_with("__") {
                            module_exports.insert(k, v);
                        }
                    }
                    let mod_val = Value::map(module_exports.clone());
                    self.modules_cache.insert(mod_name.to_string(), mod_val.clone());

                    for (k, v) in &module_exports {
                        if !self.globals.contains_key(k) {
                            self.globals.insert(k.clone(), v.clone());
                        }
                    }
                    return Ok(mod_val);
                }
            }
        }

        Err(format!("ImportError: No module named '{}'", mod_name))
    }

    pub fn find_method(class: &super::value::ClassDef, name: &str, globals: &HashMap<String, Value>) -> Option<Arc<CompiledFunction>> {
        if let Some(f) = class.methods.lock().get(name) {
            return Some(f.clone());
        }
        Self::find_method_in_bases(class, name, globals)
    }

    pub fn find_method_in_bases(class: &super::value::ClassDef, name: &str, globals: &HashMap<String, Value>) -> Option<Arc<CompiledFunction>> {
        for base_name in &class.bases {
            if let Some(Value::ClassDef(base_class)) = globals.get(base_name) {
                if let Some(f) = Self::find_method(base_class, name, globals) {
                    return Some(f);
                }
            }
        }
        None
    }

    pub fn find_class_var(class: &super::value::ClassDef, name: &str, globals: &HashMap<String, Value>) -> Option<Value> {
        if let Some(v) = class.class_vars.lock().get(name) {
            return Some(v.clone());
        }
        for base_name in &class.bases {
            if let Some(Value::ClassDef(base_class)) = globals.get(base_name) {
                if let Some(v) = Self::find_class_var(base_class, name, globals) {
                    return Some(v);
                }
            }
        }
        None
    }

    pub fn call_function(&mut self, func: Arc<CompiledFunction>, mut args: Vec<Value>) -> Result<Value, String> {
        let mut sub_vm = Self::with_globals(self.globals.clone());
        sub_vm.modules_cache = self.modules_cache.clone();
        sub_vm.stack.push(Value::Function(func.clone()));
        let mut current_args = args.len();
        while current_args < func.max_arity {
            args.push(Value::DefaultSentinel);
            current_args += 1;
        }
        if func.has_varargs && args.len() <= func.max_arity {
            args.push(Value::tuple(Vec::new()));
        }
        if func.has_kwargs {
            args.push(Value::map(HashMap::new()));
        }
        for arg in args {
            sub_vm.stack.push(arg);
        }
        sub_vm.frames.push(CallFrame {
            function: func,
            ip: 0,
            stack_offset: 0,
            constructor_instance: None,
        });
        let res = sub_vm.run()?;
        for (k, v) in sub_vm.globals {
            self.globals.insert(k, v);
        }
        Ok(res)
    }

    pub fn invoke_bound_method(receiver: &Value, method: &str, args: &[Value]) -> Result<Value, String> {
        match receiver {
            Value::Array(arr) => match method {
                "append" | "push" => {
                    if args.is_empty() { return Err("append() expects 1 argument".to_string()); }
                    arr.lock().push(args[0].clone());
                    Ok(Value::Nil)
                }
                "pop" => {
                    let mut items = arr.lock();
                    if !args.is_empty() {
                        if let Value::Int(i) = &args[0] {
                            let idx = if *i < 0 { (items.len() as i64 + *i) as usize } else { *i as usize };
                            if idx < items.len() {
                                Ok(items.remove(idx))
                            } else {
                                Err("pop() index out of bounds".to_string())
                            }
                        } else {
                            Err("pop() index must be integer".to_string())
                        }
                    } else {
                        Ok(items.pop().unwrap_or(Value::Nil))
                    }
                }
                "insert" => {
                    if args.len() < 2 { return Err("insert(index, value) expects 2 arguments".to_string()); }
                    let idx = match args[0] {
                        Value::Int(i) => i as usize,
                        _ => return Err("insert() index must be an integer".to_string()),
                    };
                    let mut items = arr.lock();
                    let idx = idx.min(items.len());
                    items.insert(idx, args[1].clone());
                    Ok(Value::Nil)
                }
                "remove" => {
                    if args.is_empty() { return Err("remove(value) expects 1 argument".to_string()); }
                    let mut items = arr.lock();
                    if let Some(pos) = items.iter().position(|x| x == &args[0]) {
                        items.remove(pos);
                    }
                    Ok(Value::Nil)
                }
                "clear" => {
                    arr.lock().clear();
                    Ok(Value::Nil)
                }
                "reverse" => {
                    arr.lock().reverse();
                    Ok(Value::Nil)
                }
                "extend" => {
                    if args.is_empty() { return Err("extend(iterable) expects 1 argument".to_string()); }
                    match &args[0] {
                        Value::Array(other) => arr.lock().extend(other.lock().iter().cloned()),
                        Value::Tuple(other) => arr.lock().extend(other.iter().cloned()),
                        Value::Set(other) => arr.lock().extend(other.lock().iter().cloned()),
                        _ => return Err("extend() expects an iterable".to_string()),
                    }
                    Ok(Value::Nil)
                }
                "contains" => {
                    if args.is_empty() { return Err("contains(value) expects 1 argument".to_string()); }
                    Ok(Value::Bool(arr.lock().contains(&args[0])))
                }
                "len" => Ok(Value::Int(arr.lock().len() as i64)),
                _ => Err(format!("Unknown method '{}' on list", method)),
            },
            Value::Tuple(tup) => match method {
                "len" => Ok(Value::Int(tup.len() as i64)),
                "contains" => {
                    if args.is_empty() { return Err("contains(value) expects 1 argument".to_string()); }
                    Ok(Value::Bool(tup.contains(&args[0])))
                }
                "index" => {
                    if args.is_empty() { return Err("index(value) expects 1 argument".to_string()); }
                    if let Some(pos) = tup.iter().position(|x| x == &args[0]) {
                        Ok(Value::Int(pos as i64))
                    } else {
                        Err("Value not found in tuple".to_string())
                    }
                }
                "count" => {
                    if args.is_empty() { return Err("count(value) expects 1 argument".to_string()); }
                    let count = tup.iter().filter(|x| *x == &args[0]).count();
                    Ok(Value::Int(count as i64))
                }
                _ => Err(format!("Unknown method '{}' on tuple", method)),
            },
            Value::Set(s) => match method {
                "add" => {
                    if args.is_empty() { return Err("add(value) expects 1 argument".to_string()); }
                    let mut items = s.lock();
                    if !items.contains(&args[0]) {
                        items.push(args[0].clone());
                    }
                    Ok(Value::Nil)
                }
                "remove" | "discard" => {
                    if args.is_empty() { return Err("remove(value) expects 1 argument".to_string()); }
                    let mut items = s.lock();
                    if let Some(pos) = items.iter().position(|x| x == &args[0]) {
                        items.remove(pos);
                    }
                    Ok(Value::Nil)
                }
                "contains" => {
                    if args.is_empty() { return Err("contains(value) expects 1 argument".to_string()); }
                    Ok(Value::Bool(s.lock().contains(&args[0])))
                }
                "clear" => {
                    s.lock().clear();
                    Ok(Value::Nil)
                }
                "union" => {
                    if args.is_empty() { return Err("union(other) expects 1 argument".to_string()); }
                    let mut combined = s.lock().clone();
                    if let Value::Set(other) = &args[0] {
                        for item in other.lock().iter() {
                            if !combined.contains(item) {
                                combined.push(item.clone());
                            }
                        }
                    }
                    Ok(Value::set(combined))
                }
                "intersection" => {
                    if args.is_empty() { return Err("intersection(other) expects 1 argument".to_string()); }
                    let mut inter = Vec::new();
                    if let Value::Set(other) = &args[0] {
                        let other_items = other.lock();
                        for item in s.lock().iter() {
                            if other_items.contains(item) {
                                inter.push(item.clone());
                            }
                        }
                    }
                    Ok(Value::set(inter))
                }
                "difference" => {
                    if args.is_empty() { return Err("difference(other) expects 1 argument".to_string()); }
                    let mut diff = Vec::new();
                    if let Value::Set(other) = &args[0] {
                        let other_items = other.lock();
                        for item in s.lock().iter() {
                            if !other_items.contains(item) {
                                diff.push(item.clone());
                            }
                        }
                    }
                    Ok(Value::set(diff))
                }
                "len" => Ok(Value::Int(s.lock().len() as i64)),
                _ => Err(format!("Unknown method '{}' on set", method)),
            },
            Value::Map(m) => match method {
                "keys" => {
                    let map = m.lock();
                    let mut keys: Vec<String> = map.keys().cloned().collect();
                    keys.sort();
                    let v_keys: Vec<Value> = keys.into_iter().map(Value::string).collect();
                    Ok(Value::array(v_keys))
                }
                "values" => {
                    let map = m.lock();
                    let mut keys: Vec<String> = map.keys().cloned().collect();
                    keys.sort();
                    let vals: Vec<Value> = keys.into_iter().map(|k| map.get(&k).cloned().unwrap_or(Value::Nil)).collect();
                    Ok(Value::array(vals))
                }
                "items" | "entries" => {
                    let map = m.lock();
                    let mut keys: Vec<String> = map.keys().cloned().collect();
                    keys.sort();
                    let items: Vec<Value> = keys.into_iter().map(|k| {
                        let v = map.get(&k).cloned().unwrap_or(Value::Nil);
                        Value::tuple(vec![Value::string(k), v])
                    }).collect();
                    Ok(Value::array(items))
                }
                "get" => {
                    if args.is_empty() { return Err("get(key, default?) expects at least 1 argument".to_string()); }
                    let key = format!("{}", args[0]);
                    let default_val = if args.len() > 1 { args[1].clone() } else { Value::Nil };
                    let map = m.lock();
                    Ok(map.get(&key).cloned().unwrap_or(default_val))
                }
                "contains" | "has" => {
                    if args.is_empty() { return Err("contains(key) expects 1 argument".to_string()); }
                    let key = format!("{}", args[0]);
                    Ok(Value::Bool(m.lock().contains_key(&key)))
                }
                "remove" | "pop" => {
                    if args.is_empty() { return Err("remove(key) expects 1 argument".to_string()); }
                    let key = format!("{}", args[0]);
                    let val = m.lock().remove(&key).unwrap_or(Value::Nil);
                    Ok(val)
                }
                "clear" => {
                    m.lock().clear();
                    Ok(Value::Nil)
                }
                "update" => {
                    if args.is_empty() { return Err("update(dict) expects 1 argument".to_string()); }
                    if let Value::Map(other) = &args[0] {
                        let mut map = m.lock();
                        for (k, v) in other.lock().iter() {
                            map.insert(k.clone(), v.clone());
                        }
                    }
                    Ok(Value::Nil)
                }
                "len" => Ok(Value::Int(m.lock().len() as i64)),
                _ => Err(format!("Unknown method '{}' on dict", method)),
            },
            Value::String(s) => match method {
                "upper" => Ok(Value::string(s.to_uppercase())),
                "lower" => Ok(Value::string(s.to_lowercase())),
                "trim" | "strip" => Ok(Value::string(s.trim())),
                "split" => {
                    let sep = if !args.is_empty() { format!("{}", args[0]) } else { " ".to_string() };
                    let parts: Vec<Value> = s.split(&sep).map(Value::string).collect();
                    Ok(Value::array(parts))
                }
                "replace" => {
                    if args.len() < 2 { return Err("replace(from, to) expects 2 arguments".to_string()); }
                    let from = format!("{}", args[0]);
                    let to = format!("{}", args[1]);
                    Ok(Value::string(s.replace(&from, &to)))
                }
                "contains" => {
                    if args.is_empty() { return Err("contains(substr) expects 1 argument".to_string()); }
                    let sub = format!("{}", args[0]);
                    Ok(Value::Bool(s.contains(&sub)))
                }
                "startswith" => {
                    if args.is_empty() { return Err("startswith(prefix) expects 1 argument".to_string()); }
                    let prefix = format!("{}", args[0]);
                    Ok(Value::Bool(s.starts_with(&prefix)))
                }
                "endswith" => {
                    if args.is_empty() { return Err("endswith(suffix) expects 1 argument".to_string()); }
                    let suffix = format!("{}", args[0]);
                    Ok(Value::Bool(s.ends_with(&suffix)))
                }
                "len" => Ok(Value::Int(s.chars().count() as i64)),
                _ => Err(format!("Unknown method '{}' on string", method)),
            },
            other => Err(format!("Cannot call method '{}' on type '{}'", method, other.type_name())),
        }
    }
}
