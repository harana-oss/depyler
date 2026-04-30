//! Heuristic type constraint solving infrastructure.
//!
//! Contains the [`InferenceContext`], [`TypeConstraint`], and [`UsagePattern`] types
//! that back the [`super::TypeHintProvider`] heuristic inference pass.

use crate::hir::Type;
use std::collections::HashMap;

/// Active inference state while analysing a function body.
#[derive(Debug, Default)]
pub(super) struct InferenceContext {
    /// Current function being analyzed
    pub(super) current_function: Option<String>,
    /// Type constraints collected
    pub(super) constraints: Vec<TypeConstraint>,
    /// Usage patterns
    pub(super) usage_patterns: HashMap<String, Vec<UsagePattern>>,
    /// Loop variable sources
    /// Maps loop variable → iterable variable (e.g., "item" → "items")
    pub(super) loop_var_sources: HashMap<String, String>,
}

/// A constraint on a variable's type, derived heuristically.
#[derive(Debug, Clone)]
pub(super) enum TypeConstraint {
    /// Variable must be compatible with type
    Compatible { var: String, ty: Type },
    /// Variable used in operation requiring specific type
    #[allow(dead_code)]
    OperatorConstraint {
        var: String,
        op: String,
        required: Type,
    },
    /// Variable passed to function expecting type
    ArgumentConstraint {
        _var: String,
        _func: String,
        _param_idx: usize,
        _expected: Type,
    },
    /// Variable returned from function
    ReturnConstraint { var: String, ty: Type },
}

/// Observed usage patterns that give evidence about a variable's type.
#[derive(Debug, Clone)]
pub(super) enum UsagePattern {
    /// Used as iterator
    Iterator,
    /// Used with numeric operators
    Numeric,
    /// Used with string methods
    StringLike,
    /// Used as container
    Container,
    /// Used as callable
    #[allow(dead_code)]
    Callable,
}
