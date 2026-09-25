// ==============================================================================
// AetherTimeTravel — Omniscient Time-Reversible VM & Execution History Replay
// Reversible State Delta Journaling, Checkpointing, Rewind & Branching
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

// ==============================================================================
// 1. Delta Event Journaling
// ==============================================================================

#[derive(Clone, Debug)]
pub enum StateDelta {
    VarMutation {
        step: u64,
        var_name: String,
        old_val: Option<Value>,
        new_val: Value,
    },
    StepOp {
        step: u64,
        op_name: String,
        details: String,
    },
    Checkpoint {
        step: u64,
        name: String,
        snapshot: HashMap<String, Value>,
    },
    LogOutput {
        step: u64,
        message: String,
    },
}

// ==============================================================================
// 2. Time-Travel Session Engine
// ==============================================================================

#[derive(Clone, Debug)]
pub struct TimeTravelSession {
    pub id: u64,
    pub current_step: u64,
    pub active_timeline: String,
    pub deltas: Vec<StateDelta>,
    pub current_state: HashMap<String, Value>,
    pub checkpoints: HashMap<String, (u64, HashMap<String, Value>)>,
    pub branches: HashMap<String, HashMap<String, Value>>,
}

impl TimeTravelSession {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            current_step: 0,
            active_timeline: "main".to_string(),
            deltas: Vec::new(),
            current_state: HashMap::new(),
            checkpoints: HashMap::new(),
            branches: HashMap::new(),
        }
    }

    /// Records an execution step / opcode
    pub fn record_step(&mut self, op_name: &str, details: &str) -> u64 {
        self.current_step += 1;
        self.deltas.push(StateDelta::StepOp {
            step: self.current_step,
            op_name: op_name.to_string(),
            details: details.to_string(),
        });
        self.current_step
    }

    /// Records a variable mutation with old and new values for reversible rewinding
    pub fn record_mutation(&mut self, var_name: &str, new_val: Value) -> u64 {
        self.current_step += 1;
        let old_val = self.current_state.get(var_name).cloned();
        self.current_state.insert(var_name.to_string(), new_val.clone());

        self.deltas.push(StateDelta::VarMutation {
            step: self.current_step,
            var_name: var_name.to_string(),
            old_val,
            new_val,
        });

        self.current_step
    }

    /// Records a log or print event
    pub fn record_log(&mut self, message: &str) {
        self.deltas.push(StateDelta::LogOutput {
            step: self.current_step,
            message: message.to_string(),
        });
    }

    /// Creates a named snapshot checkpoint of current runtime variables
    pub fn create_checkpoint(&mut self, name: &str) -> u64 {
        self.current_step += 1;
        let snapshot = self.current_state.clone();
        self.checkpoints.insert(name.to_string(), (self.current_step, snapshot.clone()));

        self.deltas.push(StateDelta::Checkpoint {
            step: self.current_step,
            name: name.to_string(),
            snapshot,
        });

        self.current_step
    }

    /// Restores the exact state from a named checkpoint
    pub fn restore_checkpoint(&mut self, name: &str) -> Result<u64, String> {
        let (step, snapshot) = self.checkpoints.get(name)
            .cloned()
            .ok_or_else(|| format!("Checkpoint '{}' not found", name))?;

        self.current_state = snapshot;
        self.current_step += 1;
        self.record_log(&format!("TimeTravel: Rewound to checkpoint '{}' at step {}", name, step));
        Ok(step)
    }

    /// Rewinds backward by N mutation steps, rolling back state deltas
    pub fn rewind_steps(&mut self, steps: usize) -> Result<usize, String> {
        if steps == 0 {
            return Ok(0);
        }

        let mut rolled_back = 0;
        let mut idx = self.deltas.len();

        while idx > 0 && rolled_back < steps {
            idx -= 1;
            match &self.deltas[idx] {
                StateDelta::VarMutation { var_name, old_val, .. } => {
                    match old_val {
                        Some(val) => {
                            self.current_state.insert(var_name.clone(), val.clone());
                        }
                        None => {
                            self.current_state.remove(var_name);
                        }
                    }
                    rolled_back += 1;
                }
                _ => {}
            }
        }

        // Truncate undone deltas
        self.deltas.truncate(idx);
        self.current_step = self.deltas.last().map(|d| match d {
            StateDelta::VarMutation { step, .. } => *step,
            StateDelta::StepOp { step, .. } => *step,
            StateDelta::Checkpoint { step, .. } => *step,
            StateDelta::LogOutput { step, .. } => *step,
        }).unwrap_or(0);

        Ok(rolled_back)
    }

    /// Forks the current timeline into an isolated counterfactual branch
    pub fn branch_timeline(&mut self, branch_name: &str) -> Result<String, String> {
        if self.branches.contains_key(branch_name) {
            return Err(format!("Branch '{}' already exists", branch_name));
        }

        let branch_state = self.current_state.clone();
        self.branches.insert(branch_name.to_string(), branch_state);
        self.active_timeline = branch_name.to_string();

        self.record_log(&format!("TimeTravel: Created and switched to branch '{}'", branch_name));
        Ok(branch_name.to_string())
    }

    /// Computes the diff between two checkpoints
    pub fn diff_checkpoints(&self, cp_a: &str, cp_b: &str) -> Result<HashMap<String, (Value, Value)>, String> {
        let (_, snap_a) = self.checkpoints.get(cp_a)
            .ok_or_else(|| format!("Checkpoint '{}' not found", cp_a))?;
        let (_, snap_b) = self.checkpoints.get(cp_b)
            .ok_or_else(|| format!("Checkpoint '{}' not found", cp_b))?;

        let mut diffs = HashMap::new();

        // Check modified or removed variables
        for (k, va) in snap_a {
            match snap_b.get(k) {
                Some(vb) => {
                    if va != vb {
                        diffs.insert(k.clone(), (va.clone(), vb.clone()));
                    }
                }
                None => {
                    diffs.insert(k.clone(), (va.clone(), Value::Nil));
                }
            }
        }

        // Check added variables
        for (k, vb) in snap_b {
            if !snap_a.contains_key(k) {
                diffs.insert(k.clone(), (Value::Nil, vb.clone()));
            }
        }

        Ok(diffs)
    }

    /// Generates structured execution history log
    pub fn history_entries(&self) -> Vec<Value> {
        self.deltas.iter().map(|d| {
            let mut map = HashMap::new();
            match d {
                StateDelta::VarMutation { step, var_name, old_val, new_val } => {
                    map.insert("type".to_string(), Value::string("mutation"));
                    map.insert("step".to_string(), Value::Int(*step as i64));
                    map.insert("variable".to_string(), Value::string(var_name.clone()));
                    map.insert("old_value".to_string(), old_val.clone().unwrap_or(Value::Nil));
                    map.insert("new_value".to_string(), new_val.clone());
                }
                StateDelta::StepOp { step, op_name, details } => {
                    map.insert("type".to_string(), Value::string("step"));
                    map.insert("step".to_string(), Value::Int(*step as i64));
                    map.insert("op".to_string(), Value::string(op_name.clone()));
                    map.insert("details".to_string(), Value::string(details.clone()));
                }
                StateDelta::Checkpoint { step, name, .. } => {
                    map.insert("type".to_string(), Value::string("checkpoint"));
                    map.insert("step".to_string(), Value::Int(*step as i64));
                    map.insert("name".to_string(), Value::string(name.clone()));
                }
                StateDelta::LogOutput { step, message } => {
                    map.insert("type".to_string(), Value::string("log"));
                    map.insert("step".to_string(), Value::Int(*step as i64));
                    map.insert("message".to_string(), Value::string(message.clone()));
                }
            }
            Value::map(map)
        }).collect()
    }
}

