// ==============================================================================
// AetherTimeTravel TUI — Live Interactive Time-Machine Visualizer
// Reversible State Navigation, Timeline Ribbon & ASCII Variable Inspector
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use super::timetravel::{get_session, StateDelta, TimeTravelSession};
use super::value::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

// ==============================================================================
// 1. ANSI Terminal Styling Codes
// ==============================================================================

pub struct Ansi;
impl Ansi {
    pub const RESET: &'static str = "\x1B[0m";
    pub const BOLD: &'static str = "\x1B[1m";
    pub const DIM: &'static str = "\x1B[2m";
    pub const UNDERLINE: &'static str = "\x1B[4m";

    pub const RED: &'static str = "\x1B[31m";
    pub const GREEN: &'static str = "\x1B[32m";
    pub const YELLOW: &'static str = "\x1B[33m";
    pub const BLUE: &'static str = "\x1B[34m";
    pub const MAGENTA: &'static str = "\x1B[35m";
    pub const CYAN: &'static str = "\x1B[36m";
    pub const WHITE: &'static str = "\x1B[37m";

    pub const BG_CYAN: &'static str = "\x1B[46m";
    pub const BG_BLUE: &'static str = "\x1B[44m";
    pub const BG_MAGENTA: &'static str = "\x1B[45m";
    pub const BG_GREEN: &'static str = "\x1B[42m";
    pub const BLACK_TEXT: &'static str = "\x1B[30m";

    pub const CLEAR_SCREEN: &'static str = "\x1B[2J\x1B[1;1H";
}

// ==============================================================================
// 2. State Reconstruction & Diff Computation
// ==============================================================================

/// Reconstructs the exact state of all variables at any given step in time
pub fn compute_state_at_step(session: &TimeTravelSession, target_step: u64) -> HashMap<String, Value> {
    let mut state = HashMap::new();

    for delta in &session.deltas {
        match delta {
            StateDelta::VarMutation { step, var_name, new_val, .. } => {
                if *step <= target_step {
                    state.insert(var_name.clone(), new_val.clone());
                }
            }
            StateDelta::Checkpoint { .. } => {}
            StateDelta::StepOp { .. } => {}
            StateDelta::LogOutput { .. } => {}
        }
    }

    state
}

/// Computes the diff between two steps
pub fn compute_step_diff(
    session: &TimeTravelSession,
    step_a: u64,
    step_b: u64,
) -> HashMap<String, (Option<Value>, Option<Value>)> {
    let state_a = compute_state_at_step(session, step_a);
    let state_b = compute_state_at_step(session, step_b);

    let mut diff = HashMap::new();

    for (k, val_a) in &state_a {
        match state_b.get(k) {
            Some(val_b) => {
                if val_a != val_b {
                    diff.insert(k.clone(), (Some(val_a.clone()), Some(val_b.clone())));
                }
            }
            None => {
                diff.insert(k.clone(), (Some(val_a.clone()), None));
            }
        }
    }

    for (k, val_b) in &state_b {
        if !state_a.contains_key(k) {
            diff.insert(k.clone(), (None, Some(val_b.clone())));
        }
    }

    diff
}

// ==============================================================================
// 3. ASCII TUI Frame Renderer
// ==============================================================================

