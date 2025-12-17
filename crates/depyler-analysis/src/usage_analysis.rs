//! Variable usage analysis for clone elimination optimization
//!
//! This module implements forward analysis to determine how variables are used
//! after their definition. This enables the transpiler to generate borrows
//! instead of clones when variables are only used in read-only contexts.

use depyler_core::hir::{AssignTarget, HirComprehension, HirExpr, HirFunction, HirStmt, Type};
use std::collections::HashMap;

/// Tracks how a variable is used throughout a function
#[derive(Debug, Clone, Default)]
pub struct VariableUsage {
    /// Variable is used in iteration context (for loop, .iter(), etc.)
    pub iter_uses: u32,
    /// Variable is passed by reference
    pub ref_uses: u32,
    /// Variable is moved/consumed (return, owned param, store)
    pub move_uses: u32,
    /// Variable is mutated
    pub mut_uses: u32,
    /// Variable is used in a closure that may escape
    pub closure_capture: bool,
    /// Variable is the source of a field access assignment
    pub is_field_source: bool,
}

impl VariableUsage {
    /// Returns true if the variable only needs to be borrowed, not owned
    pub fn can_borrow(&self) -> bool {
        self.move_uses == 0 && !self.closure_capture && self.mut_uses == 0
    }
}

/// Context for tracking how an expression is used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageContext {
    /// Used in .iter() or for loop
    Iteration,
    /// Passed as &T
    Reference,
    /// Passed as T (owned) or returned
    Move,
    /// Used as &mut T
    Mutation,
    /// Captured by a closure
    ClosureCapture,
    /// Used as method receiver (usually &self)
    MethodReceiver,
    /// Used in a read-only expression (comparison, arithmetic, etc.)
    ReadOnly,
}

/// Analyzes variable usage patterns within a function
pub struct UsageAnalyzer {
    usages: HashMap<String, VariableUsage>,
    /// Variables assigned from field access (state.field pattern)
    field_source_vars: HashMap<String, String>,
}

impl UsageAnalyzer {
    pub fn new() -> Self {
        Self {
            usages: HashMap::new(),
            field_source_vars: HashMap::new(),
        }
    }

    /// Analyze a function and return usage information for all variables
    pub fn analyze_function(func: &HirFunction) -> HashMap<String, VariableUsage> {
        let mut analyzer = Self::new();
        analyzer.analyze_stmts(&func.body);
        analyzer.usages
    }

