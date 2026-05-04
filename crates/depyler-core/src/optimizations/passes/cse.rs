//! Common subexpression elimination (CSE) pass.
//!
//! Implements [`Optimizer`] methods for the `eliminate_cse` family, plus
//! free helper functions for expression hashing and classification.

use super::super::optimizer::Optimizer;
use crate::hir::{AssignTarget, BinOp, HirExpr, HirFunction, HirModule, HirStmt, Literal};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

impl Optimizer {
    pub(crate) fn eliminate_common_subexpressions_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            let mut cse_map: HashMap<u64, (HirExpr, String)> = HashMap::new();
            let mut temp_counter = 0;

            func.body = self.eliminate_cse_in_body(&func.body, &mut cse_map, &mut temp_counter);
        }

        program
    }

    fn eliminate_cse_in_body(
        &self,
        body: &[HirStmt],
        cse_map: &mut HashMap<u64, (HirExpr, String)>,
        temp_counter: &mut usize,
    ) -> Vec<HirStmt> {
        let mut new_body = Vec::new();

        for (idx, stmt) in body.iter().enumerate() {
            let is_final_stmt = idx == body.len() - 1;

            match stmt {
                HirStmt::Assign {
                    target,
                    value,
                    type_annotation,
                } => {
                    let (new_value, mut extra_stmts) =
                        self.process_expr_for_cse(value, cse_map, temp_counter);
                    // Fold CSE temps that are immediately assigned to a named variable.
                    // Transforms: `let _cse_temp_0 = expr; let x = _cse_temp_0;`
                    // Into:       `let x = expr;`
                    let final_value =
                        if let (HirExpr::Var(temp_name), AssignTarget::Symbol(target_name)) =
                            (&new_value, target)
                        {
                            if temp_name.starts_with("_cse_temp_") {
                                self.try_fold_cse_temp(
                                    temp_name,
                                    target_name,
                                    &mut extra_stmts,
                                    cse_map,
                                )
                                .unwrap_or(new_value)
                            } else {
                                new_value
                            }
                        } else {
                            new_value
                        };
                    new_body.extend(extra_stmts);
                    new_body.push(HirStmt::Assign {
                        target: target.clone(),
                        value: final_value,
                        type_annotation: type_annotation.clone(),
                    });
                }
                HirStmt::Return(Some(expr)) => {
                    // This avoids unnecessary `let _cse_temp_0 = expr; _cse_temp_0` pattern
                    if is_final_stmt && self.is_simple_return_expr(expr) {
                        // Don't create CSE temp for final simple returns
                        new_body.push(HirStmt::Return(Some(expr.clone())));
                    } else {
                        let (new_expr, extra_stmts) =
                            self.process_expr_for_cse(expr, cse_map, temp_counter);
                        new_body.extend(extra_stmts);
                        new_body.push(HirStmt::Return(Some(new_expr)));
                    }
                }
                HirStmt::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    // Don't extract simple if-conditions to CSE temps
                    // Only extract if the condition is complex and worth caching
                    let (new_condition, extra_stmts) = if self.should_extract_for_cse(condition) {
                        self.process_expr_for_cse(condition, cse_map, temp_counter)
                    } else {
                        // Keep the condition inline for simple expressions
                        (condition.clone(), Vec::new())
                    };
                    new_body.extend(extra_stmts);

                    // CSE within branches (with separate scopes)
                    let mut then_cse = cse_map.clone();
                    let new_then =
                        self.eliminate_cse_in_body(then_body, &mut then_cse, temp_counter);

                    let new_else = else_body.as_ref().map(|else_stmts| {
                        let mut else_cse = cse_map.clone();
                        self.eliminate_cse_in_body(else_stmts, &mut else_cse, temp_counter)
                    });

                    new_body.push(HirStmt::If {
                        condition: new_condition,
                        then_body: new_then,
                        else_body: new_else,
                    });
                }
                _ => new_body.push(stmt.clone()),
            }
        }

        new_body
    }

    fn process_expr_for_cse(
        &self,
        expr: &HirExpr,
        cse_map: &mut HashMap<u64, (HirExpr, String)>,
        temp_counter: &mut usize,
    ) -> (HirExpr, Vec<HirStmt>) {
        let mut extra_stmts = Vec::new();

        // Only process complex expressions
        match expr {
            HirExpr::Binary { left, right, op } => {
                // Recursively process operands
                let (new_left, left_stmts) = self.process_expr_for_cse(left, cse_map, temp_counter);
                let (new_right, right_stmts) =
                    self.process_expr_for_cse(right, cse_map, temp_counter);
                extra_stmts.extend(left_stmts);
                extra_stmts.extend(right_stmts);

                let new_expr = HirExpr::Binary {
                    op: *op,
                    left: Box::new(new_left),
                    right: Box::new(new_right),
                };

                // Check if this expression is worth caching (not trivial)
                if self.is_complex_expr(&new_expr) {
                    let hash = self.hash_expr(&new_expr);

                    if let Some((_, var_name)) = cse_map.get(&hash) {
                        // Reuse existing computation
                        (HirExpr::Var(var_name.clone()), extra_stmts)
                    } else {
                        // Create new temporary
                        let temp_name = format!("_cse_temp_{}", temp_counter);
                        *temp_counter += 1;

                        extra_stmts.push(HirStmt::Assign {
                            target: AssignTarget::Symbol(temp_name.clone()),
                            value: new_expr.clone(),
                            type_annotation: None,
                        });

                        cse_map.insert(hash, (new_expr, temp_name.clone()));
                        (HirExpr::Var(temp_name), extra_stmts)
                    }
                } else {
                    (new_expr, extra_stmts)
                }
            }
            HirExpr::Call {
                func,
                args,
                type_params,
                ..
            } if self.is_pure_function(func) => {
                // Process arguments
                let mut new_args = Vec::new();
                for arg in args {
                    let (new_arg, arg_stmts) =
                        self.process_expr_for_cse(arg, cse_map, temp_counter);
                    extra_stmts.extend(arg_stmts);
                    new_args.push(new_arg);
                }

                let new_expr = HirExpr::Call {
                    func: func.clone(),
                    args: new_args,
                    kwargs: vec![],
                    type_params: type_params.clone(),
                };

                let hash = self.hash_expr(&new_expr);

                // Only create CSE temp if this expression is reused (already in map)
                // Don't extract on first occurrence to avoid unnecessary temps
                if let Some((_, var_name)) = cse_map.get(&hash) {
                    (HirExpr::Var(var_name.clone()), extra_stmts)
                } else {
                    // Add to map for potential future reuse, but don't create temp yet
                    // This avoids creating temps for single-use expressions
                    (new_expr, extra_stmts)
                }
            }
            HirExpr::MethodCall {
                object,
                method,
                args,
                kwargs,
                type_params,
            } if self.is_pure_method(method) => {
                // Process object and arguments
                let (new_object, object_stmts) =
                    self.process_expr_for_cse(object, cse_map, temp_counter);
                extra_stmts.extend(object_stmts);

                let mut new_args = Vec::new();
                for arg in args {
                    let (new_arg, arg_stmts) =
                        self.process_expr_for_cse(arg, cse_map, temp_counter);
                    extra_stmts.extend(arg_stmts);
                    new_args.push(new_arg);
                }

                let new_expr = HirExpr::MethodCall {
                    object: Box::new(new_object),
                    method: method.clone(),
                    args: new_args,
                    kwargs: kwargs.clone(),
                    type_params: type_params.clone(),
                };

                // Don't extract simple method calls - they're better inline
                // CSE should focus on complex expressions that are actually reused
                (new_expr, extra_stmts)
            }
            _ => (expr.clone(), extra_stmts),
        }
    }

    fn is_complex_expr(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Binary { op, left, right } => {
                // Only consider expressions with nested binary operations as complex enough for CSE.
                // Simple comparisons like `x != "string"` or `a > b` should not create temp variables.
                let has_nested_binary = matches!(left.as_ref(), HirExpr::Binary { .. })
                    || matches!(right.as_ref(), HirExpr::Binary { .. });

                // Arithmetic operations with nested operands are worth CSE'ing
                let is_arithmetic = matches!(
                    op,
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
                );

                has_nested_binary || (is_arithmetic && self.has_expensive_operand(left, right))
            }
            HirExpr::Call { .. } => true,
            _ => false,
        }
    }

    fn contains_named_expr(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::NamedExpr { .. } => true,
            HirExpr::Binary { left, right, .. } => {
                self.contains_named_expr(left) || self.contains_named_expr(right)
            }
            HirExpr::Unary { operand, .. } => self.contains_named_expr(operand),
            HirExpr::Call { args, .. } => args.iter().any(|arg| self.contains_named_expr(arg)),
            HirExpr::MethodCall { object, args, .. } => {
                self.contains_named_expr(object)
                    || args.iter().any(|arg| self.contains_named_expr(arg))
            }
            HirExpr::IfExpr { test, body, orelse } => {
                self.contains_named_expr(test)
                    || self.contains_named_expr(body)
                    || self.contains_named_expr(orelse)
            }
            _ => false,
        }
    }

    /// Check if an expression should be extracted for CSE.
    /// This is more conservative than is_complex_expr - only extract expressions that will
    /// actually be reused or are truly expensive to compute.
    fn should_extract_for_cse(&self, expr: &HirExpr) -> bool {
        // Don't extract expressions containing walrus operators (NamedExpr).
        // The codegen_if_stmt and codegen_while_stmt functions handle walrus hoisting themselves.
        if self.contains_named_expr(expr) {
            return false;
        }

        match expr {
            // Don't extract simple comparisons, they're better inline
            HirExpr::Binary { op, left, right } => {
                // Only extract if it has nested binary ops or expensive calls
                let has_nested = matches!(left.as_ref(), HirExpr::Binary { .. })
                    || matches!(right.as_ref(), HirExpr::Binary { .. });
                let has_calls = self.has_expensive_operand(left, right);

                // Don't extract simple comparisons like `x < 0` or `a == b`
                if matches!(
                    op,
                    BinOp::Eq
                        | BinOp::NotEq
                        | BinOp::Lt
                        | BinOp::LtEq
                        | BinOp::Gt
                        | BinOp::GtEq
                        | BinOp::And
                        | BinOp::Or
                ) && !has_nested
                    && !has_calls
                {
                    return false;
                }

                has_nested || has_calls
            }
            // Don't extract simple literals, variables, or attribute accesses
            HirExpr::Literal(_) | HirExpr::Var(_) | HirExpr::Attribute { .. } => false,
            // Don't extract simple method calls - they're better inline in conditionals
            HirExpr::MethodCall { .. } => false,
            // Extract function calls only if they're known to be pure and expensive
            HirExpr::Call { .. } => false,
            _ => false,
        }
    }

    /// Fold a CSE temp into a named variable assignment when the temp is
    /// only used once and immediately assigned.
    fn try_fold_cse_temp(
        &self,
        temp_name: &str,
        target_name: &str,
        extra_stmts: &mut Vec<HirStmt>,
        cse_map: &mut HashMap<u64, (HirExpr, String)>,
    ) -> Option<HirExpr> {
        if let Some(HirStmt::Assign {
            target: AssignTarget::Symbol(assign_target),
            value: actual_value,
            ..
        }) = extra_stmts.last()
        {
            if assign_target == temp_name {
                let actual = actual_value.clone();
                extra_stmts.pop();
                for (_, (_, var_name)) in cse_map.iter_mut() {
                    if var_name == temp_name {
                        *var_name = target_name.to_string();
                        break;
                    }
                }
                return Some(actual);
            }
        }
        None
    }

    fn has_expensive_operand(&self, left: &HirExpr, right: &HirExpr) -> bool {
        let is_expensive =
            |e: &HirExpr| matches!(e, HirExpr::Call { .. } | HirExpr::MethodCall { .. });
        is_expensive(left) || is_expensive(right)
    }

    /// without creating a CSE temporary variable.
    /// Simple expressions: literals, variables, basic operations, method calls
    fn is_simple_return_expr(&self, expr: &HirExpr) -> bool {
        matches!(
            expr,
            HirExpr::Literal(_)
                | HirExpr::Var(_)
                | HirExpr::Binary { .. }
                | HirExpr::Unary { .. }
                | HirExpr::MethodCall { .. }
                | HirExpr::Call { .. }
                | HirExpr::Attribute { .. }
        )
    }

    fn is_pure_function(&self, func: &str) -> bool {
        // List of known pure functions
        let pure_functions = [
            "abs", "len", "min", "max", "sum", "str", "int", "float", "bool", "round", "pow",
            "sqrt",
        ];
        pure_functions.contains(&func)
    }

    fn is_pure_method(&self, method: &str) -> bool {
        // Methods that are deterministic and have no side effects
        let pure_methods = [
            "index",
            "count",
            "find",
            "rfind",
            "startswith",
            "endswith",
            "isalpha",
            "isdigit",
            "isalnum",
            "isspace",
            "isupper",
            "islower",
            "upper",
            "lower",
            "strip",
            "lstrip",
            "rstrip",
            "split",
            "join",
            "replace",
            "get",
            "keys",
            "values",
            "items",
        ];
        pure_methods.contains(&method)
    }

    fn hash_expr(&self, expr: &HirExpr) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        self.hash_expr_recursive(expr, &mut hasher);
        hasher.finish()
    }

    fn hash_expr_recursive<H: Hasher>(&self, expr: &HirExpr, hasher: &mut H) {
        hash_expr_recursive_inner(expr, hasher);
    }
}

