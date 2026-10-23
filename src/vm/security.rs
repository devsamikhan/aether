// ==============================================================================
// AETHER v4.0 — Capability-Based Security Sandbox, Resource Quotas & Static
// Taint / Vulnerability Analyzer
// Pure Rust Standard Library — Zero External Crates
// ==============================================================================

use std::path::{Path, PathBuf};
use crate::syntax::ast::*;

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub allow_fs_read: bool,
    pub allow_fs_write: bool,
    pub allowed_read_roots: Vec<PathBuf>,
    pub allowed_write_roots: Vec<PathBuf>,
    pub allow_net: bool,
    pub allowed_hosts: Vec<String>,
    pub allow_ffi: bool,
    pub allow_env: bool,
    pub max_call_depth: usize,
    pub max_stack_size: usize,
    pub fuel_limit: Option<u64>,
    pub fuel_consumed: u64,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::unrestricted()
    }
}

impl SecurityPolicy {
    /// Default host policy with stack-overflow & stack-size safety guards enabled.
    pub fn unrestricted() -> Self {
        Self {
            allow_fs_read: true,
            allow_fs_write: true,
            allowed_read_roots: Vec::new(),
            allowed_write_roots: Vec::new(),
            allow_net: true,
            allowed_hosts: Vec::new(),
            allow_ffi: true,
            allow_env: true,
            max_call_depth: 1024,
            max_stack_size: 65_536,
            fuel_limit: None,
            fuel_consumed: 0,
        }
    }

    /// Strict zero-trust sandbox policy for untrusted scripts/plugins.
    pub fn strict_sandbox(fuel_limit: u64) -> Self {
        Self {
            allow_fs_read: false,
            allow_fs_write: false,
            allowed_read_roots: Vec::new(),
            allowed_write_roots: Vec::new(),
            allow_net: false,
            allowed_hosts: Vec::new(),
            allow_ffi: false,
            allow_env: false,
            max_call_depth: 256,
            max_stack_size: 16_384,
            fuel_limit: Some(fuel_limit),
            fuel_consumed: 0,
        }
    }

    #[inline]
    pub fn consume_fuel(&mut self, ticks: u64) -> Result<(), String> {
        self.fuel_consumed = self.fuel_consumed.saturating_add(ticks);
        if let Some(limit) = self.fuel_limit {
            if self.fuel_consumed > limit {
                return Err(format!(
                    "ResourceExhaustedError: VM instruction fuel limit ({}) exceeded",
                    limit
                ));
            }
        }
        Ok(())
    }

    pub fn check_fs_read(&self, path: &str) -> Result<(), String> {
        if !self.allow_fs_read {
            return Err(format!(
                "PermissionDeniedError: File system read access to '{}' is blocked by SecurityPolicy",
                path
            ));
        }
        if !self.allowed_read_roots.is_empty() {
            let target = Path::new(path);
            let allowed = self.allowed_read_roots.iter().any(|root| target.starts_with(root));
            if !allowed {
                return Err(format!(
                    "PermissionDeniedError: Read path '{}' is outside allowed sandbox roots",
                    path
                ));
            }
        }
        Ok(())
    }

    pub fn check_fs_write(&self, path: &str) -> Result<(), String> {
        if !self.allow_fs_write {
            return Err(format!(
                "PermissionDeniedError: File system write access to '{}' is blocked by SecurityPolicy",
                path
            ));
        }
        if !self.allowed_write_roots.is_empty() {
            let target = Path::new(path);
            let allowed = self.allowed_write_roots.iter().any(|root| target.starts_with(root));
            if !allowed {
                return Err(format!(
                    "PermissionDeniedError: Write path '{}' is outside allowed sandbox roots",
                    path
                ));
            }
        }
        Ok(())
    }

    pub fn check_net(&self, host_or_url: &str) -> Result<(), String> {
        if !self.allow_net {
            return Err(format!(
                "PermissionDeniedError: Network access to '{}' is blocked by SecurityPolicy",
                host_or_url
            ));
        }
        if !self.allowed_hosts.is_empty() {
            let allowed = self
                .allowed_hosts
                .iter()
                .any(|h| host_or_url.contains(h.as_str()));
            if !allowed {
                return Err(format!(
                    "PermissionDeniedError: Host '{}' is not in allowed network whitelist",
                    host_or_url
                ));
            }
        }
        Ok(())
    }

    pub fn check_ffi(&self, action: &str) -> Result<(), String> {
        if !self.allow_ffi {
            return Err(format!(
                "PermissionDeniedError: FFI / subprocess execution ('{}') is blocked by SecurityPolicy",
                action
            ));
        }
        Ok(())
    }
}

// ==============================================================================
// Static Security Taint & Vulnerability Scanner
// ==============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityFinding {
    pub severity: String, // "HIGH", "MEDIUM", "LOW"
    pub category: String,
    pub message: String,
    pub line: usize,
}

pub fn scan_source_security(source: &str) -> Result<Vec<SecurityFinding>, String> {
    let program = crate::syntax::parse(source)
        .map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    Ok(scan_program_security(&program))
}

