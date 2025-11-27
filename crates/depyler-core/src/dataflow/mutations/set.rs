//! Set mutation handlers for type inference

use super::{MutationHandler, refine_element_type};
use crate::dataflow::lattice::{TypeLattice, TypeState};
use crate::hir::{HirExpr, Type};

/// Handler for set.add(element)
pub struct AddMutation;

impl MutationHandler for AddMutation {
    fn method_name(&self) -> &'static str {
        "add"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "add" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::Set(elem) = current_ty {
            if let Some(arg) = args.first() {
                let arg_ty = infer_expr(arg, state);
                let refined = refine_element_type(elem, &arg_ty);
                return Some(Type::Set(Box::new(refined)));
            }
        }
        None
    }
}

/// Handler for set.update(iterable, ...)
pub struct SetUpdateMutation;

impl MutationHandler for SetUpdateMutation {
    fn method_name(&self) -> &'static str {
        "update"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "update" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::Set(elem) = current_ty {
            // update can take multiple iterables
            let mut refined = elem.as_ref().clone();
            for arg in args {
                let arg_ty = infer_expr(arg, state);
                let elem_ty = TypeLattice::element_type(&arg_ty);
                refined = refine_element_type(&refined, &elem_ty);
            }
            return Some(Type::Set(Box::new(refined)));
        }
        None
    }
}

/// Handler for set.discard(element)
pub struct DiscardMutation;

impl MutationHandler for DiscardMutation {
    fn method_name(&self) -> &'static str {
        "discard"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "discard" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // discard doesn't change the set type
        if let Type::Set(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for set.remove(element)
pub struct SetRemoveMutation;

impl MutationHandler for SetRemoveMutation {
    fn method_name(&self) -> &'static str {
        "remove"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "remove" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // remove doesn't change the set type
        if let Type::Set(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for set.pop()
pub struct SetPopMutation;

impl MutationHandler for SetPopMutation {
    fn method_name(&self) -> &'static str {
        "pop"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "pop" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // pop removes an arbitrary element but doesn't change the type
        if let Type::Set(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for set.clear()
pub struct SetClearMutation;

impl MutationHandler for SetClearMutation {
    fn method_name(&self) -> &'static str {
        "clear"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "clear" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // clear empties the set but preserves element type
        if let Type::Set(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for set.difference_update(iterable, ...)
pub struct DifferenceUpdateMutation;

impl MutationHandler for DifferenceUpdateMutation {
    fn method_name(&self) -> &'static str {
        "difference_update"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "difference_update" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // difference_update removes elements but doesn't change the type
        if let Type::Set(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for set.intersection_update(iterable, ...)
pub struct IntersectionUpdateMutation;

impl MutationHandler for IntersectionUpdateMutation {
    fn method_name(&self) -> &'static str {
        "intersection_update"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "intersection_update" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // intersection_update keeps only common elements but doesn't change the type
        if let Type::Set(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for set.symmetric_difference_update(iterable)
pub struct SymmetricDifferenceUpdateMutation;

impl MutationHandler for SymmetricDifferenceUpdateMutation {
    fn method_name(&self) -> &'static str {
        "symmetric_difference_update"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "symmetric_difference_update" && matches!(ty, Type::Set(_))
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::Set(elem) = current_ty {
            // symmetric_difference_update can add new elements from the other set
            if let Some(arg) = args.first() {
                let arg_ty = infer_expr(arg, state);
                let elem_ty = TypeLattice::element_type(&arg_ty);
                let refined = refine_element_type(elem, &elem_ty);
                return Some(Type::Set(Box::new(refined)));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::Literal;

    fn make_state() -> TypeState {
        TypeState::new()
    }

    fn infer_literal(expr: &HirExpr, _state: &TypeState) -> Type {
        match expr {
            HirExpr::Literal(Literal::Int(_)) => Type::Int,
            HirExpr::Literal(Literal::String(_)) => Type::String,
            HirExpr::Set(elems) if !elems.is_empty() => Type::Set(Box::new(infer_literal(&elems[0], _state))),
            _ => Type::Unknown,
        }
    }

    #[test]
    fn test_add_refines_unknown() {
        let handler = AddMutation;
        let current = Type::Set(Box::new(Type::Unknown));
        let args = vec![HirExpr::Literal(Literal::Int(42))];
        let state = make_state();

        let result = handler.compute_type(&current, &args, &state, &infer_literal);
        assert_eq!(result, Some(Type::Set(Box::new(Type::Int))));
    }

    #[test]
    fn test_update_multiple_args() {
        let handler = SetUpdateMutation;
        let current = Type::Set(Box::new(Type::Unknown));
        let args = vec![HirExpr::Set(vec![HirExpr::Literal(Literal::Int(1))])];
        let state = make_state();

        let result = handler.compute_type(&current, &args, &state, &infer_literal);
        assert_eq!(result, Some(Type::Set(Box::new(Type::Int))));
    }
}
