//! Type-query helpers for `CodeGenContext`.
//!
//! These methods answer "what scalar type does this expression produce?" They
//! are used by codegen to choose between integer and floating-point operations,
//! detect string concatenation vs arithmetic addition, etc.
//!
//! All type-resolution logic is centralised in `get_expr_type`; the convenience
//! predicate `expr_has_type` is the primary entry-point for call-sites that
//! need a boolean answer (plan §29).

use super::CodeGenContext;
use crate::hir::{BinOp, HirExpr, Literal, Type};

impl<'a> CodeGenContext<'a> {
    // ========================================================================
    // Primary type predicate
    // ========================================================================

    /// Returns `true` when `get_expr_type(expr)` agrees with `ty`.
    ///
    /// This is the single entry-point replacing the former `is_expr_float_type`,
    /// `is_expr_int_type`, `is_expr_string_type`, and `is_expr_bool_type`
    /// methods (plan §29).
    pub fn expr_has_type(&self, expr: &HirExpr, ty: &Type) -> bool {
        self.get_expr_type(expr).as_ref() == Some(ty)
    }

    // ========================================================================
    // General type resolution
    // ========================================================================

    /// Get the type of a field access expression (e.g., state.val1)
    pub fn get_attribute_field_type(&self, value: &Box<HirExpr>, attr: &str) -> Option<Type> {
        // Get the class name from the value expression
        let class_name = match value.as_ref() {
            HirExpr::Var(var_name) => {
                // Look up the variable's type to find the class name
                self.var_types.get(var_name).and_then(|ty| {
                    if let Type::Custom(name) = ty {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
            }
            _ => None,
        };

        // Look up the field type in class_field_types
        if let Some(class_name) = class_name {
            if let Some(field_types) = self.class_field_types.get(&class_name) {
                return field_types.get(attr).cloned();
            }
        }

        None
    }

    /// Get the inferred type of an expression.
    ///
    /// This is the single source of truth for scalar-type inference (plan §29).
    /// It covers literals, variables, attributes, arithmetic/comparison binops,
    /// unary ops, common built-in calls, conditional expressions, and f-strings.
    /// Returns `None` when the type cannot be statically determined.
    pub fn get_expr_type(&self, expr: &HirExpr) -> Option<Type> {
        match expr {
            HirExpr::Var(name) => self.var_types.get(name).cloned(),
            HirExpr::Attribute { value, attr } => self.get_attribute_field_type(value, attr),
            HirExpr::Literal(lit) => Some(match lit {
                Literal::Bool(_) => Type::Bool,
                Literal::Bytes(_) => Type::List(Box::new(Type::Int)),
                Literal::Complex(_, _) => Type::Custom("num::Complex<f64>".to_string()),
                Literal::Ellipsis => Type::None,
                Literal::Float(_) => Type::Float,
                Literal::Int(_) => Type::Int,
                Literal::None => Type::None,
                Literal::String(_) => Type::String,
            }),
            HirExpr::FString { .. } => Some(Type::String),
            HirExpr::Unary { operand, .. } => self.get_expr_type(operand),
            HirExpr::Binary { op, left, right } => {
                // Comparison / logical operators always produce bool.
                if matches!(
                    op,
                    BinOp::Eq
                        | BinOp::And
                        | BinOp::Gt
                        | BinOp::GtEq
                        | BinOp::In
                        | BinOp::Is
                        | BinOp::IsNot
                        | BinOp::Lt
                        | BinOp::LtEq
                        | BinOp::NotEq
                        | BinOp::NotIn
                        | BinOp::Or
                ) {
                    return Some(Type::Bool);
                }
                // `/` always yields float in Python (true division).
                if matches!(op, BinOp::Div) {
                    return Some(Type::Float);
                }
                // Float is infectious: if either operand is float, so is the result.
                let left_ty = self.get_expr_type(left);
                let right_ty = self.get_expr_type(right);
                if left_ty == Some(Type::Float) || right_ty == Some(Type::Float) {
                    return Some(Type::Float);
                }
                // Int if both operands are int.
                if left_ty == Some(Type::Int) && right_ty == Some(Type::Int) {
                    return Some(Type::Int);
                }
                None
            }
            HirExpr::Call { func, args, .. } => {
                // Explicit function-return-type registry takes priority.
                if let Some(ty) = self.function_return_types.get(func) {
                    return Some(ty.clone());
                }
                match func.as_str() {
                    "float" => Some(Type::Float),
                    "int" | "len" | "ord" | "round" => Some(Type::Int),
                    "str" | "repr" | "chr" | "format" => Some(Type::String),
                    "bool" | "isinstance" | "issubclass" | "callable" | "hasattr" => {
                        Some(Type::Bool)
                    }
                    // Type-preserving functions: float is infectious across all args;
                    // result is Int only when every arg is Int.
                    "abs" if !args.is_empty() => self.get_expr_type(&args[0]),
                    "min" | "max" | "sum" if !args.is_empty() => {
                        if args
                            .iter()
                            .any(|a| self.get_expr_type(a) == Some(Type::Float))
                        {
                            Some(Type::Float)
                        } else if args
                            .iter()
                            .all(|a| self.get_expr_type(a) == Some(Type::Int))
                        {
                            Some(Type::Int)
                        } else {
                            self.get_expr_type(&args[0])
                        }
                    }
                    _ => None,
                }
            }
            HirExpr::IfExpr { body, orelse, .. } => {
                let body_ty = self.get_expr_type(body);
                let orelse_ty = self.get_expr_type(orelse);
                // Float is infectious: if either branch could be float, treat as float
                // so downstream code always casts to f64 rather than silently losing
                // precision.
                if body_ty == Some(Type::Float) || orelse_ty == Some(Type::Float) {
                    Some(Type::Float)
                } else if body_ty == orelse_ty {
                    body_ty
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if an expression is Optional type and return the inner type if so.
    pub fn get_optional_inner_type(&self, expr: &HirExpr) -> Option<Type> {
        if let Some(Type::Optional(inner)) = self.get_expr_type(expr) {
            Some((*inner).clone())
        } else {
            None
        }
    }
}
