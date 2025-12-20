//! Deque mutation handlers for type inference (collections.deque)
//!
//! Note: Deques are represented as `Type::Custom("deque")` in the HIR.
//! We track them using Generic type for element type information.

use super::MutationHandler;
use crate::dataflow::lattice::{TypeLattice, TypeState};
use crate::hir::{HirExpr, Type};

fn is_deque(ty: &Type) -> bool {
    matches!(ty, Type::Custom(name) if name == "deque") || matches!(ty, Type::Generic { base, .. } if base == "deque")
}

fn make_deque_type(elem_ty: Type) -> Type {
    Type::Generic {
        base: "deque".to_string(),
        params: vec![elem_ty],
    }
}

/// Handler for deque.appendleft(x)
pub struct AppendLeftMutation;

impl MutationHandler for AppendLeftMutation {
    fn method_name(&self) -> &'static str {
        "appendleft"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "appendleft" && is_deque(ty)
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if !is_deque(current_ty) {
            return None;
        }

        if let Some(arg) = args.first() {
            let arg_type = infer_expr(arg, state);
            return Some(make_deque_type(arg_type));
        }
        Some(current_ty.clone())
    }
}

/// Handler for deque.popleft()
pub struct PopLeftMutation;

impl MutationHandler for PopLeftMutation {
    fn method_name(&self) -> &'static str {
        "popleft"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "popleft" && is_deque(ty)
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if is_deque(current_ty) {
            Some(current_ty.clone())
        } else {
            None
        }
    }
}

/// Handler for deque.extendleft(iterable)
pub struct ExtendLeftMutation;

impl MutationHandler for ExtendLeftMutation {
    fn method_name(&self) -> &'static str {
        "extendleft"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "extendleft" && is_deque(ty)
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if !is_deque(current_ty) {
            return None;
        }

        if let Some(arg) = args.first() {
            let arg_type = infer_expr(arg, state);
            let elem_ty = TypeLattice::element_type(&arg_type);
            return Some(make_deque_type(elem_ty));
        }
        Some(current_ty.clone())
    }
}

/// Handler for deque.rotate(n)
pub struct RotateMutation;

impl MutationHandler for RotateMutation {
    fn method_name(&self) -> &'static str {
        "rotate"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "rotate" && is_deque(ty)
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if is_deque(current_ty) {
            Some(current_ty.clone())
        } else {
            None
        }
    }
}
