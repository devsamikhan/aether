use super::value::Value;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// Fast zero-dependency pseudo-random number generator (SplitMix64)
static RNG_STATE: AtomicU64 = AtomicU64::new(0x853c49e6748fea9b);

fn next_random_u64() -> u64 {
    let mut z = RNG_STATE.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn random_f64() -> f64 {
    (next_random_u64() as f64) / (u64::MAX as f64)
}

pub fn register_stdlib(globals: &mut HashMap<String, Value>) {
    // Seed the PRNG with system time
    if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
        RNG_STATE.store(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    // =========================================================================
    // 1. File Module
    // =========================================================================
    let mut file_module = HashMap::new();

    file_module.insert("read".to_string(), Value::Native("File.read".into(), |args| {
        if args.is_empty() { return Err("File.read(path) requires path string".into()); }
        let path = format!("{}", args[0]);
        let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file '{}': {}", path, e))?;
        Ok(Value::string(content))
    }));

    file_module.insert("write".to_string(), Value::Native("File.write".into(), |args| {
        if args.len() < 2 { return Err("File.write(path, content) requires 2 arguments".into()); }
        let path = format!("{}", args[0]);
        let content = format!("{}", args[1]);
        fs::write(&path, content).map_err(|e| format!("Failed to write file '{}': {}", path, e))?;
        Ok(Value::Bool(true))
    }));

    file_module.insert("append".to_string(), Value::Native("File.append".into(), |args| {
        if args.len() < 2 { return Err("File.append(path, content) requires 2 arguments".into()); }
        let path = format!("{}", args[0]);
        let content = format!("{}", args[1]);
        let mut file = OpenOptions::new().create(true).append(true).open(&path)
            .map_err(|e| format!("Failed to open file for append '{}': {}", path, e))?;
        file.write_all(content.as_bytes()).map_err(|e| format!("Failed to append to '{}': {}", path, e))?;
        Ok(Value::Bool(true))
    }));

    file_module.insert("exists".to_string(), Value::Native("File.exists".into(), |args| {
        if args.is_empty() { return Err("File.exists(path) requires path".into()); }
        let path = format!("{}", args[0]);
        Ok(Value::Bool(Path::new(&path).exists()))
    }));

    file_module.insert("delete".to_string(), Value::Native("File.delete".into(), |args| {
        if args.is_empty() { return Err("File.delete(path) requires path".into()); }
        let path = format!("{}", args[0]);
        fs::remove_file(&path).map_err(|e| format!("Failed to delete '{}': {}", path, e))?;
        Ok(Value::Bool(true))
    }));

    file_module.insert("lines".to_string(), Value::Native("File.lines".into(), |args| {
        if args.is_empty() { return Err("File.lines(path) requires path".into()); }
        let path = format!("{}", args[0]);
        let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file '{}': {}", path, e))?;
        let lines: Vec<Value> = content.lines().map(Value::string).collect();
        Ok(Value::array(lines))
    }));

    globals.insert("File".to_string(), Value::map(file_module));

    // =========================================================================
    // 2. JSON Module
    // =========================================================================
    let mut json_module = HashMap::new();

    json_module.insert("parse".to_string(), Value::Native("Json.parse".into(), |args| {
        if args.is_empty() { return Err("Json.parse(string) requires string".into()); }
        let s = format!("{}", args[0]);
        let parsed: serde_json::Value = serde_json::from_str(&s)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        Ok(Value::from_json(&parsed))
    }));

    json_module.insert("stringify".to_string(), Value::Native("Json.stringify".into(), |args| {
        if args.is_empty() { return Err("Json.stringify(value) requires value".into()); }
        let json_val = args[0].to_json();
        let s = serde_json::to_string(&json_val).map_err(|e| format!("JSON serialization error: {}", e))?;
        Ok(Value::string(s))
    }));

    json_module.insert("pretty".to_string(), Value::Native("Json.pretty".into(), |args| {
        if args.is_empty() { return Err("Json.pretty(value) requires value".into()); }
        let json_val = args[0].to_json();
        let s = serde_json::to_string_pretty(&json_val).map_err(|e| format!("JSON serialization error: {}", e))?;
        Ok(Value::string(s))
    }));

    globals.insert("Json".to_string(), Value::map(json_module));

    // =========================================================================
    // 3. Math Module
    // =========================================================================
    let mut math_module = HashMap::new();

    math_module.insert("PI".to_string(), Value::Float(std::f64::consts::PI));
    math_module.insert("E".to_string(), Value::Float(std::f64::consts::E));

    math_module.insert("sqrt".to_string(), Value::Native("Math.sqrt".into(), |args| {
        if args.is_empty() { return Err("Math.sqrt requires number".into()); }
        let n = match args[0] {
            Value::Int(i) => i as f64,
            Value::Float(f) => f,
            _ => return Err("Math.sqrt requires number".into()),
        };
        Ok(Value::Float(n.sqrt()))
    }));

    math_module.insert("abs".to_string(), Value::Native("Math.abs".into(), |args| {
        if args.is_empty() { return Err("Math.abs requires number".into()); }
        match args[0] {
            Value::Int(i) => Ok(Value::Int(i.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            _ => Err("Math.abs requires number".into()),
        }
    }));

    math_module.insert("sin".to_string(), Value::Native("Math.sin".into(), |args| {
        let n = match args.first() {
            Some(Value::Int(i)) => *i as f64,
            Some(Value::Float(f)) => *f,
            _ => return Err("Math.sin requires number".into()),
        };
        Ok(Value::Float(n.sin()))
    }));

    math_module.insert("cos".to_string(), Value::Native("Math.cos".into(), |args| {
        let n = match args.first() {
            Some(Value::Int(i)) => *i as f64,
            Some(Value::Float(f)) => *f,
            _ => return Err("Math.cos requires number".into()),
        };
        Ok(Value::Float(n.cos()))
    }));

    math_module.insert("tan".to_string(), Value::Native("Math.tan".into(), |args| {
        let n = match args.first() {
            Some(Value::Int(i)) => *i as f64,
            Some(Value::Float(f)) => *f,
            _ => return Err("Math.tan requires number".into()),
        };
        Ok(Value::Float(n.tan()))
    }));

    math_module.insert("floor".to_string(), Value::Native("Math.floor".into(), |args| {
        let n = match args.first() {
            Some(Value::Int(i)) => return Ok(Value::Int(*i)),
            Some(Value::Float(f)) => *f,
            _ => return Err("Math.floor requires number".into()),
        };
        Ok(Value::Int(n.floor() as i64))
    }));

    math_module.insert("ceil".to_string(), Value::Native("Math.ceil".into(), |args| {
        let n = match args.first() {
            Some(Value::Int(i)) => return Ok(Value::Int(*i)),
            Some(Value::Float(f)) => *f,
            _ => return Err("Math.ceil requires number".into()),
        };
        Ok(Value::Int(n.ceil() as i64))
    }));

    math_module.insert("round".to_string(), Value::Native("Math.round".into(), |args| {
        let n = match args.first() {
            Some(Value::Int(i)) => return Ok(Value::Int(*i)),
            Some(Value::Float(f)) => *f,
            _ => return Err("Math.round requires number".into()),
        };
        Ok(Value::Int(n.round() as i64))
    }));

    math_module.insert("min".to_string(), Value::Native("Math.min".into(), |args| {
        if args.len() < 2 { return Err("Math.min(a, b) requires 2 numbers".into()); }
        match (&args[0], &args[1]) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
            _ => Err("Math.min requires numbers".into()),
        }
    }));

    math_module.insert("max".to_string(), Value::Native("Math.max".into(), |args| {
        if args.len() < 2 { return Err("Math.max(a, b) requires 2 numbers".into()); }
        match (&args[0], &args[1]) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
            _ => Err("Math.max requires numbers".into()),
        }
    }));

    math_module.insert("random".to_string(), Value::Native("Math.random".into(), |_| {
        Ok(Value::Float(random_f64()))
    }));

    math_module.insert("random_int".to_string(), Value::Native("Math.random_int".into(), |args| {
        if args.len() < 2 { return Err("Math.random_int(min, max) requires min and max".into()); }
        let min = match args[0] { Value::Int(i) => i, _ => return Err("min must be int".into()) };
        let max = match args[1] { Value::Int(i) => i, _ => return Err("max must be int".into()) };
        if min >= max { return Ok(Value::Int(min)); }
        let diff = (max - min) as u64;
        let rand_val = (next_random_u64() % diff) as i64;
        Ok(Value::Int(min + rand_val))
    }));

    math_module.insert("round".to_string(), Value::Native("Math.round".into(), |args| {
        if args.is_empty() { return Err("Math.round(val, decimals?) requires val".into()); }
        let val = match args[0] {
            Value::Float(f) => f,
            Value::Int(i) => i as f64,
            _ => return Err("val must be number".into()),
        };
        let decimals = if args.len() > 1 {
            match args[1] { Value::Int(i) => i.max(0) as usize, _ => 0 }
        } else {
            0
        };
        let factor = 10f64.powi(decimals as i32);
        let rounded = (val * factor).round() / factor;
        Ok(Value::Float(rounded))
    }));

    globals.insert("Math".to_string(), Value::map(math_module));

    // =========================================================================
    // 4. Strings Module
    // =========================================================================
    let mut strings_module = HashMap::new();

    strings_module.insert("split".to_string(), Value::Native("Strings.split".into(), |args| {
        if args.len() < 2 { return Err("Strings.split(str, sep) requires 2 arguments".into()); }
        let s = format!("{}", args[0]);
        let sep = format!("{}", args[1]);
        let parts: Vec<Value> = s.split(&sep).map(Value::string).collect();
        Ok(Value::array(parts))
    }));

    strings_module.insert("join".to_string(), Value::Native("Strings.join".into(), |args| {
        if args.len() < 2 { return Err("Strings.join(arr, sep) requires 2 arguments".into()); }
        let sep = format!("{}", args[1]);
        match &args[0] {
            Value::Array(arr) => {
                let s_vec: Vec<String> = arr.lock().iter().map(|v| format!("{}", v)).collect();
                Ok(Value::string(s_vec.join(&sep)))
            }
            _ => Err("First argument to Strings.join must be array".into()),
        }
    }));

    strings_module.insert("trim".to_string(), Value::Native("Strings.trim".into(), |args| {
        if args.is_empty() { return Err("Strings.trim(str) requires string".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(s.trim()))
    }));

    strings_module.insert("upper".to_string(), Value::Native("Strings.upper".into(), |args| {
        if args.is_empty() { return Err("Strings.upper(str) requires string".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(s.to_uppercase()))
    }));

    strings_module.insert("lower".to_string(), Value::Native("Strings.lower".into(), |args| {
        if args.is_empty() { return Err("Strings.lower(str) requires string".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(s.to_lowercase()))
    }));

    strings_module.insert("contains".to_string(), Value::Native("Strings.contains".into(), |args| {
        if args.len() < 2 { return Err("Strings.contains(str, substr) requires 2 arguments".into()); }
        let s = format!("{}", args[0]);
        let sub = format!("{}", args[1]);
        Ok(Value::Bool(s.contains(&sub)))
    }));

    strings_module.insert("replace".to_string(), Value::Native("Strings.replace".into(), |args| {
        if args.len() < 3 { return Err("Strings.replace(str, from, to) requires 3 arguments".into()); }
        let s = format!("{}", args[0]);
        let from = format!("{}", args[1]);
        let to = format!("{}", args[2]);
        Ok(Value::string(s.replace(&from, &to)))
    }));

    strings_module.insert("split".to_string(), Value::Native("Strings.split".into(), |args| {
        if args.is_empty() { return Err("Strings.split(str, sep?) requires at least 1 argument".into()); }
        let s = format!("{}", args[0]);
        let sep = if args.len() > 1 { format!("{}", args[1]) } else { " ".to_string() };
        let parts: Vec<Value> = if sep.is_empty() {
            s.chars().map(|c| Value::string(c.to_string())).collect()
        } else {
            s.split(&sep).map(|p| Value::string(p.to_string())).collect()
        };
        Ok(Value::array(parts))
    }));

    strings_module.insert("trim".to_string(), Value::Native("Strings.trim".into(), |args| {
        if args.is_empty() { return Err("Strings.trim(str) requires string".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(s.trim().to_string()))
    }));

    strings_module.insert("starts_with".to_string(), Value::Native("Strings.starts_with".into(), |args| {
        if args.len() < 2 { return Err("Strings.starts_with(str, prefix) requires 2 arguments".into()); }
        let s = format!("{}", args[0]);
        let prefix = format!("{}", args[1]);
        Ok(Value::Bool(s.starts_with(&prefix)))
    }));

    strings_module.insert("ends_with".to_string(), Value::Native("Strings.ends_with".into(), |args| {
        if args.len() < 2 { return Err("Strings.ends_with(str, suffix) requires 2 arguments".into()); }
        let s = format!("{}", args[0]);
        let suffix = format!("{}", args[1]);
        Ok(Value::Bool(s.ends_with(&suffix)))
    }));

    strings_module.insert("join".to_string(), Value::Native("Strings.join".into(), |args| {
        if args.len() < 2 { return Err("Strings.join requires 2 arguments (list, sep or sep, list)".into()); }
        let (sep, items) = match (&args[0], &args[1]) {
            (Value::Array(a), sep_val) => {
                let s = format!("{}", sep_val);
                let it = a.lock().iter().map(|v| format!("{}", v)).collect::<Vec<_>>();
                (s, it)
            }
            (sep_val, Value::Array(a)) => {
                let s = format!("{}", sep_val);
                let it = a.lock().iter().map(|v| format!("{}", v)).collect::<Vec<_>>();
                (s, it)
            }
            _ => return Err("Strings.join requires one array argument".into()),
        };
        Ok(Value::string(items.join(&sep)))
    }));

    globals.insert("Strings".to_string(), Value::map(strings_module));

    // =========================================================================
    // 5. Sys Module
    // =========================================================================
    let mut sys_module = HashMap::new();

    sys_module.insert("env".to_string(), Value::Native("Sys.env".into(), |args| {
        if args.is_empty() { return Err("Sys.env(key) requires key".into()); }
        let key = format!("{}", args[0]);
        match std::env::var(&key) {
            Ok(val) => Ok(Value::string(val)),
            Err(_) => Ok(Value::Nil),
        }
    }));

    sys_module.insert("cwd".to_string(), Value::Native("Sys.cwd".into(), |_| {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        Ok(Value::string(cwd.to_string_lossy().to_string()))
    }));

    sys_module.insert("time_ms".to_string(), Value::Native("Sys.time_ms".into(), |_| {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        Ok(Value::Int(duration.as_millis() as i64))
    }));

    sys_module.insert("time".to_string(), Value::Native("Sys.time".into(), |_| {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        Ok(Value::Float(duration.as_secs_f64()))
    }));

    sys_module.insert("sleep".to_string(), Value::Native("Sys.sleep".into(), |args| {
        if args.is_empty() { return Err("Sys.sleep(seconds) requires duration".into()); }
        let secs = match args[0] {
            Value::Int(i) => i as f64,
            Value::Float(f) => f,
            _ => return Err("Duration must be a number".into()),
        };
        std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        Ok(Value::Nil)
    }));

    globals.insert("Sys".to_string(), Value::map(sys_module));
}
