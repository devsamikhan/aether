pub mod actor;
pub mod ai_engine;
pub mod bio;
pub mod bytecode;
pub mod bytecode_cache;
pub mod chem;
pub mod compiler;
pub mod compute;
pub mod crypto;
pub mod dataframe;
pub mod db;
pub mod fiber;
pub mod flow;
pub mod graph;
pub mod hotreload;
pub mod interner;
pub mod interpreter;
pub mod net;
pub mod optimizer;
pub mod proof;
pub mod regex_engine;
pub mod rpc;
pub mod security;
pub mod sql;
pub mod stdlib;
pub mod swarm;
pub mod tensor;
pub mod timetravel;
pub mod timetravel_tui;
pub mod value;
pub mod vector;
pub mod wasm_runtime;

pub use bytecode::{Chunk, OpCode};
pub use compiler::BytecodeCompiler;
pub use interpreter::{TestRecord, VM};
pub use security::SecurityPolicy;
pub use value::{CompiledFunction, Value};

/// Executes an AETHER source string directly via AST/bytecode optimization and VM.
pub fn run_source(source: &str) -> Result<Value, String> {
    let raw_program = crate::syntax::parse(source)
        .map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    let (program, _stats) = optimizer::optimize_program(&raw_program);
    let compiler = BytecodeCompiler::new("<main>", 0);
    let compiled_fn = compiler.compile(&program)?;
    let mut vm = VM::new();
    vm.interpret(compiled_fn)
}

/// Executes an AETHER source string inside a capability-restricted SecurityPolicy sandbox.
pub fn run_source_sandboxed(source: &str, policy: SecurityPolicy) -> Result<Value, String> {
    let raw_program = crate::syntax::parse(source)
        .map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    let (program, _stats) = optimizer::optimize_program(&raw_program);
    let compiler = BytecodeCompiler::new("<main>", 0);
    let compiled_fn = compiler.compile(&program)?;
    let mut vm = VM::new();
    vm.security_policy = policy;
    vm.interpret(compiled_fn)
}
