//! Generator State Analysis
//!
//! Analyzes generator functions to determine:
//! - Which local variables need to be preserved across yields
//! - Yield points and control flow
//! - State machine structure

use crate::hir::{HirExpr, HirFunction, HirStmt, Type};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct GeneratorStateInfo {
    pub state_variables: Vec<StateVariable>,
    pub captured_params: Vec<String>,
    pub yield_count: usize,
    pub has_loops: bool,
}

#[derive(Debug, Clone)]
pub struct StateVariable {
    pub name: String,
    pub ty: Type,
}

impl GeneratorStateInfo {
    pub fn analyze(func: &HirFunction) -> Self {
        let mut analyzer = StateAnalyzer {
            state_variables: Vec::new(),
            captured_params: HashSet::new(),
            yield_count: 0,
            has_loops: false,
            declared_vars: HashSet::new(),
        };

        analyzer.analyze_statements(&func.body);

        let captured_params: Vec<String> = func
            .params
            .iter()
            .filter(|p| analyzer.captured_params.contains(&p.name))
            .map(|p| p.name.clone())
            .collect();

        GeneratorStateInfo {
            state_variables: analyzer.state_variables,
            captured_params,
            yield_count: analyzer.yield_count,
            has_loops: analyzer.has_loops,
        }
    }
}

struct StateAnalyzer {
    state_variables: Vec<StateVariable>,
    captured_params: HashSet<String>,
    yield_count: usize,
    has_loops: bool,
    declared_vars: HashSet<String>,
}

impl StateAnalyzer {
    fn analyze_statements(&mut self, stmts: &[HirStmt]) {
        for stmt in stmts {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Assign {
                target,
                value,
                type_annotation,
            } => {
                self.analyze_assign(target, value, type_annotation);
            }
            HirStmt::For { iter, body, .. } => {
                self.analyze_for_loop(iter, body);
            }
            HirStmt::While { condition, body } => {
                self.analyze_while_loop(condition, body);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_if_stmt(condition, then_body, else_body);
            }
            HirStmt::Expr(expr) | HirStmt::Return(Some(expr)) => {
                self.analyze_expression(expr);
            }
            _ => {}
        }
    }

    fn infer_type_from_expression(expr: &HirExpr) -> Type {
        match expr {
            HirExpr::Literal(lit) => match lit {
                crate::hir::Literal::Int(_) => Type::Int,
                crate::hir::Literal::Float(_) => Type::Float,
                crate::hir::Literal::String(_) => Type::String,
                crate::hir::Literal::Bytes(_) => Type::Custom("bytes".to_string()),
                crate::hir::Literal::Bool(_) => Type::Bool,
                crate::hir::Literal::None => Type::None,
                crate::hir::Literal::Ellipsis => Type::None,
                crate::hir::Literal::Complex(_, _) => Type::Custom("num::Complex<f64>".to_string()),
            },
            HirExpr::List(items) => {
                let elem_type = items
                    .first()
                    .map(Self::infer_type_from_expression)
                    .unwrap_or(Type::Unknown);
                Type::List(Box::new(elem_type))
            }
            HirExpr::Dict(_) => Type::Dict(Box::new(Type::String), Box::new(Type::Unknown)),
            HirExpr::Set(_) => Type::Set(Box::new(Type::Unknown)),
            _ => Type::Unknown,
        }
    }

    fn analyze_assign(&mut self, target: &crate::hir::AssignTarget, value: &HirExpr, type_annotation: &Option<Type>) {
        if let crate::hir::AssignTarget::Symbol(name) = target {
            let name_str = name.as_str();
            if !self.declared_vars.contains(name_str) {
                self.declared_vars.insert(name_str.to_string());
                let ty = type_annotation
                    .clone()
                    .unwrap_or_else(|| Self::infer_type_from_expression(value));
                self.state_variables.push(StateVariable {
                    name: name_str.to_string(),
                    ty,
                });
            }
        }
        self.analyze_expression(value);
    }

    fn analyze_for_loop(&mut self, iter: &HirExpr, body: &[HirStmt]) {
        self.has_loops = true;
        self.analyze_expression(iter);
        self.analyze_statements(body);
    }

    fn analyze_while_loop(&mut self, condition: &HirExpr, body: &[HirStmt]) {
        self.has_loops = true;
        self.analyze_expression(condition);
        self.analyze_statements(body);
    }

    fn analyze_if_stmt(&mut self, condition: &HirExpr, then_body: &[HirStmt], else_body: &Option<Vec<HirStmt>>) {
        self.analyze_expression(condition);
        self.analyze_statements(then_body);
        if let Some(else_stmts) = else_body {
            self.analyze_statements(else_stmts);
        }
    }

    fn analyze_expression(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::Yield { value } => self.analyze_yield(value),
            HirExpr::Var(name) => self.analyze_variable(name),
            HirExpr::Binary { left, right, .. } => self.analyze_binary(left, right),
            HirExpr::Unary { operand, .. } => self.analyze_expression(operand),
            HirExpr::Call { args, .. } | HirExpr::List(args) | HirExpr::Tuple(args) => {
                self.analyze_expressions(args);
            }
            HirExpr::Index { base, index } => self.analyze_binary(base, index),
            HirExpr::MethodCall { object, args, .. } => {
                self.analyze_expression(object);
                self.analyze_expressions(args);
            }
            _ => {}
        }
    }

    fn analyze_yield(&mut self, value: &Option<Box<HirExpr>>) {
        self.yield_count += 1;
        if let Some(v) = value {
            self.analyze_expression(v);
        }
    }

    fn analyze_variable(&mut self, name: &str) {
        let name_str = name;
        if !self.declared_vars.contains(name_str) {
            self.captured_params.insert(name_str.to_string());
        }
    }

    fn analyze_binary(&mut self, left: &HirExpr, right: &HirExpr) {
        self.analyze_expression(left);
        self.analyze_expression(right);
    }

    fn analyze_expressions(&mut self, exprs: &[HirExpr]) {
        for expr in exprs {
            self.analyze_expression(expr);
        }
    }
}
