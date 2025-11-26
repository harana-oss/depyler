//! List mutation handlers for type inference

use crate::hir::{HirExpr, Type};
use super::{MutationHandler, refine_element_type};
use crate::dataflow::lattice::{TypeLattice, TypeState};

/// Handler for list.append(element)
pub struct AppendMutation;

impl MutationHandler for AppendMutation {
    fn method_name(&self) -> &'static str { "append" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "append" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::List(elem) = current_ty {
            if let Some(arg) = args.first() {
                let arg_ty = infer_expr(arg, state);
                let refined = refine_element_type(elem, &arg_ty);
                return Some(Type::List(Box::new(refined)));
            }
        }
        None
    }
}

/// Handler for list.extend(iterable)
pub struct ExtendMutation;

impl MutationHandler for ExtendMutation {
    fn method_name(&self) -> &'static str { "extend" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "extend" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::List(elem) = current_ty {
            if let Some(arg) = args.first() {
                let arg_ty = infer_expr(arg, state);
                let elem_ty = TypeLattice::element_type(&arg_ty);
                let refined = refine_element_type(elem, &elem_ty);
                return Some(Type::List(Box::new(refined)));
            }
        }
        None
    }
}

/// Handler for list.insert(index, element)
pub struct InsertMutation;

impl MutationHandler for InsertMutation {
    fn method_name(&self) -> &'static str { "insert" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "insert" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        args: &[HirExpr],
        state: &TypeState,
        infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        if let Type::List(elem) = current_ty {
            // insert(index, value) - second arg is the value
            if args.len() >= 2 {
                let arg_ty = infer_expr(&args[1], state);
                let refined = refine_element_type(elem, &arg_ty);
                return Some(Type::List(Box::new(refined)));
            }
        }
        None
    }
}

/// Handler for list.pop([index])
pub struct PopMutation;

impl MutationHandler for PopMutation {
    fn method_name(&self) -> &'static str { "pop" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "pop" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // pop doesn't change the list type, it just removes an element
        // The type remains the same
        if let Type::List(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for list.remove(element)
pub struct RemoveMutation;

impl MutationHandler for RemoveMutation {
    fn method_name(&self) -> &'static str { "remove" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "remove" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // remove doesn't change the list type
        if let Type::List(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for list.clear()
pub struct ClearMutation;

impl MutationHandler for ClearMutation {
    fn method_name(&self) -> &'static str { "clear" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "clear" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // clear empties the list but preserves the element type
        if let Type::List(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for list.sort([key], [reverse])
pub struct SortMutation;

impl MutationHandler for SortMutation {
    fn method_name(&self) -> &'static str { "sort" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "sort" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // sort is in-place and doesn't change the type
        if let Type::List(_) = current_ty {
            return Some(current_ty.clone());
        }
        None
    }
}

/// Handler for list.reverse()
pub struct ReverseMutation;

impl MutationHandler for ReverseMutation {
    fn method_name(&self) -> &'static str { "reverse" }
    
    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "reverse" && matches!(ty, Type::List(_))
    }
    
    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // reverse is in-place and doesn't change the type
        if let Type::List(_) = current_ty {
            return Some(current_ty.clone());
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
            HirExpr::Literal(Literal::Float(_)) => Type::Float,
            HirExpr::List(elems) if !elems.is_empty() => {
                Type::List(Box::new(infer_literal(&elems[0], _state)))
            }
            _ => Type::Unknown,
        }
    }
    
    #[test]
    fn test_append_refines_unknown() {
        let handler = AppendMutation;
        let current = Type::List(Box::new(Type::Unknown));
        let args = vec![HirExpr::Literal(Literal::Int(42))];
        let state = make_state();
        
        let result = handler.compute_type(&current, &args, &state, &infer_literal);
        assert_eq!(result, Some(Type::List(Box::new(Type::Int))));
    }
    
    #[test]
    fn test_append_preserves_type() {
        let handler = AppendMutation;
        let current = Type::List(Box::new(Type::Int));
        let args = vec![HirExpr::Literal(Literal::Int(42))];
        let state = make_state();
        
        let result = handler.compute_type(&current, &args, &state, &infer_literal);
        assert_eq!(result, Some(Type::List(Box::new(Type::Int))));
    }
    
    #[test]
    fn test_extend_with_list() {
        let handler = ExtendMutation;
        let current = Type::List(Box::new(Type::Unknown));
        let args = vec![HirExpr::List(vec![HirExpr::Literal(Literal::String("a".to_string()))])];
        let state = make_state();
        
        let result = handler.compute_type(&current, &args, &state, &infer_literal);
        assert_eq!(result, Some(Type::List(Box::new(Type::String))));
    }
    
    #[test]
    fn test_insert_second_arg() {
        let handler = InsertMutation;
        let current = Type::List(Box::new(Type::Unknown));
        let args = vec![
            HirExpr::Literal(Literal::Int(0)), // index
            HirExpr::Literal(Literal::String("value".to_string())), // value
        ];
        let state = make_state();
        
        let result = handler.compute_type(&current, &args, &state, &infer_literal);
        assert_eq!(result, Some(Type::List(Box::new(Type::String))));
    }
}
