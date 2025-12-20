//! String/Bytearray mutation handlers for type inference

use super::MutationHandler;
use crate::dataflow::lattice::TypeState;
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
