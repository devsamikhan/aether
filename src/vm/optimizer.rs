// ==============================================================================
// AETHER Multi-Pass Compiler Optimizer (AST Constant Folding, Dead-Code
// Elimination & Bytecode Peephole Optimization)
// Pure Rust Standard Library — Zero External Crates
// ==============================================================================

use crate::syntax::ast::*;
use super::bytecode::{Chunk, OpCode};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OptimizationStats {
    pub constants_folded: usize,
    pub dead_statements_removed: usize,
    pub dead_branches_eliminated: usize,
    pub peephole_optimizations: usize,
}

impl OptimizationStats {
    pub fn total_optimizations(&self) -> usize {
        self.constants_folded
            + self.dead_statements_removed
            + self.dead_branches_eliminated
            + self.peephole_optimizations
    }
}

/// Runs multi-pass AST optimization (Constant Folding + Dead-Code Elimination)
pub fn optimize_program(program: &Program) -> (Program, OptimizationStats) {
    let mut stats = OptimizationStats::default();
    let optimized_stmts = optimize_statement_list(&program.statements, &mut stats);
    (
        Program {
            statements: optimized_stmts,
            span: program.span,
        },
        stats,
    )
}

fn optimize_statement_list(stmts: &[Statement], stats: &mut OptimizationStats) -> Vec<Statement> {
    let mut out = Vec::with_capacity(stmts.len());
    for (idx, stmt) in stmts.iter().enumerate() {
        let opt_stmt = optimize_statement(stmt, stats);
        let is_terminator = matches!(
            opt_stmt,
            Statement::Return { .. }
                | Statement::Break(_)
                | Statement::Continue(_)
                | Statement::Raise { .. }
        );
        out.push(opt_stmt);
        if is_terminator && idx + 1 < stmts.len() {
            stats.dead_statements_removed += stmts.len() - (idx + 1);
            break;
        }
    }
    out
}

fn optimize_block(block: &Block, stats: &mut OptimizationStats) -> Block {
    let statements = optimize_statement_list(&block.statements, stats);
    let result = block
        .result
        .as_ref()
        .map(|expr| Box::new(optimize_expr(expr, stats)));
    Block {
        statements,
        result,
        span: block.span,
    }
}

