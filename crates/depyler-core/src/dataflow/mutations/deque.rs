//! Deque mutation handlers for type inference (collections.deque)
//!
//! Note: Deques are represented as `Type::Custom("deque")` in the HIR.
//! We track them using Generic type for element type information.

use crate::hir::{HirExpr, Type};
use super::MutationHandler;
use crate::dataflow::lattice::{TypeLattice, TypeState};

fn is_deque(ty: &Type) -> bool {
    matches!(ty, Type::Custom(name) if name == "deque")
        || matches!(ty, Type::Generic { base, .. } if base == "deque")
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
    fn method_name(&self) -> &'static str { "appendleft" }
    
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
    fn method_name(&self) -> &'static str { "popleft" }
    
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
    fn method_name(&self) -> &'static str { "extendleft" }
    
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
    fn method_name(&self) -> &'static str { "rotate" }
    
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
            HirExpr::Literal(Literal::Float(_)) => Type::Float,
            HirExpr::List(elems) if !elems.is_empty() => {
                Type::List(Box::new(infer_literal(&elems[0], _state)))
            }
            _ => Type::Unknown,
        }
    }
    
    #[test]
    fn test_appendleft_refines_element_type() {
        let handler = AppendLeftMutation;
        let deque_ty = Type::Custom("deque".to_string());
        let state = make_state();
        let args = vec![HirExpr::Literal(Literal::Int(42))];

        let result = handler.compute_type(&deque_ty, &args, &state, &infer_literal);
        assert!(matches!(
            result,
            Some(Type::Generic { base, params }) 
                if base == "deque" && params.len() == 1 && matches!(params[0], Type::Int)
        ));
    }

    #[test]
    fn test_extendleft_from_list() {
        let handler = ExtendLeftMutation;
        let deque_ty = Type::Custom("deque".to_string());
        let state = make_state();
        let args = vec![HirExpr::List(vec![])];

        let infer_fn = |_: &HirExpr, _: &TypeState| -> Type {
            Type::List(Box::new(Type::Float))
        };

        let result = handler.compute_type(&deque_ty, &args, &state, &infer_fn);
        assert!(matches!(
            result,
            Some(Type::Generic { base, params })
                if base == "deque" && params.len() == 1 && matches!(params[0], Type::Float)
        ));
    }

    #[test]
    fn test_rotate_preserves_type() {
        let handler = RotateMutation;
        let deque_ty = Type::Custom("deque".to_string());
        let state = make_state();

        let result = handler.compute_type(&deque_ty, &[], &state, &infer_literal);
        assert_eq!(result, Some(deque_ty));
    }
}
