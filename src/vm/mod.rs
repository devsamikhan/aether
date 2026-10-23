pub mod bytecode;
pub mod compiler;
pub mod crypto;
pub mod db;
pub mod fiber;
pub mod interpreter;
pub mod net;
pub mod stdlib;
pub mod tensor;
pub mod value;
pub mod vector;
pub mod actor;
pub mod wasm_runtime;
pub mod rpc;
pub mod dataframe;
pub mod compute;
pub mod timetravel;
pub mod sql;
pub mod timetravel_tui;
pub mod flow;
pub mod proof;
pub mod graph;
pub mod hotreload;
pub mod ai_engine;
pub mod swarm;
pub mod optimizer;
pub mod security;
pub mod interner;
pub mod bytecode_cache;
pub mod regex_engine;

pub use bytecode::{Chunk, OpCode};
pub use compiler::BytecodeCompiler;
pub use interpreter::VM;
pub use value::{CompiledFunction, Value};
pub use security::SecurityPolicy;

/// Executes an AETHER source string directly via AST/bytecode optimization and VM.
pub fn run_source(source: &str) -> Result<Value, String> {
    let raw_program = crate::syntax::parse(source).map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    let (program, _stats) = optimizer::optimize_program(&raw_program);
    let compiler = BytecodeCompiler::new("<main>", 0);
    let compiled_fn = compiler.compile(&program)?;
    let mut vm = VM::new();
    vm.interpret(compiled_fn)
}

/// Executes an AETHER source string inside a capability-restricted SecurityPolicy sandbox.
pub fn run_source_sandboxed(source: &str, policy: SecurityPolicy) -> Result<Value, String> {
    let raw_program = crate::syntax::parse(source).map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    let (program, _stats) = optimizer::optimize_program(&raw_program);
    let compiler = BytecodeCompiler::new("<main>", 0);
    let compiled_fn = compiler.compile(&program)?;
    let mut vm = VM::new();
    vm.security_policy = policy;
    vm.interpret(compiled_fn)
}