// ==============================================================================
// 3. Global Session Registry
// ==============================================================================

static SESSIONS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<TimeTravelSession>>>>> = OnceLock::new();
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

fn get_sessions() -> &'static Mutex<HashMap<u64, Arc<Mutex<TimeTravelSession>>>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn create_session() -> u64 {
    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::SeqCst);
    let session = TimeTravelSession::new(id);
    get_sessions().lock().unwrap().insert(id, Arc::new(Mutex::new(session)));
    id
}

pub fn get_session(id: u64) -> Result<Arc<Mutex<TimeTravelSession>>, String> {
    get_sessions()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("TimeTravel session {} not found", id))
}

// ==============================================================================
// 4. Native VM Module Bindings
// ==============================================================================

pub fn register_timetravel_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. TimeTravel.create_session() -> session_id
    mod_map.insert(
        "create_session".to_string(),
        Value::Native("TimeTravel.create_session".into(), |_| {
            let id = create_session();
            Ok(Value::Int(id as i64))
        }),
    );

    // 2. TimeTravel.record(session_id, var_name, value)
    mod_map.insert(
        "record".to_string(),
        Value::Native("TimeTravel.record".into(), |args| {
            if args.len() < 3 {
                return Err("TimeTravel.record(session_id, var_name, value) requires 3 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };
            let var_name = args[1].to_string();
            let val = args[2].clone();

            let sess_arc = get_session(id)?;
            let mut sess = sess_arc.lock().unwrap();
            let step = sess.record_mutation(&var_name, val);
            Ok(Value::Int(step as i64))
        }),
    );

    // 3. TimeTravel.checkpoint(session_id, name) -> step
    mod_map.insert(
        "checkpoint".to_string(),
        Value::Native("TimeTravel.checkpoint".into(), |args| {
            if args.len() < 2 {
                return Err("TimeTravel.checkpoint(session_id, name) requires 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };
            let name = args[1].to_string();

            let sess_arc = get_session(id)?;
            let mut sess = sess_arc.lock().unwrap();
            let step = sess.create_checkpoint(&name);
            Ok(Value::Int(step as i64))
        }),
    );

    // 4. TimeTravel.restore(session_id, name) -> step
    mod_map.insert(
        "restore".to_string(),
        Value::Native("TimeTravel.restore".into(), |args| {
            if args.len() < 2 {
                return Err("TimeTravel.restore(session_id, name) requires 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };
            let name = args[1].to_string();

            let sess_arc = get_session(id)?;
            let mut sess = sess_arc.lock().unwrap();
            let step = sess.restore_checkpoint(&name)?;
            Ok(Value::Int(step as i64))
        }),
    );

    // 5. TimeTravel.rewind(session_id, steps) -> rolled_back_count
    mod_map.insert(
        "rewind".to_string(),
        Value::Native("TimeTravel.rewind".into(), |args| {
            if args.len() < 2 {
                return Err("TimeTravel.rewind(session_id, steps) requires 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };
            let steps = match args[1] {
                Value::Int(i) if i >= 0 => i as usize,
                _ => 1,
            };

            let sess_arc = get_session(id)?;
            let mut sess = sess_arc.lock().unwrap();
            let rolled = sess.rewind_steps(steps)?;
            Ok(Value::Int(rolled as i64))
        }),
    );

    // 6. TimeTravel.branch(session_id, branch_name) -> branch_name
    mod_map.insert(
        "branch".to_string(),
        Value::Native("TimeTravel.branch".into(), |args| {
            if args.len() < 2 {
                return Err("TimeTravel.branch(session_id, branch_name) requires 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };
            let branch = args[1].to_string();

            let sess_arc = get_session(id)?;
            let mut sess = sess_arc.lock().unwrap();
            let b = sess.branch_timeline(&branch)?;
            Ok(Value::string(b))
        }),
    );

    // 7. TimeTravel.get_state(session_id) -> map
    mod_map.insert(
        "get_state".to_string(),
        Value::Native("TimeTravel.get_state".into(), |args| {
            if args.is_empty() {
                return Err("TimeTravel.get_state(session_id) requires session_id".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };

            let sess_arc = get_session(id)?;
            let sess = sess_arc.lock().unwrap();
            Ok(Value::map(sess.current_state.clone()))
        }),
    );

    // 8. TimeTravel.diff(session_id, cp_a, cp_b) -> map of diffs
    mod_map.insert(
        "diff".to_string(),
        Value::Native("TimeTravel.diff".into(), |args| {
            if args.len() < 3 {
                return Err("TimeTravel.diff(session_id, cp_a, cp_b) requires 3 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };
            let cp_a = args[1].to_string();
            let cp_b = args[2].to_string();

            let sess_arc = get_session(id)?;
            let sess = sess_arc.lock().unwrap();
            let diffs = sess.diff_checkpoints(&cp_a, &cp_b)?;

            let mut res = HashMap::new();
            for (k, (old_v, new_v)) in diffs {
                res.insert(k, Value::array(vec![old_v, new_v]));
            }
            Ok(Value::map(res))
        }),
    );

    // 9. TimeTravel.history(session_id) -> array of entry maps
    mod_map.insert(
        "history".to_string(),
        Value::Native("TimeTravel.history".into(), |args| {
            if args.is_empty() {
                return Err("TimeTravel.history(session_id) requires session_id".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };

            let sess_arc = get_session(id)?;
            let sess = sess_arc.lock().unwrap();
            Ok(Value::array(sess.history_entries()))
        }),
    );

    globals.insert("TimeTravel".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_timetravel".to_string(), Value::map(mod_map));
}