fn hash_expr_recursive_inner<H: Hasher>(expr: &HirExpr, hasher: &mut H) {
    match expr {
        HirExpr::Literal(lit) => {
            "literal".hash(hasher);
            match lit {
                Literal::Int(n) => n.hash(hasher),
                Literal::Float(f) => f.to_bits().hash(hasher),
                Literal::String(s) => s.hash(hasher),
                Literal::Bytes(b) => b.hash(hasher),
                Literal::Bool(b) => b.hash(hasher),
                Literal::None => "none".hash(hasher),
                Literal::Ellipsis => "ellipsis".hash(hasher),
                Literal::Complex(r, i) => {
                    "complex".hash(hasher);
                    r.to_bits().hash(hasher);
                    i.to_bits().hash(hasher);
                }
            }
        }
        HirExpr::Var(name) => {
            "var".hash(hasher);
            name.hash(hasher);
        }
        HirExpr::Binary { op, left, right } => {
            "binary".hash(hasher);
            format!("{:?}", op).hash(hasher);
            hash_expr_recursive_inner(left, hasher);
            hash_expr_recursive_inner(right, hasher);
        }
        HirExpr::Call { func, args, .. } => {
            "call".hash(hasher);
            func.hash(hasher);
            for arg in args {
                hash_expr_recursive_inner(arg, hasher);
            }
        }
        HirExpr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            "method_call".hash(hasher);
            hash_expr_recursive_inner(object, hasher);
            method.hash(hasher);
            for arg in args {
                hash_expr_recursive_inner(arg, hasher);
            }
        }
        _ => {
            // For other expressions, use a simple discriminant
            format!("{:?}", std::mem::discriminant(expr)).hash(hasher);
        }
    }
}

