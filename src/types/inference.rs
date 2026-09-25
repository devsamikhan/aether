use super::unify::{apply_subst, unify, Substitution};
use super::{Type, TypeEnv};
use std::collections::HashMap;

/// Speculative compiler AST nodes for type inference logic checks.
#[derive(Debug, Clone)]
pub enum ASTExpr {
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    CharLiteral(char),
    Var(String),
    Binary(Box<ASTExpr>, String, Box<ASTExpr>),
    Call(String, Vec<ASTExpr>),
    Let(String, Box<ASTExpr>, Box<ASTExpr>),
}

/// Hindley-Milner type inference context generating fresh type variables.
pub struct InferenceContext {
    pub counter: usize,
}

impl InferenceContext {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn next_type_var(&mut self) -> Type {
        self.counter += 1;
        Type::TypeVar(format!("t{}", self.counter))
    }

    /// Infer the type of an expression in the given TypeEnv.
    pub fn infer(&mut self, expr: &ASTExpr, env: &TypeEnv) -> Result<(Type, Substitution), String> {
        match expr {
            ASTExpr::IntLiteral(_) => Ok((Type::Int, HashMap::new())),
            ASTExpr::FloatLiteral(_) => Ok((Type::Float, HashMap::new())),
            ASTExpr::BoolLiteral(_) => Ok((Type::Bool, HashMap::new())),
            ASTExpr::StringLiteral(_) => Ok((Type::String, HashMap::new())),
            ASTExpr::CharLiteral(_) => Ok((Type::Char, HashMap::new())),
            ASTExpr::Var(name) => {
                if let Some(ty) = env.lookup(name) {
                    Ok((ty, HashMap::new()))
                } else {
                    Err(format!("Undefined variable name: '{}'", name))
                }
            }
            ASTExpr::Binary(lhs, op, rhs) => {
                let (t_lhs, s1) = self.infer(lhs, env)?;
                let (t_rhs, s2) = self.infer(rhs, env)?;
                let mut subst = s1;
                subst.extend(s2);

                let unified = unify(&apply_subst(&t_lhs, &subst), &apply_subst(&t_rhs, &subst))?;
                subst.extend(unified);

                let op_type = match op.as_str() {
                    "+" | "-" | "*" | "/" => apply_subst(&t_lhs, &subst),
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => Type::Bool,
                    _ => return Err(format!("Unknown operator: '{}'", op)),
                };
                Ok((op_type, subst))
            }
            ASTExpr::Call(name, args) => {
                let func_type = if let Some(t) = env.lookup(name) {
                    t
                } else {
                    return Err(format!("Undefined function name: '{}'", name));
                };

                let return_var = self.next_type_var();
                let mut arg_types = Vec::new();
                let mut subst = HashMap::new();

                for arg in args {
                    let (t_arg, s) = self.infer(arg, env)?;
                    arg_types.push(t_arg);
                    subst.extend(s);
                }

                let expected_func = Type::Function {
                    params: arg_types,
                    return_type: Box::new(return_var.clone()),
                };

                let unified = unify(&apply_subst(&func_type, &subst), &expected_func)?;
                subst.extend(unified);

                Ok((apply_subst(&return_var, &subst), subst))
            }
            ASTExpr::Let(name, val, body) => {
                let (t_val, s1) = self.infer(val, env)?;
                let mut next_env = TypeEnv::child(env.clone());
                next_env.insert(name, t_val);

                let (t_body, s2) = self.infer(body, &next_env)?;
                let mut subst = s1;
                subst.extend(s2);
                Ok((t_body, subst))
            }
        }
    }