fn optimize_statement(stmt: &Statement, stats: &mut OptimizationStats) -> Statement {
    match stmt {
        Statement::Let {
            name,
            is_mut,
            type_annotation,
            initializer,
            span,
        } => Statement::Let {
            name: name.clone(),
            is_mut: *is_mut,
            type_annotation: type_annotation.clone(),
            initializer: initializer.as_ref().map(|e| optimize_expr(e, stats)),
            span: *span,
        },
        Statement::LetDestructure {
            pattern,
            is_mut,
            initializer,
            span,
        } => Statement::LetDestructure {
            pattern: pattern.clone(),
            is_mut: *is_mut,
            initializer: optimize_expr(initializer, stats),
            span: *span,
        },
        Statement::Assign {
            target,
            op,
            value,
            span,
        } => Statement::Assign {
            target: optimize_expr(target, stats),
            op: *op,
            value: optimize_expr(value, stats),
            span: *span,
        },
        Statement::FnDef {
            name,
            params,
            return_type,
            body,
            span,
        } => Statement::FnDef {
            name: name.clone(),
            params: params
                .iter()
                .map(|p| Param {
                    name: p.name.clone(),
                    type_ann: p.type_ann.clone(),
                    default_val: p.default_val.as_ref().map(|e| optimize_expr(e, stats)),
                    is_vararg: p.is_vararg,
                    is_kwarg: p.is_kwarg,
                })
                .collect(),
            return_type: return_type.clone(),
            body: optimize_block(body, stats),
            span: *span,
        },
        Statement::IntentDef {
            name,
            params,
            require,
            ensure,
            body,
            span,
        } => Statement::IntentDef {
            name: name.clone(),
            params: params.clone(),
            require: require.iter().map(|e| optimize_expr(e, stats)).collect(),
            ensure: ensure.iter().map(|e| optimize_expr(e, stats)).collect(),
            body: optimize_block(body, stats),
            span: *span,
        },
        Statement::While {
            condition,
            body,
            span,
        } => {
            let opt_cond = optimize_expr(condition, stats);
            if let Expr::Literal(Literal::Bool(false), _) = opt_cond {
                stats.dead_branches_eliminated += 1;
                return Statement::Pass(*span);
            }
            Statement::While {
                condition: opt_cond,
                body: optimize_block(body, stats),
                span: *span,
            }
        }
        Statement::For {
            item,
            iter,
            body,
            span,
        } => Statement::For {
            item: item.clone(),
            iter: optimize_expr(iter, stats),
            body: optimize_block(body, stats),
            span: *span,
        },
        Statement::Return { value, span } => Statement::Return {
            value: value.as_ref().map(|e| optimize_expr(e, stats)),
            span: *span,
        },
        Statement::Yield { value, span } => Statement::Yield {
            value: value.as_ref().map(|e| optimize_expr(e, stats)),
            span: *span,
        },
        Statement::Spawn { body, span } => Statement::Spawn {
            body: optimize_block(body, stats),
            span: *span,
        },
        Statement::ClassDef {
            name,
            bases,
            body,
            span,
        } => Statement::ClassDef {
            name: name.clone(),
            bases: bases.clone(),
            body: optimize_statement_list(body, stats),
            span: *span,
        },
        Statement::ImplDef {
            trait_name,
            target_type,
            methods,
            span,
        } => Statement::ImplDef {
            trait_name: trait_name.clone(),
            target_type: target_type.clone(),
            methods: optimize_statement_list(methods, stats),
            span: *span,
        },
        Statement::Decorated {
            decorators,
            stmt,
            span,
        } => Statement::Decorated {
            decorators: decorators.iter().map(|d| optimize_expr(d, stats)).collect(),
            stmt: Box::new(optimize_statement(stmt, stats)),
            span: *span,
        },
        Statement::Defer { expr, span } => Statement::Defer {
            expr: optimize_expr(expr, stats),
            span: *span,
        },
        Statement::TryCatch {
            try_block,
            handlers,
            finally_block,
            span,
        } => Statement::TryCatch {
            try_block: optimize_block(try_block, stats),
            handlers: handlers
                .iter()
                .map(|h| ExceptHandler {
                    exception_type: h.exception_type.clone(),
                    as_name: h.as_name.clone(),
                    body: optimize_block(&h.body, stats),
                    span: h.span,
                })
                .collect(),
            finally_block: finally_block.as_ref().map(|b| optimize_block(b, stats)),
            span: *span,
        },
        Statement::Raise { expr, span } => Statement::Raise {
            expr: expr.as_ref().map(|e| optimize_expr(e, stats)),
            span: *span,
        },
        Statement::With {
            resource,
            alias,
            body,
            span,
        } => Statement::With {
            resource: optimize_expr(resource, stats),
            alias: alias.clone(),
            body: optimize_block(body, stats),
            span: *span,
        },
        Statement::Expr(expr) => Statement::Expr(optimize_expr(expr, stats)),
        other => other.clone(),
    }
}