pub(crate) fn is_constant_expr_inner(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Literal(_) => true,
        HirExpr::Unary { operand, .. } => is_constant_expr_inner(operand),
        HirExpr::Binary { left, right, .. } => {
            is_constant_expr_inner(left) && is_constant_expr_inner(right)
        }
        _ => false,
    }
}

pub(crate) fn collect_used_vars_expr_inner(expr: &HirExpr, used: &mut HashMap<String, bool>) {
    match expr {
        HirExpr::Var(name) => {
            used.insert(name.clone(), true);
        }
        HirExpr::Binary { left, right, .. } => {
            collect_used_vars_expr_inner(left, used);
            collect_used_vars_expr_inner(right, used);
        }
        HirExpr::Unary { operand, .. } => {
            collect_used_vars_expr_inner(operand, used);
        }
        HirExpr::List(items) => {
            for item in items {
                collect_used_vars_expr_inner(item, used);
            }
        }
        HirExpr::Tuple(items) => {
            // This was causing dead code elimination to remove assignments
            // for variables used in tuple returns like: return (a, b, c)
            for item in items {
                collect_used_vars_expr_inner(item, used);
            }
        }
        HirExpr::Dict(pairs) => {
            for (k, v) in pairs {
                collect_used_vars_expr_inner(k, used);
                collect_used_vars_expr_inner(v, used);
            }
        }
        HirExpr::Call { func, args, .. } => {
            // Mark the function name as used (important for lambda variables)
            used.insert(func.clone(), true);
            for arg in args {
                collect_used_vars_expr_inner(arg, used);
            }
        }
        HirExpr::MethodCall { object, args, .. } => {
            collect_used_vars_expr_inner(object, used);
            for arg in args {
                collect_used_vars_expr_inner(arg, used);
            }
        }
        HirExpr::Lambda { body, .. } => {
            collect_used_vars_expr_inner(body, used);
        }
        HirExpr::ListComp {
            element,
            iter,
            condition,
            ..
        } => {
            collect_used_vars_expr_inner(element, used);
            collect_used_vars_expr_inner(iter, used);
            if let Some(cond) = condition {
                collect_used_vars_expr_inner(cond, used);
            }
        }
        HirExpr::SetComp {
            element,
            iter,
            condition,
            ..
        } => {
            collect_used_vars_expr_inner(element, used);
            collect_used_vars_expr_inner(iter, used);
            if let Some(cond) = condition {
                collect_used_vars_expr_inner(cond, used);
            }
        }
        HirExpr::Await { value } => {
            collect_used_vars_expr_inner(value, used);
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            // This was causing dead code elimination to remove assignments
            // for variables used in slice operations like: numbers[2:7]
            collect_used_vars_expr_inner(base, used);
            if let Some(start_expr) = start {
                collect_used_vars_expr_inner(start_expr, used);
            }
            if let Some(stop_expr) = stop {
                collect_used_vars_expr_inner(stop_expr, used);
            }
            if let Some(step_expr) = step {
                collect_used_vars_expr_inner(step_expr, used);
            }
        }
        HirExpr::Attribute { value, .. } => {
            // This was causing dead code elimination to remove assignments
            // for variables used in attribute access like: p.x + p.y
            collect_used_vars_expr_inner(value, used);
        }
        HirExpr::Index { base, index } => {
            // This was causing dead code elimination to remove assignments
            // for variables used in indexing like: data[key]
            collect_used_vars_expr_inner(base, used);
            collect_used_vars_expr_inner(index, used);
        }
        _ => {}
    }
}

use crate::annotations::PerformanceHint;

