//! Enhanced borrowing context for proper ownership pattern inference
//!
//! This module provides comprehensive analysis of parameter usage patterns
//! to determine optimal borrowing strategies for function parameters.
//!
//! # Purpose
//!
//! Translating Python to Rust requires deciding whether parameters should be:
//! - **Owned**: `param: T` - takes ownership, can move/consume
//! - **Immutable borrow**: `param: &T` - read-only access
//! - **Mutable borrow**: `param: &mut T` - can mutate but doesn't own
//! - **Cow**: `param: Cow<T>` - flexible, can clone if needed
//!
//! # Analysis Process
//!
//! `BorrowingContext` tracks how each parameter is used:
//! 1. **Read**: Accessing fields/methods without mutation → `&T`
//! 2. **Mutate**: Assigning to fields, mutating methods → `&mut T`
//! 3. **Move**: Passed to function taking ownership → `T`
//! 4. **Escape**: Returned from function → affects lifetime
//! 5. **Store**: Kept in struct/container → affects lifetime
//!
//! # Interprocedural Integration
//!
//! When `analyze_function_with_interprocedural()` is called with
//! `InterproceduralAnalysis`, it enhances intraprocedural analysis:
//!
//! ```text
//! Intraprocedural: state.items.append(x)  → state is mutated
//! Interprocedural: helper(state)          → if helper mutates, state is mutated
//! ```
//!
//! # Example
//!
//! ```python
//! def process(state: State, config: Config) -> None:
//!     state.value = config.multiplier * 2  # state mutated, config read
//! ```
//!
//! Analysis result:
//! - `state`: `is_mutated=true` → generates `state: &mut State`
//! - `config`: `is_read=true` → generates `config: &Config`
//!
//! # Data Flow
//!
//! ```text
//! LifetimeInference::analyze_function_with_interprocedural()
//!     ↓
//! BorrowingContext::analyze_function_with_interprocedural()
//!     ├─ analyze_statement() - intraprocedural mutations
//!     └─ interprocedural.is_param_mutated() - cross-function mutations
//!         ↓
//! determine_strategies() - choose &T, &mut T, T, or Cow<T>
//! ```

use crate::expr_utils::extract_root_var;
use crate::hir::{AssignTarget, HirExpr, HirFunction, HirStmt, Type as PythonType};
use crate::interprocedural::InterproceduralAnalysis;
use crate::type_mapper::{RustType, TypeMapper};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

/// Check if a method name represents a mutating operation
fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
        // List methods
        "append" | "extend" | "insert" | "remove" | "pop" | "clear" | "reverse" | "sort" |
        // Dict methods
        "update" | "setdefault" | "popitem" |
        // Set methods
        "add" | "discard" | "difference_update" | "intersection_update" | "symmetric_difference_update" |
        "union_update" |
        // Other mutating methods
        "push" | "pop_front" | "push_front" | "pop_back" | "push_back"
    )
}

/// Comprehensive borrowing context for analyzing parameter usage
#[derive(Debug)]
pub struct BorrowingContext {
    /// Usage patterns for each parameter
    param_usage: HashMap<String, ParameterUsagePattern>,
    /// Variables that are moved or consumed
    moved_vars: HashSet<String>,
    /// Variables that are borrowed mutably
    mut_borrowed_vars: HashSet<String>,
    /// Variables that are borrowed immutably
    immut_borrowed_vars: HashSet<String>,
    /// Control flow context stack
    context_stack: Vec<AnalysisContext>,
    /// Function return type for escape analysis
    return_type: Option<PythonType>,
    /// Known enum type names (Copy types)
    enum_names: HashSet<String>,
    /// Known Copy struct type names
    copy_structs: HashSet<String>,
}

/// Detailed parameter usage pattern
#[derive(Debug, Clone, Default)]
pub struct ParameterUsagePattern {
    /// Parameter is read without modification
    pub is_read: bool,
    /// Parameter is modified (assigned to)
    pub is_mutated: bool,
    /// Parameter is moved/consumed (passed to function that takes ownership)
    pub is_moved: bool,
    /// Parameter is used after being passed to a function (multi-use pattern)
    /// If true, we can't move to the first function; must borrow
    pub used_after_function_call: bool,
    /// Parameter escapes through return
    pub escapes_through_return: bool,
    /// A field of this parameter escapes through return (e.g., `return state.players`)
    pub field_escapes_through_return: bool,
    /// Parameter is stored in a struct/container
    pub is_stored: bool,
    /// Parameter is used in a closure
    pub used_in_closure: bool,
    /// Parameter is used in loops (affects borrowing strategy)
    pub used_in_loop: bool,
    /// Nested field access patterns
    pub field_accesses: HashSet<String>,
    /// Method calls on the parameter
    pub method_calls: HashSet<String>,
    /// Specific expressions where parameter is used
    pub usage_sites: Vec<UsageSite>,
}

