//! Mutation propagation engine for interprocedural analysis
//!
//! This module implements a fixpoint iteration algorithm to propagate
//! mutation information across function boundaries.
//!
//! # Overview
//!
//! When Python code like `update_dict(state.data, "key", 100)` is transpiled,
//! we need to determine that passing `state.data` to a function that mutates
//! its parameter means `state` itself must be mutable.
//!
//! # Algorithm
//!
//! The analysis proceeds in two phases:
//!
//! ## Phase 1: Local Mutation Collection (Intraprocedural)
//!
//! For each function, analyze its body to detect:
//! - Direct parameter mutations: `param.field = value`
//! - Method calls that mutate: `param.list.append(item)`
//! - Index assignments: `param[0] = value`
//!
//! ## Phase 2: Interprocedural Propagation (Fixpoint Iteration)
//!
//! Propagate mutations through function calls until convergence:
//! 1. Process functions in topological order (callees before callers)
//! 2. For each call site, check if arguments are passed to mutating parameters
//! 3. If `callee` mutates parameter `p` and `caller` passes `arg` to `p`:
//!    - Extract root variable from `arg` (e.g., `state.data` → `state`)
//!    - Mark root variable as mutated in `caller`
//! 4. Repeat until no changes (fixpoint)
//!
//! # Example
//!
//! ```python
//! def update_dict(data: dict[str, int], key: str, value: int) -> None:
//!     data[key] = value  # Phase 1: 'data' is mutated
//!
//! def use_helper(state: State) -> None:
//!     update_dict(state.data, "key1", 100)  # Phase 2: 'state' is mutated
//! ```
//!
//! Result after fixpoint:
//! - `update_dict`: mutated_params = {"data"}
//! - `use_helper`: mutated_params = {"state"}  ← propagated!
//!
//! # Integration
//!
//! Results are consumed by:
//! - `BorrowingContext::analyze_function_with_interprocedural()` - marks parameters as mutated
//! - `LifetimeInference` - generates `&mut` instead of `&` for mutated parameters

use crate::expr_utils::extract_root_var;
use crate::hir::{AssignTarget, FStringPart, HirExpr, HirFunction, HirModule, HirStmt};
use crate::interprocedural::call_graph::CallGraph;
use crate::interprocedural::signature_registry::FunctionSignatureRegistry;
use std::collections::{HashMap, HashSet};

/// Information about mutations in a function
#[derive(Debug, Clone)]
pub struct MutationInfo {
    /// Parameters that are mutated in this function
    pub mutated_params: HashSet<String>,
    /// Parameters that are borrowed (immutably)
    pub borrowed_params: HashSet<String>,
    /// Local variables that are mutated
    pub mutated_locals: HashSet<String>,
    /// Functions whose return values are assigned to variables that are then mutated
    pub functions_with_mutated_return: HashSet<String>,
}

impl MutationInfo {
    /// Create empty mutation info
    pub fn new() -> Self {
        Self {
            mutated_params: HashSet::new(),
            borrowed_params: HashSet::new(),
            mutated_locals: HashSet::new(),
            functions_with_mutated_return: HashSet::new(),
        }
    }

    /// Merge another mutation info into this one
    pub fn merge(&mut self, other: &MutationInfo) {
        self.mutated_params
            .extend(other.mutated_params.iter().cloned());
        self.borrowed_params
            .extend(other.borrowed_params.iter().cloned());
        self.mutated_locals
            .extend(other.mutated_locals.iter().cloned());
        self.functions_with_mutated_return
            .extend(other.functions_with_mutated_return.iter().cloned());
    }
}

impl Default for MutationInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of mutation propagation
#[derive(Debug, Clone)]
pub struct PropagationResult {
    /// Mutation information for each function
    pub mutations: HashMap<String, MutationInfo>,
    /// Number of iterations to convergence
    pub iterations: usize,
    /// Whether the analysis converged
    pub converged: bool,
}

/// Propagates mutation information across function boundaries
pub struct MutationPropagator<'a> {
    /// Function signature registry
    registry: &'a FunctionSignatureRegistry,
    /// Call graph
    call_graph: &'a CallGraph,
    /// Mutation info for each function
    mutations: HashMap<String, MutationInfo>,
    /// HIR functions for analysis
    functions: HashMap<String, &'a HirFunction>,
}