    fn analyze_stmts(&mut self, stmts: &[HirStmt]) {
        for stmt in stmts {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Assign {
                target,
                value,
                type_annotation: _,
            } => {
                // Track variables assigned from field access
                if let AssignTarget::Symbol(var_name) = target {
                    if matches!(value, HirExpr::Attribute { .. }) {
                        self.field_source_vars.insert(var_name.clone(), var_name.clone());
                        let usage = self.usages.entry(var_name.clone()).or_default();
                        usage.is_field_source = true;
                    }
                }
                // Analyze the value expression (the RHS is being read)
                self.analyze_expr(value, UsageContext::ReadOnly);
                // Analyze target for mutations (e.g., x[i] = val means x is mutated)
                self.analyze_assign_target(target);
            }
            HirStmt::Return(Some(expr)) => {
                // Returned values are moved
                self.analyze_expr(expr, UsageContext::Move);
            }
            HirStmt::Return(None) => {}
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_expr(condition, UsageContext::ReadOnly);
                self.analyze_stmts(then_body);
                if let Some(else_stmts) = else_body {
                    self.analyze_stmts(else_stmts);
                }
            }
            HirStmt::While { condition, body } => {
                self.analyze_expr(condition, UsageContext::ReadOnly);
                self.analyze_stmts(body);
            }
            HirStmt::For { target, iter, body } => {
                // The iterator is used in iteration context
                self.analyze_expr(iter, UsageContext::Iteration);
                self.analyze_stmts(body);
                // Don't mark the loop variable as moved - it's created fresh each iteration
                let _ = target;
            }
            HirStmt::Expr(expr) => {
                self.analyze_expr(expr, UsageContext::ReadOnly);
            }
            HirStmt::Raise { exception, cause } => {
                if let Some(e) = exception {
                    self.analyze_expr(e, UsageContext::Move);
                }
                if let Some(c) = cause {
                    self.analyze_expr(c, UsageContext::Move);
                }
            }
            HirStmt::Break { .. } | HirStmt::Continue { .. } | HirStmt::Pass => {}
            HirStmt::With { context, body, .. } => {
                self.analyze_expr(context, UsageContext::Move);
                self.analyze_stmts(body);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.analyze_stmts(body);
                for handler in handlers {
                    self.analyze_stmts(&handler.body);
                }
                if let Some(else_stmts) = orelse {
                    self.analyze_stmts(else_stmts);
                }
                if let Some(final_stmts) = finalbody {
                    self.analyze_stmts(final_stmts);
                }
            }
            HirStmt::Assert { test, msg } => {
                self.analyze_expr(test, UsageContext::ReadOnly);
                if let Some(m) = msg {
                    self.analyze_expr(m, UsageContext::ReadOnly);
                }
            }
            HirStmt::FunctionDef { body, .. } => {
                // Nested functions - analyze but be conservative about captures
                self.analyze_stmts(body);
            }
        }
    }

    fn analyze_assign_target(&mut self, target: &AssignTarget) {
        match target {
            AssignTarget::Symbol(_) => {
                // Simple assignment doesn't mutate the variable itself
            }
            AssignTarget::Index { base, index } => {
                // Subscript assignment mutates the base
                self.analyze_expr(base, UsageContext::Mutation);
                self.analyze_expr(index, UsageContext::ReadOnly);
            }
            AssignTarget::Slice {
                base,
                start,
                stop,
                step,
            } => {
                self.analyze_expr(base, UsageContext::Mutation);
                if let Some(s) = start {
                    self.analyze_expr(s, UsageContext::ReadOnly);
                }
                if let Some(s) = stop {
                    self.analyze_expr(s, UsageContext::ReadOnly);
                }
                if let Some(s) = step {
                    self.analyze_expr(s, UsageContext::ReadOnly);
                }
            }
            AssignTarget::Attribute { value, .. } => {
                // Setting an attribute mutates the object
                self.analyze_expr(value, UsageContext::Mutation);
            }
            AssignTarget::Tuple(targets) => {
                for t in targets {
                    self.analyze_assign_target(t);
                }
            }
        }
    }

    fn analyze_expr(&mut self, expr: &HirExpr, context: UsageContext) {
        match expr {
            HirExpr::Var(name) => {
                self.record_use(name, context);
            }
            HirExpr::Literal(_) => {}
            HirExpr::Binary { left, right, .. } => {
                self.analyze_expr(left, UsageContext::ReadOnly);
                self.analyze_expr(right, UsageContext::ReadOnly);
            }
            HirExpr::Unary { operand, .. } => {
                self.analyze_expr(operand, UsageContext::ReadOnly);
            }
            HirExpr::Call { args, kwargs, .. } => {
                // Arguments to function calls - conservatively treat as move
                // unless we have signature info (future enhancement)
                for arg in args {
                    self.analyze_expr(arg, UsageContext::Move);
                }
                for (_, v) in kwargs {
                    self.analyze_expr(v, UsageContext::Move);
                }
            }
            HirExpr::MethodCall {
                object,
                method,
                args,
                kwargs,
                ..
            } => {
                // Detect iteration patterns
                let method_str = method.as_str();
                if matches!(method_str, "iter" | "into_iter" | "iter_mut") {
                    self.analyze_expr(object, UsageContext::Iteration);
                } else if is_mutating_method(method_str) {
                    self.analyze_expr(object, UsageContext::Mutation);
                } else {
                    self.analyze_expr(object, UsageContext::MethodReceiver);
                }
                for arg in args {
                    self.analyze_expr(arg, UsageContext::Move);
                }
                for (_, v) in kwargs {
                    self.analyze_expr(v, UsageContext::Move);
                }
            }
            HirExpr::Index { base, index } => {
                self.analyze_expr(base, UsageContext::Reference);
                self.analyze_expr(index, UsageContext::ReadOnly);
            }
            HirExpr::Slice {
                base,
                start,
                stop,
                step,
            } => {
                self.analyze_expr(base, UsageContext::Reference);
                if let Some(s) = start {
                    self.analyze_expr(s, UsageContext::ReadOnly);
                }
                if let Some(s) = stop {
                    self.analyze_expr(s, UsageContext::ReadOnly);
                }
                if let Some(s) = step {
                    self.analyze_expr(s, UsageContext::ReadOnly);
                }
            }
            HirExpr::Attribute { value, .. } => {
                // Field access - the object is read
                self.analyze_expr(value, UsageContext::Reference);
            }
            HirExpr::List(elements) | HirExpr::Set(elements) | HirExpr::FrozenSet(elements) => {
                for e in elements {
                    self.analyze_expr(e, UsageContext::Move);
                }
            }
            HirExpr::Dict(items) => {
                for (k, v) in items {
                    self.analyze_expr(k, UsageContext::Move);
                    self.analyze_expr(v, UsageContext::Move);
                }
            }
            HirExpr::Tuple(elements) => {
                for e in elements {
                    self.analyze_expr(e, context);
                }
            }
            HirExpr::Borrow { expr, .. } => {
                self.analyze_expr(expr, UsageContext::Reference);
            }
            HirExpr::ListComp {
                element,
                iter,
                condition,
                ..
            } => {
                self.analyze_expr(iter, UsageContext::Iteration);
                self.analyze_expr(element, UsageContext::Move);
                if let Some(c) = condition {
                    self.analyze_expr(c, UsageContext::ReadOnly);
                }
            }
            HirExpr::SetComp {
                element,
                iter,
                condition,
                ..
            } => {
                self.analyze_expr(iter, UsageContext::Iteration);
                self.analyze_expr(element, UsageContext::Move);
                if let Some(c) = condition {
                    self.analyze_expr(c, UsageContext::ReadOnly);
                }
            }
            HirExpr::DictComp {
                key,
                value,
                iter,
                condition,
                ..
            } => {
                self.analyze_expr(iter, UsageContext::Iteration);
                self.analyze_expr(key, UsageContext::Move);
                self.analyze_expr(value, UsageContext::Move);
                if let Some(c) = condition {
                    self.analyze_expr(c, UsageContext::ReadOnly);
                }
            }
            HirExpr::Lambda { body, .. } => {
                // Lambda captures - be conservative
                self.mark_closure_captures(body);
            }
            HirExpr::Await { value } => {
                self.analyze_expr(value, UsageContext::Move);
            }
            HirExpr::FString { parts } => {
                for part in parts {
                    if let depyler_core::hir::FStringPart::Expr(e) = part {
                        self.analyze_expr(e, UsageContext::Reference);
                    }
                }
            }
            HirExpr::Yield { value } => {
                if let Some(v) = value {
                    self.analyze_expr(v, UsageContext::Move);
                }
            }
            HirExpr::Uninitialized => {}
            HirExpr::IfExpr { test, body, orelse } => {
                self.analyze_expr(test, UsageContext::ReadOnly);
                self.analyze_expr(body, context);
                self.analyze_expr(orelse, context);
            }
            HirExpr::SortByKey { iterable, key_body, .. } => {
                self.analyze_expr(iterable, UsageContext::Iteration);
                self.analyze_expr(key_body, UsageContext::ReadOnly);
            }
            HirExpr::GeneratorExp { element, generators } => {
                for gen in generators.iter() {
                    self.analyze_comprehension(gen);
                }
                self.analyze_expr(element, UsageContext::Move);
            }
        }
    }

    fn analyze_comprehension(&mut self, comp: &HirComprehension) {
        self.analyze_expr(&comp.iter, UsageContext::Iteration);
        for cond in &comp.conditions {
            self.analyze_expr(cond, UsageContext::ReadOnly);
        }
    }

    fn mark_closure_captures(&mut self, expr: &HirExpr) {
        // Walk the expression and mark any variable references as closure captures
        match expr {
            HirExpr::Var(name) => {
                let usage = self.usages.entry(name.clone()).or_default();
                usage.closure_capture = true;
            }
            HirExpr::Binary { left, right, .. } => {
                self.mark_closure_captures(left);
                self.mark_closure_captures(right);
            }
            HirExpr::Unary { operand, .. } => {
                self.mark_closure_captures(operand);
            }
            HirExpr::Call { args, kwargs, .. } => {
                for arg in args {
                    self.mark_closure_captures(arg);
                }
                for (_, v) in kwargs {
                    self.mark_closure_captures(v);
                }
            }
            HirExpr::MethodCall {
                object, args, kwargs, ..
            } => {
                self.mark_closure_captures(object);
                for arg in args {
                    self.mark_closure_captures(arg);
                }
                for (_, v) in kwargs {
                    self.mark_closure_captures(v);
                }
            }
            HirExpr::Index { base, index } => {
                self.mark_closure_captures(base);
                self.mark_closure_captures(index);
            }
            HirExpr::Attribute { value, .. } => {
                self.mark_closure_captures(value);
            }
            HirExpr::IfExpr { test, body, orelse } => {
                self.mark_closure_captures(test);
                self.mark_closure_captures(body);
                self.mark_closure_captures(orelse);
            }
            _ => {}
        }
    }

    fn record_use(&mut self, name: &str, context: UsageContext) {
        let usage = self.usages.entry(name.to_string()).or_default();
        match context {
            UsageContext::Iteration => usage.iter_uses += 1,
            UsageContext::Reference | UsageContext::MethodReceiver | UsageContext::ReadOnly => {
                usage.ref_uses += 1;
            }
            UsageContext::Move => usage.move_uses += 1,
            UsageContext::Mutation => usage.mut_uses += 1,
            UsageContext::ClosureCapture => usage.closure_capture = true,
        }
    }
}

