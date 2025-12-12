//! Mutability analysis for string set optimization.
//!
//! Tracks mutations to sets to determine if they can be optimized to bitflags.

use crate::hir::{AssignTarget, BinOp, HirExpr, HirFunction, HirModule, HirStmt, Span};
use std::collections::{HashMap, HashSet};

/// Types of mutations that can occur on a set.
#[derive(Debug, Clone)]
pub enum SetMutation {
    /// Direct method calls that mutate.
    MethodCall { method: MutatingMethod, location: Span },
    /// Reassignment to a different value.
    Reassignment { location: Span },
    /// Augmented assignment (|=, &=, -=, ^=).
    AugmentedAssignment { op: SetOp, location: Span },
    /// Passed to function that may mutate.
    PassedToMutatingFunction { function: String, location: Span },
    /// Aliased to another variable that is later mutated.
    AliasedAndMutated { alias: String, mutation_location: Span },
}

/// Methods that mutate a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutatingMethod {
    // Set methods
    Add,
    Remove,
    Discard,
    Pop,
    Clear,
    Update,
    IntersectionUpdate,
    DifferenceUpdate,
    SymmetricDifferenceUpdate,
    // List methods
    Append,
    Extend,
    Insert,
    Reverse,
    Sort,
}

impl MutatingMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // Set methods
            "add" => Some(Self::Add),
            "remove" => Some(Self::Remove),
            "discard" => Some(Self::Discard),
            "pop" => Some(Self::Pop),
            "clear" => Some(Self::Clear),
            "update" => Some(Self::Update),
            "intersection_update" => Some(Self::IntersectionUpdate),
            "difference_update" => Some(Self::DifferenceUpdate),
            "symmetric_difference_update" => Some(Self::SymmetricDifferenceUpdate),
            // List methods
            "append" => Some(Self::Append),
            "extend" => Some(Self::Extend),
            "insert" => Some(Self::Insert),
            "reverse" => Some(Self::Reverse),
            "sort" => Some(Self::Sort),
            _ => None,
        }
    }
}

/// Set operations for augmented assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    UnionUpdate,        // |=
    IntersectionUpdate, // &=
    DifferenceUpdate,   // -=
    SymDiffUpdate,      // ^=
}

/// Definition of a set variable.
#[derive(Debug, Clone)]
pub struct SetDefinition {
    pub name: String,
    pub location: Span,
    pub is_frozenset: bool,
    pub values: Option<Vec<String>>,
}

/// Parameter mutability information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMutability {
    Immutable,
    Mutable,
    Unknown,
}

/// Mutability analyzer for set variables.
#[derive(Debug, Default)]
pub struct MutabilityAnalyzer {
    /// All definitions of set variables.
    definitions: HashMap<String, SetDefinition>,
    /// Detected mutations per variable.
    mutations: HashMap<String, Vec<SetMutation>>,
    /// Alias tracking: variable -> set of aliases.
    aliases: HashMap<String, HashSet<String>>,
    /// Function signatures with mutability info.
    function_params: HashMap<String, Vec<ParamMutability>>,
    /// Current scope depth for shadowing detection.
    scope_depth: u32,
    /// Variable scopes: name -> (scope_depth, definition).
    variable_scopes: HashMap<String, Vec<(u32, SetDefinition)>>,
}