/// Site where a parameter is used
#[derive(Debug, Clone)]
pub struct UsageSite {
    /// Type of usage
    pub usage_type: UsageType,
    /// Whether in a loop context
    pub in_loop: bool,
    /// Whether in a conditional context
    pub in_conditional: bool,
    /// Depth of borrowing (for nested borrows)
    pub borrow_depth: usize,
}

/// Type of parameter usage
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageType {
    /// Simple read access
    Read,
    /// Mutable access
    Write,
    /// Method call that might mutate
    MethodCall(String),
    /// Passed to another function
    FunctionArg { takes_ownership: bool },
    /// Returned from function
    Return,
    /// Stored in a data structure
    Store,
    /// Used in a closure
    Closure { captures_by_value: bool },
    /// Field access
    FieldAccess(String),
    /// Index access
    IndexAccess,
}

/// Analysis context for control flow
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum AnalysisContext {
    Loop,
    Conditional,
    Closure { captures: HashSet<String> },
    Function,
}

/// Result of borrowing analysis
#[derive(Debug, Clone)]
pub struct BorrowingAnalysisResult {
    /// Recommended borrowing strategy for each parameter
    pub param_strategies: IndexMap<String, BorrowingStrategy>,
    /// Parameter usage patterns (for field escape analysis etc.)
    pub param_usage: HashMap<String, ParameterUsagePattern>,
    /// Additional insights
    pub insights: Vec<BorrowingInsight>,
}

/// Recommended borrowing strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowingStrategy {
    /// Take ownership (move)
    TakeOwnership,
    /// Borrow immutably
    BorrowImmutable { lifetime: Option<String> },
    /// Borrow mutably
    BorrowMutable { lifetime: Option<String> },
    /// Use Cow for flexibility
    UseCow { lifetime: String },
    /// Use Arc/Rc for shared ownership
    UseSharedOwnership { is_thread_safe: bool },
}

/// Insights from borrowing analysis
#[derive(Debug, Clone)]
pub enum BorrowingInsight {
    /// Parameter could be borrowed but is currently moved
    UnnecessaryMove(String),
    /// Parameter could use a more specific lifetime
    LifetimeOptimization { param: String, suggestion: String },
    /// Parameter access pattern suggests Copy trait
    SuggestCopyDerive(String),
    /// Multiple mutable borrows detected
    PotentialBorrowConflict {
        param: String,
        locations: Vec<String>,
    },
}

impl BorrowingContext {
    pub fn new(
        return_type: Option<PythonType>,
        enum_names: HashSet<String>,
        copy_structs: HashSet<String>,
    ) -> Self {
        Self {
            param_usage: HashMap::new(),
            moved_vars: HashSet::new(),
            mut_borrowed_vars: HashSet::new(),
            immut_borrowed_vars: HashSet::new(),
            context_stack: vec![AnalysisContext::Function],
            return_type,
            enum_names,
            copy_structs,
        }
    }

    /// Analyze a function to determine optimal borrowing strategies
    pub fn analyze_function(
        &mut self,
        func: &HirFunction,
        type_mapper: &TypeMapper,
    ) -> BorrowingAnalysisResult {
        // Initialize parameter tracking
        for param in &func.params {
            self.param_usage
                .insert(param.name.clone(), ParameterUsagePattern::default());
        }

        // Analyze function body
        for stmt in &func.body {
            self.analyze_statement(stmt);
        }

        // Determine borrowing strategies based on usage patterns
        self.determine_strategies(func, type_mapper)
    }