impl Default for UsageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a method name indicates mutation
fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
        "append"
            | "extend"
            | "insert"
            | "remove"
            | "pop"
            | "clear"
            | "sort"
            | "reverse"
            | "update"
            | "add"
            | "discard"
            | "push"
            | "push_back"
            | "push_front"
            | "pop_back"
            | "pop_front"
    )
}

/// Get the names of variables that were assigned from field access and can potentially borrow
pub fn get_borrowable_field_vars(func: &HirFunction) -> HashMap<String, VariableUsage> {
    let usages = UsageAnalyzer::analyze_function(func);
    usages
        .into_iter()
        .filter(|(_, usage)| usage.is_field_source && usage.can_borrow())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use depyler_annotations::TranspilationAnnotations;
    use depyler_core::hir::{BinOp, FunctionProperties, Literal};
    use smallvec::smallvec;

    fn make_test_function(body: Vec<HirStmt>) -> HirFunction {
        HirFunction {
            name: "test".to_string(),
            params: smallvec![],
            ret_type: Type::None,
            body,
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        }
    }

    #[test]
    fn test_iteration_only_can_borrow() {
        let body = vec![
            HirStmt::Assign {
                target: AssignTarget::Symbol("players".to_string()),
                value: HirExpr::Attribute {
                    value: Box::new(HirExpr::Var("state".to_string())),
                    attr: "all_players".to_string(),
                },
                type_annotation: None,
            },
            HirStmt::For {
                target: AssignTarget::Symbol("p".to_string()),
                iter: HirExpr::Var("players".to_string()),
                body: vec![HirStmt::Expr(HirExpr::Call {
                    func: "print".to_string(),
                    args: vec![HirExpr::Var("p".to_string())],
                    kwargs: vec![],
                    type_params: vec![],
                })],
            },
        ];

        let func = make_test_function(body);
        let usages = UsageAnalyzer::analyze_function(&func);

        let players_usage = usages.get("players").expect("players should be tracked");
        assert!(players_usage.is_field_source);
        assert_eq!(players_usage.iter_uses, 1);
        assert_eq!(players_usage.move_uses, 0);
        assert!(players_usage.can_borrow());
    }

    #[test]
    fn test_returned_var_cannot_borrow() {
        let body = vec![
            HirStmt::Assign {
                target: AssignTarget::Symbol("players".to_string()),
                value: HirExpr::Attribute {
                    value: Box::new(HirExpr::Var("state".to_string())),
                    attr: "all_players".to_string(),
                },
                type_annotation: None,
            },
            HirStmt::Return(Some(HirExpr::Var("players".to_string()))),
        ];

        let func = make_test_function(body);
        let usages = UsageAnalyzer::analyze_function(&func);

        let players_usage = usages.get("players").expect("players should be tracked");
        assert!(players_usage.is_field_source);
        assert_eq!(players_usage.move_uses, 1);
        assert!(!players_usage.can_borrow());
    }

    #[test]
    fn test_mutated_var_cannot_borrow() {
        let body = vec![
            HirStmt::Assign {
                target: AssignTarget::Symbol("items".to_string()),
                value: HirExpr::Attribute {
                    value: Box::new(HirExpr::Var("state".to_string())),
                    attr: "items".to_string(),
                },
                type_annotation: None,
            },
            HirStmt::Expr(HirExpr::MethodCall {
                object: Box::new(HirExpr::Var("items".to_string())),
                method: "append".to_string(),
                args: vec![HirExpr::Literal(Literal::Int(1))],
                kwargs: vec![],
                type_params: vec![],
            }),
        ];

        let func = make_test_function(body);
        let usages = UsageAnalyzer::analyze_function(&func);

        let items_usage = usages.get("items").expect("items should be tracked");
        assert!(items_usage.is_field_source);
        assert_eq!(items_usage.mut_uses, 1);
        assert!(!items_usage.can_borrow());
    }

    #[test]
    fn test_read_only_operations_can_borrow() {
        let body = vec![
            HirStmt::Assign {
                target: AssignTarget::Symbol("data".to_string()),
                value: HirExpr::Attribute {
                    value: Box::new(HirExpr::Var("obj".to_string())),
                    attr: "data".to_string(),
                },
                type_annotation: None,
            },
            HirStmt::Assign {
                target: AssignTarget::Symbol("length".to_string()),
                value: HirExpr::MethodCall {
                    object: Box::new(HirExpr::Var("data".to_string())),
                    method: "len".to_string(),
                    args: vec![],
                    kwargs: vec![],
                    type_params: vec![],
                },
                type_annotation: None,
            },
        ];

        let func = make_test_function(body);
        let usages = UsageAnalyzer::analyze_function(&func);

        let data_usage = usages.get("data").expect("data should be tracked");
        assert!(data_usage.is_field_source);
        assert!(data_usage.can_borrow());
    }

    #[test]
    fn test_list_comprehension_iteration() {
        let body = vec![
            HirStmt::Assign {
                target: AssignTarget::Symbol("items".to_string()),
                value: HirExpr::Attribute {
                    value: Box::new(HirExpr::Var("state".to_string())),
                    attr: "items".to_string(),
                },
                type_annotation: None,
            },
            HirStmt::Assign {
                target: AssignTarget::Symbol("doubled".to_string()),
                value: HirExpr::ListComp {
                    element: Box::new(HirExpr::Binary {
                        op: BinOp::Mul,
                        left: Box::new(HirExpr::Var("x".to_string())),
                        right: Box::new(HirExpr::Literal(Literal::Int(2))),
                    }),
                    target: "x".to_string(),
                    iter: Box::new(HirExpr::Var("items".to_string())),
                    condition: None,
                },
                type_annotation: None,
            },
        ];

        let func = make_test_function(body);
        let usages = UsageAnalyzer::analyze_function(&func);

        let items_usage = usages.get("items").expect("items should be tracked");
        assert!(items_usage.is_field_source);
        assert_eq!(items_usage.iter_uses, 1);
        assert!(items_usage.can_borrow());
    }
}
