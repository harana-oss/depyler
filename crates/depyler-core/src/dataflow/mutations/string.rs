//! String/Bytearray mutation handlers for type inference

use super::{MutationHandler, refine_element_type};
use crate::dataflow::lattice::{TypeLattice, TypeState};
use crate::hir::{HirExpr, Type};

/// Handler for bytearray.append(byte)
pub struct BytearrayAppendMutation;

impl MutationHandler for BytearrayAppendMutation {
    fn method_name(&self) -> &'static str {
        "append"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "append" && matches!(ty, Type::Custom(name) if name == "bytearray")
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // Bytearray type doesn't change with append
        if matches!(current_ty, Type::Custom(name) if name == "bytearray") {
            Some(current_ty.clone())
        } else {
            None
        }
    }
}

/// Handler for bytearray.extend(iterable)
pub struct BytearrayExtendMutation;

impl MutationHandler for BytearrayExtendMutation {
    fn method_name(&self) -> &'static str {
        "extend"
    }

    fn applies_to(&self, ty: &Type, method: &str) -> bool {
        method == "extend" && matches!(ty, Type::Custom(name) if name == "bytearray")
    }

    fn compute_type(
        &self,
        current_ty: &Type,
        _args: &[HirExpr],
        _state: &TypeState,
        _infer_expr: &dyn Fn(&HirExpr, &TypeState) -> Type,
    ) -> Option<Type> {
        // Bytearray type doesn't change with extend
        if matches!(current_ty, Type::Custom(name) if name == "bytearray") {
            Some(current_ty.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> TypeState {
        TypeState::new()
    }

    fn dummy_infer(_expr: &HirExpr, _state: &TypeState) -> Type {
        Type::Unknown
    }

    #[test]
    fn test_bytearray_append_preserves_type() {
        let handler = BytearrayAppendMutation;
        let ba_ty = Type::Custom("bytearray".to_string());
        let state = make_state();

        let result = handler.compute_type(&ba_ty, &[], &state, &dummy_infer);
        assert_eq!(result, Some(ba_ty));
    }

    #[test]
    fn test_bytearray_extend_preserves_type() {
        let handler = BytearrayExtendMutation;
        let ba_ty = Type::Custom("bytearray".to_string());
        let state = make_state();

        let result = handler.compute_type(&ba_ty, &[], &state, &dummy_infer);
        assert_eq!(result, Some(ba_ty));
    }
}
