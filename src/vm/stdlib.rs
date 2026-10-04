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

    // =========================================================================
    // 6. Mobile Module (Android Device APIs & UI Bridge)
    // =========================================================================
    let mut mobile_module = HashMap::new();

    mobile_module.insert("show_toast".to_string(), Value::Native("Mobile.show_toast".into(), |args| {
        let msg = if !args.is_empty() { format!("{}", args[0]) } else { "".to_string() };
        println!("[📱 Android Toast] {}", msg);
        Ok(Value::Bool(true))
    }));

    mobile_module.insert("vibrate".to_string(), Value::Native("Mobile.vibrate".into(), |args| {
        let ms = if !args.is_empty() {
            match args[0] {
                Value::Int(i) => i,
                _ => 50,
            }
        } else {
            50
        };
        println!("[📳 Android Haptic Feedback] Vibrating for {} ms", ms);
        Ok(Value::Bool(true))
    }));

    mobile_module.insert("battery_level".to_string(), Value::Native("Mobile.battery_level".into(), |_| {
        Ok(Value::Int(96))
    }));

    mobile_module.insert("device_info".to_string(), Value::Native("Mobile.device_info".into(), |_| {
        let mut info = HashMap::new();
        info.insert("os".to_string(), Value::string("Android 14 (API 34)"));
        info.insert("brand".to_string(), Value::string("Google / AETHER"));
        info.insert("model".to_string(), Value::string("Aether Phone Pro"));
        info.insert("arch".to_string(), Value::string("arm64-v8a"));
        Ok(Value::map(info))
    }));

    mobile_module.insert("network_status".to_string(), Value::Native("Mobile.network_status".into(), |_| {
        Ok(Value::string("wifi_connected"))
    }));

    mobile_module.insert("set_title".to_string(), Value::Native("Mobile.set_title".into(), |args| {
        let title = if !args.is_empty() { format!("{}", args[0]) } else { "".to_string() };
        println!("[📱 Android Window Title] {}", title);
        Ok(Value::Bool(true))
    }));

    globals.insert("Mobile".to_string(), Value::map(mobile_module));

    // =========================================================================
    // 7. Google Material 3 (M3 / Material You) Design System
    // =========================================================================
    let mut m3_module = HashMap::new();

    // M3 Typography tokens
    let mut typography = HashMap::new();
    let mut add_type_token = |name: &str, size: i64, weight: &str, tracking: f64| {
        let mut t = HashMap::new();
        t.insert("size".to_string(), Value::Int(size));
        t.insert("weight".to_string(), Value::string(weight));
        t.insert("tracking".to_string(), Value::Float(tracking));
        typography.insert(name.to_string(), Value::map(t));
    };
    add_type_token("display_large", 57, "regular", -0.25);
    add_type_token("display_medium", 45, "regular", 0.0);
    add_type_token("headline_large", 32, "regular", 0.0);
    add_type_token("headline_medium", 28, "regular", 0.0);
    add_type_token("title_large", 22, "regular", 0.0);
    add_type_token("title_medium", 16, "medium", 0.15);
    add_type_token("body_large", 16, "regular", 0.5);
    add_type_token("body_medium", 14, "regular", 0.25);
    add_type_token("label_large", 14, "medium", 0.1);
    add_type_token("label_small", 11, "medium", 0.5);
    m3_module.insert("typography".to_string(), Value::map(typography));

    // M3 Shape tokens (corner radius in dp)
    let mut shapes = HashMap::new();
    shapes.insert("none".to_string(), Value::Int(0));
    shapes.insert("xs".to_string(), Value::Int(4));
    shapes.insert("sm".to_string(), Value::Int(8));
    shapes.insert("md".to_string(), Value::Int(12));
    shapes.insert("lg".to_string(), Value::Int(16));
    shapes.insert("xl".to_string(), Value::Int(28));
    shapes.insert("full".to_string(), Value::Int(9999));
    m3_module.insert("shape".to_string(), Value::map(shapes));

    // M3.theme(seed_color, dark_mode) -> Dynamic Tonal Palette generator
    m3_module.insert("theme".to_string(), Value::Native("M3.theme".into(), |args| {
        let seed = if !args.is_empty() { format!("{}", args[0]) } else { "#00F5FF".to_string() };
        let dark_mode = if args.len() > 1 {
            match args[1] {
                Value::Bool(b) => b,
                _ => true,
            }
        } else {
            true
        };

        let seed_rgb = parse_hex_color(&seed);
        let mut palette = HashMap::new();
        palette.insert("seed".to_string(), Value::string(seed.clone()));
        palette.insert("dark_mode".to_string(), Value::Bool(dark_mode));

        if dark_mode {
            let primary = blend_color(seed_rgb, (255, 255, 255), 0.25);
            let on_primary = blend_color(seed_rgb, (0, 0, 0), 0.85);
            let primary_container = blend_color(seed_rgb, (0, 0, 0), 0.65);
            let on_primary_container = blend_color(seed_rgb, (255, 255, 255), 0.70);

            palette.insert("primary".to_string(), Value::string(rgb_to_hex(primary.0, primary.1, primary.2)));
            palette.insert("on_primary".to_string(), Value::string(rgb_to_hex(on_primary.0, on_primary.1, on_primary.2)));
            palette.insert("primary_container".to_string(), Value::string(rgb_to_hex(primary_container.0, primary_container.1, primary_container.2)));
            palette.insert("on_primary_container".to_string(), Value::string(rgb_to_hex(on_primary_container.0, on_primary_container.1, on_primary_container.2)));

            palette.insert("surface".to_string(), Value::string("#111318"));
            palette.insert("on_surface".to_string(), Value::string("#E1E2E8"));
            palette.insert("surface_variant".to_string(), Value::string("#43474E"));
            palette.insert("on_surface_variant".to_string(), Value::string("#C3C6CF"));
            palette.insert("outline".to_string(), Value::string("#8D9199"));
            palette.insert("error".to_string(), Value::string("#FFB4AB"));
            palette.insert("on_error".to_string(), Value::string("#690005"));
        } else {
            let primary = blend_color(seed_rgb, (0, 0, 0), 0.20);
            let on_primary = (255, 255, 255);
            let primary_container = blend_color(seed_rgb, (255, 255, 255), 0.75);
            let on_primary_container = blend_color(seed_rgb, (0, 0, 0), 0.80);

            palette.insert("primary".to_string(), Value::string(rgb_to_hex(primary.0, primary.1, primary.2)));
            palette.insert("on_primary".to_string(), Value::string(rgb_to_hex(on_primary.0, on_primary.1, on_primary.2)));
            palette.insert("primary_container".to_string(), Value::string(rgb_to_hex(primary_container.0, primary_container.1, primary_container.2)));
            palette.insert("on_primary_container".to_string(), Value::string(rgb_to_hex(on_primary_container.0, on_primary_container.1, on_primary_container.2)));

            palette.insert("surface".to_string(), Value::string("#FDF8FD"));
            palette.insert("on_surface".to_string(), Value::string("#1D1B1E"));
            palette.insert("surface_variant".to_string(), Value::string("#E7E0EB"));
            palette.insert("on_surface_variant".to_string(), Value::string("#49454E"));
            palette.insert("outline".to_string(), Value::string("#79747E"));
            palette.insert("error".to_string(), Value::string("#BA1A1A"));
            palette.insert("on_error".to_string(), Value::string("#FFFFFF"));
        }

        Ok(Value::map(palette))
    }));

    // M3.card(type, title, subtitle?, content?)
    m3_module.insert("card".to_string(), Value::Native("M3.card".into(), |args| {
        let card_type = if !args.is_empty() { format!("{}", args[0]) } else { "elevated".to_string() };
        let title = if args.len() > 1 { format!("{}", args[1]) } else { "".to_string() };
        let subtitle = if args.len() > 2 { format!("{}", args[2]) } else { "".to_string() };
        let content = if args.len() > 3 { format!("{}", args[3]) } else { "".to_string() };

        let mut card = HashMap::new();
        card.insert("component".to_string(), Value::string("M3.Card"));
        card.insert("type".to_string(), Value::string(card_type.clone()));
        card.insert("title".to_string(), Value::string(title));
        card.insert("subtitle".to_string(), Value::string(subtitle));
        card.insert("content".to_string(), Value::string(content));
        card.insert("shape".to_string(), Value::string("md"));
        card.insert("elevation".to_string(), Value::Int(if card_type == "elevated" { 1 } else { 0 }));

        Ok(Value::map(card))
    }));

    // M3.filled_button(label, icon?)
    m3_module.insert("filled_button".to_string(), Value::Native("M3.filled_button".into(), |args| {
        let label = if !args.is_empty() { format!("{}", args[0]) } else { "Button".to_string() };
        let icon = if args.len() > 1 { format!("{}", args[1]) } else { "".to_string() };
        let mut btn = HashMap::new();
        btn.insert("component".to_string(), Value::string("M3.FilledButton"));
        btn.insert("label".to_string(), Value::string(label));
        btn.insert("icon".to_string(), Value::string(icon));
        btn.insert("elevation".to_string(), Value::Int(0));
        btn.insert("shape".to_string(), Value::string("full"));
        Ok(Value::map(btn))
    }));

    // M3.tonal_button(label, icon?)
    m3_module.insert("tonal_button".to_string(), Value::Native("M3.tonal_button".into(), |args| {
        let label = if !args.is_empty() { format!("{}", args[0]) } else { "Button".to_string() };
        let icon = if args.len() > 1 { format!("{}", args[1]) } else { "".to_string() };
        let mut btn = HashMap::new();
        btn.insert("component".to_string(), Value::string("M3.TonalButton"));
        btn.insert("label".to_string(), Value::string(label));
        btn.insert("icon".to_string(), Value::string(icon));
        btn.insert("elevation".to_string(), Value::Int(0));
        btn.insert("shape".to_string(), Value::string("full"));
        Ok(Value::map(btn))
    }));

    // M3.outlined_button(label, icon?)
    m3_module.insert("outlined_button".to_string(), Value::Native("M3.outlined_button".into(), |args| {
        let label = if !args.is_empty() { format!("{}", args[0]) } else { "Button".to_string() };
        let icon = if args.len() > 1 { format!("{}", args[1]) } else { "".to_string() };
        let mut btn = HashMap::new();
        btn.insert("component".to_string(), Value::string("M3.OutlinedButton"));
        btn.insert("label".to_string(), Value::string(label));
        btn.insert("icon".to_string(), Value::string(icon));
        btn.insert("shape".to_string(), Value::string("full"));
        Ok(Value::map(btn))
    }));

    // M3.fab(icon, label?, extended?)
    m3_module.insert("fab".to_string(), Value::Native("M3.fab".into(), |args| {
        let icon = if !args.is_empty() { format!("{}", args[0]) } else { "add".to_string() };
        let label = if args.len() > 1 { format!("{}", args[1]) } else { "".to_string() };
        let extended = if args.len() > 2 {
            match args[2] {
                Value::Bool(b) => b,
                _ => false,
            }
        } else {
            !label.is_empty()
        };

        let mut fab = HashMap::new();
        fab.insert("component".to_string(), Value::string("M3.FAB"));
        fab.insert("icon".to_string(), Value::string(icon));
        fab.insert("label".to_string(), Value::string(label));
        fab.insert("extended".to_string(), Value::Bool(extended));
        fab.insert("elevation".to_string(), Value::Int(3));
        fab.insert("shape".to_string(), Value::string("lg"));
        Ok(Value::map(fab))
    }));

    // M3.top_app_bar(title, subtitle?)
    m3_module.insert("top_app_bar".to_string(), Value::Native("M3.top_app_bar".into(), |args| {
        let title = if !args.is_empty() { format!("{}", args[0]) } else { "App".to_string() };
        let subtitle = if args.len() > 1 { format!("{}", args[1]) } else { "".to_string() };
        let mut bar = HashMap::new();
        bar.insert("component".to_string(), Value::string("M3.TopAppBar"));
        bar.insert("title".to_string(), Value::string(title));
        bar.insert("subtitle".to_string(), Value::string(subtitle));
        bar.insert("elevation".to_string(), Value::Int(0));
        Ok(Value::map(bar))
    }));

    // M3.navigation_bar(items)
    m3_module.insert("navigation_bar".to_string(), Value::Native("M3.navigation_bar".into(), |args| {
        let items = if !args.is_empty() { args[0].clone() } else { Value::array(Vec::new()) };
        let mut bar = HashMap::new();
        bar.insert("component".to_string(), Value::string("M3.NavigationBar"));
        bar.insert("items".to_string(), items);
        bar.insert("elevation".to_string(), Value::Int(2));
        Ok(Value::map(bar))
    }));

    // M3.chip(label, selected?, icon?)
    m3_module.insert("chip".to_string(), Value::Native("M3.chip".into(), |args| {
        let label = if !args.is_empty() { format!("{}", args[0]) } else { "Chip".to_string() };
        let selected = if args.len() > 1 {
            match args[1] {
                Value::Bool(b) => b,
                _ => false,
            }
        } else {
            false
        };
        let icon = if args.len() > 2 { format!("{}", args[2]) } else { "".to_string() };

        let mut chip = HashMap::new();
        chip.insert("component".to_string(), Value::string("M3.Chip"));
        chip.insert("label".to_string(), Value::string(label));
        chip.insert("selected".to_string(), Value::Bool(selected));
        chip.insert("icon".to_string(), Value::string(icon));
        chip.insert("shape".to_string(), Value::string("sm"));
        Ok(Value::map(chip))
    }));

    // M3.badge(value, color?)
    m3_module.insert("badge".to_string(), Value::Native("M3.badge".into(), |args| {
        let val = if !args.is_empty() { format!("{}", args[0]) } else { "".to_string() };
        let color = if args.len() > 1 { format!("{}", args[1]) } else { "error".to_string() };
        let mut b = HashMap::new();
        b.insert("component".to_string(), Value::string("M3.Badge"));
        b.insert("value".to_string(), Value::string(val));
        b.insert("color".to_string(), Value::string(color));
        b.insert("shape".to_string(), Value::string("full"));
        Ok(Value::map(b))
    }));

    // M3.progress_indicator(value, type?)
    m3_module.insert("progress_indicator".to_string(), Value::Native("M3.progress_indicator".into(), |args| {
        let val = if !args.is_empty() {
            match args[0] {
                Value::Float(f) => f,
                Value::Int(i) => i as f64,
                _ => 0.0,
            }
        } else {
            0.0
        };
        let p_type = if args.len() > 1 { format!("{}", args[1]) } else { "linear".to_string() };
        let mut p = HashMap::new();
        p.insert("component".to_string(), Value::string("M3.ProgressIndicator"));
        p.insert("value".to_string(), Value::Float(val));
        p.insert("type".to_string(), Value::string(p_type));
        Ok(Value::map(p))
    }));

    // M3.render_preview(component) -> Pretty terminal/console visual renderer
    m3_module.insert("render_preview".to_string(), Value::Native("M3.render_preview".into(), |args| {
        if args.is_empty() { return Err("M3.render_preview(component) requires component map".into()); }
        if let Value::Map(m) = &args[0] {
            let map = m.lock();
            let comp = map.get("component").map(|v| format!("{}", v)).unwrap_or_default();
            match comp.as_str() {
                "M3.Card" => {
                    let card_type = map.get("type").map(|v| format!("{}", v)).unwrap_or_else(|| "elevated".to_string());
                    let title = map.get("title").map(|v| format!("{}", v)).unwrap_or_default();
                    let subtitle = map.get("subtitle").map(|v| format!("{}", v)).unwrap_or_default();
                    let content = map.get("content").map(|v| format!("{}", v)).unwrap_or_default();
                    println!("╭────────────────────────────────────────────────────────────╮");
                    println!("│ [M3 {} Card] (Shape: md / 12dp)                  │", card_type);
                    if !title.is_empty() {
                        println!("│ 🏷️  Title:    {:<44} │", title);
                    }
                    if !subtitle.is_empty() {
                        println!("│ ℹ️  Subtitle: {:<44} │", subtitle);
                    }
                    if !content.is_empty() {
                        println!("│ {:<58} │", content);
                    }
                    println!("╰────────────────────────────────────────────────────────────╯");
                }
                "M3.FilledButton" => {
                    let label = map.get("label").map(|v| format!("{}", v)).unwrap_or_default();
                    let icon = map.get("icon").map(|v| format!("{}", v)).unwrap_or_default();
                    let icon_str = if !icon.is_empty() { format!("{} ", icon) } else { "".to_string() };
                    println!("╭[ {}{} ]╮  (M3 Filled Button - Shape: full/pill)", icon_str, label);
                }
                "M3.FAB" => {
                    let icon = map.get("icon").map(|v| format!("{}", v)).unwrap_or_default();
                    let label = map.get("label").map(|v| format!("{}", v)).unwrap_or_default();
                    println!("╭───[ ➕ {} {} ]───╮  (M3 Floating Action Button - Shape: lg/16dp)", icon, label);
                }
                "M3.TopAppBar" => {
                    let title = map.get("title").map(|v| format!("{}", v)).unwrap_or_default();
                    println!("╔══════════════════ [M3 TopAppBar: {}] ══════════════════╗", title);
                }
                "M3.NavigationBar" => {
                    println!("╠════════════════════ [M3 NavigationBar] ════════════════════╣");
                    if let Some(Value::Array(items)) = map.get("items") {
                        let lock = items.lock();
                        let labels: Vec<String> = lock.iter().map(|item| {
                            if let Value::Map(m) = item {
                                let l = m.lock();
                                let label = l.get("label").map(|v| format!("{}", v)).unwrap_or_default();
                                let active = l.get("active").map(|v| matches!(v, Value::Bool(true))).unwrap_or(false);
                                if active {
                                    format!("[🔘 {}]", label)
                                } else {
                                    format!("[⚪ {}]", label)
                                }
                            } else {
                                format!("{}", item)
                            }
                        }).collect();
                        println!("  {}", labels.join("    "));
                    }
                    println!("╚════════════════════════════════════════════════════════════╝");
                }
                _ => {
                    println!("[M3 Component: {}]", comp);
                }
            }
            Ok(Value::Bool(true))
        } else {
            Err("M3.render_preview requires a valid M3 component map".into())
        }
    }));

    globals.insert("M3".to_string(), Value::map(m3_module));
}

// =========================================================================
// Standalone Color Math Utilities for Material 3
// =========================================================================

fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let s = hex.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (0, 245, 255)
    }
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn blend_color(c: (u8, u8, u8), target: (u8, u8, u8), ratio: f64) -> (u8, u8, u8) {
    let r = ((c.0 as f64) * (1.0 - ratio) + (target.0 as f64) * ratio).round().clamp(0.0, 255.0) as u8;
    let g = ((c.1 as f64) * (1.0 - ratio) + (target.1 as f64) * ratio).round().clamp(0.0, 255.0) as u8;
    let b = ((c.2 as f64) * (1.0 - ratio) + (target.2 as f64) * ratio).round().clamp(0.0, 255.0) as u8;
    (r, g, b)
}
