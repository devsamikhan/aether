// ==============================================================================
// AETHER v4.0 — Global String Interner, Inline Shape Cache & Cycle-Detecting GC
// Pure Rust Standard Library — Zero External Crates
// ==============================================================================

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};
use super::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

struct StringInterner {
    map: HashMap<String, SymbolId>,
    strings: Vec<String>,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            map: HashMap::with_capacity(256),
            strings: Vec::with_capacity(256),
        }
    }

    fn intern(&mut self, s: &str) -> SymbolId {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = SymbolId(self.strings.len() as u32);
        let owned = s.to_string();
        self.strings.push(owned.clone());
        self.map.insert(owned, id);
        id
    }

    fn resolve(&self, id: SymbolId) -> Option<String> {
        self.strings.get(id.0 as usize).cloned()
    }

    fn count(&self) -> usize {
        self.strings.len()
    }
}

static GLOBAL_INTERNER: OnceLock<RwLock<StringInterner>> = OnceLock::new();

fn get_interner() -> &'static RwLock<StringInterner> {
    GLOBAL_INTERNER.get_or_init(|| RwLock::new(StringInterner::new()))
}

pub fn intern_symbol(s: &str) -> SymbolId {
    if let Ok(guard) = get_interner().read() {
        if let Some(&id) = guard.map.get(s) {
            return id;
        }
    }
    let mut guard = get_interner().write().unwrap();
    guard.intern(s)
}

pub fn resolve_symbol(id: SymbolId) -> Option<String> {
    let guard = get_interner().read().unwrap();
    guard.resolve(id)
}

pub fn interned_symbol_count() -> usize {
    let guard = get_interner().read().unwrap();
    guard.count()
}

// ==============================================================================
// Cycle-Detecting Reference Collector for Arc<Mutex<...>> Object Graphs
// ==============================================================================

/// Traverses a `Value` container graph, detects self/mutual reference cycles
/// (back-edges on the active traversal stack), breaks cyclic edges by replacing
/// them with `Value::Nil`, and returns the number of cyclic references broken.
pub fn break_reference_cycles(root: &Value) -> usize {
    let mut active_stack = HashSet::new();
    let mut visited = HashSet::new();
    collect_cycles_inner(root, &mut active_stack, &mut visited)
}

fn container_ptr_id(val: &Value) -> Option<usize> {
    match val {
        Value::Array(arr) => Some(std::sync::Arc::as_ptr(arr) as usize),
        Value::Map(map) => Some(std::sync::Arc::as_ptr(map) as usize),
        Value::StructInstance(inst) => Some(std::sync::Arc::as_ptr(&inst.fields) as usize),
        Value::ClassInstance(inst) => Some(std::sync::Arc::as_ptr(&inst.fields) as usize),
        _ => None,
    }
}

fn collect_cycles_inner(
    val: &Value,
    active_stack: &mut HashSet<usize>,
    visited: &mut HashSet<usize>,
) -> usize {
    let Some(ptr_id) = container_ptr_id(val) else {
        return 0;
    };

    if visited.contains(&ptr_id) {
        return 0;
    }

    active_stack.insert(ptr_id);
    let mut broken = 0;

    match val {
        Value::Array(arr) => {
            let mut guard = arr.lock();
            for i in 0..guard.len() {
                if let Some(child_ptr) = container_ptr_id(&guard[i]) {
                    if active_stack.contains(&child_ptr) {
                        guard[i] = Value::Nil;
                        broken += 1;
                        continue;
                    }
                }
                let child = guard[i].clone();
                drop(guard);
                broken += collect_cycles_inner(&child, active_stack, visited);
                guard = arr.lock();
            }
        }
        Value::Map(map) => {
            let keys: Vec<String> = map.lock().keys().cloned().collect();
            for k in keys {
                let child_opt = map.lock().get(&k).cloned();
                if let Some(child) = child_opt {
                    if let Some(child_ptr) = container_ptr_id(&child) {
                        if active_stack.contains(&child_ptr) {
                            map.lock().insert(k, Value::Nil);
                            broken += 1;
                            continue;
                        }
                    }
                    broken += collect_cycles_inner(&child, active_stack, visited);
                }
            }
        }
        Value::StructInstance(inst) => {
            let keys: Vec<String> = inst.fields.lock().keys().cloned().collect();
            for k in keys {
                let child_opt = inst.fields.lock().get(&k).cloned();
                if let Some(child) = child_opt {
                    if let Some(child_ptr) = container_ptr_id(&child) {
                        if active_stack.contains(&child_ptr) {
                            inst.fields.lock().insert(k, Value::Nil);
                            broken += 1;
                            continue;
                        }
                    }
                    broken += collect_cycles_inner(&child, active_stack, visited);
                }
            }
        }
        Value::ClassInstance(inst) => {
            let keys: Vec<String> = inst.fields.lock().keys().cloned().collect();
            for k in keys {
                let child_opt = inst.fields.lock().get(&k).cloned();
                if let Some(child) = child_opt {
                    if let Some(child_ptr) = container_ptr_id(&child) {
                        if active_stack.contains(&child_ptr) {
                            inst.fields.lock().insert(k, Value::Nil);
                            broken += 1;
                            continue;
                        }
                    }
                    broken += collect_cycles_inner(&child, active_stack, visited);
                }
            }
        }
        _ => {}
    }

    active_stack.remove(&ptr_id);
    visited.insert(ptr_id);
    broken
}
