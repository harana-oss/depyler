//! Stack-promotion pass — `Optimizer`.
//!
//! Promotes small, short-lived `List` literals to fixed-size stack arrays by
//! changing their HIR `type_annotation` to `Type::Array { element_type, size }`.
//!
//! ## Criteria (all must hold)
//!
//! 1. The assignment target is a plain `Symbol`.
//! 2. The value is `HirExpr::List([…])` with ≤ `threshold` elements (default 8).
//! 3. The variable is **never mutated** after construction (no `push`, `pop`,
//!    `append`, `extend`, `remove`, `insert`, `sort`, `reverse`, index-assign).
//! 4. The variable does not **escape** the function (not returned, not passed
//!    as an argument to another function).
//!
//! When all criteria hold the `type_annotation` of the `Assign` statement is
//! set to `Type::Array { element_type, size: ConstGeneric::Literal(n) }`.
//! The code generator should lower this to `[T; N]` instead of `Vec<T>`.

use super::super::optimizer::Optimizer;
use crate::hir::{AssignTarget, ConstGeneric, HirExpr, HirModule, HirStmt, Type};
use std::collections::HashSet;

/// Default maximum number of elements for stack promotion.
pub const STACK_PROMOTION_THRESHOLD: usize = 8;

impl Optimizer {
    pub(crate) fn promote_to_stack_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            promote_in_function(&mut func.body, STACK_PROMOTION_THRESHOLD);
        }
        program
    }
}

fn promote_in_function(body: &mut Vec<HirStmt>, threshold: usize) {
    // 1. Collect candidate variable names (List literals of ≤ threshold elements).
    let mut candidates: Vec<(String, usize, Type)> = Vec::new();
    for stmt in body.iter() {
        if let HirStmt::Assign {
            target: AssignTarget::Symbol(name),
            value: HirExpr::List(elems),
            ..
        } = stmt
        {
            if elems.len() <= threshold {
                let elem_ty = infer_element_type(elems);
                candidates.push((name.clone(), elems.len(), elem_ty));
            }
        }
    }

    if candidates.is_empty() {
        return;
    }

    // 2. Disqualify candidates that are mutated or escaped.
    let mutated = collect_mutated(body);
    let escaped = collect_escaped(body);

    // 3. Apply promotion.
    for stmt in body.iter_mut() {
        if let HirStmt::Assign {
            target: AssignTarget::Symbol(name),
            type_annotation,
            ..
        } = stmt
        {
            if let Some(pos) = candidates.iter().position(|(n, _, _)| n == name) {
                if !mutated.contains(name) && !escaped.contains(name) {
                    let (_, size, elem_ty) = candidates[pos].clone();
                    *type_annotation = Some(Type::Array {
                        element_type: Box::new(elem_ty),
                        size: ConstGeneric::Literal(size),
                    });
                }
            }
        }
    }
}

/// Infer the element type from a list of expressions (best-effort).
fn infer_element_type(elems: &[HirExpr]) -> Type {
    use crate::hir::Literal;
    elems.first().map_or(Type::Unknown, |e| match e {
        HirExpr::Literal(Literal::Int(_)) => Type::Int,
        HirExpr::Literal(Literal::Float(_)) => Type::Float,
        HirExpr::Literal(Literal::Bool(_)) => Type::Bool,
        HirExpr::Literal(Literal::String(_)) => Type::String,
        _ => Type::Unknown,
    })
}

/// Collect names that are mutated via method calls (push, pop, etc.) or
/// index-assignment after their initial construction.
fn collect_mutated(body: &[HirStmt]) -> HashSet<String> {
    let mutating_methods: HashSet<&str> = [
        "push", "pop", "append", "extend", "remove", "insert", "sort",
        "reverse", "clear", "drain", "truncate", "retain",
    ]
    .iter()
    .copied()
    .collect();

    let mut mutated = HashSet::new();
    for stmt in body {
        collect_mutated_stmt(stmt, &mutating_methods, &mut mutated);
    }
    mutated
}

fn collect_mutated_stmt(
    stmt: &HirStmt,
    methods: &HashSet<&str>,
    mutated: &mut HashSet<String>,
) {
    match stmt {
        // Index assignment: x[i] = v
        HirStmt::Assign {
            target: AssignTarget::Index { base, .. },
            ..
        } => {
            if let HirExpr::Var(name) = base.as_ref() {
                mutated.insert(name.clone());
            }
        }
        // Method call as expression: x.push(v)
        HirStmt::Expr(HirExpr::MethodCall { object, method, .. }) => {
            if methods.contains(method.as_str()) {
                if let HirExpr::Var(name) = object.as_ref() {
                    mutated.insert(name.clone());
                }
            }
        }
        HirStmt::If { then_body, else_body, .. } => {
            for s in then_body {
                collect_mutated_stmt(s, methods, mutated);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    collect_mutated_stmt(s, methods, mutated);
                }
            }
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            for s in body {
                collect_mutated_stmt(s, methods, mutated);
            }
        }
        _ => {}
    }
}