impl MutabilityAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a module for set mutability.
    pub fn analyze_module(&mut self, module: &HirModule) {
        // Analyze module-level constants
        for constant in &module.constants {
            if let Some(def) = self.extract_set_definition(&constant.name, &constant.value) {
                self.definitions.insert(constant.name.clone(), def);
            }
        }

        // Analyze functions
        for func in &module.functions {
            self.analyze_function(func);
        }
    }

    /// Analyze a function for set mutations.
    pub fn analyze_function(&mut self, func: &HirFunction) {
        self.scope_depth += 1;

        for stmt in &func.body {
            self.analyze_statement(stmt);
        }

        self.scope_depth -= 1;
    }

    /// Analyze a statement for mutations.
    fn analyze_statement(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                // Check for augmented assignment pattern (x = x op y)
                if let AssignTarget::Symbol(name) = target {
                    if self.definitions.contains_key(name) {
                        if let HirExpr::Binary { op, left, .. } = value {
                            if let HirExpr::Var(var_name) = left.as_ref() {
                                if var_name == name {
                                    let set_op = match op {
                                        BinOp::BitOr => Some(SetOp::UnionUpdate),
                                        BinOp::BitAnd => Some(SetOp::IntersectionUpdate),
                                        BinOp::Sub => Some(SetOp::DifferenceUpdate),
                                        BinOp::BitXor => Some(SetOp::SymDiffUpdate),
                                        _ => None,
                                    };
                                    if let Some(op) = set_op {
                                        self.mutations.entry(name.clone()).or_default().push(
                                            SetMutation::AugmentedAssignment {
                                                op,
                                                location: Span::default(),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Check for set definition
                if let AssignTarget::Symbol(name) = target {
                    if let Some(def) = self.extract_set_definition(name, value) {
                        self.definitions.insert(name.clone(), def.clone());
                        self.variable_scopes
                            .entry(name.clone())
                            .or_default()
                            .push((self.scope_depth, def));
                    } else if self.definitions.contains_key(name) {
                        // Reassignment of existing set
                        self.mutations
                            .entry(name.clone())
                            .or_default()
                            .push(SetMutation::Reassignment {
                                location: Span::default(),
                            });
                    }

                    // Check for aliasing
                    if let HirExpr::Var(source) = value {
                        if self.definitions.contains_key(source) {
                            self.aliases.entry(source.clone()).or_default().insert(name.clone());
                        }
                    }
                }

                self.analyze_expression(value);
            }
            HirStmt::Expr(expr) => {
                self.analyze_expression(expr);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_expression(condition);
                self.scope_depth += 1;
                for s in then_body {
                    self.analyze_statement(s);
                }
                self.scope_depth -= 1;
                if let Some(else_stmts) = else_body {
                    self.scope_depth += 1;
                    for s in else_stmts {
                        self.analyze_statement(s);
                    }
                    self.scope_depth -= 1;
                }
            }
            HirStmt::While { condition, body }
            | HirStmt::For {
                iter: condition, body, ..
            } => {
                self.analyze_expression(condition);
                self.scope_depth += 1;
                for s in body {
                    self.analyze_statement(s);
                }
                self.scope_depth -= 1;
            }
            HirStmt::Try { body, handlers, .. } => {
                self.scope_depth += 1;
                for s in body {
                    self.analyze_statement(s);
                }
                self.scope_depth -= 1;

                for handler in handlers {
                    self.scope_depth += 1;
                    for s in &handler.body {
                        self.analyze_statement(s);
                    }
                    self.scope_depth -= 1;
                }
            }
            HirStmt::Return(Some(expr)) => {
                self.analyze_expression(expr);
            }
            _ => {}
        }
    }

    /// Analyze an expression for mutations.
    fn analyze_expression(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::MethodCall {
                object, method, args, ..
            } => {
                // Check for mutating methods on sets
                if let HirExpr::Var(name) = object.as_ref() {
                    if self.definitions.contains_key(name) {
                        if let Some(mutating_method) = MutatingMethod::from_str(method) {
                            self.mutations
                                .entry(name.clone())
                                .or_default()
                                .push(SetMutation::MethodCall {
                                    method: mutating_method,
                                    location: Span::default(),
                                });
                        }
                    }
                }
                self.analyze_expression(object);
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            HirExpr::Call { func, args, .. } => {
                // Check if a set is passed to a potentially mutating function
                for arg in args {
                    if let HirExpr::Var(name) = arg {
                        if self.definitions.contains_key(name) {
                            // Some known safe functions
                            let safe_functions = ["len", "bool", "print", "str", "repr", "sorted", "list"];
                            if !safe_functions.contains(&func.as_str()) {
                                self.mutations.entry(name.clone()).or_default().push(
                                    SetMutation::PassedToMutatingFunction {
                                        function: func.clone(),
                                        location: Span::default(),
                                    },
                                );
                            }
                        }
                    }
                    self.analyze_expression(arg);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            HirExpr::Unary { operand, .. } => {
                self.analyze_expression(operand);
            }
            HirExpr::List(items) | HirExpr::Tuple(items) | HirExpr::Set(items) => {
                for item in items {
                    self.analyze_expression(item);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    self.analyze_expression(k);
                    self.analyze_expression(v);
                }
            }
            HirExpr::Index { base, index } => {
                self.analyze_expression(base);
                self.analyze_expression(index);
            }
            HirExpr::Attribute { value, .. } => {
                self.analyze_expression(value);
            }
            HirExpr::IfExpr { test, body, orelse } => {
                self.analyze_expression(test);
                self.analyze_expression(body);
                self.analyze_expression(orelse);
            }
            HirExpr::Lambda { body, .. } => {
                self.analyze_expression(body);
            }
            _ => {}
        }
    }

    /// Extract set/list definition from an expression.
    fn extract_set_definition(&self, name: &str, expr: &HirExpr) -> Option<SetDefinition> {
        match expr {
            HirExpr::Set(items) | HirExpr::List(items) => {
                let values = self.extract_string_values(items);
                Some(SetDefinition {
                    name: name.to_string(),
                    location: Span::default(),
                    is_frozenset: false,
                    values,
                })
            }
            HirExpr::FrozenSet(items) => {
                let values = self.extract_string_values(items);
                Some(SetDefinition {
                    name: name.to_string(),
                    location: Span::default(),
                    is_frozenset: true,
                    values,
                })
            }
            _ => None,
        }
    }

    /// Extract string values from set items.
    fn extract_string_values(&self, items: &[HirExpr]) -> Option<Vec<String>> {
        let mut values = Vec::new();
        for item in items {
            if let HirExpr::Literal(crate::hir::Literal::String(s)) = item {
                values.push(s.clone());
            } else {
                return None;
            }
        }
        Some(values)
    }

    /// Determines if a set variable is immutable throughout its scope.
    pub fn is_immutable(&self, name: &str) -> ImmutabilityResult {
        // Frozensets are always immutable
        if let Some(def) = self.definitions.get(name) {
            if def.is_frozenset {
                return ImmutabilityResult::Immutable;
            }
        }

        // Check direct mutations
        if let Some(mutations) = self.mutations.get(name) {
            if !mutations.is_empty() {
                return ImmutabilityResult::Mutable {
                    reasons: mutations.clone(),
                };
            }
        }

        // Check alias mutations
        if let Some(aliases) = self.aliases.get(name) {
            for alias in aliases {
                if let Some(mutations) = self.mutations.get(alias) {
                    if !mutations.is_empty() {
                        return ImmutabilityResult::MutableViaAlias {
                            alias: alias.clone(),
                            reasons: mutations.clone(),
                        };
                    }
                }
            }
        }

        ImmutabilityResult::Immutable
    }

    /// Get all immutable set variables.
    pub fn get_immutable_sets(&self) -> Vec<&str> {
        self.definitions
            .keys()
            .filter(|name| matches!(self.is_immutable(name), ImmutabilityResult::Immutable))
            .map(|s| s.as_str())
            .collect()
    }

    /// Get mutations for a variable.
    pub fn get_mutations(&self, name: &str) -> Option<&Vec<SetMutation>> {
        self.mutations.get(name)
    }
}

/// Result of immutability analysis.
#[derive(Debug, Clone)]
pub enum ImmutabilityResult {
    Immutable,
    Mutable { reasons: Vec<SetMutation> },
    MutableViaAlias { alias: String, reasons: Vec<SetMutation> },
    Unknown,
}

impl ImmutabilityResult {
    pub fn is_immutable(&self) -> bool {
        matches!(self, Self::Immutable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutating_method_from_str() {
        assert_eq!(MutatingMethod::from_str("add"), Some(MutatingMethod::Add));
        assert_eq!(MutatingMethod::from_str("remove"), Some(MutatingMethod::Remove));
        assert_eq!(MutatingMethod::from_str("clear"), Some(MutatingMethod::Clear));
        assert_eq!(MutatingMethod::from_str("union"), None);
        assert_eq!(MutatingMethod::from_str("intersection"), None);
    }

    #[test]
    fn test_immutability_result_is_immutable() {
        assert!(ImmutabilityResult::Immutable.is_immutable());
        assert!(!ImmutabilityResult::Unknown.is_immutable());
        assert!(!ImmutabilityResult::Mutable { reasons: vec![] }.is_immutable());
    }

    #[test]
    fn test_set_definition() {
        let def = SetDefinition {
            name: "PERMISSIONS".to_string(),
            location: Span::default(),
            is_frozenset: true,
            values: Some(vec!["read".to_string(), "write".to_string()]),
        };
        assert!(def.is_frozenset);
        assert_eq!(def.values.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_set_mutation_types() {
        let method_mutation = SetMutation::MethodCall {
            method: MutatingMethod::Add,
            location: Span::default(),
        };
        assert!(matches!(method_mutation, SetMutation::MethodCall { .. }));

        let aug_mutation = SetMutation::AugmentedAssignment {
            op: SetOp::UnionUpdate,
            location: Span::default(),
        };
        assert!(matches!(aug_mutation, SetMutation::AugmentedAssignment { .. }));
    }

    #[test]
    fn test_mutability_analyzer_new() {
        let analyzer = MutabilityAnalyzer::new();
        assert!(analyzer.definitions.is_empty());
        assert!(analyzer.mutations.is_empty());
    }
}
