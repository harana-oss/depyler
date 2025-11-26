//! Dict mutation handlers for type inference

use crate::hir::{HirExpr, Type};
use super::{MutationHandler, refine_element_type};
use crate::dataflow::lattice::TypeState;

/// Handler for dict.update(other)
pub struct DictUpdateMutation;

impl MutationHandler for DictUpdateMutation {
    fn method_name(&self) -> &'static str { "update" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "update" && matches!(ty, Type::Dict(_, _))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::Dict(key_ty, val_ty) = current_ty {
            if let Some(arg) = args.first() {
                let arg_type = infer_expr(arg, state);
                if let Type::Dict(new_key, new_val) = arg_type {
                    let refined_key = refine_element_type(key_ty, &new_key);
                    let refined_val = refine_element_type(val_ty, &new_val);
                    return Some(Type::Dict(Box::new(refined_key), Box::new(refined_val)));
                }
            }
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for dict.setdefault(key, default)
pub struct SetDefaultMutation;

impl MutationHandler for SetDefaultMutation {
    fn method_name(&self) -> &'static str { "setdefault" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "setdefault" && matches!(ty, Type::Dict(_, _))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::Dict(key_ty, val_ty) = current_ty {
            let mut new_key = key_ty.as_ref().clone();
            let mut new_val = val_ty.as_ref().clone();

            if let Some(key_arg) = args.first() {
                let key_type = infer_expr(key_arg, state);
                new_key = refine_element_type(&new_key, &key_type);
            }

            if let Some(val_arg) = args.get(1) {
                let val_type = infer_expr(val_arg, state);
                new_val = refine_element_type(&new_val, &val_type);
            }

            return Some(Type::Dict(Box::new(new_key), Box::new(new_val)));
        }
        None
    }
}

/// Handler for dict.pop(key)
pub struct DictPopMutation;

impl MutationHandler for DictPopMutation {
    fn method_name(&self) -> &'static str { "pop" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "pop" && matches!(ty, Type::Dict(_, _))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if matches!(current_ty, Type::Dict(_, _)) {
            Some(current_ty.clone())
        } else {
            None
        }
    }
}

/// Handler for dict.popitem()
pub struct PopItemMutation;

impl MutationHandler for PopItemMutation {
    fn method_name(&self) -> &'static str { "popitem" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "popitem" && matches!(ty, Type::Dict(_, _))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if matches!(current_ty, Type::Dict(_, _)) {
            Some(current_ty.clone())
        } else {
            None
        }
    }
}

/// Handler for dict.clear()
pub struct DictClearMutation;

impl MutationHandler for DictClearMutation {
    fn method_name(&self) -> &'static str { "clear" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "clear" && matches!(ty, Type::Dict(_, _))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if matches!(current_ty, Type::Dict(_, _)) {
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
            HirExpr::Dict(pairs) if !pairs.is_empty() => {
                let (k, v) = &pairs[0];
                Type::Dict(
                    Box::new(infer_literal(k, _state)),
                    Box::new(infer_literal(v, _state)),
                )
            }
            _ => Type::Unknown,
        }
    }
    
    #[test]
    fn test_dict_update_refines_types() {
        let handler = DictUpdateMutation;
        let dict_ty = Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown));
        let state = make_state();
        let args = vec![HirExpr::Dict(vec![])];

        let infer_fn = |_: &HirExpr, _: &TypeState| -> Type {
            Type::Dict(Box::new(Type::String), Box::new(Type::Int))
        };

        let result = handler.compute_type(&dict_ty, &args, &state, &infer_fn);
        assert!(matches!(
            result,
            Some(Type::Dict(k, v)) if matches!(*k, Type::String) && matches!(*v, Type::Int)
        ));
    }

    #[test]
    fn test_dict_setdefault_refines_key_and_value() {
        let handler = SetDefaultMutation;
        let dict_ty = Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown));
        let state = make_state();
        let args = vec![
            HirExpr::Literal(Literal::String("key".to_string())),
            HirExpr::Literal(Literal::Int(42)),
        ];

        let result = handler.compute_type(&dict_ty, &args, &state, &infer_literal);
        assert!(matches!(
            result,
            Some(Type::Dict(k, v)) if matches!(*k, Type::String) && matches!(*v, Type::Int)
        ));
    }

    #[test]
    fn test_dict_pop_preserves_type() {
        let handler = DictPopMutation;
        let dict_ty = Type::Dict(Box::new(Type::String), Box::new(Type::Int));
        let state = make_state();

        let result = handler.compute_type(&dict_ty, &[], &state, &infer_literal);
        assert_eq!(result, Some(dict_ty));
    }

    #[test]
    fn test_dict_clear_preserves_type() {
        let handler = DictClearMutation;
        let dict_ty = Type::Dict(Box::new(Type::String), Box::new(Type::Int));
        let state = make_state();

        let result = handler.compute_type(&dict_ty, &[], &state, &infer_literal);
        assert_eq!(result, Some(dict_ty));
    }
}
