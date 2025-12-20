//! Utility functions for working with HIR expressions

use crate::hir::HirExpr;

/// Extract the root variable name from a potentially nested expression.
///
/// This function recursively traverses attribute accesses and index operations
/// to find the base variable name.
///
/// This is crucial for interprocedural mutation analysis: when `state.data`
/// is passed to a function that mutates its parameter, we need to know that
/// the root variable `state` is being mutated.
pub fn extract_root_var(expr: &HirExpr) -> Option<String> {
    match expr {
        HirExpr::Var(name) => Some(name.clone()),
        HirExpr::Attribute { value, .. } => extract_root_var(value),
        HirExpr::Index { base, .. } => extract_root_var(base),
        _ => None,
    }
}
