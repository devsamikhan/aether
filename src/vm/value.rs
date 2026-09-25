use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Clone)]
pub struct CompiledFunction {
    pub name: String,
    pub arity: usize,            // min required positional arguments
    pub max_arity: usize,        // total positional parameters
    pub param_names: Vec<String>,
    pub has_varargs: bool,       // *args
    pub varargs_param: Option<String>,
    pub has_kwargs: bool,        // **kwargs
    pub kwargs_param: Option<String>,
    pub chunk: super::bytecode::Chunk,
}

impl CompiledFunction {
    pub fn new(name: impl Into<String>, arity: usize) -> Self {
        let name_str = name.into();
        Self {
            name: name_str,
            arity,
            max_arity: arity,
            param_names: Vec::new(),
            has_varargs: false,
            varargs_param: None,
            has_kwargs: false,
            kwargs_param: None,
            chunk: super::bytecode::Chunk::new(),
        }
    }
}

impl fmt::Debug for CompiledFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fn {}/{}>", self.name, self.arity)
    }
}

impl PartialEq for CompiledFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.arity == other.arity
    }
}

pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

#[derive(Clone)]
pub struct ChannelHandle {
    pub sender: Arc<Mutex<Sender<Value>>>,
    pub receiver: Arc<Mutex<Receiver<Value>>>,
}

impl ChannelHandle {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            sender: Arc::new(Mutex::new(tx)),
            receiver: Arc::new(Mutex::new(rx)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Clone)]
pub struct StructInstance {
    pub name: String,
    pub fields: Arc<Mutex<HashMap<String, Value>>>,
}

#[derive(Clone, Debug)]
pub struct ClassDef {
    pub name: String,
    pub bases: Vec<String>,
    pub methods: Arc<Mutex<HashMap<String, Arc<CompiledFunction>>>>,
    pub class_vars: Arc<Mutex<HashMap<String, Value>>>,
}

#[derive(Clone)]
pub struct ClassInstance {
    pub class: Arc<ClassDef>,
    pub fields: Arc<Mutex<HashMap<String, Value>>>,
}

#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<String>),
    Array(Arc<Mutex<Vec<Value>>>),
    Tuple(Arc<Vec<Value>>),
    Set(Arc<Mutex<Vec<Value>>>),
    Map(Arc<Mutex<HashMap<String, Value>>>),
    Function(Arc<CompiledFunction>),
    Closure {
        function: Arc<CompiledFunction>,
        upvalues: Arc<Vec<Value>>,
    },
    Native(String, NativeFn),
    BoundMethod { receiver: Arc<Value>, method: String },
    BoundUserMethod {
        receiver: Arc<ClassInstance>,
        function: Arc<CompiledFunction>,
    },
    SuperInstance {
        receiver: Arc<ClassInstance>,
        class: Arc<ClassDef>,
    },
    Channel(ChannelHandle),
    StructDef(Arc<StructDef>),
    StructInstance(StructInstance),
    ClassDef(Arc<ClassDef>),
    ClassInstance(ClassInstance),
    DefaultSentinel,
}