    /// Analyze a function with interprocedural context
    /// This version incorporates cross-function mutation analysis
    pub fn analyze_function_with_interprocedural(
        &mut self,
        func: &HirFunction,
        type_mapper: &TypeMapper,
        interprocedural: Option<&InterproceduralAnalysis>,
    ) -> BorrowingAnalysisResult {
        // Initialize parameter tracking
        for param in &func.params {
            self.param_usage
                .insert(param.name.clone(), ParameterUsagePattern::default());
        }

        // Analyze function body
        for stmt in &func.body {
            self.analyze_statement(stmt);
        }

        // Apply interprocedural analysis results if available
        if let Some(analysis) = interprocedural {
            for param in &func.params {
                if analysis.is_param_mutated(&func.name, &param.name) {
                    if let Some(usage) = self.param_usage.get_mut(&param.name) {
                        usage.is_mutated = true;
                    }
                }
            }
        }

        // Determine borrowing strategies based on usage patterns
        self.determine_strategies(func, type_mapper)
    }

    /// Analyze a statement for parameter usage
    fn analyze_statement(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                // Check if assigning to a parameter (mutation)
                let in_loop = self.is_in_loop();
                let in_conditional = self.is_in_conditional();
                match target {
                    AssignTarget::Symbol(symbol) => {
                        if let Some(usage) = self.param_usage.get_mut(symbol) {
                            usage.is_mutated = true;
                            usage.usage_sites.push(UsageSite {
                                usage_type: UsageType::Write,
                                in_loop,
                                in_conditional,
                                borrow_depth: 0,
                            });
                        }
                    }
                    AssignTarget::Attribute { value: obj, .. } => {
                        // Assigning to parameter.field mutates the parameter
                        if let Some(root_var) = extract_root_var(obj) {
                            if let Some(usage) = self.param_usage.get_mut(&root_var) {
                                usage.is_mutated = true;
                                usage.usage_sites.push(UsageSite {
                                    usage_type: UsageType::Write,
                                    in_loop,
                                    in_conditional,
                                    borrow_depth: 1,
                                });
                            }
                        }
                    }
                    AssignTarget::Index { base: obj, .. } => {
                        // Assigning to parameter[index] mutates the parameter
                        if let Some(root_var) = extract_root_var(obj) {
                            if let Some(usage) = self.param_usage.get_mut(&root_var) {
                                usage.is_mutated = true;
                                usage.usage_sites.push(UsageSite {
                                    usage_type: UsageType::Write,
                                    in_loop,
                                    in_conditional,
                                    borrow_depth: 1,
                                });
                            }
                        }
                    }
                    _ => {}
                }
                self.analyze_expression(value, 0);
            }
            HirStmt::Return(expr) => {
                if let Some(e) = expr {
                    self.analyze_expression_for_return(e);
                }
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_expression(condition, 0);
                self.context_stack.push(AnalysisContext::Conditional);
                for stmt in then_body {
                    self.analyze_statement(stmt);
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        self.analyze_statement(stmt);
                    }
                }
                self.context_stack.pop();
            }
            HirStmt::While { condition, body } => {
                self.context_stack.push(AnalysisContext::Loop);
                self.analyze_expression(condition, 0);
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.context_stack.pop();
            }
            HirStmt::For {
                target: _,
                iter,
                body,
            } => {
                self.context_stack.push(AnalysisContext::Loop);
                self.analyze_expression(iter, 0);
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.context_stack.pop();
            }
            HirStmt::Expr(expr) => {
                self.analyze_expression(expr, 0);
            }
            HirStmt::Raise { exception, cause } => {
                if let Some(exc) = exception {
                    self.analyze_expression(exc, 0);
                }
                if let Some(c) = cause {
                    self.analyze_expression(c, 0);
                }
            }
            HirStmt::Break { .. } | HirStmt::Continue { .. } | HirStmt::Pass => {
                // Break, continue, and pass don't analyze any expressions
            }
            HirStmt::Assert { test, msg } => {
                // Analyze the test expression and optional message
                self.analyze_expression(test, 0);
                if let Some(message) = msg {
                    self.analyze_expression(message, 0);
                }
            }
            HirStmt::With {
                context,
                target: _,
                body,
            } => {
                // Analyze context expression
                self.analyze_expression(context, 0);

                // Track the target variable if present
                // Note: We don't track local variables here, only parameters

                // Analyze body statements
                for stmt in body {
                    self.analyze_statement(stmt);
                }
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                // Analyze try body
                for stmt in body {
                    self.analyze_statement(stmt);
                }

                // Analyze except handlers
                for handler in handlers {
                    for stmt in &handler.body {
                        self.analyze_statement(stmt);
                    }
                }

                // Analyze else clause
                if let Some(else_stmts) = orelse {
                    for stmt in else_stmts {
                        self.analyze_statement(stmt);
                    }
                }

                // Analyze finally clause
                if let Some(finally_stmts) = finalbody {
                    for stmt in finally_stmts {
                        self.analyze_statement(stmt);
                    }
                }
            }
            // Analyze nested function body for parameter usage
            HirStmt::FunctionDef { body, .. } => {
                for stmt in body {
                    self.analyze_statement(stmt);
                }
            }
            // Global and Nonlocal are declaration markers, no parameter usage
            HirStmt::Global { .. } | HirStmt::Nonlocal { .. } => {}
            // Import and ImportFrom are declaration markers, no parameter usage
            HirStmt::Import { .. } | HirStmt::ImportFrom { .. } => {}
            // AsyncFor - analyze iterator and body
            HirStmt::AsyncFor { iter, body, .. } => {
                self.analyze_expression(iter, 0);
                for stmt in body {
                    self.analyze_statement(stmt);
                }
            }
            // AsyncWith - analyze context and body
            HirStmt::AsyncWith { context, body, .. } => {
                self.analyze_expression(context, 0);
                for stmt in body {
                    self.analyze_statement(stmt);
                }
            }
            // Delete - targets may be mutated (removed)
            HirStmt::Delete { targets } => {
                for target in targets {
                    if let AssignTarget::Symbol(name) = target {
                        if let Some(usage) = self.param_usage.get_mut(name) {
                            usage.is_mutated = true;
                        }
                    }
                }
            }
            // AsyncFunctionDef - analyze body for nested parameter captures
            HirStmt::AsyncFunctionDef { body, .. } => {
                for stmt in body {
                    self.analyze_statement(stmt);
                }
            }
            // Match - analyze subject and all case bodies
            HirStmt::Match { subject, cases } => {
                self.analyze_expression(subject, 0);
                for case in cases {
                    if let Some(guard) = &case.guard {
                        self.analyze_expression(guard, 0);
                    }
                    for stmt in &case.body {
                        self.analyze_statement(stmt);
                    }
                }
            }
        }
    }

    /// Analyze an expression for parameter usage
    fn analyze_expression(&mut self, expr: &HirExpr, borrow_depth: usize) {
        match expr {
            HirExpr::Var(name) => {
                let in_loop = self.is_in_loop();
                let in_conditional = self.is_in_conditional();
                if let Some(usage) = self.param_usage.get_mut(name) {
                    usage.is_read = true;
                    usage.usage_sites.push(UsageSite {
                        usage_type: UsageType::Read,
                        in_loop,
                        in_conditional,
                        borrow_depth,
                    });
                    if in_loop {
                        usage.used_in_loop = true;
                    }
                }
            }
            HirExpr::Attribute { value, attr } => {
                if let HirExpr::Var(name) = &**value {
                    let in_loop = self.is_in_loop();
                    let in_conditional = self.is_in_conditional();
                    if let Some(usage) = self.param_usage.get_mut(name) {
                        usage.field_accesses.insert(attr.clone());
                        usage.usage_sites.push(UsageSite {
                            usage_type: UsageType::FieldAccess(attr.clone()),
                            in_loop,
                            in_conditional,
                            borrow_depth: borrow_depth + 1,
                        });
                    }
                }
                self.analyze_expression(value, borrow_depth + 1);
            }
            HirExpr::Call { func, args, .. } => {
                // Analyze function calls to determine if parameters are moved
                let in_loop = self.is_in_loop();
                let in_conditional = self.is_in_conditional();
                for (i, arg) in args.iter().enumerate() {
                    // Check if directly passing a parameter
                    if let HirExpr::Var(name) = arg {
                        // If so, mark as used_after_function_call
                        if self.moved_vars.contains(name) {
                            if let Some(usage) = self.param_usage.get_mut(name) {
                                usage.used_after_function_call = true;
                            }
                        }

                        let takes_ownership = self.function_takes_ownership(func, i);
                        if let Some(usage) = self.param_usage.get_mut(name) {
                            // Conservative: assume ownership transfer unless we know better
                            if takes_ownership {
                                usage.is_moved = true;
                                self.moved_vars.insert(name.clone());
                            }
                            usage.usage_sites.push(UsageSite {
                                usage_type: UsageType::FunctionArg { takes_ownership },
                                in_loop,
                                in_conditional,
                                borrow_depth,
                            });
                        }
                    } else if let Some(root_var) = extract_root_var(arg) {
                        // and the function mutates that parameter, the root object must be marked as mutated
                        let needs_mut = self.function_requires_mutable_param(func, i);
                        let takes_ownership = self.function_takes_ownership(func, i);

                        if let Some(usage) = self.param_usage.get_mut(&root_var) {
                            // Only mark as mutated if the function actually mutates the param
                            // Taking ownership alone doesn't mean mutation
                            if needs_mut {
                                usage.is_mutated = true;
                                usage.usage_sites.push(UsageSite {
                                    usage_type: UsageType::FunctionArg { takes_ownership },
                                    in_loop,
                                    in_conditional,
                                    borrow_depth: borrow_depth + 1,
                                });
                            } else if takes_ownership {
                                // Track usage but don't mark as mutated
                                usage.usage_sites.push(UsageSite {
                                    usage_type: UsageType::FunctionArg { takes_ownership },
                                    in_loop,
                                    in_conditional,
                                    borrow_depth: borrow_depth + 1,
                                });
                            }
                        }
                    }
                    self.analyze_expression(arg, borrow_depth);
                }
            }
            HirExpr::Index { base, index } => {
                if let HirExpr::Var(name) = &**base {
                    let in_loop = self.is_in_loop();
                    let in_conditional = self.is_in_conditional();
                    if let Some(usage) = self.param_usage.get_mut(name) {
                        usage.usage_sites.push(UsageSite {
                            usage_type: UsageType::IndexAccess,
                            in_loop,
                            in_conditional,
                            borrow_depth: borrow_depth + 1,
                        });
                    }
                }
                self.analyze_expression(base, borrow_depth + 1);
                self.analyze_expression(index, borrow_depth);
            }
            HirExpr::Binary { left, right, .. } => {
                self.analyze_expression(left, borrow_depth);
                self.analyze_expression(right, borrow_depth);
            }
            HirExpr::Unary { operand, .. } => {
                self.analyze_expression(operand, borrow_depth);
            }
            HirExpr::List(elements) | HirExpr::Tuple(elements) => {
                for elem in elements {
                    self.analyze_expression(elem, borrow_depth);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    self.analyze_expression(k, borrow_depth);
                    self.analyze_expression(v, borrow_depth);
                }
            }
            HirExpr::Borrow { expr, mutable } => {
                if let HirExpr::Var(name) = &**expr {
                    if *mutable {
                        self.mut_borrowed_vars.insert(name.clone());
                    } else {
                        self.immut_borrowed_vars.insert(name.clone());
                    }
                }
                self.analyze_expression(expr, borrow_depth + 1);
            }
            HirExpr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                // Check if this is a mutating method call on a parameter
                let in_loop = self.is_in_loop();
                let in_conditional = self.is_in_conditional();
                let is_mutating = is_mutating_method(method);

                // e.g., state.items.append() should mark 'state' as mutated
                if is_mutating {
                    if let Some(root_var) = extract_root_var(object) {
                        if let Some(usage) = self.param_usage.get_mut(&root_var) {
                            usage.is_mutated = true;
                            usage.usage_sites.push(UsageSite {
                                usage_type: UsageType::MethodCall(method.clone()),
                                in_loop,
                                in_conditional,
                                borrow_depth,
                            });
                        }
                    }
                }
                self.analyze_expression(object, borrow_depth);
                for arg in args {
                    self.analyze_expression(arg, borrow_depth);
                }
            }
            HirExpr::Slice {
                base,
                start,
                stop,
                step,
            } => {
                self.analyze_expression(base, borrow_depth);
                if let Some(s) = start {
                    self.analyze_expression(s, borrow_depth);
                }
                if let Some(s) = stop {
                    self.analyze_expression(s, borrow_depth);
                }
                if let Some(s) = step {
                    self.analyze_expression(s, borrow_depth);
                }
            }
            HirExpr::Literal(_) => {}
            HirExpr::ListComp {
                element,
                target: _,
                iter,
                condition,
            } => {
                // List comprehensions create a new scope
                self.context_stack.push(AnalysisContext::Loop);

                // Analyze the iterator
                self.analyze_expression(iter, borrow_depth);

                // The target variable is local to the comprehension
                // We don't track it as a parameter usage

                // Analyze the element expression
                self.analyze_expression(element, borrow_depth);

                // Analyze the condition if present
                if let Some(cond) = condition {
                    self.analyze_expression(cond, borrow_depth);
                }

                self.context_stack.pop();
            }
            HirExpr::FlattenedListComp {
                element,
                generators,
            } => {
                self.context_stack.push(AnalysisContext::Loop);
                for generator in generators {
                    self.analyze_expression(&generator.iter, borrow_depth);
                    for cond in &generator.conditions {
                        self.analyze_expression(cond, borrow_depth);
                    }
                }
                self.analyze_expression(element, borrow_depth);
                self.context_stack.pop();
            }
            HirExpr::FString { .. } => {
                // FString support not yet implemented for borrowing analysis
            }
            HirExpr::Lambda { params: _, body } => {
                // Lambda functions capture variables by reference
                // For now, treat lambda bodies like any other expression
                self.analyze_expression(body, borrow_depth);
            }
            HirExpr::Set(elements) | HirExpr::FrozenSet(elements) => {
                for elem in elements {
                    self.analyze_expression(elem, borrow_depth);
                }
            }
            HirExpr::SetComp {
                element,
                target: _,
                iter,
                condition,
            } => {
                // Set comprehensions create a new scope
                self.context_stack.push(AnalysisContext::Loop);

                // Analyze the iterator
                self.analyze_expression(iter, borrow_depth);

                // The target variable is local to the comprehension
                // We don't track it as a parameter usage

                // Analyze the element expression
                self.analyze_expression(element, borrow_depth);

                // Analyze the condition if present
                if let Some(cond) = condition {
                    self.analyze_expression(cond, borrow_depth);
                }

                self.context_stack.pop();
            }
            HirExpr::DictComp {
                key,
                value,
                target: _,
                iter,
                condition,
            } => {
                // Dict comprehensions create a new scope
                self.context_stack.push(AnalysisContext::Loop);

                // Analyze the iterator
                self.analyze_expression(iter, borrow_depth);

                // The target variable is local to the comprehension
                // We don't track it as a parameter usage

                // Analyze the key and value expressions
                self.analyze_expression(key, borrow_depth);
                self.analyze_expression(value, borrow_depth);

                // Analyze the condition if present
                if let Some(cond) = condition {
                    self.analyze_expression(cond, borrow_depth);
                }

                self.context_stack.pop();
            }
            HirExpr::Await { value } => {
                // Await expressions don't change parameter usage patterns
                self.analyze_expression(value, borrow_depth);
            }
            HirExpr::Yield { value } => {
                // Yield expressions pass values to the iterator
                if let Some(v) = value {
                    self.analyze_expression(v, borrow_depth);
                }
            }
            HirExpr::IfExpr { test, body, orelse } => {
                // Analyze all three parts of the ternary expression
                self.analyze_expression(test, borrow_depth);
                self.analyze_expression(body, borrow_depth);
                self.analyze_expression(orelse, borrow_depth);
            }
            HirExpr::SortByKey {
                iterable, key_body, ..
            } => {
                // Analyze the iterable and the key lambda body
                self.analyze_expression(iterable, borrow_depth);
                self.analyze_expression(key_body, borrow_depth);
            }
            HirExpr::GeneratorExp {
                element,
                generators,
            } => {
                // Analyze element expression and all generator iterables
                self.analyze_expression(element, borrow_depth);
                for generator in generators {
                    self.analyze_expression(&generator.iter, borrow_depth);
                    for cond in &generator.conditions {
                        self.analyze_expression(cond, borrow_depth);
                    }
                }
            }
            HirExpr::NamedExpr { value, .. } => {
                // Named expression (walrus operator) - analyze the value
                self.analyze_expression(value, borrow_depth);
            }
            HirExpr::Uninitialized => {
                // Nothing to analyze for uninitialized marker
            }
        }
    }

    /// Analyze expression in return context
    /// not when used in operations that produce different types (e.g., `key in dict` → bool)
    fn analyze_expression_for_return(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::Var(name) => {
                // Parameter is directly returned - it escapes
                if let Some(usage) = self.param_usage.get_mut(name) {
                    usage.escapes_through_return = true;
                    usage.usage_sites.push(UsageSite {
                        usage_type: UsageType::Return,
                        in_loop: false,
                        in_conditional: false,
                        borrow_depth: 0,
                    });
                }
            }
            HirExpr::Attribute { value, .. } => {
                // Returning a field from a parameter (e.g., state.players)
                // The parameter's field escapes through return
                if let Some(root_var) = extract_root_var(value) {
                    if let Some(usage) = self.param_usage.get_mut(&root_var) {
                        usage.escapes_through_return = true;
                        usage.field_escapes_through_return = true;
                        usage.usage_sites.push(UsageSite {
                            usage_type: UsageType::Return,
                            in_loop: false,
                            in_conditional: false,
                            borrow_depth: 1,
                        });
                    }
                }
            }
            HirExpr::IfExpr { test, body, orelse } => {
                // Ternary expression: propagate return context to both branches
                self.analyze_expression(test, 0);
                self.analyze_expression_for_return(body);
                self.analyze_expression_for_return(orelse);
            }
            HirExpr::Binary { left, right, .. } => {
                // Binary operations (comparisons, arithmetic, etc.) produce NEW values
                // Parameters are used but don't escape - just analyze normally
                self.analyze_expression(left, 0);
                self.analyze_expression(right, 0);
            }
            HirExpr::FString { parts } => {
                // F-strings create NEW String values by formatting
                // Parameters are used but don't escape - just analyze the interpolated expressions
                for part in parts {
                    if let crate::hir::FStringPart::Expr(expr) = part {
                        self.analyze_expression(expr, 0);
                    }
                }
            }
            _ => self.analyze_expression(expr, 0),
        }
    }

    /// Determine if a function takes ownership of its argument
    fn function_takes_ownership(&self, func_name: &str, _arg_index: usize) -> bool {
        // Known functions that borrow
        let borrowing_functions = [
            "len",
            "str",
            "repr",
            "format",
            "print",
            "isinstance",
            "hasattr",
            "getattr",
            "setattr",
            "contains",
            "startswith",
            "endswith",
            "find",
            "index",
            "count",
            "int",
            "float",
            "bool",
        ];

        // Known functions that take ownership
        let ownership_functions = ["append", "extend", "insert", "remove", "pop", "sort"];

        if borrowing_functions.contains(&func_name) {
            false
        } else if ownership_functions.contains(&func_name) {
            true
        } else {
            // Conservative default: assume ownership transfer
            true
        }
    }

    /// Determine if a function requires its argument to be mutable
    /// This is used when passing field accesses (like state.items) to functions
    fn function_requires_mutable_param(&self, func_name: &str, _arg_index: usize) -> bool {
        // Known mutating methods/functions that require &mut T
        let mutating_functions = [
            "append", "extend", "insert", "remove", "pop", "clear", "sort", "reverse", "push",
            "push_str",
            // Custom user functions - would need interprocedural analysis
            // For now, conservatively assume any non-standard library function
            // might mutate if it's not in the known-immutable list
        ];

        mutating_functions.contains(&func_name)
    }

    /// Check if currently in a loop context
    fn is_in_loop(&self) -> bool {
        self.context_stack
            .iter()
            .any(|ctx| matches!(ctx, AnalysisContext::Loop))
    }

    /// Check if currently in a conditional context
    fn is_in_conditional(&self) -> bool {
        self.context_stack
            .iter()
            .any(|ctx| matches!(ctx, AnalysisContext::Conditional))
    }

    /// Determine optimal borrowing strategies based on usage patterns
    fn determine_strategies(
        &self,
        func: &HirFunction,
        type_mapper: &TypeMapper,
    ) -> BorrowingAnalysisResult {
        let mut strategies = IndexMap::new();
        let mut insights = Vec::new();

        for param in &func.params {
            let usage = self
                .param_usage
                .get(&param.name)
                .cloned()
                .unwrap_or_default();
            let rust_type = type_mapper.map_type(&param.ty);

            let strategy = self.determine_parameter_strategy(
                &param.name,
                &usage,
                &rust_type,
                &param.ty,
                &mut insights,
            );

            strategies.insert(param.name.clone(), strategy);
        }

        BorrowingAnalysisResult {
            param_strategies: strategies,
            param_usage: self.param_usage.clone(),
            insights,
        }
    }

    /// Determine strategy for a single parameter
    fn determine_parameter_strategy(
        &self,
        param_name: &str,
        usage: &ParameterUsagePattern,
        rust_type: &RustType,
        python_type: &PythonType,
        insights: &mut Vec<BorrowingInsight>,
    ) -> BorrowingStrategy {
        // Check if type is Copy first - Copy types can simply be copied, no borrowing needed
        if self.is_copy_type(rust_type) {
            insights.push(BorrowingInsight::SuggestCopyDerive(param_name.to_string()));
            return BorrowingStrategy::TakeOwnership; // Cheap to copy
        }

        // This handles multi-use patterns like:
        //   func1(state)
        //   func2(state)  # state used again after func1
        // If we moved to func1, func2 would fail - need to borrow or clone
        if usage.used_after_function_call {
            if usage.is_mutated {
                // Parameter is mutated locally, must borrow mutably
                return BorrowingStrategy::BorrowMutable { lifetime: None };
            }
            // Parameter is NOT mutated locally - use immutable borrow
            // The caller can pass by value and let callees clone if needed,
            // OR if interprocedural analysis shows callees need mut, that will
            // be handled by force_borrow_from_call_chain in codegen_single_param
            return BorrowingStrategy::BorrowImmutable { lifetime: None };
        }

        // This handles cases like:
        //   state.x = 10  # mutation
        //   func(state)   # would be move, but we need to borrow
        // If we took ownership, we couldn't mutate before passing
        if usage.is_moved && usage.is_mutated {
            return BorrowingStrategy::BorrowMutable { lifetime: None };
        }

        // If parameter is moved (but not mutated), check if it's a struct/dataclass type
        // For struct types that are only passed to other functions, prefer borrowing
        // This enables cleaner call chains like first(s) -> second(s) -> third(s)
        if usage.is_moved {
            // For custom/struct types, prefer borrowing over ownership
            // This matches the common pattern where structs are passed through function chains
            if self.is_struct_type(rust_type) && !usage.escapes_through_return && !usage.is_stored {
                return BorrowingStrategy::BorrowImmutable { lifetime: None };
            }
            // Check if move is necessary
            if !usage.escapes_through_return && !usage.is_stored {
                insights.push(BorrowingInsight::UnnecessaryMove(param_name.to_string()));
            }
            return BorrowingStrategy::TakeOwnership;
        }

        // If parameter escapes through return and matches return type, take ownership
        // (except for strings which have special handling)
        if usage.escapes_through_return && !matches!(python_type, PythonType::String) {
            if let Some(ref ret_type) = self.return_type {
                if python_type == ret_type {
                    return BorrowingStrategy::TakeOwnership;
                }
            }
        }

        // If parameter is stored in a structure, consider shared ownership
        if usage.is_stored {
            return BorrowingStrategy::UseSharedOwnership {
                is_thread_safe: false,
            };
        }

        // If used in closure, determine capture strategy
        if usage.used_in_closure {
            // Complex analysis needed - for now, be conservative
            return BorrowingStrategy::TakeOwnership;
        }

        // String-specific optimizations
        if matches!(python_type, PythonType::String) {
            return self.determine_string_strategy(param_name, usage);
        }

        // Determine mutability needs
        if usage.is_mutated {
            BorrowingStrategy::BorrowMutable { lifetime: None }
        } else if usage.is_read {
            BorrowingStrategy::BorrowImmutable { lifetime: None }
        } else {
            // Parameter unused - for struct types, prefer borrowing for consistency
            // This allows passing the same reference through multiple functions
            if self.is_struct_type(rust_type) {
                BorrowingStrategy::BorrowImmutable { lifetime: None }
            } else {
                BorrowingStrategy::TakeOwnership
            }
        }
    }

    /// Determine optimal string handling strategy
    /// STRING_INTEROP: Always use owned String for Python str parameters
    /// This ensures consistent semantics with Python where strings are immutable values.
    /// Using String instead of &str:
    /// - Matches Python's value semantics for strings
    /// - Simplifies interoperability (no lifetime issues)
    /// - Allows straightforward function composition
    fn determine_string_strategy(
        &self,
        _param_name: &str,
        _usage: &ParameterUsagePattern,
    ) -> BorrowingStrategy {
        // Always take ownership for strings - matches Python semantics
        // Python strings are immutable values, so Rust String is the closest match
        BorrowingStrategy::TakeOwnership
    }

    /// Check if a type implements Copy
    #[allow(clippy::only_used_in_recursion)]
    fn is_copy_type(&self, rust_type: &RustType) -> bool {
        match rust_type {
            RustType::Primitive(_) => true,
            RustType::Unit => true,
            RustType::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            // Enums are immutable Copy values in Python — always pass by value
            RustType::Custom(name) => self.enum_names.contains(name),
            _ => false,
        }
    }

    /// Check if a type is a struct/dataclass type that should prefer borrowing
    fn is_struct_type(&self, rust_type: &RustType) -> bool {
        match rust_type {
            // Enums are not structs — they're Copy values that don't need borrowing
            RustType::Custom(name) => !self.enum_names.contains(name),
            _ => false,
        }
    }
}