pub fn scan_program_security(program: &Program) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut tainted_vars = std::collections::HashSet::new();

    for stmt in &program.statements {
        scan_statement(stmt, &mut tainted_vars, &mut findings);
    }

    findings
}

fn scan_statement(
    stmt: &Statement,
    tainted_vars: &mut std::collections::HashSet<String>,
    findings: &mut Vec<SecurityFinding>,
) {
    match stmt {
        Statement::Let {
            name,
            initializer: Some(init),
            span,
            ..
        } => {
            let lower = name.to_lowercase();
            let is_sensitive_name = lower.contains("password")
                || lower.contains("secret")
                || lower.contains("api_key")
                || lower.contains("apikey")
                || lower.contains("private_key")
                || lower.contains("token");

            if is_sensitive_name {
                if let Expr::Literal(Literal::String(val), _) = init {
                    if !val.is_empty() && !val.starts_with("***") {
                        findings.push(SecurityFinding {
                            severity: "HIGH".to_string(),
                            category: "HardcodedSecret".to_string(),
                            message: format!(
                                "Hardcoded plaintext credential in variable '{}'; wrap with Secret(...) or load from env",
                                name
                            ),
                            line: span.line,
                        });
                    }
                }
            }

            if expr_is_untrusted_input(init, tainted_vars) {
                tainted_vars.insert(name.clone());
            }
            scan_expr(init, tainted_vars, findings);
        }
        Statement::Assign { target, value, .. } => {
            if let Expr::Ident(name, _) = target {
                if expr_is_untrusted_input(value, tainted_vars) {
                    tainted_vars.insert(name.clone());
                }
            }
            scan_expr(value, tainted_vars, findings);
        }
        Statement::FnDef { body, .. } | Statement::IntentDef { body, .. } => {
            for s in &body.statements {
                scan_statement(s, tainted_vars, findings);
            }
        }
        Statement::While {
            condition,
            body,
            span,
        } => {
            if let Expr::Literal(Literal::Bool(true), _) = condition {
                let has_break_or_return = body.statements.iter().any(|s| {
                    matches!(
                        s,
                        Statement::Break(_) | Statement::Return { .. } | Statement::Yield { .. }
                    )
                });
                if !has_break_or_return {
                    findings.push(SecurityFinding {
                        severity: "MEDIUM".to_string(),
                        category: "UnboundedLoop".to_string(),
                        message: "Potential infinite 'while true' loop without direct break/return/yield".to_string(),
                        line: span.line,
                    });
                }
            }
            for s in &body.statements {
                scan_statement(s, tainted_vars, findings);
            }
        }
        Statement::Expr(e) => scan_expr(e, tainted_vars, findings),
        _ => {}
    }
}

fn expr_is_untrusted_input(
    expr: &Expr,
    tainted_vars: &std::collections::HashSet<String>,
) -> bool {
    match expr {
        Expr::Ident(name, _) => {
            tainted_vars.contains(name)
                || name.contains("input")
                || name.contains("user_")
                || name.contains("req_")
                || name.contains("query_param")
        }
        Expr::Call(callee, args, _) => {
            if let Expr::Ident(fn_name, _) = callee.as_ref() {
                if fn_name == "input" {
                    return true;
                }
            }
            args.iter().any(|a| expr_is_untrusted_input(a, tainted_vars))
        }
        Expr::Binary(l, _, r, _) => {
            expr_is_untrusted_input(l, tainted_vars) || expr_is_untrusted_input(r, tainted_vars)
        }
        Expr::FString(parts, _) => parts.iter().any(|p| expr_is_untrusted_input(p, tainted_vars)),
        _ => false,
    }
}

fn scan_expr(
    expr: &Expr,
    tainted_vars: &std::collections::HashSet<String>,
    findings: &mut Vec<SecurityFinding>,
) {
    match expr {
        Expr::Call(callee, args, span) => {
            let callee_name = match callee.as_ref() {
                Expr::Ident(n, _) => n.clone(),
                Expr::Member(obj, m, _) => {
                    if let Expr::Ident(o, _) = obj.as_ref() {
                        format!("{}.{}", o, m)
                    } else {
                        m.clone()
                    }
                }
                _ => String::new(),
            };

            let is_sql_sink = callee_name.ends_with(".execute")
                || callee_name.ends_with(".query")
                || callee_name == "Sql.execute"
                || callee_name == "Database.query";

            if is_sql_sink && !args.is_empty() {
                let first_arg = &args[0];
                let is_dynamic_concat = matches!(
                    first_arg,
                    Expr::Binary(_, BinaryOp::Add, _, _) | Expr::FString(_, _)
                );
                if is_dynamic_concat && expr_is_untrusted_input(first_arg, tainted_vars) {
                    findings.push(SecurityFinding {
                        severity: "HIGH".to_string(),
                        category: "SqlInjectionTaint".to_string(),
                        message: format!(
                            "Tainted input concatenated directly into SQL sink '{}'",
                            callee_name
                        ),
                        line: span.line,
                    });
                }
            }
        }
        Expr::Binary(l, _, r, _) => {
            scan_expr(l, tainted_vars, findings);
            scan_expr(r, tainted_vars, findings);
        }
        _ => {}
    }
}
