//! Modular mutation tracking for dataflow type inference
//!
//! Each mutation method (append, extend, add, etc.) has its own handler
//! that knows how to refine container types based on the mutation.

mod deque;
mod dict;
mod list;
mod set;
mod string;

use super::lattice::{LatticeType, TypeLattice, TypeState};
use crate::hir::{HirExpr, Type};

pub use deque::*;
pub use dict::*;
pub use list::*;
pub use set::*;
pub use string::*;

/// Trait for mutation handlers that can refine types
pub trait MutationHandler {
    /// The method name this handler responds to (e.g., "append", "add")
    fn method_name(&self) -> &'static str;

    /// Check if this handler applies to the given type and method
    fn applies_to(&self, ty: &Type, method: &str) -> bool;

    /// Compute the new type after mutation
    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type>;
}

/// Registry of all mutation handlers
pub struct MutationRegistry {
    handlers: Vec<Box<dyn MutationHandler + Send + Sync>>,
}

impl MutationRegistry {
    pub fn new() -> Self {
        let mut registry = Self { handlers: Vec::new() };

        // Register list mutations
        registry.register(Box::new(list::AppendMutation));
        registry.register(Box::new(list::ExtendMutation));
        registry.register(Box::new(list::InsertMutation));
        registry.register(Box::new(list::PopMutation));
        registry.register(Box::new(list::RemoveMutation));
        registry.register(Box::new(list::ClearMutation));
        registry.register(Box::new(list::SortMutation));
        registry.register(Box::new(list::ReverseMutation));

        // Register set mutations
        registry.register(Box::new(set::AddMutation));
        registry.register(Box::new(set::SetUpdateMutation));
        registry.register(Box::new(set::DiscardMutation));
        registry.register(Box::new(set::SetRemoveMutation));
        registry.register(Box::new(set::SetPopMutation));
        registry.register(Box::new(set::SetClearMutation));
        registry.register(Box::new(set::DifferenceUpdateMutation));
        registry.register(Box::new(set::IntersectionUpdateMutation));
        registry.register(Box::new(set::SymmetricDifferenceUpdateMutation));

        // Register dict mutations
        registry.register(Box::new(dict::DictUpdateMutation));
        registry.register(Box::new(dict::SetDefaultMutation));
        registry.register(Box::new(dict::DictPopMutation));
        registry.register(Box::new(dict::PopItemMutation));
        registry.register(Box::new(dict::DictClearMutation));

        // Register deque mutations
        registry.register(Box::new(deque::AppendLeftMutation));
        registry.register(Box::new(deque::PopLeftMutation));
        registry.register(Box::new(deque::ExtendLeftMutation));
        registry.register(Box::new(deque::RotateMutation));

        // Register string mutations (in-place for bytearray, etc.)
        registry.register(Box::new(string::BytearrayAppendMutation));
        registry.register(Box::new(string::BytearrayExtendMutation));

        registry
    }

    pub fn register(&mut self, handler: Box<dyn MutationHandler + Send + Sync>) {
        self.handlers.push(handler);
    }

    /// Find and apply the appropriate mutation handler
    pub fn compute_mutation_type(
        &self,
        current_ty: &Type,
        method: &str,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // First try registered handlers
        for handler in &self.handlers {
            if handler.applies_to(current_ty, method) {
                if let Some(new_ty) = handler.compute_type(current_ty, args, state, infer_expr) {
                    return Some(new_ty);
                }
            }
        }

        // Fallback for unknown containers being mutated
        self.handle_unknown_container(current_ty, method, args, state, infer_expr)
    }

    /// Handle mutations on Type::Unknown containers
    fn handle_unknown_container(
        &self,
        current_ty: &Type,
        method: &str,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        match (current_ty, method) {
            // Unknown being mutated with list methods
            (Type::Unknown, "append" | "extend" | "insert") => {
                if let Some(arg) = args.first() {
                    let arg_ty = infer_expr(arg, state);
                    let elem_ty = if method == "extend" {
                        TypeLattice::element_type(&arg_ty)
                    } else if method == "insert" && args.len() >= 2 {
                        infer_expr(&args[1], state)
                    } else {
                        arg_ty
                    };
                    Some(Type::List(Box::new(elem_ty)))
                } else {
                    Some(Type::List(Box::new(Type::Unknown)))
                }
            }

            // Unknown being mutated with set methods
            (Type::Unknown, "add" | "update" | "discard") => {
                if let Some(arg) = args.first() {
                    let arg_ty = infer_expr(arg, state);
                    let elem_ty = if method == "update" {
                        TypeLattice::element_type(&arg_ty)
                    } else {
                        arg_ty
                    };
                    Some(Type::Set(Box::new(elem_ty)))
                } else {
                    Some(Type::Set(Box::new(Type::Unknown)))
                }
            }

            // Unknown being mutated with deque methods
            (Type::Unknown, "appendleft" | "extendleft" | "popleft") => {
                if let Some(arg) = args.first() {
                    let arg_ty = infer_expr(arg, state);
                    let elem_ty = if method == "extendleft" {
                        TypeLattice::element_type(&arg_ty)
                    } else {
                        arg_ty
                    };
                    Some(Type::Custom("deque".to_string()))
                } else {
                    Some(Type::Custom("deque".to_string()))
                }
            }

            _ => None,
        }
    }
}

impl Default for MutationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to refine element types during mutation
pub fn refine_element_type(current: &Type, new: &Type) -> Type {
    if matches!(current, Type::Unknown) {
        new.clone()
    } else if matches!(new, Type::Unknown) {
        current.clone()
    } else {
        let joined = LatticeType::from_hir_type(current).join(&LatticeType::from_hir_type(new));
        joined.to_hir_type()
    }
}