/// Collect names that are returned or passed as function arguments.
fn collect_escaped(body: &[HirStmt]) -> HashSet<String> {
    let mut escaped = HashSet::new();
    for stmt in body {
        collect_escaped_stmt(stmt, &mut escaped);
    }
    escaped
}

fn collect_escaped_stmt(stmt: &HirStmt, escaped: &mut HashSet<String>) {
    match stmt {
        HirStmt::Return(Some(expr)) => collect_escaped_expr(expr, escaped),
        HirStmt::Assign { value, .. } => collect_escaped_in_calls(value, escaped),
        HirStmt::Expr(e) => collect_escaped_in_calls(e, escaped),
        HirStmt::If { then_body, else_body, .. } => {
            for s in then_body { collect_escaped_stmt(s, escaped); }
            if let Some(eb) = else_body { for s in eb { collect_escaped_stmt(s, escaped); } }
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            for s in body { collect_escaped_stmt(s, escaped); }
        }
        _ => {}
    }
}

fn collect_escaped_expr(expr: &HirExpr, escaped: &mut HashSet<String>) {
    if let HirExpr::Var(name) = expr {
        escaped.insert(name.clone());
    }
}

fn collect_escaped_in_calls(expr: &HirExpr, escaped: &mut HashSet<String>) {
    match expr {
        HirExpr::Call { args, .. } => {
            for a in args {
                if let HirExpr::Var(name) = a {
                    escaped.insert(name.clone());
                }
            }
        }
        HirExpr::MethodCall { args, .. } => {
            for a in args {
                if let HirExpr::Var(name) = a {
                    escaped.insert(name.clone());
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{HirFunction, FunctionProperties, Literal};
    use crate::annotations::TranspilationAnnotations;
    use crate::optimizations::optimizer::{Optimizer, OptimizerConfig};

    fn optimizer() -> Optimizer {
        Optimizer::new(OptimizerConfig::default())
    }

    fn make_module(body: Vec<HirStmt>) -> HirModule {
        HirModule {
            functions: vec![HirFunction {
                name: "f".to_string(),
                params: vec![],
                ret_type: Type::Unknown,
                body,
                properties: FunctionProperties::default(),
                annotations: TranspilationAnnotations::default(),
                docstring: None,
            }],
            imports: vec![],
            type_aliases: vec![],
            protocols: vec![],
            classes: vec![],
            constants: vec![],
            statements: vec![],
        }
    }

    #[test]
    fn test_small_list_gets_promoted() {
        let stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("xs".to_string()),
            value: HirExpr::List(vec![
                HirExpr::Literal(Literal::Int(1)),
                HirExpr::Literal(Literal::Int(2)),
                HirExpr::Literal(Literal::Int(3)),
            ]),
            type_annotation: None,
        };
        let module = make_module(vec![stmt]);
        let result = optimizer().promote_to_stack_program(module);
        if let HirStmt::Assign { type_annotation: Some(ty), .. } = &result.functions[0].body[0] {
            assert!(matches!(ty, Type::Array { size: ConstGeneric::Literal(3), .. }));
        } else {
            panic!("expected promoted type annotation");
        }
    }

    #[test]
    fn test_large_list_not_promoted() {
        let elems: Vec<HirExpr> = (0..9)
            .map(|i| HirExpr::Literal(Literal::Int(i)))
            .collect();
        let stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("xs".to_string()),
            value: HirExpr::List(elems),
            type_annotation: None,
        };
        let module = make_module(vec![stmt]);
        let result = optimizer().promote_to_stack_program(module);
        if let HirStmt::Assign { type_annotation, .. } = &result.functions[0].body[0] {
            assert!(type_annotation.is_none(), "should not promote large list");
        }
    }

    #[test]
    fn test_mutated_list_not_promoted() {
        let s0 = HirStmt::Assign {
            target: AssignTarget::Symbol("xs".to_string()),
            value: HirExpr::List(vec![HirExpr::Literal(Literal::Int(1))]),
            type_annotation: None,
        };
        let s1 = HirStmt::Expr(HirExpr::MethodCall {
            object: Box::new(HirExpr::Var("xs".to_string())),
            method: "push".to_string(),
            args: vec![HirExpr::Literal(Literal::Int(2))],
            kwargs: vec![],
            type_params: vec![],
        });
        let module = make_module(vec![s0, s1]);
        let result = optimizer().promote_to_stack_program(module);
        if let HirStmt::Assign { type_annotation, .. } = &result.functions[0].body[0] {
            assert!(type_annotation.is_none(), "should not promote mutated list");
        }
    }
}