/// Renders a full interactive TUI frame for the current cursor step
pub fn render_tui_frame(session: &TimeTravelSession, cursor_step: u64, show_diff: bool) -> String {
    let mut out = String::new();

    let max_step = session.current_step.max(
        session.deltas.iter().map(|d| match d {
            StateDelta::VarMutation { step, .. } => *step,
            StateDelta::StepOp { step, .. } => *step,
            StateDelta::Checkpoint { step, .. } => *step,
            StateDelta::LogOutput { step, .. } => *step,
        }).max().unwrap_or(0)
    );

    let active_step = cursor_step.min(max_step);

    // 1. Header Banner
    out.push_str(&format!(
        "{}{}+------------------------------------------------------------------------------+{}\n",
        Ansi::CYAN, Ansi::BOLD, Ansi::RESET
    ));
    out.push_str(&format!(
        "{}|{}  ⏳ {}AETHER TIME-TRAVEL OMNISCIENT DEBUGGER{}                                     {}|{}\n",
        Ansi::CYAN, Ansi::BOLD, Ansi::YELLOW, Ansi::RESET, Ansi::CYAN, Ansi::RESET
    ));
    out.push_str(&format!(
        "{}|{}  Timeline: {}[{:<10}]{} │ Step: {}{:>3} / {:<3}{} │ Total Events: {}{:<3}{}            {}|{}\n",
        Ansi::CYAN, Ansi::BOLD,
        Ansi::GREEN, session.active_timeline, Ansi::RESET,
        Ansi::YELLOW, active_step, max_step, Ansi::RESET,
        Ansi::CYAN, session.deltas.len(), Ansi::RESET,
        Ansi::CYAN, Ansi::RESET
    ));
    out.push_str(&format!(
        "{}{}+------------------------------------------------------------------------------+{}\n\n",
        Ansi::CYAN, Ansi::BOLD, Ansi::RESET
    ));

    // 2. Timeline Track Ribbon
    out.push_str(&format!("{}══ TIMELINE RIBBON ══════════════════════════════════════════════════════════{} \n", Ansi::BOLD, Ansi::RESET));
    let mut ribbon = String::new();
    let start_step = if active_step > 5 { active_step - 5 } else { 0 };
    let end_step = (start_step + 10).min(max_step);

    if start_step > 0 {
        ribbon.push_str("...-");
    }

    for s in start_step..=end_step {
        // Check if checkpoint exists at step s
        let is_cp = session.checkpoints.values().any(|(st, _)| *st == s);

        if s == active_step {
            if is_cp {
                ribbon.push_str(&format!("{}[*C:{}*]{}", Ansi::BG_MAGENTA, s, Ansi::RESET));
            } else {
                ribbon.push_str(&format!("{}[*S:{}*]{}", Ansi::BG_GREEN, s, Ansi::RESET));
            }
        } else if is_cp {
            ribbon.push_str(&format!("{}[C:{}]{}", Ansi::MAGENTA, s, Ansi::RESET));
        } else {
            ribbon.push_str(&format!("[{}]", s));
        }

        if s < end_step {
            ribbon.push_str("---");
        }
    }

    if end_step < max_step {
        ribbon.push_str("-...");
    }

    out.push_str(&format!("  {}\n\n", ribbon));

    // 3. Event Delta at Cursor Step
    out.push_str(&format!("{}══ EVENT AT STEP {} ═════════════════════════════════════════════════════════{} \n", Ansi::BOLD, active_step, Ansi::RESET));
    let matching_deltas: Vec<&StateDelta> = session.deltas.iter().filter(|d| {
        let st = match d {
            StateDelta::VarMutation { step, .. } => *step,
            StateDelta::StepOp { step, .. } => *step,
            StateDelta::Checkpoint { step, .. } => *step,
            StateDelta::LogOutput { step, .. } => *step,
        };
        st == active_step
    }).collect();

    if matching_deltas.is_empty() {
        if active_step == 0 {
            out.push_str(&format!("  {}• Baseline Initial State (Step 0){}\n", Ansi::DIM, Ansi::RESET));
        } else {
            out.push_str(&format!("  {}• (No mutation event at step {}){}\n", Ansi::DIM, active_step, Ansi::RESET));
        }
    } else {
        for delta in matching_deltas {
            match delta {
                StateDelta::VarMutation { var_name, old_val, new_val, .. } => {
                    let old_str = old_val.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "nil (undefined)".to_string());
                    out.push_str(&format!(
                        "  {}• MUTATION:{} {}{}{} = {}{}{}  (was: {}{}{})\n",
                        Ansi::YELLOW, Ansi::RESET,
                        Ansi::CYAN, var_name, Ansi::RESET,
                        Ansi::GREEN, new_val, Ansi::RESET,
                        Ansi::RED, old_str, Ansi::RESET
                    ));
                }
                StateDelta::Checkpoint { name, .. } => {
                    out.push_str(&format!(
                        "  {}• CHECKPOINT:{} '{}'\n",
                        Ansi::MAGENTA, Ansi::RESET, name
                    ));
                }
                StateDelta::StepOp { op_name, details, .. } => {
                    out.push_str(&format!(
                        "  {}• OP:{} {} ({})\n",
                        Ansi::BLUE, Ansi::RESET, op_name, details
                    ));
                }
                StateDelta::LogOutput { message, .. } => {
                    out.push_str(&format!(
                        "  {}• LOG:{} {}\n",
                        Ansi::WHITE, Ansi::RESET, message
                    ));
                }
            }
        }
    }
    out.push('\n');

    // 4. Variable Inspector Table
    let current_state = compute_state_at_step(session, active_step);
    out.push_str(&format!("{}══ VARIABLE INSPECTOR (STATE AT STEP {}) ══════════════════════════════════{} \n", Ansi::BOLD, active_step, Ansi::RESET));

    if current_state.is_empty() {
        out.push_str("  (No variables bound)\n");
    } else {
        out.push_str("  +-----------------------+------------+--------------------------------------+\n");
        out.push_str("  | Variable Name         | Type       | Value                                |\n");
        out.push_str("  +-----------------------+------------+--------------------------------------+\n");

        let mut keys: Vec<&String> = current_state.keys().collect();
        keys.sort();

        for key in keys {
            let val = &current_state[key];
            let type_str = val.type_name();
            let val_str = val.to_string();
            let truncated_val = if val_str.len() > 36 {
                format!("{}...", &val_str[..33])
            } else {
                val_str
            };

            out.push_str(&format!(
                "  | {:<21} | {:<10} | {:<36} |\n",
                key, type_str, truncated_val
            ));
        }
        out.push_str("  +-----------------------+------------+--------------------------------------+\n");
    }
    out.push('\n');

    // 5. Diff Inspector (Optional)
    if show_diff && active_step > 0 {
        let diff = compute_step_diff(session, active_step - 1, active_step);
        out.push_str(&format!("{}══ STEP DELTA DIFF (STEP {} vs STEP {}) ═════════════════════════════════{} \n", Ansi::BOLD, active_step - 1, active_step, Ansi::RESET));
        if diff.is_empty() {
            out.push_str("  (No variable changes between these two steps)\n");
        } else {
            for (k, (v_old, v_new)) in diff {
                let old_str = v_old.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string());
                let new_str = v_new.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string());
                out.push_str(&format!(
                    "  {}var {}{}: {}{}{} -> {}{}{}\n",
                    Ansi::BOLD, k, Ansi::RESET,
                    Ansi::RED, old_str, Ansi::RESET,
                    Ansi::GREEN, new_str, Ansi::RESET
                ));
            }
        }
        out.push('\n');
    }

    // 6. Navigation Controls Footer
    out.push_str(&format!("{}+------------------------------------------------------------------------------+{}\n", Ansi::CYAN, Ansi::RESET));
    out.push_str(&format!(
        "{}| CONTROLS: [H/P/<-] Rewind  [L/N/->] Forward  [B] Branch  [C] Save CP  [D] Diff  [Q] Exit |{}\n",
        Ansi::CYAN, Ansi::RESET
    ));
    out.push_str(&format!("{}+------------------------------------------------------------------------------+{}\n", Ansi::CYAN, Ansi::RESET));

    out
}

