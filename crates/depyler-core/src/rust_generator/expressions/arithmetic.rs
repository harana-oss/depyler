//! Expression code generation - arithmetic

#![allow(unused_imports)]

use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, ToRustExpr};
use crate::rust_generator::direct_rules::to_pascal_case;
use crate::rust_generator::return_type_expects_float;
use crate::rust_generator::type_gen::convert_binop;
use crate::optimizations::string_optimization::{StringContext, StringOptimizer};
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::{self, parse_quote};
use super::ExpressionConverter;

impl<'a, 'b> ExpressionConverter<'a, 'b> {
    pub(super) fn convert_binary(&mut self, op: BinOp, left: &HirExpr, right: &HirExpr) -> Result<syn::Expr> {
        // Handle `x is None` and `x is not None` patterns
        // Python: `x is None` → Rust: `x.is_none()`
        // Python: `x is not None` → Rust: `x.is_some()`
        if matches!(op, BinOp::Is | BinOp::IsNot) {
            let is_left_none = matches!(left, HirExpr::Literal(Literal::None));
            let is_right_none = matches!(right, HirExpr::Literal(Literal::None));

            if is_right_none {
                // `x is None` or `x is not None`
                // Use convert_attribute_without_clone for field access to avoid unnecessary .clone()
                // since is_none()/is_some() only need a reference
                let expr = if matches!(left, HirExpr::Attribute { .. }) {
                    self.convert_attribute_without_clone(left)?
                } else {
                    left.to_rust_expr(self.ctx)?
                };
                return Ok(if op == BinOp::Is {
                    parse_quote! { #expr.is_none() }
                } else {
                    parse_quote! { #expr.is_some() }
                });
            } else if is_left_none {
                // `None is x` or `None is not x` (less common)
                let expr = if matches!(right, HirExpr::Attribute { .. }) {
                    self.convert_attribute_without_clone(right)?
                } else {
                    right.to_rust_expr(self.ctx)?
                };
                return Ok(if op == BinOp::Is {
                    parse_quote! { #expr.is_none() }
                } else {
                    parse_quote! { #expr.is_some() }
                });
            }
        }

        // Convert operands, unwrapping Optional fields automatically
        // Python allows direct access to Optional fields - operations on None fail at runtime
        // Track whether clone was applied during conversion (needed for ref param comparison)
        self.ctx.clone_already_applied = false;
        // For 'in' and 'not in' operators with Optional left operand, use .as_ref().unwrap()
        // to borrow instead of move. This allows the Optional to be used again later.
        let left_is_optional = self.expr_is_optional(left);
        let left_expr = if left_is_optional && matches!(op, BinOp::In | BinOp::NotIn) {
            let base_expr = self.convert_attribute_without_clone(left)?;
            parse_quote! { #base_expr.as_ref().unwrap() }
        } else {
            self.convert_with_optional_unwrap(left)?
        };
        let left_was_cloned = self.ctx.clone_already_applied;

        // For 'in' and 'not in' operators, we need special handling for Optional collections:
        // Use .as_ref().unwrap() to avoid moving the collection out of the struct
        let right_is_optional = self.expr_is_optional(right);
        self.ctx.clone_already_applied = false;
        let right_expr = if right_is_optional && matches!(op, BinOp::In | BinOp::NotIn) {
            // For Optional collections, use .as_ref().unwrap() to borrow instead of clone
            // convert_attribute_without_clone handles intermediate fields, but we need to also
            // unwrap the final field itself
            let base_expr = self.convert_attribute_without_clone(right)?;
            parse_quote! { #base_expr.as_ref().unwrap() }
        } else if matches!(op, BinOp::In | BinOp::NotIn) {
            // .contains()/.contains_key() take &self, no need to clone the container
            let was_prevent_clone = self.ctx.prevent_clone;
            self.ctx.prevent_clone = true;
            let expr = self.convert_with_optional_unwrap(right)?;
            self.ctx.prevent_clone = was_prevent_clone;
            expr
        } else {
            self.convert_with_optional_unwrap(right)?
        };
        let right_was_cloned = self.ctx.clone_already_applied;

        match op {
            BinOp::In => {
                // Convert "x in container" to appropriate method call
                // - String: string.contains(substring)
                // - HashSet: container.contains(&x)
                // - HashMap/dict: container.contains_key(&x)
                // - Tuple: convert to array and use .contains()

                // os.environ in Python is like a dict, but in Rust we check with std::env::var().is_ok()
                if let HirExpr::Attribute { value, attr } = right {
                    if let HirExpr::Var(module_name) = &**value {
                        if module_name == "os" && attr == "environ" {
                            // var in os.environ → std::env::var(var).is_ok()
                            return Ok(parse_quote! { std::env::var(#left_expr).is_ok() });
                        }
                    }
                }

                // Check if right side is a tuple or list literal - convert to array for .contains()
                let elements_opt = match right {
                    HirExpr::Tuple(elements) | HirExpr::List(elements) => Some(elements),
                    _ => None,
                };

                if let Some(elements) = elements_opt {
                    // Check if collection contains only string literals
                    let all_string_literals = !elements.is_empty()
                        && elements
                            .iter()
                            .all(|e| matches!(e, HirExpr::Literal(Literal::String(_))));

                    if all_string_literals {
                        // Keep string literals as &str (don't convert to String)
                        let elem_exprs: Vec<syn::Expr> = elements
                            .iter()
                            .map(|e| e.to_rust_expr(self.ctx))
                            .collect::<Result<Vec<_>>>()?;

                        // Check if left side is a String that needs .as_str()
                        let left_is_string_literal =
                            matches!(left, HirExpr::Literal(Literal::String(_)));
                        if left_is_string_literal {
                            // String literal is already &str, just use it directly
                            return Ok(parse_quote! { [#(#elem_exprs),*].contains(&#left_expr) });
                        }

                        // For String variables/fields, convert to &str using .as_str()
                        // This is more efficient than converting all literals to String
                        let left_is_attribute = matches!(left, HirExpr::Attribute { .. });
                        let left_is_var = matches!(left, HirExpr::Var(_));
                        let left_is_string_type = self.is_string_type(left);

                        if left_is_string_type || left_is_attribute || left_is_var {
                            // For attribute access, avoid the .clone() that's normally added
                            let left_expr_no_clone = if left_is_attribute {
                                self.convert_attribute_without_clone(left)?
                            } else {
                                left_expr.clone()
                            };
                            // Convert String to &str for comparison with &str array
                            return Ok(
                                parse_quote! { [#(#elem_exprs),*].contains(&#left_expr_no_clone.as_str()) },
                            );
                        }
                        return Ok(parse_quote! { [#(#elem_exprs),*].contains(&#left_expr) });
                    }

                    // Non-string elements: convert normally
                    let elem_exprs: Vec<syn::Expr> = elements
                        .iter()
                        .map(|e| e.to_rust_expr(self.ctx))
                        .collect::<Result<Vec<_>>>()?;
                    return Ok(parse_quote! { [#(#elem_exprs),*].contains(&#left_expr) });
                }

                let is_string = self.is_string_type(right);

                // Check if right side is a set based on type information
                let is_set = self.is_set_expr(right) || self.is_set_var(right);

                // Check if right side is a list/array
                let is_list = self.is_list_expr(right);

                // Check if right side is a dict/HashMap
                let is_dict = self.is_dict_expr(right);

                // Check if left side is a string literal
                let left_is_string_literal = matches!(left, HirExpr::Literal(Literal::String(_)));

                // - String: .contains() method
                // - Set: .contains() method
                // - List/Array: .contains() method
                // - HashMap: .contains_key() method with smart reference handling

                // Five-Whys Root Cause:
                // 1. Why: E0308 - expected `&_`, found `String` for contains_key(item)
                // 2. Why: The transpiler doesn't add & when item is owned
                // 3. Why: needs_borrow returns false when type is Type::String
                // 4. Why: Logic is inverted: !matches!(...Type::String) returns false for owned String
                // 5. ROOT CAUSE: The borrowing detection logic is backwards

                // Always add & for HashMap methods. The HIR Type::String doesn't
                // distinguish between owned String and borrowed &str, so we can't reliably
                // detect when to skip the borrow. Since most cases (iterators with .cloned(),
                // owned variables) need the borrow, we default to always borrowing.
                // However, for HashSet<String>, string literals are already &str, so don't add extra &
                let needs_borrow = !left_is_string_literal || !is_set;

                if is_dict {
                    // HashMap: use .get().is_some() because:
                    // 1. serde_json::Value doesn't have .contains_key() method
                    // 2. .get().is_some() is equivalent to .contains_key() for HashMap
                    // 3. Works universally for both HashMap and Value types
                    if needs_borrow {
                        Ok(parse_quote! { #right_expr.get(&#left_expr).is_some() })
                    } else {
                        Ok(parse_quote! { #right_expr.get(#left_expr).is_some() })
                    }
                } else {
                    // Strings, Sets, Lists, and unknown types all use .contains(&value)
                    // Default to .contains() because it works for Vec, HashSet, String, and slices
                    if needs_borrow {
                        Ok(parse_quote! { #right_expr.contains(&#left_expr) })
                    } else {
                        Ok(parse_quote! { #right_expr.contains(#left_expr) })
                    }
                }
            }
            BinOp::NotIn => {
                // Convert "x not in container" to !container.method(&x)

                // os.environ in Python is like a dict, but in Rust we check with !std::env::var().is_ok()
                if let HirExpr::Attribute { value, attr } = right {
                    if let HirExpr::Var(module_name) = &**value {
                        if module_name == "os" && attr == "environ" {
                            // var not in os.environ → !std::env::var(var).is_ok()
                            return Ok(parse_quote! { !std::env::var(#left_expr).is_ok() });
                        }
                    }
                }

                // Check if right side is a tuple - convert to array for .contains()
                if let HirExpr::Tuple(elements) | HirExpr::List(elements) = right {
                    // Check if collection contains only string literals
                    let all_string_literals = !elements.is_empty()
                        && elements
                            .iter()
                            .all(|e| matches!(e, HirExpr::Literal(Literal::String(_))));

                    let elem_exprs: Vec<syn::Expr> = elements
                        .iter()
                        .map(|e| e.to_rust_expr(self.ctx))
                        .collect::<Result<Vec<_>>>()?;

                    if all_string_literals {
                        // Check if left side is a String that needs .as_str()
                        let left_is_string_literal =
                            matches!(left, HirExpr::Literal(Literal::String(_)));
                        if left_is_string_literal {
                            return Ok(parse_quote! { ![#(#elem_exprs),*].contains(&#left_expr) });
                        }

                        // For String variables/fields, convert to &str using .as_str()
                        let left_is_attribute = matches!(left, HirExpr::Attribute { .. });
                        let left_is_var = matches!(left, HirExpr::Var(_));
                        let left_is_string_type = self.is_string_type(left);

                        if left_is_string_type || left_is_attribute || left_is_var {
                            let left_expr_no_clone = if left_is_attribute {
                                self.convert_attribute_without_clone(left)?
                            } else {
                                left_expr.clone()
                            };
                            return Ok(
                                parse_quote! { ![#(#elem_exprs),*].contains(&#left_expr_no_clone.as_str()) },
                            );
                        }
                    }
                    return Ok(parse_quote! { ![#(#elem_exprs),*].contains(&#left_expr) });
                }

                let is_string = self.is_string_type(right);

                // Check if right side is a set based on type information
                let is_set = self.is_set_expr(right) || self.is_set_var(right);

                // Check if right side is a list/array
                let is_list = self.is_list_expr(right);

                // Check if right side is a dict/HashMap
                let is_dict = self.is_dict_expr(right);

                // Check if left side is a string literal
                let left_is_string_literal = matches!(left, HirExpr::Literal(Literal::String(_)));

                // Same logic as BinOp::In, but negated
                // For string contains, always need &sub because str::contains takes &str/Pattern
                // However, for HashSet<String>, string literals are already &str, so don't add extra &
                let needs_borrow = !left_is_string_literal || !is_set;

                if is_dict {
                    // HashMap: use !.get().is_some() because:
                    // 1. serde_json::Value doesn't have .contains_key() method
                    // 2. .get().is_some() is equivalent to .contains_key() for HashMap
                    // 3. Works universally for both HashMap and Value types
                    if needs_borrow {
                        Ok(parse_quote! { !#right_expr.get(&#left_expr).is_some() })
                    } else {
                        Ok(parse_quote! { !#right_expr.get(#left_expr).is_some() })
                    }
                } else {
                    // Strings, Sets, Lists, and unknown types all use !.contains(&value)
                    // Default to .contains() because it works for Vec, HashSet, String, and slices
                    if needs_borrow {
                        Ok(parse_quote! { !#right_expr.contains(&#left_expr) })
                    } else {
                        Ok(parse_quote! { !#right_expr.contains(#left_expr) })
                    }
                }
            }
            BinOp::Add => {
                // Check if we're dealing with lists/vectors (explicit detection only)
                let is_definitely_list = self.is_list_expr(left) || self.is_list_expr(right);

                let is_list_var = match (left, right) {
                    (HirExpr::Var(name), _) | (_, HirExpr::Var(name)) => self
                        .ctx
                        .var_types
                        .get(name)
                        .map(|t| matches!(t, Type::List(_)))
                        .unwrap_or(false),
                    _ => false,
                };

                // Slices produce Vec via .to_vec(), so slice + slice needs extend pattern
                let is_slice_concat =
                    matches!(left, HirExpr::Slice { .. }) || matches!(right, HirExpr::Slice { .. });

                // Check if we're dealing with strings (literals, type-inferred, or heuristic)
                let is_definitely_string = matches!(left, HirExpr::Literal(Literal::String(_)))
                    || matches!(right, HirExpr::Literal(Literal::String(_)))
                    || matches!(self.ctx.current_return_type, Some(Type::String))
                    || self.is_string_base(left)
                    || self.is_string_base(right)
                    || self.is_string_type_from_context(left)
                    || self.is_string_type_from_context(right);

                if (is_definitely_list || is_slice_concat || is_list_var) && !is_definitely_string {
                    // List/slice concatenation - use chain pattern for references
                    // Convert: list1 + list2 (where both are &Vec or Vec)
                    // To: list1.iter().chain(list2.iter()).cloned().collect()
                    Ok(parse_quote! {
                        #left_expr.iter().chain(#right_expr.iter()).cloned().collect::<Vec<_>>()
                    })
                } else if is_definitely_string {
                    // This is string concatenation - use format! to handle references properly
                    // Don't call to_rust_expr on literals since that adds unnecessary .to_string()
                    // format! can handle both &str and String directly
                    let left_fmt = self.generate_format_arg(left)?;
                    let right_fmt = self.generate_format_arg(right)?;
                    Ok(parse_quote! { format!("{}{}", #left_fmt, #right_fmt) })
                } else {
                    // Regular arithmetic addition - handle mixed int/float types
                    let left_is_float = self.ctx.expr_has_type(left, &Type::Float);
                    let right_is_float = self.ctx.expr_has_type(right, &Type::Float);
                    let left_is_int_type = self.ctx.expr_has_type(left, &Type::Int);
                    let right_is_int_type = self.ctx.expr_has_type(right, &Type::Int);

                    // If neither side is known to be numeric, it might be string concatenation
                    // BUT: if either side involves arithmetic operators (*, /, -, etc.), it's numeric
                    let left_involves_arithmetic = self.involves_arithmetic_op(left);
                    let right_involves_arithmetic = self.involves_arithmetic_op(right);
                    let any_is_numeric = left_is_float
                        || right_is_float
                        || left_is_int_type
                        || right_is_int_type
                        || left_involves_arithmetic
                        || right_involves_arithmetic;

                    if !any_is_numeric {
                        let rust_op = convert_binop(op)?;
                        Ok(syn::Expr::Binary(syn::ExprBinary {
                            attrs: vec![],
                            left: Box::new(left_expr),
                            op: rust_op,
                            right: Box::new(right_expr),
                        }))
                    } else {
                        self.emit_arithmetic_with_mixed_cast(
                            op, left, right, left_expr, right_expr,
                        )
                    }
                }
            }
            BinOp::FloorDiv => {
                // Python floor division semantics differ from Rust integer division
                // Python: rounds towards negative infinity (floor)
                // Rust: truncates towards zero
                // Check types to handle mixed float/int cases
                let left_is_float = self.ctx.expr_has_type(left, &Type::Float);
                let right_is_float = self.ctx.expr_has_type(right, &Type::Float);
                let left_is_int_type = self.ctx.expr_has_type(left, &Type::Int);
                let right_is_int_type = self.ctx.expr_has_type(right, &Type::Int);

                // If one operand is float and the other is int, cast the int to f64
                if left_is_float && right_is_int_type {
                    // Left is float, right is int - cast right to f64
                    Ok(parse_quote! { (#left_expr / (#right_expr as f64)).floor() })
                } else if left_is_int_type && right_is_float {
                    // Left is int, right is float - cast left to f64
                    Ok(parse_quote! { ((#left_expr as f64) / #right_expr).floor() })
                } else if left_is_float && right_is_float {
                    // Both are floats
                    Ok(parse_quote! { (#left_expr / #right_expr).floor() })
                } else {
                    // Both are integers (or default case)
                    // Skip temp bindings when operands are simple vars/literals
                    if matches!(left, HirExpr::Var(_) | HirExpr::Literal(_))
                        && matches!(right, HirExpr::Var(_) | HirExpr::Literal(_))
                    {
                        Ok(parse_quote! {
                            {
                                let d = #left_expr / #right_expr;
                                let r = #left_expr % #right_expr;
                                if r != 0 && (#left_expr ^ #right_expr) < 0 { d - 1 } else { d }
                            }
                        })
                    } else {
                        Ok(parse_quote! {
                            {
                                let a = #left_expr;
                                let b = #right_expr;
                                let d = a / b;
                                let r = a % b;
                                if r != 0 && (a ^ b) < 0 { d - 1 } else { d }
                            }
                        })
                    }
                }
            }
            // Python 3.9+ supports d1 | d2 for dictionary merge
            // Translate to: { let mut result = d1; result.extend(d2); result }
            BinOp::BitOr if self.is_dict_expr(left) || self.is_dict_expr(right) => {
                self.ctx.require(crate::rust_generator::context::Import::HashMap);
                Ok(parse_quote! {
                    {
                        let mut __merge_result = #left_expr.clone();
                        __merge_result.extend(#right_expr.iter().map(|(k, v)| (k.clone(), *v)));
                        __merge_result
                    }
                })
            }
            // Set operators - check if both operands are sets
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor
                if self.is_set_expr(left) && self.is_set_expr(right) =>
            {
                self.convert_set_operation(op, left_expr, right_expr)
            }
            BinOp::Sub if self.is_set_expr(left) && self.is_set_expr(right) => {
                // Set difference operation
                self.convert_set_operation(op, left_expr, right_expr)
            }
            BinOp::Sub => {
                // Check if we're subtracting from a .len() call to prevent underflow
                if self.is_len_call(left) {
                    // Use saturating_sub to prevent underflow when subtracting from array length
                    // Wrap left_expr in parens because it contains a cast: (arr.len() as i32).saturating_sub(x)
                    // Without parens, Rust parses "as i32.saturating_sub" incorrectly
                    Ok(parse_quote! { (#left_expr).saturating_sub(#right_expr) })
                } else {
                    self.emit_arithmetic_with_mixed_cast(op, left, right, left_expr, right_expr)
                }
            }
            BinOp::Mul => {
                // Check if we have string * integer or integer * string
                let left_is_string = self.is_string_base(left);
                let right_is_string = self.is_string_base(right);
                let left_is_int =
                    matches!(left, HirExpr::Literal(Literal::Int(_)) | HirExpr::Var(_));
                let right_is_int =
                    matches!(right, HirExpr::Literal(Literal::Int(_)) | HirExpr::Var(_));

                if left_is_string && right_is_int {
                    // Pattern: s * n (string * integer)
                    return Ok(parse_quote! { #left_expr.repeat(#right_expr as usize) });
                } else if left_is_int && right_is_string {
                    // Pattern: n * s (integer * string)
                    return Ok(parse_quote! { #right_expr.repeat(#left_expr as usize) });
                }

                // Special case: [value] * n or n * [value] creates an array
                match (left, right) {
                    // Pattern: [x] * n (small arrays)
                    (HirExpr::List(elts), HirExpr::Literal(Literal::Int(size)))
                        if elts.len() == 1 && *size > 0 && *size <= 32 =>
                    {
                        let elem = elts[0].to_rust_expr(self.ctx)?;
                        let size_lit =
                            syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
                        // Check if element is a variable referring to a non-Copy type (Vec, String, etc.)
                        let needs_clone = self.element_needs_clone(&elts[0]);
                        if needs_clone {
                            // Non-Copy types: use vec![elem.clone(); n]
                            Ok(parse_quote! { vec![#elem.clone(); #size_lit] })
                        } else {
                            Ok(parse_quote! { [#elem; #size_lit] })
                        }
                    }
                    (HirExpr::List(elts), HirExpr::Literal(Literal::Int(size)))
                        if elts.len() == 1 && *size > 32 =>
                    {
                        let elem = elts[0].to_rust_expr(self.ctx)?;
                        let size_lit =
                            syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
                        // Non-Copy types need clone
                        let needs_clone = self.element_needs_clone(&elts[0]);
                        if needs_clone {
                            Ok(parse_quote! { vec![#elem.clone(); #size_lit] })
                        } else {
                            Ok(parse_quote! { vec![#elem; #size_lit] })
                        }
                    }
                    // Pattern: n * [x] (small arrays)
                    (HirExpr::Literal(Literal::Int(size)), HirExpr::List(elts))
                        if elts.len() == 1 && *size > 0 && *size <= 32 =>
                    {
                        let elem = elts[0].to_rust_expr(self.ctx)?;
                        let size_lit =
                            syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
                        let needs_clone = self.element_needs_clone(&elts[0]);
                        if needs_clone {
                            Ok(parse_quote! { vec![#elem.clone(); #size_lit] })
                        } else {
                            Ok(parse_quote! { [#elem; #size_lit] })
                        }
                    }
                    (HirExpr::Literal(Literal::Int(size)), HirExpr::List(elts))
                        if elts.len() == 1 && *size > 32 =>
                    {
                        let elem = elts[0].to_rust_expr(self.ctx)?;
                        let size_lit =
                            syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
                        let needs_clone = self.element_needs_clone(&elts[0]);
                        if needs_clone {
                            Ok(parse_quote! { vec![#elem.clone(); #size_lit] })
                        } else {
                            Ok(parse_quote! { vec![#elem; #size_lit] })
                        }
                    }
                    _ => {
                        self.emit_arithmetic_with_mixed_cast(op, left, right, left_expr, right_expr)
                    }
                }
            }
            BinOp::Div => {
                // Python's `/` operator ALWAYS returns float, even with integer operands
                // This is different from `//` (floor division) which returns int
                // Rust's `/` does integer division when both operands are integers,
                // so we must cast to f64 when both operands are integers.
                let left_is_float = self.ctx.expr_has_type(left, &Type::Float);
                let right_is_float = self.ctx.expr_has_type(right, &Type::Float);

                if !left_is_float && !right_is_float {
                    // Both operands are integers: cast both to f64 for Python's true division
                    Ok(parse_quote! { (#left_expr as f64) / (#right_expr as f64) })
                } else if !left_is_float && right_is_float {
                    // Mixed types: int / float - cast int to f64
                    Ok(parse_quote! { (#left_expr as f64) / #right_expr })
                } else if left_is_float && !right_is_float {
                    // Mixed types: float / int - cast int to f64
                    Ok(parse_quote! { #left_expr / (#right_expr as f64) })
                } else {
                    // Both are floats: regular float division
                    let rust_op = convert_binop(op)?;
                    Ok(syn::Expr::Binary(syn::ExprBinary {
                        attrs: vec![],
                        left: Box::new(left_expr),
                        op: rust_op,
                        right: Box::new(right_expr),
                    }))
                }
            }
            BinOp::Pow => {
                // Python power operator ** needs type-specific handling in Rust
                // For integers: use .pow() with u32 exponent
                // For floats: use .powf() with f64 exponent
                // For negative integer exponents: convert to float

                // Check if we have literals to determine types
                match (left, right) {
                    // Integer literal base with integer literal exponent
                    (HirExpr::Literal(Literal::Int(_)), HirExpr::Literal(Literal::Int(exp))) => {
                        if *exp < 0 {
                            // Negative exponent: convert to float operation
                            Ok(parse_quote! {
                                (#left_expr as f64).powf(#right_expr as f64)
                            })
                        } else {
                            // Positive integer exponent: use simple .pow() with type suffix
                            // Generate: 2_i32.pow(8 as u32) instead of (2 as i32).pow(...)
                            // Extract the integer value to create a typed literal
                            if let HirExpr::Literal(Literal::Int(base_val)) = left {
                                let base_str = base_val.to_string();
                                let typed_base = syn::LitInt::new(
                                    &format!("{}_i32", base_str),
                                    proc_macro2::Span::call_site(),
                                );
                                Ok(parse_quote! {
                                    #typed_base.pow(#right_expr as u32)
                                })
                            } else {
                                // Fallback (shouldn't happen)
                                Ok(parse_quote! {
                                    (#left_expr as i32).pow(#right_expr as u32)
                                })
                            }
                        }
                    }
                    // Float literal base: always use .powf()
                    (HirExpr::Literal(Literal::Float(_)), _) => Ok(parse_quote! {
                        (#left_expr as f64).powf(#right_expr as f64)
                    }),
                    // Any base with float exponent: use .powf()
                    (_, HirExpr::Literal(Literal::Float(_))) => Ok(parse_quote! {
                        (#left_expr as f64).powf(#right_expr as f64)
                    }),
                    // Integer literal base with variable exponent - need type suffix
                    (HirExpr::Literal(Literal::Int(base_val)), _) => {
                        let base_str = base_val.to_string();
                        let typed_base = syn::LitInt::new(
                            &format!("{}_i32", base_str),
                            proc_macro2::Span::call_site(),
                        );
                        Ok(parse_quote! {
                            #typed_base.checked_pow(#right_expr as u32).expect("Power operation overflowed")
                        })
                    }
                    // Variable base with integer literal exponent
                    (_, HirExpr::Literal(Literal::Int(exp))) if *exp >= 0 => {
                        // Use simple .pow() for positive literal exponents
                        // Generate: a.pow(2 as u32)
                        Ok(parse_quote! {
                            #left_expr.pow(#right_expr as u32)
                        })
                    }
                    // Variable or expression base with variable exponent
                    _ => {
                        // Check if we're dealing with float types
                        let left_is_float = matches!(left, HirExpr::Var(name) if self.ctx.var_types.get(name) == Some(&Type::Float))
                            || matches!(left, HirExpr::Literal(Literal::Float(_)));
                        let right_is_float = matches!(right, HirExpr::Var(name) if self.ctx.var_types.get(name) == Some(&Type::Float))
                            || matches!(right, HirExpr::Literal(Literal::Float(_)));

                        if left_is_float || right_is_float {
                            // For float types, use .powf()
                            Ok(parse_quote! {
                                (#left_expr as f64).powf(#right_expr as f64)
                            })
                        } else {
                            // For integer types, use checked_pow for safety
                            // Generate: a.checked_pow(b as u32)
                            Ok(parse_quote! {
                                #left_expr.checked_pow(#right_expr as u32).expect("Power operation overflowed")
                            })
                        }
                    }
                }
            }
            BinOp::MatMul => {
                // Matrix multiplication operator @ in Python
                // Rust doesn't have a built-in @ operator, so we emit a matmul function call
                Ok(parse_quote! { matmul(#left_expr, #right_expr) })
            }
            // Python: `if a and b:` where a, b are strings/lists/etc.
            // Rust: `if (!a.is_empty()) && (!b.is_empty())`
            BinOp::And => {
                // Apply truthiness conversion to both operands
                let left_converted = Self::apply_truthiness_conversion(left, left_expr, self.ctx);
                let right_converted =
                    Self::apply_truthiness_conversion(right, right_expr, self.ctx);

                Ok(parse_quote! { (#left_converted) && (#right_converted) })
            }
            // Python: `optional_val or default` returns the first truthy value
            // Rust: `optional_val.unwrap_or_else(|| default)`
            BinOp::Or => {
                // Check if left operand is Optional - use unwrap_or_else pattern
                if let Some(inner_type) = self.ctx.get_optional_inner_type(left) {
                    // Get the left expression without auto-unwrap
                    let left_no_unwrap = self.convert_expr_no_unwrap(left)?;

                    // Check if we need to handle chained optionals: opt1 or opt2 or default
                    let right_is_optional = self.ctx.get_optional_inner_type(right).is_some();

                    if right_is_optional {
                        // Chained optionals: opt1 or opt2 -> opt1.or_else(|| opt2)
                        let right_no_unwrap = self.convert_expr_no_unwrap(right)?;
                        return Ok(parse_quote! {
                            #left_no_unwrap.clone().or_else(|| #right_no_unwrap.clone())
                        });
                    }

                    // For string default values, ensure proper conversion
                    let right_is_string_literal =
                        matches!(right, HirExpr::Literal(Literal::String(_)));

                    // Optional[String] with string literal default - add .to_string()
                    if matches!(inner_type, Type::String) && right_is_string_literal {
                        let right_expr = right.to_rust_expr(self.ctx)?;
                        return Ok(parse_quote! {
                            #left_no_unwrap.clone().unwrap_or_else(|| #right_expr.to_string())
                        });
                    }

                    // Other Optional types - use unwrap_or_else directly
                    let right_expr = right.to_rust_expr(self.ctx)?;
                    return Ok(parse_quote! {
                        #left_no_unwrap.clone().unwrap_or_else(|| #right_expr)
                    });
                }

                // Check if left is a chained Optional (result of or_else) - this handles: (opt1 or opt2) or "default"
                // When the HirExpr is a Binary Or where left was Optional, the result is still Optional
                if let HirExpr::Binary {
                    op: BinOp::Or,
                    left: inner_left,
                    ..
                } = left
                {
                    if self.ctx.get_optional_inner_type(inner_left).is_some() {
                        // This is a chained or expression ending with a non-Optional default
                        let right_is_string_literal =
                            matches!(right, HirExpr::Literal(Literal::String(_)));
                        let right_expr = right.to_rust_expr(self.ctx)?;

                        if right_is_string_literal {
                            return Ok(parse_quote! {
                                #left_expr.unwrap_or_else(|| #right_expr.to_string())
                            });
                        }
                        return Ok(parse_quote! {
                            #left_expr.unwrap_or_else(|| #right_expr)
                        });
                    }
                }

                // Non-Optional: use boolean or with truthiness conversion
                let left_converted = Self::apply_truthiness_conversion(left, left_expr, self.ctx);
                let right_converted =
                    Self::apply_truthiness_conversion(right, right_expr, self.ctx);

                Ok(parse_quote! { (#left_converted) || (#right_converted) })
            }
            BinOp::Eq | BinOp::NotEq => {
                // Handle reference parameter compared to enum variant:
                // &Team == Team::Home needs to become *team == Team::Home
                // BUT: if .clone() was already applied, the ref is already dereferenced
                // since &T.clone() returns T, not &T
                let left_needs_deref =
                    !left_was_cloned && self.is_ref_param_compared_to_enum_variant(left, right);
                let right_needs_deref =
                    !right_was_cloned && self.is_ref_param_compared_to_enum_variant(right, left);

                // String comparison optimization: String implements PartialEq<&str>
                // so we can compare `my_string == "literal"` directly without cloning
                // or converting the literal to String.
                let left_is_string = self.is_string_type(left);
                let right_is_string = self.is_string_type(right);
                let left_is_literal = matches!(left, HirExpr::Literal(Literal::String(_)));
                let right_is_literal = matches!(right, HirExpr::Literal(Literal::String(_)));

                // For string vs literal comparisons, avoid unnecessary clone and .to_string()
                let final_left = if left_needs_deref {
                    // Dereference the reference parameter for enum comparison
                    parse_quote! { *#left_expr }
                } else if !left_is_literal && left_is_string && right_is_literal {
                    // Left is a String variable/expr, right is a literal
                    // Convert without clone since comparison only borrows
                    self.convert_expr_without_clone(left)?
                } else if left_is_literal && right_is_string && !right_is_literal {
                    // Left is a literal being compared to a String - keep as &str
                    left_expr
                } else {
                    left_expr
                };

                let final_right = if right_needs_deref {
                    // Dereference the reference parameter for enum comparison
                    parse_quote! { *#right_expr }
                } else if !right_is_literal && right_is_string && left_is_literal {
                    // Right is a String variable/expr, left is a literal
                    // Convert without clone since comparison only borrows
                    self.convert_expr_without_clone(right)?
                } else if right_is_literal && left_is_string && !left_is_literal {
                    // Right is a literal being compared to a String - keep as &str
                    right_expr
                } else {
                    right_expr
                };

                let rust_op = convert_binop(op)?;
                Ok(syn::Expr::Binary(syn::ExprBinary {
                    attrs: vec![],
                    left: Box::new(final_left),
                    op: rust_op,
                    right: Box::new(final_right),
                }))
            }
            BinOp::Mod => {
                self.emit_arithmetic_with_mixed_cast(op, left, right, left_expr, right_expr)
            }
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                // Comparison between float and int requires casting the int operand to f64.
                // Python: `state.time_elapsed > HALF_LENGTH_IN_SECONDS` (float > int)
                // Rust: `state.time_elapsed > (HALF_LENGTH_IN_SECONDS as f64)`
                self.emit_comparison_with_mixed_cast(op, left, right, left_expr, right_expr)
            }
            _ => {
                let rust_op = convert_binop(op)?;
                Ok(syn::Expr::Binary(syn::ExprBinary {
                    attrs: vec![],
                    left: Box::new(left_expr),
                    op: rust_op,
                    right: Box::new(right_expr),
                }))
            }
        }
    }

    /// Emit an arithmetic binary expression, casting the int operand to f64 when mixed.
    pub(super) fn emit_arithmetic_with_mixed_cast(
        &self,
        op: BinOp,
        left: &HirExpr,
        right: &HirExpr,
        left_expr: syn::Expr,
        right_expr: syn::Expr,
    ) -> Result<syn::Expr> {
        let left_is_float = self.ctx.expr_has_type(left, &Type::Float);
        let right_is_float = self.ctx.expr_has_type(right, &Type::Float);

        // Strip .clone() from constants in mixed arithmetic expressions
        fn strip_clone(expr: &syn::Expr) -> syn::Expr {
            match expr {
                syn::Expr::MethodCall(mc) if mc.method == "clone" && mc.args.is_empty() => {
                    if let syn::Expr::Path(ref path) = *mc.receiver {
                        syn::Expr::Path(path.clone())
                    } else {
                        strip_clone(&mc.receiver)
                    }
                }
                syn::Expr::Binary(bin) => syn::Expr::Binary(syn::ExprBinary {
                    left: Box::new(strip_clone(&bin.left)),
                    right: Box::new(strip_clone(&bin.right)),
                    attrs: bin.attrs.clone(),
                    op: bin.op.clone(),
                }),
                _ => expr.clone(),
            }
        }

        // Cast non-float operand to f64 when mixed with float.
        // If exactly one side is float, always promote the other side to f64.
        // This handles known-int, unknown-type, and nested expressions (e.g.,
        // CSE temps, unregistered constants, Add-of-ints) uniformly.
        if (left_is_float && !right_is_float) || (!left_is_float && right_is_float) {
            let rust_op = convert_binop(op)?;
            let cast_to_f64 = |expr: &syn::Expr, is_float: bool| {
                let expr = strip_clone(expr);
                if is_float {
                    expr
                } else {
                    parse_quote! { ((#expr) as f64) }
                }
            };
            let cast_left = cast_to_f64(&left_expr, left_is_float);
            let cast_right = cast_to_f64(&right_expr, right_is_float);
            Ok(syn::Expr::Binary(syn::ExprBinary {
                attrs: vec![],
                left: Box::new(cast_left),
                op: rust_op,
                right: Box::new(cast_right),
            }))
        } else if left_is_float && right_is_float {
            // Both float: strip .clone() but no cast needed
            let rust_op = convert_binop(op)?;
            let clean_left = strip_clone(&left_expr);
            let clean_right = strip_clone(&right_expr);
            Ok(syn::Expr::Binary(syn::ExprBinary {
                attrs: vec![],
                left: Box::new(clean_left),
                op: rust_op,
                right: Box::new(clean_right),
            }))
        } else {
            let rust_op = convert_binop(op)?;
            Ok(syn::Expr::Binary(syn::ExprBinary {
                attrs: vec![],
                left: Box::new(left_expr),
                op: rust_op,
                right: Box::new(right_expr),
            }))
        }
    }

    /// Emit a comparison expression, casting the int operand to f64 when mixed.
    pub(super) fn emit_comparison_with_mixed_cast(
        &self,
        op: BinOp,
        left: &HirExpr,
        right: &HirExpr,
        left_expr: syn::Expr,
        right_expr: syn::Expr,
    ) -> Result<syn::Expr> {
        let left_is_float = self.ctx.expr_has_type(left, &Type::Float);
        let right_is_float = self.ctx.expr_has_type(right, &Type::Float);
        let rust_op = convert_binop(op)?;

        if left_is_float && !right_is_float {
            Ok(syn::Expr::Binary(syn::ExprBinary {
                attrs: vec![],
                left: Box::new(left_expr),
                op: rust_op,
                right: Box::new(parse_quote! { (#right_expr as f64) }),
            }))
        } else if !left_is_float && right_is_float {
            Ok(syn::Expr::Binary(syn::ExprBinary {
                attrs: vec![],
                left: Box::new(parse_quote! { (#left_expr as f64) }),
                op: rust_op,
                right: Box::new(right_expr),
            }))
        } else {
            Ok(syn::Expr::Binary(syn::ExprBinary {
                attrs: vec![],
                left: Box::new(left_expr),
                op: rust_op,
                right: Box::new(right_expr),
            }))
        }
    }

    pub(super) fn convert_unary(&mut self, op: &UnaryOp, operand: &HirExpr) -> Result<syn::Expr> {
        let is_optional = self.expr_is_optional(operand);
        let operand_expr = operand.to_rust_expr(self.ctx)?;

        // Unwrap Optional operands for unary operations (except for special cases handled below)
        let unwrapped_expr = if is_optional {
            parse_quote! { #operand_expr.unwrap() }
        } else {
            operand_expr.clone()
        };

        match op {
            UnaryOp::Not => {
                // For collections (list, dict, set, string), use .is_empty() instead of !
                // because Rust doesn't allow ! operator on non-bool types
                let is_collection = if let HirExpr::Var(var_name) = operand {
                    if let Some(var_type) = self.ctx.var_types.get(var_name) {
                        matches!(
                            var_type,
                            Type::List(_) | Type::Dict(_, _) | Type::Set(_) | Type::String
                        )
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Python: `if not re.match(...)` or `if not compiled.find(...)`
                // Rust: Cannot use ! on Option<Match>, need .is_none()
                let is_option_returning_call = if let HirExpr::MethodCall {
                    object: _,
                    method,
                    args: _,
                    kwargs: _,
                    type_params: _,
                } = operand
                {
                    // Regex methods that return Option<Match>
                    matches!(method.as_str(), "find" | "search" | "match")
                } else if let HirExpr::Call {
                    func,
                    args: _,
                    kwargs: _,
                    type_params: _,
                } = operand
                {
                    // Module-level regex functions (re.match, re.search, re.find)
                    matches!(func.as_str(), "match" | "search" | "find")
                } else {
                    false
                };

                // Python: `not (x & y)` where x, y are integers
                // Rust: Cannot use ! on integers, need `(x & y) == 0`
                let is_integer_bitwise_op = matches!(
                    operand,
                    HirExpr::Binary {
                        op: BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor,
                        ..
                    }
                );

                if is_collection {
                    Ok(parse_quote! { #operand_expr.is_empty() })
                } else if is_option_returning_call {
                    // For Option-returning methods, use .is_none() instead of !
                    Ok(parse_quote! { #operand_expr.is_none() })
                } else if is_integer_bitwise_op {
                    // For bitwise operations on integers, compare result to 0
                    Ok(parse_quote! { (#operand_expr) == 0 })
                } else {
                    Ok(parse_quote! { !#unwrapped_expr })
                }
            }
            UnaryOp::Neg => {
                // Only wrap in parentheses for complex operands to avoid `x < -1` becoming `x<-1`
                // Simple literals like `-1` don't need parentheses
                let is_simple_literal = matches!(
                    operand,
                    HirExpr::Literal(Literal::Int(_) | Literal::Float(_))
                );
                if is_simple_literal {
                    Ok(parse_quote! { -#unwrapped_expr })
                } else {
                    Ok(parse_quote! { (-#unwrapped_expr) })
                }
            }
            UnaryOp::Pos => Ok(unwrapped_expr), // No +x in Rust
            UnaryOp::BitNot => Ok(parse_quote! { !#unwrapped_expr }),
        }
    }

}