pub fn optimize_expr(expr: &Expr, stats: &mut OptimizationStats) -> Expr {
    match expr {
        Expr::Unary(op, inner, span) => {
            let opt_inner = optimize_expr(inner, stats);
            match (op, &opt_inner) {
                (UnaryOp::Negate, Expr::Literal(Literal::Int(i), _)) => {
                    stats.constants_folded += 1;
                    Expr::Literal(Literal::Int(-i), *span)
                }
                (UnaryOp::Negate, Expr::Literal(Literal::Float(f), _)) => {
                    stats.constants_folded += 1;
                    Expr::Literal(Literal::Float(-f), *span)
                }
                (UnaryOp::Not, Expr::Literal(Literal::Bool(b), _)) => {
                    stats.constants_folded += 1;
                    Expr::Literal(Literal::Bool(!b), *span)
                }
                (UnaryOp::Not, Expr::Literal(Literal::Nil, _)) => {
                    stats.constants_folded += 1;
                    Expr::Literal(Literal::Bool(true), *span)
                }
                _ => Expr::Unary(*op, Box::new(opt_inner), *span),
            }
        }
        Expr::Binary(left, op, right, span) => {
            let l = optimize_expr(left, stats);
            let r = optimize_expr(right, stats);

            // Fold constant literals
            if let (Expr::Literal(lit_l, _), Expr::Literal(lit_r, _)) = (&l, &r) {
                if let Some(folded) = fold_binary_literals(lit_l, *op, lit_r) {
                    stats.constants_folded += 1;
                    return Expr::Literal(folded, *span);
                }
            }

            // Null-coalescing with constant left operand
            if *op == BinaryOp::NullCoalesce {
                if let Expr::Literal(lit_l, _) = &l {
                    stats.constants_folded += 1;
                    if *lit_l == Literal::Nil {
                        return r;
                    } else {
                        return l;
                    }
                }
            }

            // Short-circuit boolean identities
            match (op, &l) {
                (BinaryOp::And, Expr::Literal(Literal::Bool(true), _)) => {
                    stats.constants_folded += 1;
                    return r;
                }
                (BinaryOp::And, Expr::Literal(Literal::Bool(false), _)) => {
                    stats.constants_folded += 1;
                    return Expr::Literal(Literal::Bool(false), *span);
                }
                (BinaryOp::Or, Expr::Literal(Literal::Bool(false), _)) => {
                    stats.constants_folded += 1;
                    return r;
                }
                (BinaryOp::Or, Expr::Literal(Literal::Bool(true), _)) => {
                    stats.constants_folded += 1;
                    return Expr::Literal(Literal::Bool(true), *span);
                }
                _ => {}
            }

            Expr::Binary(Box::new(l), *op, Box::new(r), *span)
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            let opt_cond = optimize_expr(condition, stats);
            let opt_then = optimize_block(then_branch, stats);
            let opt_else = else_branch.as_ref().map(|b| optimize_block(b, stats));

            if let Expr::Literal(Literal::Bool(b), _) = opt_cond {
                stats.dead_branches_eliminated += 1;
                stats.dead_statements_removed += 1;
                if b {
                    if opt_then.statements.is_empty() {
                        if let Some(res) = opt_then.result {
                            return *res;
                        }
                    }
                    return Expr::Block(opt_then, *span);
                } else if let Some(eb) = opt_else {
                    if eb.statements.is_empty() {
                        if let Some(res) = eb.result {
                            return *res;
                        }
                    }
                    return Expr::Block(eb, *span);
                } else {
                    return Expr::Literal(Literal::Nil, *span);
                }
            }

            Expr::If {
                condition: Box::new(opt_cond),
                then_branch: opt_then,
                else_branch: opt_else,
                span: *span,
            }
        }
        Expr::Call(callee, args, span) => Expr::Call(
            Box::new(optimize_expr(callee, stats)),
            args.iter().map(|a| optimize_expr(a, stats)).collect(),
            *span,
        ),
        Expr::Array(items, span) => Expr::Array(
            items.iter().map(|e| optimize_expr(e, stats)).collect(),
            *span,
        ),
        Expr::Tuple(items, span) => Expr::Tuple(
            items.iter().map(|e| optimize_expr(e, stats)).collect(),
            *span,
        ),
        Expr::Set(items, span) => Expr::Set(
            items.iter().map(|e| optimize_expr(e, stats)).collect(),
            *span,
        ),
        Expr::Map(entries, span) => Expr::Map(
            entries
                .iter()
                .map(|(k, v)| (optimize_expr(k, stats), optimize_expr(v, stats)))
                .collect(),
            *span,
        ),
        Expr::Pipe(l, r, span) => Expr::Pipe(
            Box::new(optimize_expr(l, stats)),
            Box::new(optimize_expr(r, stats)),
            *span,
        ),
        Expr::Block(b, span) => Expr::Block(optimize_block(b, stats), *span),
        Expr::OptionalMember(obj, m, span) => {
            Expr::OptionalMember(Box::new(optimize_expr(obj, stats)), m.clone(), *span)
        }
        Expr::OptionalIndex(obj, idx, span) => Expr::OptionalIndex(
            Box::new(optimize_expr(obj, stats)),
            Box::new(optimize_expr(idx, stats)),
            *span,
        ),
        Expr::Try(inner, span) => Expr::Try(Box::new(optimize_expr(inner, stats)), *span),
        Expr::Await(inner, span) => Expr::Await(Box::new(optimize_expr(inner, stats)), *span),
        Expr::Comptime(block, span) => Expr::Comptime(optimize_block(block, stats), *span),
        Expr::Slice {
            obj,
            start,
            end,
            step,
            span,
        } => Expr::Slice {
            obj: Box::new(optimize_expr(obj, stats)),
            start: start.as_ref().map(|e| Box::new(optimize_expr(e, stats))),
            end: end.as_ref().map(|e| Box::new(optimize_expr(e, stats))),
            step: step.as_ref().map(|e| Box::new(optimize_expr(e, stats))),
            span: *span,
        },
        other => other.clone(),
    }
}