// ==============================================================================
// 4. Interactive Terminal Controller Loop
// ==============================================================================

/// Runs the interactive terminal loop over a live TimeTravelSession
pub fn run_interactive_tui<R: BufRead, W: Write>(
    session_arc: Arc<Mutex<TimeTravelSession>>,
    reader: &mut R,
    writer: &mut W,
) -> Result<(), String> {
    let mut cursor_step = {
        let s = session_arc.lock().unwrap();
        s.current_step
    };
    let mut show_diff = true;

    loop {
        let frame = {
            let s = session_arc.lock().unwrap();
            render_tui_frame(&s, cursor_step, show_diff)
        };

        // Write clear screen + frame
        write!(writer, "{}{}", Ansi::CLEAR_SCREEN, frame).map_err(|e| e.to_string())?;
        write!(writer, "\n{}TimeTravel >{} ", Ansi::BOLD, Ansi::RESET).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;

        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            // EOF reached
            break;
        }

        let cmd = line.trim().to_lowercase();
        match cmd.as_str() {
            "h" | "p" | "prev" | "back" | "rewind" | "left" => {
                if cursor_step > 0 {
                    cursor_step -= 1;
                }
            }
            "l" | "n" | "next" | "forward" | "redo" | "right" => {
                let max_step = {
                    let s = session_arc.lock().unwrap();
                    s.current_step
                };
                if cursor_step < max_step {
                    cursor_step += 1;
                }
            }
            "d" | "diff" => {
                show_diff = !show_diff;
            }
            "c" | "checkpoint" => {
                let cp_name = format!("cp_step_{}", cursor_step);
                let mut s = session_arc.lock().unwrap();
                s.create_checkpoint(&cp_name);
                cursor_step = s.current_step;
            }
            "b" | "branch" => {
                let branch_name = format!("branch_{}", cursor_step);
                let mut s = session_arc.lock().unwrap();
                let _ = s.branch_timeline(&branch_name);
            }
            "q" | "exit" | "quit" => {
                writeln!(writer, "\n{}Exiting TimeTravel Debugger. Goodbye!{}", Ansi::YELLOW, Ansi::RESET)
                    .map_err(|e| e.to_string())?;
                break;
            }
            _ => {
                // If it's a number, jump directly to step
                if let Ok(jump_step) = cmd.parse::<u64>() {
                    let max_step = {
                        let s = session_arc.lock().unwrap();
                        s.current_step
                    };
                    cursor_step = jump_step.min(max_step);
                }
            }
        }
    }

    Ok(())
}