impl<'a> MutationPropagator<'a> {
    /// Create a new mutation propagator
    pub fn new(registry: &'a FunctionSignatureRegistry, call_graph: &'a CallGraph) -> Self {
        Self {
            registry,
            call_graph,
            mutations: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    /// Set the HIR functions for analysis
    pub fn with_module(mut self, module: &'a HirModule) -> Self {
        for func in &module.functions {
            self.functions.insert(func.name.clone(), func);
        }
        self
    }

    /// Run fixpoint iteration to propagate mutations
    /// Also updates the registry with mutation information
    pub fn propagate(&mut self) -> PropagationResult {
        // Phase 1: Collect local mutations (intraprocedural)
        self.collect_local_mutations();

        // Phase 2: Propagate through call graph (interprocedural)
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            // Process functions in topological order (callees before callers)
            for func_name in self.call_graph.topological_order() {
                if self.propagate_for_function(&func_name) {
                    changed = true;
                }
            }
        }

        PropagationResult {
            mutations: self.mutations.clone(),
            iterations,
            converged: !changed,
        }
    }

    /// Collect local mutations in each function
    fn collect_local_mutations(&mut self) {
        for (func_name, func) in &self.functions {
            let mut mutation_info = MutationInfo::new();

            // Collect parameters (all are potentially borrowed)
            for param in &func.params {
                mutation_info.borrowed_params.insert(param.name.clone());
            }

            // Phase 1: Track which variables are assigned from function call results
            let mut vars_from_calls: HashMap<String, String> = HashMap::new();
            for stmt in &func.body {
                self.collect_vars_from_calls(stmt, &mut vars_from_calls);
            }

            // Phase 2: Analyze function body for mutations
            for stmt in &func.body {
                self.analyze_stmt_for_mutations(
                    stmt,
                    &mut mutation_info,
                    &func.params.iter().map(|p| p.name.clone()).collect(),
                    &vars_from_calls,
                );
            }

            self.mutations.insert(func_name.to_string(), mutation_info);
        }
    }

    /// Collect variables that are assigned from function call results
    fn collect_vars_from_calls(
        &self,
        stmt: &HirStmt,
        vars_from_calls: &mut HashMap<String, String>,
    ) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                // Check if the value is a function call
                if let AssignTarget::Symbol(var_name) = target {
                    if let HirExpr::Call { func, .. } = value {
                        vars_from_calls.insert(var_name.clone(), func.clone());
                    }
                }
                // Recurse into nested statements if any
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                for stmt in then_body {
                    self.collect_vars_from_calls(stmt, vars_from_calls);
                }
                if let Some(else_body) = else_body {
                    for stmt in else_body {
                        self.collect_vars_from_calls(stmt, vars_from_calls);
                    }
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                for stmt in body {
                    self.collect_vars_from_calls(stmt, vars_from_calls);
                }
            }
            _ => {}
        }
    }

    /// Analyze a statement for mutations
    fn analyze_stmt_for_mutations(
        &self,
        stmt: &HirStmt,
        mutation_info: &mut MutationInfo,
        param_names: &HashSet<String>,
        vars_from_calls: &HashMap<String, String>,
    ) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                // Check what's being mutated
                match target {
                    AssignTarget::Symbol(name) => {
                        if param_names.contains(name) {
                            mutation_info.mutated_params.insert(name.clone());
                        } else {
                            mutation_info.mutated_locals.insert(name.clone());
                        }
                    }
                    AssignTarget::Index { base, .. } => {
                        // Indexing mutates the base
                        if let Some(root_var) = extract_root_var(base) {
                            if param_names.contains(&root_var) {
                                mutation_info.mutated_params.insert(root_var.clone());
                            } else {
                                mutation_info.mutated_locals.insert(root_var.clone());
                            }
                            // Check if this variable was assigned from a function call
                            if let Some(func_name) = vars_from_calls.get(&root_var) {
                                mutation_info
                                    .functions_with_mutated_return
                                    .insert(func_name.clone());
                            }
                        }
                    }
                    AssignTarget::Slice { base, .. } => {
                        // Slice assignment mutates the base
                        if let Some(root_var) = extract_root_var(base) {
                            if param_names.contains(&root_var) {
                                mutation_info.mutated_params.insert(root_var.clone());
                            } else {
                                mutation_info.mutated_locals.insert(root_var.clone());
                            }
                            // Check if this variable was assigned from a function call
                            if let Some(func_name) = vars_from_calls.get(&root_var) {
                                mutation_info
                                    .functions_with_mutated_return
                                    .insert(func_name.clone());
                            }
                        }
                    }
                    AssignTarget::Attribute { value, .. } => {
                        // Attribute assignment mutates the object
                        if let Some(root_var) = extract_root_var(value) {
                            if param_names.contains(&root_var) {
                                mutation_info.mutated_params.insert(root_var.clone());
                            } else {
                                mutation_info.mutated_locals.insert(root_var.clone());
                            }
                            // Check if this variable was assigned from a function call
                            if let Some(func_name) = vars_from_calls.get(&root_var) {
                                mutation_info
                                    .functions_with_mutated_return
                                    .insert(func_name.clone());
                            }
                        }
                    }
                    AssignTarget::Tuple(targets) => {
                        // Tuple unpacking - analyze each target for mutations
                        for t in targets {
                            match t {
                                AssignTarget::Symbol(name) => {
                                    if param_names.contains(name) {
                                        mutation_info.mutated_params.insert(name.clone());
                                    } else {
                                        mutation_info.mutated_locals.insert(name.clone());
                                    }
                                }
                                AssignTarget::Index { base, .. } => {
                                    if let Some(root_var) = extract_root_var(base) {
                                        if param_names.contains(&root_var) {
                                            mutation_info.mutated_params.insert(root_var.clone());
                                        } else {
                                            mutation_info.mutated_locals.insert(root_var.clone());
                                        }
                                        if let Some(func_name) = vars_from_calls.get(&root_var) {
                                            mutation_info
                                                .functions_with_mutated_return
                                                .insert(func_name.clone());
                                        }
                                    }
                                }
                                AssignTarget::Attribute { value, .. } => {
                                    if let Some(root_var) = extract_root_var(value) {
                                        if param_names.contains(&root_var) {
                                            mutation_info.mutated_params.insert(root_var.clone());
                                        } else {
                                            mutation_info.mutated_locals.insert(root_var.clone());
                                        }
                                        if let Some(func_name) = vars_from_calls.get(&root_var) {
                                            mutation_info
                                                .functions_with_mutated_return
                                                .insert(func_name.clone());
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Also analyze the value expression for method calls
                self.analyze_expr_for_mutations(value, mutation_info, param_names, vars_from_calls);
            }
            HirStmt::Expr(expr) => {
                self.analyze_expr_for_mutations(expr, mutation_info, param_names, vars_from_calls);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_expr_for_mutations(
                    condition,
                    mutation_info,
                    param_names,
                    vars_from_calls,
                );
                for stmt in then_body {
                    self.analyze_stmt_for_mutations(
                        stmt,
                        mutation_info,
                        param_names,
                        vars_from_calls,
                    );
                }
                if let Some(else_body) = else_body {
                    for stmt in else_body {
                        self.analyze_stmt_for_mutations(
                            stmt,
                            mutation_info,
                            param_names,
                            vars_from_calls,
                        );
                    }
                }
            }
            HirStmt::While { condition, body } => {
                self.analyze_expr_for_mutations(
                    condition,
                    mutation_info,
                    param_names,
                    vars_from_calls,
                );
                for stmt in body {
                    self.analyze_stmt_for_mutations(
                        stmt,
                        mutation_info,
                        param_names,
                        vars_from_calls,
                    );
                }
            }
            HirStmt::For { iter, body, .. } => {
                self.analyze_expr_for_mutations(iter, mutation_info, param_names, vars_from_calls);
                for stmt in body {
                    self.analyze_stmt_for_mutations(
                        stmt,
                        mutation_info,
                        param_names,
                        vars_from_calls,
                    );
                }
            }
            HirStmt::Return(Some(expr)) => {
                self.analyze_expr_for_mutations(expr, mutation_info, param_names, vars_from_calls);
            }
            _ => {}
        }
    }

    /// Analyze an expression for mutations (mainly method calls)
    fn analyze_expr_for_mutations(
        &self,
        expr: &HirExpr,
        mutation_info: &mut MutationInfo,
        param_names: &HashSet<String>,
        vars_from_calls: &HashMap<String, String>,
    ) {
        match expr {
            HirExpr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                // Check if method is mutating
                if is_mutating_method(method) {
                    if let Some(root_var) = extract_root_var(object) {
                        if param_names.contains(&root_var) {
                            mutation_info.mutated_params.insert(root_var.clone());
                        } else {
                            mutation_info.mutated_locals.insert(root_var.clone());
                        }
                        // Check if this variable was assigned from a function call
                        if let Some(func_name) = vars_from_calls.get(&root_var) {
                            mutation_info
                                .functions_with_mutated_return
                                .insert(func_name.clone());
                        }
                    }
                }

                // Recurse into object and args
                self.analyze_expr_for_mutations(
                    object,
                    mutation_info,
                    param_names,
                    vars_from_calls,
                );
                for arg in args {
                    self.analyze_expr_for_mutations(
                        arg,
                        mutation_info,
                        param_names,
                        vars_from_calls,
                    );
                }
            }
            HirExpr::Call { args, .. } => {
                for arg in args {
                    self.analyze_expr_for_mutations(
                        arg,
                        mutation_info,
                        param_names,
                        vars_from_calls,
                    );
                }
            }
            HirExpr::Binary { left, right, .. } => {
                self.analyze_expr_for_mutations(left, mutation_info, param_names, vars_from_calls);
                self.analyze_expr_for_mutations(right, mutation_info, param_names, vars_from_calls);
            }
            HirExpr::Unary { operand, .. } => {
                self.analyze_expr_for_mutations(
                    operand,
                    mutation_info,
                    param_names,
                    vars_from_calls,
                );
            }
            HirExpr::Attribute { value, .. } => {
                self.analyze_expr_for_mutations(value, mutation_info, param_names, vars_from_calls);
            }
            HirExpr::Index { base, index } => {
                self.analyze_expr_for_mutations(base, mutation_info, param_names, vars_from_calls);
                self.analyze_expr_for_mutations(index, mutation_info, param_names, vars_from_calls);
            }
            _ => {}
        }
    }

    /// Propagate mutations for a specific function by analyzing its calls
    fn propagate_for_function(&mut self, func_name: &str) -> bool {
        let mut changed = false;

        // Get the function
        let Some(func) = self.functions.get(func_name) else {
            return false;
        };

        // Get current mutation info
        let current_mutations = self.mutations.get(func_name).cloned().unwrap_or_default();

        let mut new_mutations = current_mutations.clone();
        let param_names: HashSet<String> = func.params.iter().map(|p| p.name.clone()).collect();

        // Analyze all calls in this function
        for stmt in &func.body {
            if self.propagate_calls_in_stmt(stmt, &mut new_mutations, &param_names) {
                changed = true;
            }
        }

        // Update mutations if changed
        if new_mutations.mutated_params != current_mutations.mutated_params
            || new_mutations.borrowed_params != current_mutations.borrowed_params
        {
            self.mutations.insert(func_name.to_string(), new_mutations);
            changed = true;
        }

        changed
    }

    /// Propagate mutations through calls in a statement
    fn propagate_calls_in_stmt(
        &self,
        stmt: &HirStmt,
        new_mutations: &mut MutationInfo,
        param_names: &HashSet<String>,
    ) -> bool {
        let mut changed = false;

        match stmt {
            HirStmt::Expr(HirExpr::Call { func, args, .. }) => {
                if let Some(callee_sig) = self.registry.get(func) {
                    if let Some(callee_mutations) = self.mutations.get(func) {
                        // For each argument, check if corresponding parameter is mutated in callee
                        for (arg, param) in args.iter().zip(&callee_sig.params) {
                            if callee_mutations.mutated_params.contains(&param.name) {
                                // The argument is passed to a mutating parameter
                                // So we need to mark the root variable as mutated
                                if let Some(root_var) = extract_root_var(arg) {
                                    if param_names.contains(&root_var) {
                                        if !new_mutations.mutated_params.contains(&root_var) {
                                            new_mutations.mutated_params.insert(root_var);
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            HirStmt::Assign { value, .. } => {
                if let HirExpr::Call { func, args, .. } = value {
                    if let Some(callee_sig) = self.registry.get(func) {
                        if let Some(callee_mutations) = self.mutations.get(func) {
                            for (arg, param) in args.iter().zip(&callee_sig.params) {
                                if callee_mutations.mutated_params.contains(&param.name) {
                                    if let Some(root_var) = extract_root_var(arg) {
                                        if param_names.contains(&root_var) {
                                            if !new_mutations.mutated_params.contains(&root_var) {
                                                new_mutations.mutated_params.insert(root_var);
                                                changed = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                changed |= self.propagate_calls_in_expr(condition, new_mutations, param_names);
                for stmt in then_body {
                    changed |= self.propagate_calls_in_stmt(stmt, new_mutations, param_names);
                }
                if let Some(else_body) = else_body {
                    for stmt in else_body {
                        changed |= self.propagate_calls_in_stmt(stmt, new_mutations, param_names);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                changed |= self.propagate_calls_in_expr(condition, new_mutations, param_names);
                for stmt in body {
                    changed |= self.propagate_calls_in_stmt(stmt, new_mutations, param_names);
                }
            }
            HirStmt::For { iter, body, .. } => {
                changed |= self.propagate_calls_in_expr(iter, new_mutations, param_names);
                for stmt in body {
                    changed |= self.propagate_calls_in_stmt(stmt, new_mutations, param_names);
                }
            }
            _ => {}
        }

        changed
    }

    /// Propagate mutations through calls in an expression
    fn propagate_calls_in_expr(
        &self,
        expr: &HirExpr,
        new_mutations: &mut MutationInfo,
        param_names: &HashSet<String>,
    ) -> bool {
        let mut changed = false;

        match expr {
            HirExpr::Call { func, args, .. } => {
                if let Some(callee_sig) = self.registry.get(func) {
                    if let Some(callee_mutations) = self.mutations.get(func) {
                        for (arg, param) in args.iter().zip(&callee_sig.params) {
                            if callee_mutations.mutated_params.contains(&param.name) {
                                if let Some(root_var) = extract_root_var(arg) {
                                    if param_names.contains(&root_var)
                                        && !new_mutations.mutated_params.contains(&root_var)
                                    {
                                        new_mutations.mutated_params.insert(root_var);
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
                // Recurse into arguments
                for arg in args {
                    changed |= self.propagate_calls_in_expr(arg, new_mutations, param_names);
                }
            }
            HirExpr::MethodCall { object, args, .. } => {
                changed |= self.propagate_calls_in_expr(object, new_mutations, param_names);
                for arg in args {
                    changed |= self.propagate_calls_in_expr(arg, new_mutations, param_names);
                }
            }
            HirExpr::Attribute { value, .. } => {
                changed |= self.propagate_calls_in_expr(value, new_mutations, param_names);
            }
            HirExpr::Index { base, index } => {
                changed |= self.propagate_calls_in_expr(base, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(index, new_mutations, param_names);
            }
            HirExpr::Slice {
                base,
                start,
                stop,
                step,
            } => {
                changed |= self.propagate_calls_in_expr(base, new_mutations, param_names);
                if let Some(start) = start {
                    changed |= self.propagate_calls_in_expr(start, new_mutations, param_names);
                }
                if let Some(stop) = stop {
                    changed |= self.propagate_calls_in_expr(stop, new_mutations, param_names);
                }
                if let Some(step) = step {
                    changed |= self.propagate_calls_in_expr(step, new_mutations, param_names);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                changed |= self.propagate_calls_in_expr(left, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(right, new_mutations, param_names);
            }
            HirExpr::Unary { operand, .. } => {
                changed |= self.propagate_calls_in_expr(operand, new_mutations, param_names);
            }
            HirExpr::IfExpr { test, body, orelse } => {
                changed |= self.propagate_calls_in_expr(test, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(body, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(orelse, new_mutations, param_names);
            }
            HirExpr::List(items)
            | HirExpr::Tuple(items)
            | HirExpr::Set(items)
            | HirExpr::FrozenSet(items) => {
                for item in items {
                    changed |= self.propagate_calls_in_expr(item, new_mutations, param_names);
                }
            }
            HirExpr::Dict(entries) => {
                for (key, value) in entries {
                    changed |= self.propagate_calls_in_expr(key, new_mutations, param_names);
                    changed |= self.propagate_calls_in_expr(value, new_mutations, param_names);
                }
            }
            HirExpr::ListComp {
                element,
                iter,
                condition,
                ..
            }
            | HirExpr::SetComp {
                element,
                iter,
                condition,
                ..
            } => {
                changed |= self.propagate_calls_in_expr(element, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(iter, new_mutations, param_names);
                if let Some(cond) = condition {
                    changed |= self.propagate_calls_in_expr(cond, new_mutations, param_names);
                }
            }
            HirExpr::FlattenedListComp { element, generators } => {
                changed |= self.propagate_calls_in_expr(element, new_mutations, param_names);
                for generator in generators {
                    changed |= self.propagate_calls_in_expr(&generator.iter, new_mutations, param_names);
                    for cond in &generator.conditions {
                        changed |= self.propagate_calls_in_expr(cond, new_mutations, param_names);
                    }
                }
            }
            HirExpr::DictComp {
                key,
                value,
                iter,
                condition,
                ..
            } => {
                changed |= self.propagate_calls_in_expr(key, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(value, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(iter, new_mutations, param_names);
                if let Some(cond) = condition {
                    changed |= self.propagate_calls_in_expr(cond, new_mutations, param_names);
                }
            }
            HirExpr::Lambda { body, .. } => {
                changed |= self.propagate_calls_in_expr(body, new_mutations, param_names);
            }
            HirExpr::Borrow { expr, .. } => {
                changed |= self.propagate_calls_in_expr(expr, new_mutations, param_names);
            }
            HirExpr::Await { value } => {
                changed |= self.propagate_calls_in_expr(value, new_mutations, param_names);
            }
            HirExpr::Yield { value } => {
                if let Some(value) = value {
                    changed |= self.propagate_calls_in_expr(value, new_mutations, param_names);
                }
            }
            HirExpr::SortByKey {
                iterable, key_body, ..
            } => {
                changed |= self.propagate_calls_in_expr(iterable, new_mutations, param_names);
                changed |= self.propagate_calls_in_expr(key_body, new_mutations, param_names);
            }
            HirExpr::GeneratorExp {
                element,
                generators,
            } => {
                changed |= self.propagate_calls_in_expr(element, new_mutations, param_names);
                for generator in generators {
                    changed |=
                        self.propagate_calls_in_expr(&generator.iter, new_mutations, param_names);
                    for cond in &generator.conditions {
                        changed |= self.propagate_calls_in_expr(cond, new_mutations, param_names);
                    }
                }
            }
            HirExpr::FString { parts } => {
                for part in parts {
                    if let FStringPart::Expr(expr) = part {
                        changed |= self.propagate_calls_in_expr(expr, new_mutations, param_names);
                    }
                }
            }
            HirExpr::NamedExpr { value, .. } => {
                changed |= self.propagate_calls_in_expr(value, new_mutations, param_names);
            }
            // Leaf expressions - no recursion needed
            HirExpr::Literal(_) | HirExpr::Var(_) | HirExpr::Uninitialized => {}
        }

        changed
    }
}

/// Check if a method name represents a mutating operation
fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
        // List methods
        "append" | "extend" | "insert" | "remove" | "pop" | "clear" | "reverse" | "sort" |
        // Dict methods
        "update" | "setdefault" | "popitem" |
        // Set methods
        "add" | "discard" | "difference_update" | "intersection_update" | 
        "symmetric_difference_update" | "union_update" |
        // Other mutating methods
        "push" | "pop_front" | "push_front" | "pop_back" | "push_back"
    )
}