    /// Infer the type of an AETHER 2.0 AST expression in the given TypeEnv.
    pub fn infer_syntax_expr(
        &mut self,
        expr: &crate::syntax::ast::Expr,
        env: &TypeEnv,
    ) -> Result<(Type, Substitution), String> {
        use crate::syntax::ast::*;
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(_) => Ok((Type::Int, HashMap::new())),
                Literal::Float(_) => Ok((Type::Float, HashMap::new())),
                Literal::Bool(_) => Ok((Type::Bool, HashMap::new())),
                Literal::String(_) => Ok((Type::String, HashMap::new())),
                Literal::Nil => Ok((Type::Unit, HashMap::new())),
            },
            Expr::Ident(name, _) => {
                if let Some(ty) = env.lookup(name) {
                    Ok((ty, HashMap::new()))
                } else {
                    Err(format!("Undefined variable name: '{}'", name))
                }
            }
            Expr::Binary(lhs, op, rhs, _) => {
                let (t_lhs, s1) = self.infer_syntax_expr(lhs, env)?;
                let (t_rhs, s2) = self.infer_syntax_expr(rhs, env)?;
                let mut subst = s1;
                subst.extend(s2);

                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                        let unified = unify(&apply_subst(&t_lhs, &subst), &apply_subst(&t_rhs, &subst))?;
                        subst.extend(unified);
                        Ok((apply_subst(&t_lhs, &subst), subst))
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual | BinaryOp::In => {
                        Ok((Type::Bool, subst))
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        let unified1 = unify(&apply_subst(&t_lhs, &subst), &Type::Bool)?;
                        subst.extend(unified1);
                        let unified2 = unify(&apply_subst(&t_rhs, &subst), &Type::Bool)?;
                        subst.extend(unified2);
                        Ok((Type::Bool, subst))
                    }
                }
            }
            Expr::Unary(op, inner, _) => {
                let (t_inner, subst) = self.infer_syntax_expr(inner, env)?;
                match op {
                    UnaryOp::Negate => Ok((t_inner, subst)),
                    UnaryOp::Not => {
                        let unified = unify(&apply_subst(&t_inner, &subst), &Type::Bool)?;
                        let mut new_subst = subst;
                        new_subst.extend(unified);
                        Ok((Type::Bool, new_subst))
                    }
                }
            }
            Expr::Call(callee, args, _) => {
                let (callee_type, mut subst) = self.infer_syntax_expr(callee, env)?;
                let return_var = self.next_type_var();
                let mut arg_types = Vec::new();

                for arg in args {
                    let (t_arg, s) = self.infer_syntax_expr(arg, env)?;
                    arg_types.push(t_arg);
                    subst.extend(s);
                }

                let expected_func = Type::Function {
                    params: arg_types,
                    return_type: Box::new(return_var.clone()),
                };

                let unified = unify(&apply_subst(&callee_type, &subst), &expected_func)?;
                subst.extend(unified);

                Ok((apply_subst(&return_var, &subst), subst))
            }
            Expr::Array(elements, _) => {
                if elements.is_empty() {
                    let elem_var = self.next_type_var();
                    Ok((Type::Vec(Box::new(elem_var)), HashMap::new()))
                } else {
                    let (first_type, mut subst) = self.infer_syntax_expr(&elements[0], env)?;
                    for elem in &elements[1..] {
                        let (elem_type, s) = self.infer_syntax_expr(elem, env)?;
                        subst.extend(s);
                        let unified = unify(&apply_subst(&first_type, &subst), &apply_subst(&elem_type, &subst))?;
                        subst.extend(unified);
                    }
                    Ok((Type::Vec(Box::new(apply_subst(&first_type, &subst))), subst))
                }
            }
            Expr::Index(target, _, _) => {
                let (target_ty, subst) = self.infer_syntax_expr(target, env)?;
                match apply_subst(&target_ty, &subst) {
                    Type::Vec(elem_ty) => Ok((*elem_ty, subst)),
                    Type::Array(elem_ty, _) => Ok((*elem_ty, subst)),
                    Type::String => Ok((Type::Char, subst)),
                    _ => Ok((Type::Unknown, subst)),
                }
            }
            _ => Ok((Type::Unknown, HashMap::new())),
        }
    }
}