fn fold_binary_literals(left: &Literal, op: BinaryOp, right: &Literal) -> Option<Literal> {
    match (left, op, right) {
        (Literal::Int(a), BinaryOp::Add, Literal::Int(b)) => a.checked_add(*b).map(Literal::Int),
        (Literal::Int(a), BinaryOp::Sub, Literal::Int(b)) => a.checked_sub(*b).map(Literal::Int),
        (Literal::Int(a), BinaryOp::Mul, Literal::Int(b)) => a.checked_mul(*b).map(Literal::Int),
        (Literal::Int(a), BinaryOp::Div, Literal::Int(b)) if *b != 0 => {
            a.checked_div(*b).map(Literal::Int)
        }
        (Literal::Int(a), BinaryOp::Mod, Literal::Int(b)) if *b != 0 => {
            a.checked_rem(*b).map(Literal::Int)
        }
        (Literal::Int(a), BinaryOp::Pow, Literal::Int(b)) if *b >= 0 && *b <= 62 => {
            a.checked_pow(*b as u32).map(Literal::Int)
        }
        (Literal::Int(a), BinaryOp::Equal, Literal::Int(b)) => Some(Literal::Bool(a == b)),
        (Literal::Int(a), BinaryOp::NotEqual, Literal::Int(b)) => Some(Literal::Bool(a != b)),
        (Literal::Int(a), BinaryOp::Less, Literal::Int(b)) => Some(Literal::Bool(a < b)),
        (Literal::Int(a), BinaryOp::LessEqual, Literal::Int(b)) => Some(Literal::Bool(a <= b)),
        (Literal::Int(a), BinaryOp::Greater, Literal::Int(b)) => Some(Literal::Bool(a > b)),
        (Literal::Int(a), BinaryOp::GreaterEqual, Literal::Int(b)) => Some(Literal::Bool(a >= b)),

        (Literal::Float(a), BinaryOp::Add, Literal::Float(b)) => Some(Literal::Float(a + b)),
        (Literal::Float(a), BinaryOp::Sub, Literal::Float(b)) => Some(Literal::Float(a - b)),
        (Literal::Float(a), BinaryOp::Mul, Literal::Float(b)) => Some(Literal::Float(a * b)),
        (Literal::Float(a), BinaryOp::Div, Literal::Float(b)) if *b != 0.0 => {
            Some(Literal::Float(a / b))
        }
        (Literal::Int(a), BinaryOp::Add, Literal::Float(b)) => Some(Literal::Float(*a as f64 + b)),
        (Literal::Float(a), BinaryOp::Add, Literal::Int(b)) => Some(Literal::Float(a + *b as f64)),
        (Literal::Int(a), BinaryOp::Mul, Literal::Float(b)) => Some(Literal::Float(*a as f64 * b)),
        (Literal::Float(a), BinaryOp::Mul, Literal::Int(b)) => Some(Literal::Float(a * *b as f64)),

        (Literal::String(a), BinaryOp::Add, Literal::String(b)) => {
            Some(Literal::String(format!("{}{}", a, b)))
        }
        (Literal::String(a), BinaryOp::Equal, Literal::String(b)) => Some(Literal::Bool(a == b)),
        (Literal::String(a), BinaryOp::NotEqual, Literal::String(b)) => Some(Literal::Bool(a != b)),

        (Literal::Bool(a), BinaryOp::And, Literal::Bool(b)) => Some(Literal::Bool(*a && *b)),
        (Literal::Bool(a), BinaryOp::Or, Literal::Bool(b)) => Some(Literal::Bool(*a || *b)),
        (Literal::Bool(a), BinaryOp::Equal, Literal::Bool(b)) => Some(Literal::Bool(a == b)),
        (Literal::Bool(a), BinaryOp::NotEqual, Literal::Bool(b)) => Some(Literal::Bool(a != b)),
        _ => None,
    }
}

/// Peephole bytecode inspection pass for redundant instruction sequences
pub fn count_peephole_opportunities(chunk: &Chunk) -> usize {
    let mut count = 0;
    let mut ip = 0;
    let code = &chunk.code;
    while ip < code.len() {
        let op = OpCode::from(code[ip]);
        match op {
            OpCode::Dup if ip + 1 < code.len() && OpCode::from(code[ip + 1]) == OpCode::Pop => {
                count += 1;
                ip += 2;
            }
            OpCode::Constant
            | OpCode::DefineGlobal
            | OpCode::GetGlobal
            | OpCode::SetGlobal
            | OpCode::GetLocal
            | OpCode::SetLocal
            | OpCode::Jump
            | OpCode::JumpIfFalse
            | OpCode::Loop
            | OpCode::BuildClosure
            | OpCode::BuildArray
            | OpCode::BuildMap
            | OpCode::BuildTuple
            | OpCode::BuildSet
            | OpCode::UnpackSequence
            | OpCode::PushExceptionHandler
            | OpCode::ImportModule => {
                ip += 3;
            }
            OpCode::ImportFrom => {
                ip += 5;
            }
            OpCode::Call | OpCode::Spawn => {
                ip += 2;
            }
            OpCode::CallKw => {
                ip += 3;
            }
            _ => {
                ip += 1;
            }
        }
    }
    count
}