// ==============================================================================
// 5. Native Module Registration
// ==============================================================================

pub fn register_timetravel_tui_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. TimeTravelTUI.render_frame(session_id, step_opt, diff_opt) -> String
    mod_map.insert(
        "render_frame".to_string(),
        Value::Native("TimeTravelTUI.render_frame".into(), |args| {
            if args.is_empty() {
                return Err("TimeTravelTUI.render_frame requires at least session_id".into());
            }
            let sess_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };

            let sess_arc = get_session(sess_id)?;
            let sess = sess_arc.lock().unwrap();

            let step = if args.len() > 1 && args[1] != Value::Nil {
                match args[1] {
                    Value::Int(i) => i as u64,
                    _ => sess.current_step,
                }
            } else {
                sess.current_step
            };

            let show_diff = if args.len() > 2 {
                args[2].is_truthy()
            } else {
                true
            };

            let frame = render_tui_frame(&sess, step, show_diff);
            Ok(Value::string(frame))
        }),
    );

    // 2. TimeTravelTUI.state_at(session_id, step) -> Map
    mod_map.insert(
        "state_at".to_string(),
        Value::Native("TimeTravelTUI.state_at".into(), |args| {
            if args.len() < 2 {
                return Err("TimeTravelTUI.state_at(session_id, step) requires 2 arguments".into());
            }
            let sess_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };
            let step = match args[1] {
                Value::Int(i) => i as u64,
                _ => return Err("step must be integer".into()),
            };

            let sess_arc = get_session(sess_id)?;
            let sess = sess_arc.lock().unwrap();
            let state = compute_state_at_step(&sess, step);

            Ok(Value::map(state))
        }),
    );

    // 3. TimeTravelTUI.interactive(session_id)
    mod_map.insert(
        "interactive".to_string(),
        Value::Native("TimeTravelTUI.interactive".into(), |args| {
            if args.is_empty() {
                return Err("TimeTravelTUI.interactive(session_id) requires session_id".into());
            }
            let sess_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("session_id must be integer".into()),
            };

            let sess_arc = get_session(sess_id)?;
            let stdin = io::stdin();
            let mut reader = stdin.lock();
            let mut stdout = io::stdout();

            run_interactive_tui(sess_arc, &mut reader, &mut stdout)?;
            Ok(Value::Bool(true))
        }),
    );

    let val = Value::map(mod_map);
    globals.insert("__native_tui".to_string(), val.clone());
    globals.insert("timetravel_tui".to_string(), val.clone());
    globals.insert("aether_tui".to_string(), val);
}