impl Value {
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(Arc::new(s.into()))
    }

    pub fn array(elements: Vec<Value>) -> Self {
        Value::Array(Arc::new(Mutex::new(elements)))
    }

    pub fn tuple(elements: Vec<Value>) -> Self {
        Value::Tuple(Arc::new(elements))
    }

    pub fn set(elements: Vec<Value>) -> Self {
        let mut unique = Vec::new();
        for el in elements {
            if !unique.contains(&el) {
                unique.push(el);
            }
        }
        Value::Set(Arc::new(Mutex::new(unique)))
    }

    pub fn map(entries: HashMap<String, Value>) -> Self {
        Value::Map(Arc::new(Mutex::new(entries)))
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.lock().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Set(s) => !s.lock().is_empty(),
            Value::Map(m) => !m.lock().is_empty(),
            Value::DefaultSentinel => false,
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Tuple(_) => "tuple",
            Value::Set(_) => "set",
            Value::Map(_) => "map",
            Value::Function(_) => "function",
            Value::Closure { .. } => "function",
            Value::Native(_, _) => "native_fn",
            Value::BoundMethod { .. } => "bound_method",
            Value::BoundUserMethod { .. } => "method",
            Value::SuperInstance { .. } => "super",
            Value::Channel(_) => "channel",
            Value::StructDef(_) => "struct_def",
            Value::StructInstance(_) => "struct",
            Value::ClassDef(_) => "class",
            Value::ClassInstance(_) => "instance",
            Value::DefaultSentinel => "default",
        }
    }

    pub fn from_json(jv: &serde_json::Value) -> Self {
        match jv {
            serde_json::Value::Null => Value::Nil,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Nil
                }
            }
            serde_json::Value::String(s) => Value::string(s.clone()),
            serde_json::Value::Array(arr) => {
                let items: Vec<Value> = arr.iter().map(Self::from_json).collect();
                Value::array(items)
            }
            serde_json::Value::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), Self::from_json(v));
                }
                Value::map(map)
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Nil => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int(i) => serde_json::Value::Number((*i).into()),
            Value::Float(f) => {
                if let Some(num) = serde_json::Number::from_f64(*f) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::Null
                }
            }
            Value::String(s) => serde_json::Value::String((**s).clone()),
            Value::Array(arr) => {
                let items: Vec<serde_json::Value> = arr.lock().iter().map(|v| v.to_json()).collect();
                serde_json::Value::Array(items)
            }
            Value::Tuple(tup) => {
                let items: Vec<serde_json::Value> = tup.iter().map(|v| v.to_json()).collect();
                serde_json::Value::Array(items)
            }
            Value::Set(s) => {
                let items: Vec<serde_json::Value> = s.lock().iter().map(|v| v.to_json()).collect();
                serde_json::Value::Array(items)
            }
            Value::Map(m) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in m.lock().iter() {
                    obj.insert(k.clone(), v.to_json());
                }
                serde_json::Value::Object(obj)
            }
            Value::StructInstance(inst) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in inst.fields.lock().iter() {
                    obj.insert(k.clone(), v.to_json());
                }
                serde_json::Value::Object(obj)
            }
            other => serde_json::Value::String(format!("{}", other)),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                let items = arr.lock();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Tuple(tup) => {
                write!(f, "(")?;
                for (i, item) in tup.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                if tup.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Value::Set(set) => {
                let items = set.lock();
                write!(f, "{{")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "}}")
            }
            Value::Map(map) => {
                let m = map.lock();
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Closure { function, .. } => write!(f, "<closure {}>", function.name),
            Value::Native(name, _) => write!(f, "<native fn {}>", name),
            Value::BoundMethod { receiver, method } => {
                write!(f, "<bound method {} of {}>", method, receiver.type_name())
            }
            Value::Channel(_) => write!(f, "<channel>"),
            Value::StructDef(def) => write!(f, "<struct {}>", def.name),
            Value::StructInstance(inst) => {
                write!(f, "{} {{", inst.name)?;
                let fields = inst.fields.lock();
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::DefaultSentinel => write!(f, "<default>"),
            Value::ClassDef(def) => write!(f, "<class {}>", def.name),
            Value::ClassInstance(inst) => {
                write!(f, "<{} object>", inst.class.name)
            }
            Value::BoundUserMethod { receiver, function } => {
                write!(f, "<bound method {} of <{} object>>", function.name, receiver.class.name)
            }
            Value::SuperInstance { class, .. } => write!(f, "<super: <class {}>>", class.name),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            _ => write!(f, "{}", self),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => *a.lock() == *b.lock(),
            (Value::Tuple(a), Value::Tuple(b)) => *a == *b,
            (Value::Set(a), Value::Set(b)) => {
                let s_a = a.lock();
                let s_b = b.lock();
                if s_a.len() != s_b.len() {
                    false
                } else {
                    s_a.iter().all(|x| s_b.contains(x))
                }
            }
            (Value::Map(a), Value::Map(b)) => *a.lock() == *b.lock(),
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Closure { function: f1, upvalues: u1 }, Value::Closure { function: f2, upvalues: u2 }) => f1 == f2 && u1 == u2,
            (Value::StructDef(a), Value::StructDef(b)) => a.name == b.name && a.fields == b.fields,
            (Value::StructInstance(a), Value::StructInstance(b)) => {
                a.name == b.name && *a.fields.lock() == *b.fields.lock()
            }
            (Value::DefaultSentinel, Value::DefaultSentinel) => true,
            (Value::ClassDef(a), Value::ClassDef(b)) => Arc::ptr_eq(a, b) || (a.name == b.name && a.bases == b.bases),
            (Value::ClassInstance(a), Value::ClassInstance(b)) => Arc::ptr_eq(&a.fields, &b.fields),
            (Value::SuperInstance { receiver: r1, class: c1 }, Value::SuperInstance { receiver: r2, class: c2 }) => {
                Arc::ptr_eq(&r1.fields, &r2.fields) && Arc::ptr_eq(c1, c2)
            }
            (Value::BoundUserMethod { receiver: r1, function: f1 }, Value::BoundUserMethod { receiver: r2, function: f2 }) => {
                Arc::ptr_eq(&r1.fields, &r2.fields) && f1 == f2
            }
            _ => false,
        }
    }
}
