//! Expression code generation
//!
//! This module handles converting HIR expressions to Rust syn::Expr nodes.
//! It includes the ExpressionConverter for complex expression transformations
//! and the ToRustExpr trait implementation for HirExpr.

use crate::direct_rules::to_pascal_case;
use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, ToRustExpr};
use crate::rust_gen::return_type_expects_float;
use crate::rust_gen::type_gen::convert_binop;
use crate::string_optimization::{StringContext, StringOptimizer};
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::{self, parse_quote};

struct ExpressionConverter<'a, 'b> {
    ctx: &'a mut CodeGenContext<'b>,
}

impl<'a, 'b> ExpressionConverter<'a, 'b> {
    fn new(ctx: &'a mut CodeGenContext<'b>) -> Self {
        Self { ctx }
    }

    /// Check if a name is a Rust keyword that requires raw identifier syntax
    fn is_rust_keyword(name: &str) -> bool {
        matches!(
            name,
            "as" | "break"
                | "const"
                | "continue"
                | "crate"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "Self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "type"
                | "unsafe"
                | "use"
                | "where"
                | "while"
                | "async"
                | "await"
                | "dyn"
                | "abstract"
                | "become"
                | "box"
                | "do"
                | "final"
                | "macro"
                | "override"
                | "priv"
                | "typeof"
                | "unsized"
                | "virtual"
                | "yield"
                | "try"
        )
    }

    /// Check if a keyword cannot be used as a raw identifier
    /// These special keywords (self, Self, super, crate) cannot use r# syntax
    fn is_non_raw_keyword(name: &str) -> bool {
        matches!(name, "Self" | "super" | "crate")
    }

    /// Sanitize a variable name for use in Rust
    /// Renames self -> self_param since `self` is a special keyword in Rust
    fn sanitize_var_name(name: &str) -> &str {
        if name == "self" { "self_param" } else { name }
    }

    fn convert_variable(&self, name: &str) -> Result<syn::Expr> {
        // Sanitize the name - rename self to self_param
        let name = Self::sanitize_var_name(name);

        // Check for special keywords that cannot be raw identifiers
        if Self::is_non_raw_keyword(name) {
            bail!(
                "Python variable '{}' conflicts with a special Rust keyword that cannot be escaped. \
                 Please rename this variable (e.g., '{}_var' or 'py_{}')",
                name,
                name,
                name
            );
        }

        // Inside generators, check if variable is a state variable
        if self.ctx.in_generator && self.ctx.generator_state_vars.contains(name) {
            // Generate self.field for state variables
            let ident = if Self::is_rust_keyword(name) {
                syn::Ident::new_raw(name, proc_macro2::Span::call_site())
            } else {
                syn::Ident::new(name, proc_macro2::Span::call_site())
            };
            Ok(parse_quote! { self.#ident })
        } else {
            // Regular variable - use raw identifier if it's a Rust keyword
            let ident = if Self::is_rust_keyword(name) {
                syn::Ident::new_raw(name, proc_macro2::Span::call_site())
            } else {
                syn::Ident::new(name, proc_macro2::Span::call_site())
            };
            Ok(parse_quote! { #ident })
        }
    }

    /// Generate an expression for use in format! macro
    /// String literals don't need .to_string() since format! handles &str
    fn generate_format_arg(&mut self, expr: &HirExpr) -> Result<syn::Expr> {
        match expr {
            HirExpr::Literal(Literal::String(s)) => {
                // For string literals in format!, just use the string directly
                // format! can handle &str, so no need for .to_string()
                let lit = syn::LitStr::new(s, proc_macro2::Span::call_site());
                Ok(parse_quote! { #lit })
            }
            _ => {
                // For other expressions, use normal conversion
                expr.to_rust_expr(self.ctx)
            }
        }
    }

    fn convert_binary(&mut self, op: BinOp, left: &HirExpr, right: &HirExpr) -> Result<syn::Expr> {
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
                    let left_is_float = self.ctx.is_expr_float_type(left);
                    let right_is_float = self.ctx.is_expr_float_type(right);
                    let left_is_int_type = self.ctx.is_expr_int_type(left);
                    let right_is_int_type = self.ctx.is_expr_int_type(right);

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
                let left_is_float = self.ctx.is_expr_float_type(left);
                let right_is_float = self.ctx.is_expr_float_type(right);
                let left_is_int_type = self.ctx.is_expr_int_type(left);
                let right_is_int_type = self.ctx.is_expr_int_type(right);

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
                self.ctx.needs_hashmap = true;
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
                let left_is_float = self.ctx.is_expr_float_type(left);
                let right_is_float = self.ctx.is_expr_float_type(right);

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
    fn emit_arithmetic_with_mixed_cast(
        &self,
        op: BinOp,
        left: &HirExpr,
        right: &HirExpr,
        left_expr: syn::Expr,
        right_expr: syn::Expr,
    ) -> Result<syn::Expr> {
        let left_is_float = self.ctx.is_expr_float_type(left);
        let right_is_float = self.ctx.is_expr_float_type(right);

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

    /// Convert functools.partial() to Rust closure
    /// partial(func, arg1, arg2, kwarg1=val1) → move |remaining...| func(arg1, arg2, remaining..., kwarg1=val1)
    fn convert_partial_call(
        &mut self,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<syn::Expr> {
        if args.is_empty() {
            bail!("functools.partial() requires at least one argument (the function)");
        }

        let target_func = &args[0];
        let bound_args = &args[1..];

        // Get the function name or expression
        let func_expr = target_func.to_rust_expr(self.ctx)?;

        // Generate bound argument expressions
        let bound_arg_exprs: Vec<syn::Expr> = bound_args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Generate keyword argument expressions
        let kwarg_exprs: Vec<syn::Expr> = kwargs
            .iter()
            .map(|(_, v)| v.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Generate closure parameter names for remaining arguments
        // Use generic names since we don't know the function signature
        let remaining_param_count =
            self.infer_partial_remaining_params(target_func, bound_args.len());
        let param_names: Vec<syn::Ident> = (0..remaining_param_count)
            .map(|i| syn::Ident::new(&format!("__arg{}", i), proc_macro2::Span::call_site()))
            .collect();

        // Build the closure
        if param_names.is_empty() {
            // No remaining parameters - return a nullary closure
            if kwarg_exprs.is_empty() {
                Ok(parse_quote! { move || #func_expr(#(#bound_arg_exprs),*) })
            } else {
                // With kwargs - append them
                Ok(parse_quote! { move || #func_expr(#(#bound_arg_exprs,)* #(#kwarg_exprs),*) })
            }
        } else if param_names.len() == 1 {
            let param = &param_names[0];
            if kwarg_exprs.is_empty() {
                if bound_arg_exprs.is_empty() {
                    Ok(parse_quote! { move |#param| #func_expr(#param) })
                } else {
                    Ok(parse_quote! { move |#param| #func_expr(#(#bound_arg_exprs,)* #param) })
                }
            } else {
                if bound_arg_exprs.is_empty() {
                    Ok(parse_quote! { move |#param| #func_expr(#param, #(#kwarg_exprs),*) })
                } else {
                    Ok(
                        parse_quote! { move |#param| #func_expr(#(#bound_arg_exprs,)* #param, #(#kwarg_exprs),*) },
                    )
                }
            }
        } else {
            // Multiple remaining parameters
            if kwarg_exprs.is_empty() {
                if bound_arg_exprs.is_empty() {
                    Ok(parse_quote! { move |#(#param_names),*| #func_expr(#(#param_names),*) })
                } else {
                    Ok(
                        parse_quote! { move |#(#param_names),*| #func_expr(#(#bound_arg_exprs,)* #(#param_names),*) },
                    )
                }
            } else {
                if bound_arg_exprs.is_empty() {
                    Ok(
                        parse_quote! { move |#(#param_names),*| #func_expr(#(#param_names,)* #(#kwarg_exprs),*) },
                    )
                } else {
                    Ok(
                        parse_quote! { move |#(#param_names),*| #func_expr(#(#bound_arg_exprs,)* #(#param_names,)* #(#kwarg_exprs),*) },
                    )
                }
            }
        }
    }

    /// Infer how many remaining parameters a partial function needs
    fn infer_partial_remaining_params(&self, target_func: &HirExpr, bound_count: usize) -> usize {
        // Try to determine the total parameter count from function info
        if let HirExpr::Var(func_name) = target_func {
            if let Some(param_names) = self.ctx.function_param_names.get(func_name) {
                let total_params = param_names.len();
                return total_params.saturating_sub(bound_count);
            }
        }
        // Default: assume one remaining parameter
        1
    }

    fn convert_unary(&mut self, op: &UnaryOp, operand: &HirExpr) -> Result<syn::Expr> {
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

    fn convert_call(
        &mut self,
        func: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<syn::Expr> {
        // Handle functools.partial - convert to closure
        // partial(func, arg1, arg2, ...) → |remaining_args...| func(arg1, arg2, ..., remaining_args...)
        if func == "partial" {
            return self.convert_partial_call(args, kwargs);
        }

        if func == "__os_path_join_starred" {
            if args.len() != 1 {
                bail!("__os_path_join_starred expects exactly 1 argument");
            }
            let parts = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! {
                if #parts.is_empty() {
                    String::new()
                } else {
                    #parts.join(std::path::MAIN_SEPARATOR_STR)
                }
            });
        }

        if func == "__print_starred" {
            if args.len() != 1 {
                bail!("__print_starred expects exactly 1 argument");
            }
            let items = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! {
                {
                    for item in #items {
                        print!("{} ", item);
                    }
                    println!();
                }
            });
        }

        // Python: zeros(n) → Rust: vec![0; n]
        // Python: ones(n) → Rust: vec![1; n]
        // Python: full(n, val) → Rust: vec![val; n]
        if func == "zeros" && args.len() == 1 {
            let size_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { vec![0; #size_expr as usize] });
        }

        if func == "ones" && args.len() == 1 {
            let size_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { vec![1; #size_expr as usize] });
        }

        if func == "full" && args.len() == 2 {
            let size_expr = args[0].to_rust_expr(self.ctx)?;
            let value_expr = args[1].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { vec![#value_expr; #size_expr as usize] });
        }

        // ArgumentParser pattern requires complex transformation:
        // - Accumulate add_argument() calls
        // - Generate #[derive(Parser)] struct
        // - Replace parse_args() with Args::parse()
        // For now, return unit to make code compile while transformation is implemented
        if func.contains("ArgumentParser") {
            // NOTE: Full argparse implementation requires generating Args struct with clap derives ()
            // For now, just return unit to allow compilation
            return Ok(parse_quote! { () });
        }

        // Handle classmethod cls(args) → Self::new(args)
        if func == "cls" && self.ctx.is_classmethod {
            let arg_exprs: Vec<syn::Expr> = args
                .iter()
                .map(|arg| arg.to_rust_expr(self.ctx))
                .collect::<Result<Vec<_>>>()?;
            return Ok(parse_quote! { Self::new(#(#arg_exprs),*) });
        }

        // Handle map() with lambda → convert to Rust iterator pattern
        if func == "map" && args.len() >= 2 {
            if let Some(result) = self.try_convert_map_with_zip(args)? {
                return Ok(result);
            }
        }

        if func == "filter" && args.len() == 2 {
            if let HirExpr::Lambda { params, body } = &args[0] {
                if params.len() != 1 {
                    bail!("filter() lambda must have exactly one parameter");
                }
                let iterable_expr = args[1].to_rust_expr(self.ctx)?;
                let param_ident = syn::Ident::new(&params[0], proc_macro2::Span::call_site());
                let body_expr = body.to_rust_expr(self.ctx)?;

                return Ok(parse_quote! {
                    #iterable_expr.into_iter().filter(|#param_ident| #body_expr)
                });
            }
        }

        // Handle sum(generator_exp) → generator_exp.sum::<T>()
        // Need turbofish type annotation to help Rust's type inference
        if func == "sum" && args.len() == 1 && matches!(args[0], HirExpr::GeneratorExp { .. }) {
            let gen_expr = args[0].to_rust_expr(self.ctx)?;

            // Infer the target type from return type context
            let target_type = self
                .ctx
                .current_return_type
                .as_ref()
                .and_then(|t| match t {
                    Type::Int => Some(quote! { i32 }),
                    Type::Float => Some(quote! { f64 }),
                    _ => None,
                })
                .unwrap_or_else(|| quote! { i32 });

            return Ok(parse_quote! { #gen_expr.sum::<#target_type>() });
        }

        // Handle max(generator_exp) → generator_exp.max()
        if func == "max" && args.len() == 1 && matches!(args[0], HirExpr::GeneratorExp { .. }) {
            let gen_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { #gen_expr.max() });
        }

        if func == "sorted" && args.len() == 1 {
            let iter_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! {
                {
                    let mut __sorted_result = #iter_expr.clone();
                    __sorted_result.sort();
                    __sorted_result
                }
            });
        }

        if func == "reversed" && args.len() == 1 {
            let iter_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! {
                {
                    let mut __reversed_result = #iter_expr.clone();
                    __reversed_result.reverse();
                    __reversed_result
                }
            });
        }

        // Rust byte slices (&[u8]) already provide memoryview functionality (zero-copy view)
        // Python's memoryview provides a buffer interface - Rust slices are already references
        if func == "memoryview" && args.len() == 1 {
            return args[0].to_rust_expr(self.ctx);
        }

        // Need turbofish type annotation to help Rust's type inference
        // Ranges in Rust (0..n) are already iterators - don't call .iter() on them
        if func == "sum" && args.len() == 1 {
            if let HirExpr::Call {
                func: range_func, ..
            } = &args[0]
            {
                if range_func == "range" {
                    let range_expr = args[0].to_rust_expr(self.ctx)?;

                    let target_type = self
                        .ctx
                        .current_return_type
                        .as_ref()
                        .and_then(|t| match t {
                            Type::Int => Some(quote! { i32 }),
                            Type::Float => Some(quote! { f64 }),
                            _ => None,
                        })
                        .unwrap_or_else(|| quote! { i32 });

                    // Wrap range in parentheses to fix precedence: (0..n).sum() not 0..n.sum()
                    return Ok(parse_quote! { (#range_expr).sum::<#target_type>() });
                }
            }

            // .values()/.keys() already return Vec, but we can optimize by using the iterator directly
            // This avoids .collect::<Vec<_>>().iter() pattern
            if let HirExpr::MethodCall {
                object,
                method,
                args: method_args,
                ..
            } = &args[0]
            {
                if (method == "values" || method == "keys") && method_args.is_empty() {
                    let object_expr = object.to_rust_expr(self.ctx)?;

                    // For d.values() where d: HashMap<K, i32>, sum should be .sum::<i32>()
                    // even if function returns f64 (the cast happens after sum)
                    let target_type = if method == "values" {
                        // Try to get value type from HashMap
                        if let HirExpr::Var(var_name) = object.as_ref() {
                            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                                match var_type {
                                    Type::Dict(_key_type, value_type) => {
                                        match value_type.as_ref() {
                                            Type::Int => Some(quote! { i32 }),
                                            Type::Float => Some(quote! { f64 }),
                                            _ => None,
                                        }
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        // For .keys(), always String → can't sum strings
                        // Fall back to default (should not happen for keys)
                        None
                    }
                    .unwrap_or_else(|| quote! { i32 });

                    // Use .values().cloned().sum() directly - skip the .collect()
                    let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                    return Ok(parse_quote! {
                        #object_expr.#method_ident().cloned().sum::<#target_type>()
                    });
                }
            }

            // Default: assume iterable that needs .iter()
            let iter_expr = args[0].to_rust_expr(self.ctx)?;

            // Infer the target type from the argument's type (element type of list)
            let target_type = self
                .infer_sum_element_type(&args[0])
                .unwrap_or_else(|| quote! { i32 });

            return Ok(parse_quote! { #iter_expr.iter().sum::<#target_type>() });
        }

        // Handle max() builtin
        if func == "max" {
            return crate::rust_gen::builtins::handle_max(args, self.ctx);
        }

        // Handle min() builtin
        if func == "min" {
            return crate::rust_gen::builtins::handle_min(args, self.ctx);
        }

        if func == "abs" {
            return crate::rust_gen::builtins::handle_abs(args, self.ctx);
        }

        // Generator expressions (e.g., any(n > 0 for n in numbers)) return iterators
        // Don't call .iter() on them - call .any() directly
        if func == "any" && args.len() == 1 && matches!(args[0], HirExpr::GeneratorExp { .. }) {
            let gen_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { #gen_expr.any(|x| x) });
        }

        if func == "any" && args.len() == 1 {
            let iter_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { #iter_expr.iter().any(|&x| x) });
        }

        // Generator expressions (e.g., all(n > 0 for n in numbers)) return iterators
        // Don't call .iter() on them - call .all() directly
        if func == "all" && args.len() == 1 && matches!(args[0], HirExpr::GeneratorExp { .. }) {
            let gen_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { #gen_expr.all(|x| x) });
        }

        if func == "all" && args.len() == 1 {
            let iter_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { #iter_expr.iter().all(|&x| x) });
        }

        // Handle round() builtin
        if func == "round" {
            return crate::rust_gen::builtins::handle_round(args, self.ctx);
        }

        // Handle pow() builtin
        if func == "pow" {
            return crate::rust_gen::builtins::handle_pow(args, self.ctx);
        }

        if func == "chr" && args.len() == 1 {
            let code_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { char::from_u32(#code_expr as u32).unwrap().to_string() });
        }

        if func == "ord" && args.len() == 1 {
            let char_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { #char_expr.chars().next().unwrap() as i32 });
        }

        if func == "bool" && args.len() == 1 {
            let value_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { #value_expr != 0 });
        }

        //
        // Decimal("123.45") → Decimal::from_str("123.45").unwrap()
        // Decimal(123) → Decimal::from(123)
        // Decimal(3.14) → Decimal::from_f64_retain(3.14).unwrap()
        if func == "Decimal" && args.len() == 1 {
            self.ctx.needs_rust_decimal = true;
            let arg = &args[0];

            // Determine the conversion based on argument type
            let result = match arg {
                HirExpr::Literal(Literal::String(_)) => {
                    let arg_expr = arg.to_rust_expr(self.ctx)?;
                    parse_quote! { rust_decimal::Decimal::from_str(&#arg_expr).unwrap() }
                }
                HirExpr::Literal(Literal::Int(_)) => {
                    let arg_expr = arg.to_rust_expr(self.ctx)?;
                    parse_quote! { rust_decimal::Decimal::from(#arg_expr) }
                }
                HirExpr::Literal(Literal::Float(_)) => {
                    let arg_expr = arg.to_rust_expr(self.ctx)?;
                    parse_quote! { rust_decimal::Decimal::from_f64_retain(#arg_expr).unwrap() }
                }
                _ => {
                    // Generic case: try from_str for variables
                    let arg_expr = arg.to_rust_expr(self.ctx)?;
                    parse_quote! { rust_decimal::Decimal::from_str(&(#arg_expr).to_string()).unwrap() }
                }
            };

            return Ok(result);
        }

        //
        // Fraction(numerator, denominator) → Ratio::new(num, denom)
        // Fraction("1/2") → Ratio::from_str("1/2") (simplified - needs parsing)
        // Fraction(3.14) → Ratio::approximate_float(3.14)
        if func == "Fraction" {
            self.ctx.needs_num_rational = true;

            if args.len() == 1 {
                let arg = &args[0];
                // Determine type and convert appropriately
                let result = match arg {
                    HirExpr::Literal(Literal::String(_)) => {
                        let arg_expr = arg.to_rust_expr(self.ctx)?;
                        // Parse "numerator/denominator" format
                        parse_quote! {
                            {
                                let s = #arg_expr;
                                let parts: Vec<&str> = s.split('/').collect();
                                if parts.len() == 2 {
                                    let num = parts[0].trim().parse::<i32>().unwrap();
                                    let denom = parts[1].trim().parse::<i32>().unwrap();
                                    num::rational::Ratio::new(num, denom)
                                } else {
                                    let num = s.parse::<i32>().unwrap();
                                    num::rational::Ratio::from_integer(num)
                                }
                            }
                        }
                    }
                    HirExpr::Literal(Literal::Int(_)) => {
                        let arg_expr = arg.to_rust_expr(self.ctx)?;
                        parse_quote! { num::rational::Ratio::from_integer(#arg_expr) }
                    }
                    HirExpr::Literal(Literal::Float(_)) => {
                        let arg_expr = arg.to_rust_expr(self.ctx)?;
                        parse_quote! { num::rational::Ratio::approximate_float(#arg_expr).unwrap() }
                    }
                    _ => {
                        let arg_expr = arg.to_rust_expr(self.ctx)?;
                        parse_quote! { num::rational::Ratio::approximate_float(#arg_expr as f64).unwrap() }
                    }
                };
                return Ok(result);
            } else if args.len() == 2 {
                // Fraction(numerator, denominator)
                let num_expr = args[0].to_rust_expr(self.ctx)?;
                let denom_expr = args[1].to_rust_expr(self.ctx)?;
                return Ok(parse_quote! { num::rational::Ratio::new(#num_expr, #denom_expr) });
            }
            bail!("Fraction() requires 1 or 2 arguments");
        }

        //
        // Path("/foo/bar") → PathBuf::from("/foo/bar")
        // Path(p) / "subdir" → p.join("subdir")
        if func == "Path" && args.len() == 1 {
            let path_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! { std::path::PathBuf::from(#path_expr) });
        }

        // datetime(year, month, day, [hour], [minute], [second]) → NaiveDateTime
        if func == "datetime" {
            self.ctx.needs_chrono = true;

            if args.len() >= 3 {
                let year = args[0].to_rust_expr(self.ctx)?;
                let month = args[1].to_rust_expr(self.ctx)?;
                let day = args[2].to_rust_expr(self.ctx)?;

                let hour = if args.len() > 3 {
                    args[3].to_rust_expr(self.ctx)?
                } else {
                    parse_quote! { 0 }
                };
                let minute = if args.len() > 4 {
                    args[4].to_rust_expr(self.ctx)?
                } else {
                    parse_quote! { 0 }
                };
                let second = if args.len() > 5 {
                    args[5].to_rust_expr(self.ctx)?
                } else {
                    parse_quote! { 0 }
                };

                return Ok(parse_quote! {
                    chrono::NaiveDate::from_ymd_opt(#year as i32, #month as u32, #day as u32)
                        .unwrap()
                        .and_hms_opt(#hour as u32, #minute as u32, #second as u32)
                        .unwrap()
                });
            }
            bail!("datetime() requires at least 3 arguments (year, month, day)");
        }

        // date(year, month, day) → NaiveDate::from_ymd_opt(y, m, d).unwrap()
        if func == "date" && args.len() == 3 {
            self.ctx.needs_chrono = true;
            let year = args[0].to_rust_expr(self.ctx)?;
            let month = args[1].to_rust_expr(self.ctx)?;
            let day = args[2].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! {
                chrono::NaiveDate::from_ymd_opt(#year as i32, #month as u32, #day as u32).unwrap()
            });
        }

        // time(hour, minute, second) → NaiveTime::from_hms_opt(h, m, s).unwrap()
        if func == "time" && args.len() >= 2 {
            self.ctx.needs_chrono = true;
            let hour = args[0].to_rust_expr(self.ctx)?;
            let minute = args[1].to_rust_expr(self.ctx)?;

            if args.len() == 2 {
                return Ok(parse_quote! {
                    chrono::NaiveTime::from_hms_opt(#hour as u32, #minute as u32, 0).unwrap()
                });
            } else if args.len() >= 3 {
                let second = args[2].to_rust_expr(self.ctx)?;
                return Ok(parse_quote! {
                    chrono::NaiveTime::from_hms_opt(#hour as u32, #minute as u32, #second as u32).unwrap()
                });
            }
        }

        // timedelta(days=..., seconds=...) → Duration::days(...) + Duration::seconds(...)
        // Note: Python timedelta uses keyword args, but we'll support positional for now
        if func == "timedelta" {
            self.ctx.needs_chrono = true;

            if args.is_empty() {
                // timedelta() with no args → zero duration
                return Ok(parse_quote! { chrono::Duration::zero() });
            } else if args.len() == 1 {
                // Assume days parameter
                let days = args[0].to_rust_expr(self.ctx)?;
                return Ok(parse_quote! { chrono::Duration::days(#days as i64) });
            } else if args.len() == 2 {
                // Assume days, seconds parameters
                let days = args[0].to_rust_expr(self.ctx)?;
                let seconds = args[1].to_rust_expr(self.ctx)?;
                return Ok(parse_quote! {
                    chrono::Duration::days(#days as i64) + chrono::Duration::seconds(#seconds as i64)
                });
            }
        }

        // Handle enumerate(items) → items.iter().enumerate().map(|(i, x)| (i as i32, x.clone()))
        // Use .iter() to avoid consuming the collection (allows references in filter closures)
        // Cast index to i32 to match Python's int type
        // Clone the item to get owned value from reference
        if func == "enumerate" && args.len() == 1 {
            let items_expr = args[0].to_rust_expr(self.ctx)?;
            return Ok(
                parse_quote! { #items_expr.iter().enumerate().map(|(i, x)| (i as i32, x.clone())) },
            );
        }

        // Handle zip(a, b, ...) → a.into_iter().zip(b.into_iter())...
        // When zip() receives function parameters of type Vec<T>, we need to consume them
        // to yield owned values, not references. This is critical for dict(zip(...)) patterns.
        if func == "zip" && args.len() >= 2 {
            let arg_exprs: Vec<syn::Expr> = args
                .iter()
                .map(|arg| arg.to_rust_expr(self.ctx))
                .collect::<Result<Vec<_>>>()?;

            // Determine if we should use .into_iter() or .iter()
            // Use .into_iter() if all arguments are owned collections (Vec, not slices)
            let use_into_iter = args.iter().all(|arg| self.is_owned_collection(arg));

            // Start with first.into_iter() or first.iter()
            let first = &arg_exprs[0];
            let mut chain: syn::Expr = if use_into_iter {
                parse_quote! { #first.into_iter() }
            } else {
                parse_quote! { #first.iter() }
            };

            // Chain .zip() for each subsequent argument
            for arg in &arg_exprs[1..] {
                chain = if use_into_iter {
                    parse_quote! { #chain.zip(#arg.into_iter()) }
                } else {
                    parse_quote! { #chain.zip(#arg.iter()) }
                };
            }

            return Ok(chain);
        }

        // In statically-typed Rust, type system guarantees make runtime checks unnecessary
        // isinstance(x, T) where x: T is always true at compile-time
        if func == "isinstance" && args.len() == 2 {
            // Return literal true since Rust's type system guarantees correctness
            return Ok(parse_quote! { true });
        }

        let is_user_class = self.ctx.class_names.contains(func);

        // Set in_primitive_cast flag for int/float/bool casts to prevent adding
        // references to if-expression branches inside cast arguments
        let is_primitive_cast = matches!(func, "int" | "float" | "bool");
        let was_in_primitive_cast = self.ctx.in_primitive_cast;
        if is_primitive_cast {
            self.ctx.in_primitive_cast = true;
        }

        // This fixes "expected String, found &str" errors when calling constructors
        let arg_exprs: Vec<syn::Expr> = if is_user_class {
            args.iter()
                .map(|arg| {
                    let expr = arg.to_rust_expr(self.ctx)?;
                    // Wrap string literals with .to_string()
                    if matches!(arg, HirExpr::Literal(Literal::String(_))) {
                        Ok(parse_quote! { #expr.to_string() })
                    } else {
                        Ok(expr)
                    }
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            args.iter()
                .map(|arg| arg.to_rust_expr(self.ctx))
                .collect::<Result<Vec<_>>>()?
        };

        // Restore the previous in_primitive_cast state
        self.ctx.in_primitive_cast = was_in_primitive_cast;

        // based on the function's parameter order from function_param_names.
        // Python: format_message(country="USA", name="Alice", city="New York", age=30)
        // If func signature is (name, age, city, country), reorder to:
        // format_message("Alice", 30, "New York", "USA")
        let (all_args, all_hir_args) = if !kwargs.is_empty() {
            // Look up function parameter names for proper reordering
            // Clone to avoid borrowing ctx while we call to_rust_expr
            let maybe_param_names = self.ctx.function_param_names.get(func).cloned();
            let can_reorder = maybe_param_names
                .as_ref()
                .map(|p| p.len() >= args.len() + kwargs.len())
                .unwrap_or(false);
            if can_reorder {
                let param_names = maybe_param_names.unwrap();
                // Build argument list by matching kwargs to parameter positions
                let mut reordered_args: Vec<syn::Expr> = Vec::with_capacity(param_names.len());
                let mut reordered_hir_args: Vec<HirExpr> = Vec::with_capacity(param_names.len());

                // Create a map from kwarg name to its value for quick lookup
                let kwarg_map: std::collections::HashMap<&str, &HirExpr> = kwargs
                    .iter()
                    .map(|(name, value)| (name.as_str(), value))
                    .collect();

                for (idx, param_name) in param_names.iter().enumerate() {
                    if idx < args.len() {
                        // This position is filled by a positional argument
                        reordered_hir_args.push(args[idx].clone());
                        let expr = args[idx].to_rust_expr(self.ctx)?;
                        if is_user_class
                            && matches!(&args[idx], HirExpr::Literal(Literal::String(_)))
                        {
                            reordered_args.push(parse_quote! { #expr.to_string() });
                        } else {
                            reordered_args.push(expr);
                        }
                    } else if let Some(value) = kwarg_map.get(param_name.as_str()) {
                        // This position is filled by a keyword argument
                        reordered_hir_args.push((*value).clone());
                        let expr = value.to_rust_expr(self.ctx)?;
                        if is_user_class && matches!(value, HirExpr::Literal(Literal::String(_))) {
                            reordered_args.push(parse_quote! { #expr.to_string() });
                        } else {
                            reordered_args.push(expr);
                        }
                    }
                    // If neither positional nor kwarg fills this position,
                    // the function likely has a default value (skip it)
                }
                (reordered_args, reordered_hir_args)
            } else {
                // Fall back to appending kwargs in order if function signature not found
                let kwarg_exprs: Vec<syn::Expr> = if is_user_class {
                    kwargs
                        .iter()
                        .map(|(_name, value)| {
                            let expr = value.to_rust_expr(self.ctx)?;
                            if matches!(value, HirExpr::Literal(Literal::String(_))) {
                                Ok(parse_quote! { #expr.to_string() })
                            } else {
                                Ok(expr)
                            }
                        })
                        .collect::<Result<Vec<_>>>()?
                } else {
                    kwargs
                        .iter()
                        .map(|(_name, value)| value.to_rust_expr(self.ctx))
                        .collect::<Result<Vec<_>>>()?
                };
                let mut all_args = arg_exprs.clone();
                all_args.extend(kwarg_exprs);
                let mut all_hir_args: Vec<HirExpr> = args.to_vec();
                for (_name, value) in kwargs {
                    all_hir_args.push(value.clone());
                }
                (all_args, all_hir_args)
            }
        } else {
            // No kwargs - just use positional args
            (arg_exprs.clone(), args.to_vec())
        };

        match func {
            // Python built-in type conversions → Rust casting
            "int" => self.convert_int_cast(&all_hir_args, &arg_exprs),
            "float" => self.convert_float_cast(&all_hir_args, &arg_exprs),
            "str" => self.convert_str_conversion(&all_hir_args, &arg_exprs),
            "bool" => self.convert_bool_cast(&arg_exprs),
            // Other built-in functions
            "len" => self.convert_len_call(&all_hir_args, &arg_exprs),
            "range" => self.convert_range_call(&arg_exprs),
            "zeros" | "ones" | "full" => {
                self.convert_array_init_call(func, &all_hir_args, &arg_exprs)
            }
            "set" => self.convert_set_constructor(&arg_exprs),
            "frozenset" => self.convert_frozenset_constructor(&arg_exprs),
            "Counter" if !is_user_class => self.convert_counter_builtin(&arg_exprs),
            "dict" if !is_user_class => self.convert_dict_builtin(&arg_exprs),
            "deque" if !is_user_class => self.convert_deque_builtin(&arg_exprs),
            "list" if !is_user_class => self.convert_list_builtin(&arg_exprs),
            //
            "all" => self.convert_all_builtin(&arg_exprs),
            "any" => self.convert_any_builtin(&arg_exprs),
            "divmod" => self.convert_divmod_builtin(&arg_exprs),
            "enumerate" => self.convert_enumerate_builtin(&arg_exprs),
            "zip" => self.convert_zip_builtin(&arg_exprs),
            "reversed" => self.convert_reversed_builtin(&arg_exprs),
            "sorted" => self.convert_sorted_builtin(&arg_exprs),
            "filter" => self.convert_filter_builtin(&all_hir_args, &arg_exprs),
            "sum" => self.convert_sum_builtin(&arg_exprs),
            //
            "round" => self.convert_round_builtin(&arg_exprs),
            "abs" => self.convert_abs_builtin(&arg_exprs),
            "min" => self.convert_min_builtin(&arg_exprs),
            "max" => self.convert_max_builtin(&arg_exprs),
            "pow" => self.convert_pow_builtin(&arg_exprs),
            "hex" => self.convert_hex_builtin(&arg_exprs),
            "bin" => self.convert_bin_builtin(&arg_exprs),
            "oct" => self.convert_oct_builtin(&arg_exprs),
            "chr" => self.convert_chr_builtin(&arg_exprs),
            "ord" => self.convert_ord_builtin(&arg_exprs),
            "hash" => self.convert_hash_builtin(&arg_exprs),
            "repr" => self.convert_repr_builtin(&arg_exprs),
            "open" => self.convert_open_builtin(&all_hir_args, &arg_exprs),
            //
            "next" => self.convert_next_builtin(&all_hir_args, &arg_exprs),
            "getattr" => self.convert_getattr_builtin(&all_hir_args),
            "setattr" => self.convert_setattr_builtin(&all_hir_args),
            "iter" => self.convert_iter_builtin(&arg_exprs),
            "type" => self.convert_type_builtin(&arg_exprs),
            _ => self.convert_generic_call(func, &all_hir_args, &all_args),
        }
    }

    fn try_convert_map_with_zip(&mut self, args: &[HirExpr]) -> Result<Option<syn::Expr>> {
        // Check if first argument is a lambda
        if let HirExpr::Lambda { params, body } = &args[0] {
            let num_iterables = args.len() - 1;

            // Check if lambda has matching number of parameters
            if params.len() != num_iterables {
                bail!(
                    "Lambda has {} parameters but map() called with {} iterables",
                    params.len(),
                    num_iterables
                );
            }

            // Convert the iterables
            let mut iterable_exprs: Vec<syn::Expr> = Vec::new();
            for iterable in &args[1..] {
                iterable_exprs.push(iterable.to_rust_expr(self.ctx)?);
            }

            // Create lambda parameter pattern
            let param_idents: Vec<syn::Ident> = params
                .iter()
                .map(|p| syn::Ident::new(p, proc_macro2::Span::call_site()))
                .collect();

            // Convert lambda body
            let body_expr = body.to_rust_expr(self.ctx)?;

            // Handle based on number of iterables
            if num_iterables == 1 {
                // Single iterable: iterable.iter().map(|x| ...).collect()
                let iter_expr = &iterable_exprs[0];
                let param = &param_idents[0];
                Ok(Some(parse_quote! {
                    #iter_expr.iter().map(|#param| #body_expr).collect::<Vec<_>>()
                }))
            } else {
                // Multiple iterables: use zip pattern
                // Build the zip chain
                let first_iter = &iterable_exprs[0];
                let mut zip_expr: syn::Expr = parse_quote! { #first_iter.iter() };

                for iter_expr in &iterable_exprs[1..] {
                    zip_expr = parse_quote! { #zip_expr.zip(#iter_expr.iter()) };
                }

                // Build the tuple pattern based on number of parameters
                let tuple_pat: syn::Pat = if param_idents.len() == 2 {
                    let p0 = &param_idents[0];
                    let p1 = &param_idents[1];
                    parse_quote! { (#p0, #p1) }
                } else if param_idents.len() == 3 {
                    // For 3 parameters, zip creates ((a, b), c)
                    let p0 = &param_idents[0];
                    let p1 = &param_idents[1];
                    let p2 = &param_idents[2];
                    parse_quote! { ((#p0, #p1), #p2) }
                } else {
                    // For 4+ parameters, continue the nested pattern
                    bail!("map() with more than 3 iterables is not yet supported");
                };

                // Generate the final expression
                Ok(Some(parse_quote! {
                    #zip_expr.map(|#tuple_pat| #body_expr).collect::<Vec<_>>()
                }))
            }
        } else {
            // Not a lambda, fall through to normal handling
            Ok(None)
        }
    }

    fn convert_len_call(&mut self, hir_args: &[HirExpr], _args: &[syn::Expr]) -> Result<syn::Expr> {
        if hir_args.len() != 1 {
            bail!("len() requires exactly one argument");
        }

        // Generate expression without clone - .len() only borrows and doesn't need ownership
        let arg = self.convert_expr_without_clone(&hir_args[0])?;

        // Check if argument is Optional - if so, unwrap before calling .len()
        let is_optional = self.expr_is_optional(&hir_args[0]);

        // Python's len() returns int (maps to i32)
        // Rust's .len() returns usize, so we cast to i32
        if is_optional {
            Ok(parse_quote! { #arg.as_ref().unwrap().len() as i32 })
        } else {
            Ok(parse_quote! { #arg.len() as i32 })
        }
    }

    /// Returns true if the expression needs wrapping in parentheses for a cast or method call.
    fn needs_parens_for_cast(expr: &syn::Expr) -> bool {
        !matches!(expr, syn::Expr::Path(_) | syn::Expr::Lit(_) | syn::Expr::Field(_) | syn::Expr::MethodCall(_) | syn::Expr::Call(_) | syn::Expr::Paren(_))
    }

    fn convert_int_cast(&self, hir_args: &[HirExpr], arg_exprs: &[syn::Expr]) -> Result<syn::Expr> {
        if arg_exprs.is_empty() || arg_exprs.len() > 2 {
            bail!("int() requires 1-2 arguments");
        }
        let arg = &arg_exprs[0];

        // Python int() serves four purposes:
        // 1. Parse strings to integers (requires .parse())
        // 2. Convert floats to integers (truncation via as i32)
        // 3. Convert bools to integers (False→0, True→1 via as i32)
        // 4. Ensure integer type for indexing (via as i32)

        // String variables need .parse().unwrap() not 'as i32' cast

        // Check if expression is a String-typed method call (e.g., Vec<String>.get())

        // Strategy:
        // - For String variables/params → .parse().unwrap()
        // - For String literals → .parse().unwrap()
        // - For String-typed method calls → .parse().unwrap()
        // - For known bool expressions → as i32 cast
        // - For integer literals → no cast needed
        // - For other variables → as i32 cast conservatively
        if !hir_args.is_empty() {
            match &hir_args[0] {
                // Integer literals don't need casting
                HirExpr::Literal(Literal::Int(_)) => return Ok(arg.clone()),

                HirExpr::Literal(Literal::String(_)) => {
                    return Ok(parse_quote! { #arg.parse::<i32>().unwrap() });
                }

                HirExpr::Var(var_name) => {
                    // Check the variable's actual type first (type-based decision)
                    if let Some(var_type) = self.ctx.var_types.get(var_name) {
                        match var_type {
                            // String types require parsing
                            Type::String => {
                                return Ok(parse_quote! { #arg.parse::<i32>().unwrap() });
                            }
                            // Numeric types use simple cast
                            Type::Int | Type::Float | Type::Bool => {
                                if Self::needs_parens_for_cast(arg) {
                                    return Ok(parse_quote! { (#arg) as i32 });
                                }
                                return Ok(parse_quote! { #arg as i32 });
                            }
                            // For other known types, use cast conservatively
                            _ => {
                                if Self::needs_parens_for_cast(arg) {
                                    return Ok(parse_quote! { (#arg) as i32 });
                                }
                                return Ok(parse_quote! { #arg as i32 });
                            }
                        }
                    }

                    // Type is unknown - fall back to name-based heuristics
                    // Heuristic: variable names ending in _str, _string, or common string names
                    let name = var_name.as_str();
                    let looks_like_string = name.ends_with("_str")
                        || name.ends_with("_string")
                        || name == "s"
                        || name == "string"
                        || name == "text"
                        || name == "word"
                        || name == "line"
                        || name == "value_str"  // Explicit case for string values
                        || name.starts_with("str_")
                        || name.starts_with("string_");

                    if looks_like_string {
                        // String → int requires parsing, not casting
                        return Ok(parse_quote! { #arg.parse::<i32>().unwrap() });
                    }
                    // Default: use as i32 cast for other types
                    if Self::needs_parens_for_cast(arg) {
                        return Ok(parse_quote! { (#arg) as i32 });
                    }
                    return Ok(parse_quote! { #arg as i32 });
                }

                // E.g., Vec<String>.get() or str methods
                HirExpr::MethodCall {
                    object,
                    method,
                    args: method_args,
                    ..
                } => {
                    // Check if this is .get() on a Vec<String> or similar
                    if self.is_string_method_call(object, method, method_args) {
                        return Ok(parse_quote! { #arg.parse::<i32>().unwrap() });
                    }
                    // Otherwise, use default cast
                    if Self::needs_parens_for_cast(arg) {
                        return Ok(parse_quote! { (#arg) as i32 });
                    }
                    return Ok(parse_quote! { #arg as i32 });
                }

                // Check if it's a known bool expression
                expr => {
                    if let Some(is_bool) = self.is_bool_expr(expr) {
                        if is_bool {
                            if Self::needs_parens_for_cast(arg) {
                                return Ok(parse_quote! { (#arg) as i32 });
                            }
                            return Ok(parse_quote! { #arg as i32 });
                        }
                    }
                    // For other complex expressions, apply cast conservatively
                    if Self::needs_parens_for_cast(arg) {
                        return Ok(parse_quote! { (#arg) as i32 });
                    }
                    return Ok(parse_quote! { #arg as i32 });
                }
            }
        }

        // Default: cast for safety
        if Self::needs_parens_for_cast(arg) {
            return Ok(parse_quote! { (#arg) as i32 });
        }
        Ok(parse_quote! { #arg as i32 })
    }

    fn convert_float_cast(&self, hir_args: &[HirExpr], args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("float() requires exactly one argument");
        }
        let arg = &args[0];

        // If the expression is already a float type, don't cast - just return it.
        // This avoids invalid casts like `(&mut f64) as f64`.
        if !hir_args.is_empty() && self.ctx.is_expr_float_type(&hir_args[0]) {
            return Ok(arg.clone());
        }

        if Self::needs_parens_for_cast(arg) {
            return Ok(parse_quote! { (#arg) as f64 });
        }
        Ok(parse_quote! { #arg as f64 })
    }

    fn convert_str_conversion(
        &self,
        hir_args: &[HirExpr],
        args: &[syn::Expr],
    ) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("str() requires exactly one argument");
        }
        let arg = &args[0];

        // Check if argument is Optional - if so, unwrap before calling .to_string()
        let is_optional = !hir_args.is_empty() && self.expr_is_optional(&hir_args[0]);

        if is_optional {
            if Self::needs_parens_for_cast(arg) {
                return Ok(parse_quote! { (#arg).as_ref().unwrap().to_string() });
            }
            Ok(parse_quote! { #arg.as_ref().unwrap().to_string() })
        } else if Self::needs_parens_for_cast(arg) {
            Ok(parse_quote! { (#arg).to_string() })
        } else {
            Ok(parse_quote! { #arg.to_string() })
        }
    }

    fn convert_bool_cast(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("bool() requires exactly one argument");
        }
        let arg = &args[0];
        // In Python, bool(x) checks truthiness
        // In Rust, we cast to bool or use appropriate conversion
        Ok(parse_quote! { (#arg) as bool })
    }

    fn convert_range_call(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        match args.len() {
            1 => {
                let end = &args[0];
                Ok(parse_quote! { 0..#end })
            }
            2 => {
                let start = &args[0];
                let end = &args[1];
                Ok(parse_quote! { #start..#end })
            }
            3 => self.convert_range_with_step(&args[0], &args[1], &args[2]),
            _ => bail!("Invalid number of arguments for range()"),
        }
    }

    fn convert_range_with_step(
        &self,
        start: &syn::Expr,
        end: &syn::Expr,
        step: &syn::Expr,
    ) -> Result<syn::Expr> {
        // Check if step is negative by looking at the expression
        let is_negative_step =
            matches!(step, syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)));

        if is_negative_step {
            self.convert_range_negative_step(start, end, step)
        } else {
            self.convert_range_positive_step(start, end, step)
        }
    }

    fn convert_range_negative_step(
        &self,
        start: &syn::Expr,
        end: &syn::Expr,
        step: &syn::Expr,
    ) -> Result<syn::Expr> {
        // For negative steps, we need to reverse the range
        // Python: range(10, 0, -1) → Rust: (0..10).rev()
        Ok(parse_quote! {
            {
                let step = (#step as i32).abs() as usize;
                if step == 0 {
                    panic!("range() arg 3 must not be zero");
                }
                // This avoids if/else branches returning different types:
                // - Rev<Range<i32>> vs StepBy<Rev<Range<i32>>>
                // Using step.max(1) ensures step is never 0 (already checked above)
                (#end..#start).rev().step_by(step.max(1))
            }
        })
    }

    fn convert_range_positive_step(
        &self,
        start: &syn::Expr,
        end: &syn::Expr,
        step: &syn::Expr,
    ) -> Result<syn::Expr> {
        // Positive step - check for zero
        Ok(parse_quote! {
            {
                let step = #step as usize;
                if step == 0 {
                    panic!("range() arg 3 must not be zero");
                }
                (#start..#end).step_by(step)
            }
        })
    }

    fn convert_array_init_call(
        &mut self,
        func: &str,
        args: &[HirExpr],
        _arg_exprs: &[syn::Expr],
    ) -> Result<syn::Expr> {
        // Handle zeros(n), ones(n), full(n, value) patterns
        if args.is_empty() {
            bail!("{} requires at least one argument", func);
        }

        // Extract size from first argument if it's a literal
        if let HirExpr::Literal(Literal::Int(size)) = &args[0] {
            if *size > 0 && *size <= 32 {
                self.convert_array_small_literal(func, args, *size)
            } else {
                self.convert_array_large_literal(func, args)
            }
        } else {
            self.convert_array_dynamic_size(func, args)
        }
    }

    fn convert_array_small_literal(
        &mut self,
        func: &str,
        args: &[HirExpr],
        size: i64,
    ) -> Result<syn::Expr> {
        let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
        match func {
            "zeros" => Ok(parse_quote! { [0; #size_lit] }),
            "ones" => Ok(parse_quote! { [1; #size_lit] }),
            "full" => {
                if args.len() >= 2 {
                    let value = args[1].to_rust_expr(self.ctx)?;
                    Ok(parse_quote! { [#value; #size_lit] })
                } else {
                    bail!("full() requires a value argument");
                }
            }
            _ => unreachable!(),
        }
    }

    fn convert_array_large_literal(&mut self, func: &str, args: &[HirExpr]) -> Result<syn::Expr> {
        let size_expr = args[0].to_rust_expr(self.ctx)?;
        match func {
            "zeros" => Ok(parse_quote! { vec![0; #size_expr as usize] }),
            "ones" => Ok(parse_quote! { vec![1; #size_expr as usize] }),
            "full" => {
                if args.len() >= 2 {
                    let value = args[1].to_rust_expr(self.ctx)?;
                    Ok(parse_quote! { vec![#value; #size_expr as usize] })
                } else {
                    bail!("full() requires a value argument");
                }
            }
            _ => unreachable!(),
        }
    }

    fn convert_array_dynamic_size(&mut self, func: &str, args: &[HirExpr]) -> Result<syn::Expr> {
        let size_expr = args[0].to_rust_expr(self.ctx)?;
        match func {
            "zeros" => Ok(parse_quote! { vec![0; #size_expr as usize] }),
            "ones" => Ok(parse_quote! { vec![1; #size_expr as usize] }),
            "full" => {
                if args.len() >= 2 {
                    let value = args[1].to_rust_expr(self.ctx)?;
                    Ok(parse_quote! { vec![#value; #size_expr as usize] })
                } else {
                    bail!("full() requires a value argument");
                }
            }
            _ => unreachable!(),
        }
    }

    fn convert_set_constructor(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.needs_hashset = true;
        if args.is_empty() {
            // Empty set: set()
            // when the variable is unused or type can't be inferred from context
            Ok(parse_quote! { HashSet::<i32>::new() })
        } else if args.len() == 1 {
            // Set from iterable: set([1, 2, 3])
            let arg = &args[0];
            Ok(parse_quote! {
                #arg.into_iter().collect::<HashSet<_>>()
            })
        } else {
            bail!("set() takes at most 1 argument ({} given)", args.len())
        }
    }

    fn convert_frozenset_constructor(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.needs_hashset = true;
        if args.is_empty() {
            // Empty frozenset: frozenset()
            // In Rust, we can use Arc<HashSet> to make it immutable
            Ok(parse_quote! { std::sync::Arc::new(HashSet::<i32>::new()) })
        } else if args.len() == 1 {
            // Frozenset from iterable: frozenset([1, 2, 3])
            let arg = &args[0];
            Ok(parse_quote! {
                std::sync::Arc::new(#arg.into_iter().collect::<HashSet<_>>())
            })
        } else {
            bail!(
                "frozenset() takes at most 1 argument ({} given)",
                args.len()
            )
        }
    }

    // ========================================================================
    // ========================================================================

    fn convert_counter_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.needs_hashmap = true;
        if args.is_empty() {
            // Counter() with no args → empty HashMap
            Ok(parse_quote! { HashMap::new() })
        } else if args.len() == 1 {
            // Counter(iterable) → count elements using fold
            let arg = &args[0];
            Ok(parse_quote! {
                #arg.into_iter().fold(HashMap::new(), |mut acc, item| {
                    *acc.entry(item).or_insert(0) += 1;
                    acc
                })
            })
        } else {
            bail!("Counter() takes at most 1 argument ({} given)", args.len())
        }
    }

    fn convert_dict_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.needs_hashmap = true;
        if args.is_empty() {
            // dict() with no args → empty HashMap
            Ok(parse_quote! { HashMap::new() })
        } else if args.len() == 1 {
            // dict(mapping) → convert to HashMap
            let arg = &args[0];
            Ok(parse_quote! {
                #arg.into_iter().collect::<HashMap<_, _>>()
            })
        } else {
            bail!("dict() takes at most 1 argument ({} given)", args.len())
        }
    }

    fn convert_deque_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.needs_vecdeque = true;
        if args.is_empty() {
            // deque() with no args → empty VecDeque
            Ok(parse_quote! { VecDeque::new() })
        } else if args.len() == 1 {
            // deque(iterable) → VecDeque::from()
            let arg = &args[0];
            Ok(parse_quote! {
                VecDeque::from(#arg)
            })
        } else {
            bail!("deque() takes at most 1 argument ({} given)", args.len())
        }
    }

    fn convert_list_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() {
            // list() with no args → empty Vec
            Ok(parse_quote! { Vec::new() })
        } else if args.len() == 1 {
            let arg = &args[0];

            // map(lambda...) already includes .collect(), don't add another
            if self.already_collected(arg) {
                Ok(arg.clone())
            } else if self.is_range_expr(arg) {
                Ok(parse_quote! {
                    (#arg).collect::<Vec<_>>()
                })
            } else if self.is_iterator_expr(arg) {
                // Don't add redundant .into_iter()
                Ok(parse_quote! {
                    #arg.collect::<Vec<_>>()
                })
            } else if self.is_csv_reader_var(arg) {
                // list(reader) → reader.deserialize::<HashMap<String, String>>().collect()
                self.ctx.needs_csv = true;
                Ok(parse_quote! {
                    #arg.deserialize::<HashMap<String, String>>().collect::<Vec<_>>()
                })
            } else {
                // Regular iterable → collect to Vec
                Ok(parse_quote! {
                    #arg.into_iter().collect::<Vec<_>>()
                })
            }
        } else {
            bail!("list() takes at most 1 argument ({} given)", args.len())
        }
    }

    //

    fn convert_all_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("all() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter().all(|x| x) })
    }

    fn convert_any_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("any() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter().any(|x| x) })
    }

    fn convert_divmod_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 2 {
            bail!("divmod() requires exactly 2 arguments");
        }
        let a = &args[0];
        let b = &args[1];
        Ok(parse_quote! { (#a / #b, #a % #b) })
    }

    fn convert_enumerate_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() || args.len() > 2 {
            bail!("enumerate() requires 1 or 2 arguments");
        }
        let iterable = &args[0];
        if args.len() == 2 {
            let start = &args[1];
            Ok(
                parse_quote! { #iterable.into_iter().enumerate().map(|(i, x)| ((i + #start as usize) as i32, x)) },
            )
        } else {
            Ok(parse_quote! { #iterable.into_iter().enumerate().map(|(i, x)| (i as i32, x)) })
        }
    }

    fn convert_zip_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() < 2 {
            bail!("zip() requires at least 2 arguments");
        }
        let first = &args[0];
        let second = &args[1];
        if args.len() == 2 {
            Ok(parse_quote! { #first.into_iter().zip(#second.into_iter()) })
        } else {
            // For 3+ iterables, chain zip calls
            let mut zip_expr: syn::Expr =
                parse_quote! { #first.into_iter().zip(#second.into_iter()) };
            for iter in &args[2..] {
                zip_expr = parse_quote! { #zip_expr.zip(#iter.into_iter()) };
            }
            Ok(zip_expr)
        }
    }

    fn convert_reversed_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("reversed() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter().rev() })
    }

    fn convert_sorted_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() || args.len() > 2 {
            bail!("sorted() requires 1 or 2 arguments");
        }
        let iterable = &args[0];
        // Simplified: ignore key/reverse parameters for now
        Ok(parse_quote! {
            {
                let mut sorted_vec = #iterable.into_iter().collect::<Vec<_>>();
                sorted_vec.sort();
                sorted_vec
            }
        })
    }

    fn convert_filter_builtin(
        &mut self,
        hir_args: &[HirExpr],
        args: &[syn::Expr],
    ) -> Result<syn::Expr> {
        if args.len() != 2 {
            bail!("filter() requires exactly 2 arguments");
        }
        // Check if first arg is lambda
        if let HirExpr::Lambda { params, body } = &hir_args[0] {
            if params.len() != 1 {
                bail!("filter() lambda must have exactly 1 parameter");
            }
            let param_ident = syn::Ident::new(&params[0], proc_macro2::Span::call_site());
            let body_expr = body.to_rust_expr(self.ctx)?;
            let iterable = &args[1];
            Ok(parse_quote! {
                #iterable.into_iter().filter(|#param_ident| #body_expr)
            })
        } else {
            let predicate = &args[0];
            let iterable = &args[1];
            Ok(parse_quote! {
                #iterable.into_iter().filter(#predicate)
            })
        }
    }

    fn convert_sum_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() || args.len() > 2 {
            bail!("sum() requires 1 or 2 arguments");
        }
        let iterable = &args[0];
        if args.len() == 2 {
            let start = &args[1];
            Ok(parse_quote! { #iterable.into_iter().fold(#start, |acc, x| acc + x) })
        } else {
            Ok(parse_quote! { #iterable.into_iter().sum() })
        }
    }

    //

    fn convert_round_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() || args.len() > 2 {
            bail!("round() requires 1 or 2 arguments");
        }
        let value = &args[0];
        // Simplified: ignore ndigits parameter
        Ok(parse_quote! { (#value as f64).round() as i32 })
    }

    fn convert_abs_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("abs() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { (#value).abs() })
    }

    fn convert_min_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() {
            bail!("min() requires at least 1 argument");
        }
        if args.len() == 1 {
            // min(iterable)
            let iterable = &args[0];
            Ok(parse_quote! { #iterable.into_iter().min().unwrap() })
        } else {
            // min(a, b, c, ...)
            let first = &args[0];
            let mut min_expr = parse_quote! { #first };
            for arg in &args[1..] {
                min_expr = parse_quote! { #min_expr.min(#arg) };
            }
            Ok(min_expr)
        }
    }

    fn convert_max_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() {
            bail!("max() requires at least 1 argument");
        }
        if args.len() == 1 {
            // max(iterable)
            let iterable = &args[0];
            Ok(parse_quote! { #iterable.into_iter().max().unwrap() })
        } else {
            // max(a, b, c, ...)
            let first = &args[0];
            let mut max_expr = parse_quote! { #first };
            for arg in &args[1..] {
                max_expr = parse_quote! { #max_expr.max(#arg) };
            }
            Ok(max_expr)
        }
    }

    fn convert_pow_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() < 2 || args.len() > 3 {
            bail!("pow() requires 2 or 3 arguments");
        }
        let base = &args[0];
        let exp = &args[1];
        // Simplified: ignore modulo parameter
        Ok(parse_quote! { (#base as f64).powf(#exp as f64) as i32 })
    }

    fn convert_hex_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("hex() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("0x{:x}", #value) })
    }

    fn convert_bin_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("bin() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("0b{:b}", #value) })
    }

    fn convert_oct_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("oct() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("0o{:o}", #value) })
    }

    fn convert_chr_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("chr() requires exactly 1 argument");
        }
        let code = &args[0];
        Ok(parse_quote! {
            char::from_u32(#code as u32).unwrap().to_string()
        })
    }

    fn convert_ord_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("ord() requires exactly 1 argument");
        }
        let char_str = &args[0];
        Ok(parse_quote! {
            #char_str.chars().next().unwrap() as i32
        })
    }

    /// Convert Python open() to Rust file I/O
    ///
    /// Maps Python open() to Rust std::fs:
    /// - open(path) or open(path, 'r') → std::fs::File::open(path)?
    /// - open(path, 'w') → std::fs::File::create(path)?
    /// - open(path, 'a') → std::fs::OpenOptions::new().append(true).open(path)?
    ///
    /// # Complexity
    /// ≤10 (match with 3 branches)
    fn convert_open_builtin(&self, hir_args: &[HirExpr], args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() || args.len() > 2 {
            bail!("open() requires 1 or 2 arguments");
        }

        let path = &args[0];

        // Determine mode from second argument (default is 'r')
        let mode = if args.len() == 2 {
            // Try to extract string literal from HIR
            if let Some(HirExpr::Literal(Literal::String(mode_str))) = hir_args.get(1) {
                mode_str.as_str()
            } else {
                // If not a literal, default to read mode
                "r"
            }
        } else {
            "r" // Default mode
        };

        match mode {
            "r" | "rb" => {
                // Read mode → std::fs::File::open(path)?
                Ok(parse_quote! { std::fs::File::open(#path)? })
            }
            "w" | "wb" => {
                // Write mode → std::fs::File::create(path)?
                Ok(parse_quote! { std::fs::File::create(#path)? })
            }
            "a" | "ab" => {
                // Append mode → OpenOptions with append
                Ok(parse_quote! {
                    std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(#path)?
                })
            }
            _ => {
                // Unsupported mode, default to read
                Ok(parse_quote! { std::fs::File::open(#path)? })
            }
        }
    }

    fn convert_hash_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("hash() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! {
            {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                #value.hash(&mut hasher);
                hasher.finish() as i64
            }
        })
    }

    fn convert_repr_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("repr() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("{:?}", #value) })
    }

    /// Convert next() builtin to Rust.
    /// Optimizes `next((x for x in items if cond), None)` to `items.iter().find(|x| cond).cloned()`
    /// Optimizes `next((i for i, x in enumerate(items) if cond), default)` to `items.iter().position(|x| cond).map(|i| i as i32).unwrap_or(default)`
    fn convert_next_builtin(
        &mut self,
        hir_args: &[HirExpr],
        args: &[syn::Expr],
    ) -> Result<syn::Expr> {
        if hir_args.is_empty() || hir_args.len() > 2 {
            bail!("next() requires 1 or 2 arguments (iterator, optional default)");
        }

        // Optimize: next((i for i, x in enumerate(items) if cond), default) → items.iter().position(|x| cond).map(|i| i as i32).unwrap_or(default)
        if let HirExpr::GeneratorExp {
            element,
            generators,
        } = &hir_args[0]
        {
            if generators.len() == 1 {
                let generator = &generators[0];

                // Check for enumerate pattern: target is "(i, x)" and iter is "enumerate(items)"
                if let Some(position_expr) =
                    self.try_optimize_enumerate_position(element, generator, hir_args, args)?
                {
                    return Ok(position_expr);
                }

                let is_identity =
                    matches!(&**element, HirExpr::Var(name) if name == &generator.target);

                if is_identity && !generator.conditions.is_empty() {
                    // This is a findable pattern: next((x for x in items if cond), default)
                    let iter_expr = generator.iter.to_rust_expr(self.ctx)?;
                    let target_pat = self.parse_target_pattern(&generator.target)?;

                    // Build combined condition from all conditions
                    let conditions: Vec<syn::Expr> = generator
                        .conditions
                        .iter()
                        .map(|c| c.to_rust_expr(self.ctx))
                        .collect::<Result<Vec<_>>>()?;

                    let combined_condition: syn::Expr = if conditions.len() == 1 {
                        conditions.into_iter().next().unwrap()
                    } else {
                        conditions
                            .into_iter()
                            .reduce(|acc, c| parse_quote! { #acc && #c })
                            .unwrap()
                    };

                    // Determine if the element type needs cloned() based on collection type
                    let needs_cloned = if let HirExpr::Var(var_name) = &*generator.iter {
                        if let Some(var_type) = self.ctx.var_types.get(var_name) {
                            match var_type {
                                crate::hir::Type::List(elem_type)
                                | crate::hir::Type::Set(elem_type) => {
                                    self.type_needs_clone(elem_type)
                                }
                                _ => true,
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    };

                    let find_expr: syn::Expr = if needs_cloned {
                        parse_quote! { #iter_expr.iter().find(|#target_pat| #combined_condition).cloned() }
                    } else {
                        parse_quote! { #iter_expr.iter().find(|#target_pat| #combined_condition).copied() }
                    };

                    // Handle default value
                    if hir_args.len() == 2 {
                        if matches!(&hir_args[1], HirExpr::Literal(crate::hir::Literal::None)) {
                            return Ok(find_expr);
                        } else {
                            let default = args[1].clone();
                            return Ok(parse_quote! { #find_expr.unwrap_or(#default) });
                        }
                    } else {
                        return Ok(
                            parse_quote! { #find_expr.expect("StopIteration: iterator is empty") },
                        );
                    }
                }
            }
        }

        // Fallback: use generic iterator.next() pattern
        let iterator = &args[0];
        if hir_args.len() == 2 {
            if matches!(&hir_args[1], HirExpr::Literal(crate::hir::Literal::None)) {
                Ok(parse_quote! { #iterator.next() })
            } else {
                let default = &args[1];
                Ok(parse_quote! { #iterator.next().unwrap_or(#default) })
            }
        } else {
            Ok(parse_quote! { #iterator.next().expect("StopIteration: iterator is empty") })
        }
    }

    /// Optimize `next((i for i, x in enumerate(items) if cond), default)` to
    /// `items.iter().position(|x| cond).map(|i| i as i32).unwrap_or(default)`
    fn try_optimize_enumerate_position(
        &mut self,
        element: &HirExpr,
        comprehension: &crate::hir::HirComprehension,
        hir_args: &[HirExpr],
        args: &[syn::Expr],
    ) -> Result<Option<syn::Expr>> {
        // Check if the target is a tuple pattern "(idx_var, elem_var)"
        let (idx_var, elem_var) =
            if comprehension.target.starts_with('(') && comprehension.target.ends_with(')') {
                let inner = &comprehension.target[1..comprehension.target.len() - 1];
                let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            };

        // Check if element is just the index variable
        let element_is_index = matches!(element, HirExpr::Var(name) if name == &idx_var);
        if !element_is_index {
            return Ok(None);
        }

        // Check if iter is enumerate(collection)
        let (collection_expr, collection_hir) = if let HirExpr::Call {
            func,
            args: call_args,
            ..
        } = &*comprehension.iter
        {
            if func == "enumerate" && call_args.len() == 1 {
                (call_args[0].to_rust_expr(self.ctx)?, &call_args[0])
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        };

        // Must have at least one condition
        if comprehension.conditions.is_empty() {
            return Ok(None);
        }

        // Determine if the collection's element type is Copy
        let element_needs_clone = if let HirExpr::Var(var_name) = collection_hir {
            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                match var_type {
                    Type::List(elem_type) | Type::Set(elem_type) => {
                        self.type_needs_clone(elem_type)
                    }
                    _ => true,
                }
            } else {
                true
            }
        } else {
            true
        };

        // Build the condition, using elem_var as the closure parameter.
        // .position() receives &T, so elem_var needs deref handling.
        let elem_ident = syn::Ident::new(&elem_var, proc_macro2::Span::call_site());

        if element_needs_clone {
            self.ctx.filter_deref_vars.insert(elem_var.clone());
        }
        let conditions: Vec<syn::Expr> = comprehension
            .conditions
            .iter()
            .map(|c| c.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;
        if element_needs_clone {
            self.ctx.filter_deref_vars.remove(&elem_var);
        }

        let combined_condition: syn::Expr = if conditions.len() == 1 {
            conditions.into_iter().next().unwrap()
        } else {
            conditions
                .into_iter()
                .reduce(|acc, c| parse_quote! { #acc && #c })
                .unwrap()
        };

        // Generate: collection.iter().position(|elem| cond).map(|i| i as i32)
        // For Copy types use |&elem| destructuring; for non-Copy use |elem| with deref in body.
        let position_expr: syn::Expr = if element_needs_clone {
            parse_quote! {
                #collection_expr.iter().position(|#elem_ident| #combined_condition).map(|i| i as i32)
            }
        } else {
            parse_quote! {
                #collection_expr.iter().position(|&#elem_ident| #combined_condition).map(|i| i as i32)
            }
        };

        // Handle default value
        if hir_args.len() == 2 {
            if matches!(&hir_args[1], HirExpr::Literal(crate::hir::Literal::None)) {
                // next(..., None) → return Option<i32>
                return Ok(Some(position_expr));
            } else {
                let default = args[1].clone();
                return Ok(Some(parse_quote! { #position_expr.unwrap_or(#default) }));
            }
        } else {
            // No default - use expect
            return Ok(Some(
                parse_quote! { #position_expr.expect("StopIteration: iterator is empty") },
            ));
        }
    }

    /// Helper to mark a class as needing dynamic field access methods (_get_field/_set_field)
    /// when getattr/setattr is used with dynamic attribute names.
    fn mark_class_needs_dynamic_access(&mut self, obj_expr: &HirExpr) {
        // Try to determine the class name from the object expression
        let class_name = match obj_expr {
            // self → current class (tracked in context, but we can infer from var_types if needed)
            HirExpr::Var(var_name) => {
                if let Some(Type::Custom(class_name)) = self.ctx.var_types.get(var_name) {
                    Some(class_name.clone())
                } else {
                    None
                }
            }
            // Method call or attribute that has a custom type
            HirExpr::Attribute { value, .. } | HirExpr::MethodCall { object: value, .. } => {
                if let HirExpr::Var(var_name) = value.as_ref() {
                    if let Some(Type::Custom(class_name)) = self.ctx.var_types.get(var_name) {
                        Some(class_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(class_name) = class_name {
            self.ctx.classes_needing_dynamic_access.insert(class_name);
        }
    }

    //
    /// getattr(obj, name) → obj.name
    /// getattr(obj, name, default) → obj.name (default is ignored in static Rust)
    ///
    /// Python's getattr() retrieves an attribute on an object dynamically. In Rust, we generate
    /// a direct field access when the attribute name is a string literal, or HashMap access
    /// when the attribute name is an f-string (dynamic attribute access).
    fn convert_getattr_builtin(&mut self, hir_args: &[HirExpr]) -> Result<syn::Expr> {
        if hir_args.len() < 2 || hir_args.len() > 3 {
            bail!("getattr() requires 2 or 3 arguments (object, name, optional default)");
        }

        let obj_expr = hir_args[0].to_rust_expr(self.ctx)?;

        match &hir_args[1] {
            // Static attribute name - generate direct field access
            HirExpr::Literal(Literal::String(attr_name)) => {
                let attr_ident = syn::Ident::new(attr_name, proc_macro2::Span::call_site());
                Ok(parse_quote! { #obj_expr.#attr_ident })
            }
            // Dynamic attribute name (f-string) - generate _get_field() call
            HirExpr::FString { parts } => {
                // Mark the class as needing dynamic field access
                self.mark_class_needs_dynamic_access(&hir_args[0]);
                
                let key_expr = self.convert_fstring(parts)?;
                if hir_args.len() == 3 {
                    let default_expr = hir_args[2].to_rust_expr(self.ctx)?;
                    Ok(parse_quote! {
                        #obj_expr._get_field(&#key_expr).map(|v| *v.downcast().unwrap()).unwrap_or(#default_expr)
                    })
                } else {
                    Ok(parse_quote! {
                        *#obj_expr._get_field(&#key_expr).expect("attribute not found").downcast().unwrap()
                    })
                }
            }
            // Variable containing the attribute name - generate _get_field() call
            HirExpr::Var(_) => {
                // Mark the class as needing dynamic field access
                self.mark_class_needs_dynamic_access(&hir_args[0]);
                
                let key_expr = hir_args[1].to_rust_expr(self.ctx)?;
                if hir_args.len() == 3 {
                    let default_expr = hir_args[2].to_rust_expr(self.ctx)?;
                    Ok(parse_quote! {
                        #obj_expr._get_field(&#key_expr).map(|v| *v.downcast().unwrap()).unwrap_or(#default_expr)
                    })
                } else {
                    Ok(parse_quote! {
                        *#obj_expr._get_field(&#key_expr).expect("attribute not found").downcast().unwrap()
                    })
                }
            }
            _ => {
                bail!(
                    "getattr() attribute name must be a string literal, f-string, or variable. \
                    For dynamic attribute access, consider using a Dict/HashMap instead of a struct."
                )
            }
        }
    }

    /// setattr(obj, name, value) → obj.name = value
    ///
    /// Python's setattr() sets an attribute on an object dynamically. In Rust, we generate
    /// a direct field assignment when the attribute name is a string literal.
    /// String literals get `.to_string()`, and String/object variables get `.clone()`.
    fn convert_setattr_builtin(&mut self, hir_args: &[HirExpr]) -> Result<syn::Expr> {
        if hir_args.len() != 3 {
            bail!("setattr() requires exactly 3 arguments (object, name, value)");
        }

        let obj_expr = hir_args[0].to_rust_expr(self.ctx)?;

        // Handle value based on its type - strings and objects need special handling
        let value_expr = match &hir_args[2] {
            // String literal → .to_string()
            HirExpr::Literal(Literal::String(_)) => {
                let raw_expr = hir_args[2].to_rust_expr(self.ctx)?;
                parse_quote! { #raw_expr.to_string() }
            }
            // Variable → check type and clone if String or Custom object
            HirExpr::Var(var_name) => {
                let raw_expr = hir_args[2].to_rust_expr(self.ctx)?;
                if let Some(var_type) = self.ctx.var_types.get(var_name) {
                    match var_type {
                        Type::String | Type::Custom(_) => {
                            parse_quote! { #raw_expr.clone() }
                        }
                        _ => raw_expr,
                    }
                } else {
                    raw_expr
                }
            }
            // Attribute access → check base type and clone if String/Custom field
            HirExpr::Attribute { value, .. } => {
                let raw_expr = hir_args[2].to_rust_expr(self.ctx)?;
                // If base is a Custom type, clone the field access (it's likely String or another struct)
                if let HirExpr::Var(base_var) = value.as_ref() {
                    if let Some(Type::Custom(_)) = self.ctx.var_types.get(base_var) {
                        parse_quote! { #raw_expr.clone() }
                    } else {
                        raw_expr
                    }
                } else {
                    raw_expr
                }
            }
            // Other expressions (literals, method calls, etc.) → use as-is
            _ => hir_args[2].to_rust_expr(self.ctx)?,
        };

        // Extract attribute name
        match &hir_args[1] {
            // Static attribute name - generate direct field assignment
            HirExpr::Literal(Literal::String(attr_name)) => {
                let attr_ident = syn::Ident::new(attr_name, proc_macro2::Span::call_site());
                // Generate: { obj.attr = value; }
                Ok(parse_quote! {
                    {
                        #obj_expr.#attr_ident = #value_expr;
                    }
                })
            }
            // Dynamic attribute name (f-string) - generate _set_field() call
            HirExpr::FString { parts } => {
                // Mark the class as needing dynamic field access
                self.mark_class_needs_dynamic_access(&hir_args[0]);
                
                let key_expr = self.convert_fstring(parts)?;
                Ok(parse_quote! {
                    {
                        #obj_expr._set_field(&#key_expr, &#value_expr);
                    }
                })
            }
            // Variable containing the attribute name - generate _set_field() call
            HirExpr::Var(_) => {
                // Mark the class as needing dynamic field access
                self.mark_class_needs_dynamic_access(&hir_args[0]);
                
                let key_expr = hir_args[1].to_rust_expr(self.ctx)?;
                Ok(parse_quote! {
                    {
                        #obj_expr._set_field(&#key_expr, &#value_expr);
                    }
                })
            }
            _ => {
                bail!(
                    "setattr() attribute name must be a string literal, f-string, or variable. \
                    For dynamic attribute access, consider using a Dict/HashMap instead of a struct."
                )
            }
        }
    }

    //
    fn convert_iter_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("iter() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter() })
    }

    //
    fn convert_type_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("type() requires exactly 1 argument");
        }
        let value = &args[0];
        // Return a string representation of the type name
        // This is a simplified implementation - full Python type() is more complex
        Ok(parse_quote! { std::any::type_name_of_val(&#value) })
    }

    /// Check if expression already ends with .collect()
    fn already_collected(&self, expr: &syn::Expr) -> bool {
        if let syn::Expr::MethodCall(method_call) = expr {
            method_call.method == "collect"
        } else {
            false
        }
    }

    /// Check if expression is a range (0..5, start..end, etc.)
    fn is_range_expr(&self, expr: &syn::Expr) -> bool {
        matches!(expr, syn::Expr::Range(_))
    }

    /// Check if expression is an iterator-producing expression
    fn is_iterator_expr(&self, expr: &syn::Expr) -> bool {
        // Check if it's a method call that returns an iterator
        if let syn::Expr::MethodCall(method_call) = expr {
            let method_name = method_call.method.to_string();
            matches!(
                method_name.as_str(),
                "iter"
                    | "iter_mut"
                    | "into_iter"
                    | "zip"
                    | "map"
                    | "filter"
                    | "enumerate"
                    | "chain"
                    | "flat_map"
                    | "take"
                    | "skip"
                    | "collect"
            )
        } else {
            false
        }
    }

    /// Uses heuristic name-based detection (reader, csv_reader, etc.)
    fn is_csv_reader_var(&self, expr: &syn::Expr) -> bool {
        if let syn::Expr::Path(path) = expr {
            if let Some(ident) = path.path.get_ident() {
                let var_name = ident.to_string();
                return var_name == "reader"
                    || var_name.contains("csv")
                    || var_name.ends_with("_reader")
                    || var_name.starts_with("reader_");
            }
        }
        false
    }

    fn convert_generic_call(
        &self,
        func: &str,
        hir_args: &[HirExpr],
        args: &[syn::Expr],
    ) -> Result<syn::Expr> {
        // Special case: Python print() → Rust log::info!()
        if func == "print" {
            return if args.is_empty() {
                // print() with no arguments → log::info!()
                Ok(parse_quote! { log::info!("") })
            } else if args.len() == 1 {
                // Check if arg needs {:?} format (collections, tuples, or custom types)
                let needs_debug = if let Some(hir_arg) = hir_args.first() {
                    match hir_arg {
                        HirExpr::Var(name) => {
                            // Check variable type
                            self.ctx
                                .var_types
                                .get(name)
                                .map(|t| {
                                    matches!(
                                        t,
                                        Type::List(_)
                                            | Type::Dict(_, _)
                                            | Type::Set(_)
                                            | Type::Tuple(_)
                                            | Type::Custom(_)
                                    )
                                })
                                .unwrap_or(false)
                        }
                        HirExpr::List(_)
                        | HirExpr::Dict(_)
                        | HirExpr::Set(_)
                        | HirExpr::FrozenSet(_)
                        | HirExpr::Tuple(_) => true,
                        HirExpr::Binary {
                            op: BinOp::Add,
                            left,
                            right,
                        } => {
                            // Result of list concatenation
                            self.is_list_expr(left) || self.is_list_expr(right)
                        }
                        _ => false,
                    }
                } else {
                    false
                };

                let arg = &args[0];
                if needs_debug {
                    Ok(parse_quote! { log::info!("{:?}", #arg) })
                } else {
                    Ok(parse_quote! { log::info!("{}", #arg) })
                }
            } else {
                // print(a, b, c) → log::info!("{} {} {}", a, b, c) or with {:?} for non-Display types
                let format_specs: Vec<&str> = hir_args
                    .iter()
                    .map(|hir_arg| {
                        let needs_debug = match hir_arg {
                            HirExpr::Var(name) => self
                                .ctx
                                .var_types
                                .get(name)
                                .map(|t| {
                                    matches!(
                                        t,
                                        Type::List(_)
                                            | Type::Dict(_, _)
                                            | Type::Set(_)
                                            | Type::Tuple(_)
                                            | Type::Custom(_)
                                    )
                                })
                                .unwrap_or(false),
                            HirExpr::List(_)
                            | HirExpr::Dict(_)
                            | HirExpr::Set(_)
                            | HirExpr::FrozenSet(_)
                            | HirExpr::Tuple(_) => true,
                            HirExpr::Binary {
                                op: BinOp::Add,
                                left,
                                right,
                            } => self.is_list_expr(left) || self.is_list_expr(right),
                            _ => false,
                        };
                        if needs_debug { "{:?}" } else { "{}" }
                    })
                    .collect();
                let format_str = format_specs.join(" ");
                Ok(parse_quote! { log::info!(#format_str, #(#args),*) })
            };
        }

        // Check if this is an imported function
        if let Some(rust_path) = self.ctx.imported_items.get(func) {
            // Parse the rust path and generate the call
            let path_parts: Vec<&str> = rust_path.split("::").collect();
            let mut path = quote! {};
            for (i, part) in path_parts.iter().enumerate() {
                let part_ident = syn::Ident::new(part, proc_macro2::Span::call_site());
                if i == 0 {
                    path = quote! { #part_ident };
                } else {
                    path = quote! { #path::#part_ident };
                }
            }
            if args.is_empty() {
                return Ok(parse_quote! { #path() });
            } else {
                return Ok(parse_quote! { #path(#(#args),*) });
            }
        }

        // Check if this might be a constructor call (capitalized name)
        if func
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            let class_ident = syn::Ident::new(func, proc_macro2::Span::call_site());

            // Check if this is an enum - use from_i32_or_default instead of new
            if self.ctx.enum_names.contains(func) {
                if args.len() == 1 {
                    let arg = &args[0];
                    return Ok(parse_quote! { #class_ident::from_i32_or_default(#arg) });
                } else if args.is_empty() {
                    // No args - can't create enum from nothing, return first variant as default
                    return Ok(parse_quote! { #class_ident::from_i32_or_default(0) });
                }
            }

            // Regular class constructor - ClassName::new(args)
            if args.is_empty() {
                let is_user_class = self.ctx.class_names.contains(func);
                if !is_user_class && func == "Counter" {
                    return Ok(parse_quote! { #class_ident::new(0) });
                }
                Ok(parse_quote! { #class_ident::new() })
            } else {
                // When passing reference parameters to constructors, clone them since
                // constructors typically expect owned values
                let cloned_args: Vec<syn::Expr> = hir_args
                    .iter()
                    .zip(args.iter())
                    .map(|(hir_arg, arg_expr)| {
                        if let HirExpr::Var(var_name) = hir_arg {
                            // Check if this variable is a reference parameter in the current function
                            if self.ctx.current_func_ref_params.contains(var_name)
                                || self.ctx.current_func_mut_ref_params.contains(var_name)
                            {
                                // Clone reference parameters when passing to constructors
                                parse_quote! { #arg_expr.clone() }
                            } else {
                                arg_expr.clone()
                            }
                        } else {
                            arg_expr.clone()
                        }
                    })
                    .collect();
                Ok(parse_quote! { #class_ident::new(#(#cloned_args),*) })
            }
        } else {
            // Check for special keywords that cannot be raw identifiers
            if Self::is_non_raw_keyword(func) {
                bail!(
                    "Python function '{}' conflicts with a special Rust keyword that cannot be escaped. \
                     Please rename this function (e.g., '{}_func' or 'py_{}'). \
                     Note: If this is 'super()', it should be handled as a method call, not a function call.",
                    func,
                    func,
                    func
                );
            }
            
            // Regular function call - use raw identifier if function name is a Rust keyword
            let func_ident = if Self::is_rust_keyword(func) {
                syn::Ident::new_raw(func, proc_macro2::Span::call_site())
            } else {
                syn::Ident::new(func, proc_macro2::Span::call_site())
            };

            // When passing a Vec/HashMap/HashSet variable to a function expecting &Vec/&HashMap/&HashSet, automatically borrow it
            // This handles cases like: sum_list_recursive(rest) where rest is Vec but param is &Vec

            // Strategy:
            // 1. Look up function signature to see which params are borrowed
            // 2. Only borrow if: (a) arg is List/Dict/Set AND (b) function expects borrow
            // 3. Check if param needs &mut (mutated in callee) or just &
            // 4. Otherwise pass as-is (either owned or primitive)

            // BORROW CONFLICT DETECTION:
            // Collect variables that are passed as &mut to avoid borrow conflicts.
            // If `state` is passed as &mut, then `state.field` in another arg creates a conflict.
            //
            // A variable needs to be in mut_borrowed_vars if:
            // 1. The callee function expects &mut for that parameter position, OR
            // 2. The variable is already a &mut reference in the current function
            //    (from current_func_mut_ref_params) and is being passed to a function
            let mut_borrowed_vars: HashSet<String> = hir_args
                .iter()
                .enumerate()
                .filter_map(|(idx, arg)| {
                    if let HirExpr::Var(var_name) = arg {
                        // Check if this param expects &mut in the callee
                        let callee_expects_mut = self
                            .ctx
                            .function_param_muts
                            .get(func)
                            .and_then(|muts| muts.get(idx))
                            .copied()
                            .unwrap_or(false);

                        // Check if this variable is already a &mut ref in current function
                        let is_already_mut_ref =
                            self.ctx.current_func_mut_ref_params.contains(var_name);

                        // Either condition means we have a mutable borrow happening
                        if callee_expects_mut || is_already_mut_ref {
                            return Some(var_name.clone());
                        }
                    }
                    None
                })
                .collect();

            let borrowed_args: Vec<syn::Expr> = hir_args
                .iter()
                .zip(args.iter())
                .enumerate()
                .map(|(param_idx, (hir_arg, arg_expr))| {
                    // If so, always pass by reference (&args)
                    if let HirExpr::Var(var_name) = hir_arg {
                        let is_argparse_args =
                            self.ctx
                                .argparser_tracker
                                .parsers
                                .values()
                                .any(|parser_info| {
                                    parser_info
                                        .args_var
                                        .as_ref()
                                        .is_some_and(|args_var| args_var == var_name)
                                });

                        if is_argparse_args {
                            return parse_quote! { &#arg_expr };
                        }
                    }

                    // Check if the callee's parameter is declared as &mut reference
                    let param_expects_mut_ref = self
                        .ctx
                        .function_param_borrows
                        .get(func)
                        .and_then(|borrows| borrows.get(param_idx))
                        .map(|info| info.should_borrow && info.needs_mut)
                        .unwrap_or(false);

                    let should_borrow = match hir_arg {
                        HirExpr::Var(var_name) => {
                            // Check if variable has List, Dict, Set, or String type
                            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                                if matches!(
                                    var_type,
                                    Type::List(_) | Type::Dict(_, _) | Type::Set(_)
                                ) {
                                    // Check if function param expects a borrow
                                    self.ctx
                                        .function_param_borrows
                                        .get(func)
                                        .and_then(|borrows| borrows.get(param_idx))
                                        .copied()
                                        .map(|info| info.should_borrow)
                                        .unwrap_or(true) // Default to borrow if unknown
                                } else if matches!(var_type, Type::String) {
                                    // If function param expects &str (borrowed=true), add &
                                    // If function param expects String (borrowed=false), pass as-is
                                    self.ctx
                                        .function_param_borrows
                                        .get(func)
                                        .and_then(|borrows| borrows.get(param_idx))
                                        .copied()
                                        .map(|info| info.should_borrow)
                                        .unwrap_or(true) // Default to borrow (&str) if unknown
                                } else {
                                    // For user-defined types passed to functions,
                                    // check if the parameter needs &mut or just &
                                    param_expects_mut_ref
                                        || self
                                            .ctx
                                            .function_param_borrows
                                            .get(func)
                                            .and_then(|borrows| borrows.get(param_idx))
                                            .copied()
                                            .map(|info| info.should_borrow)
                                            .unwrap_or(false)
                                }
                            } else {
                                // Unknown type - check if it needs borrow
                                param_expects_mut_ref
                                    || self
                                        .ctx
                                        .function_param_borrows
                                        .get(func)
                                        .and_then(|borrows| borrows.get(param_idx))
                                        .map(|info| info.should_borrow)
                                        .unwrap_or(false)
                            }
                        }
                        // List/Dict/Set literal: borrow if callee expects a reference parameter
                        HirExpr::List(_) | HirExpr::Dict(_) | HirExpr::Set(_) => {
                            self.ctx
                                .function_param_borrows
                                .get(func)
                                .and_then(|borrows| borrows.get(param_idx))
                                .map(|info| info.should_borrow)
                                .unwrap_or(false)
                        }
                        // Check if string literal needs .to_string()
                        // String literals are &str, but if function expects String (owned),
                        // we need to add .to_string()
                        HirExpr::Literal(crate::hir::Literal::String(_)) => {
                            // Check if function param is borrowed (true = &str, false = String)
                            let _param_is_borrowed = self
                                .ctx
                                .function_param_borrows
                                .get(func)
                                .and_then(|borrows| borrows.get(param_idx))
                                .map(|info| info.should_borrow)
                                .unwrap_or(false); // Default to owned (String) if unknown

                            // If param is borrowed (&str), we DON'T need .to_string()
                            // If param is owned (String), we DO need .to_string()
                            // But this is handled differently - we modify arg_expr below
                            false // Don't add & reference for strings
                        }
                        // STRING_INTEROP_FIX: Handle string concatenation (Binary Add) in function arguments
                        // String concatenation like `s + "?"` becomes `format!("{}{}", s, "?")` which returns String
                        // When passed to a function expecting &str, we need to add & to borrow it
                        HirExpr::Binary {
                            op: BinOp::Add,
                            left,
                            right,
                        } => {
                            // Check if this is string concatenation by checking if operands are string-like
                            let is_string_concat = matches!(
                                (&**left, &**right),
                                (
                                    HirExpr::Var(_),
                                    HirExpr::Literal(crate::hir::Literal::String(_))
                                ) | (
                                    HirExpr::Literal(crate::hir::Literal::String(_)),
                                    HirExpr::Var(_)
                                ) | (
                                    HirExpr::Literal(crate::hir::Literal::String(_)),
                                    HirExpr::Literal(crate::hir::Literal::String(_))
                                )
                            );

                            if is_string_concat {
                                // Check if function param expects a borrowed string (&str)
                                // format!() returns String, so if param expects &str we need to borrow
                                self.ctx
                                    .function_param_borrows
                                    .get(func)
                                    .and_then(|borrows| borrows.get(param_idx))
                                    .map(|info| info.should_borrow)
                                    .unwrap_or(true) // Default to borrow (&str) if unknown
                            } else {
                                false
                            }
                        }
                        // Check for attribute access (e.g., state.numbers)
                        HirExpr::Attribute { .. } => {
                            // Attribute access on user types needs borrowing if param expects it
                            param_expects_mut_ref
                                || self
                                    .ctx
                                    .function_param_borrows
                                    .get(func)
                                    .and_then(|borrows| borrows.get(param_idx))
                                    .map(|info| info.should_borrow)
                                    .unwrap_or(false)
                        }
                        // Handle function calls that return owned values (Vec, HashMap, etc.)
                        // When passed to parameters expecting borrowed references (&Vec, &HashMap, etc.)
                        HirExpr::Call {
                            func: called_func, ..
                        } => {
                            // Check return type of called function
                            let return_type =
                                self.ctx.function_return_types.get(called_func.as_str());

                            // Check if parameter expects a borrow
                            let param_expects_borrow = self
                                .ctx
                                .function_param_borrows
                                .get(func)
                                .and_then(|borrows| borrows.get(param_idx))
                                .map(|info| info.should_borrow)
                                .unwrap_or(false);

                            // If function returns Vec/HashMap/HashSet and param expects borrow, add &
                            if param_expects_borrow {
                                if let Some(ret_type) = return_type {
                                    matches!(
                                        ret_type,
                                        Type::List(_) | Type::Dict(_, _) | Type::Set(_)
                                    )
                                } else {
                                    // Unknown return type, check if param expects borrow
                                    true
                                }
                            } else {
                                false
                            }
                        }
                        _ => {
                            // Fallback: check if expression creates a Vec via .to_vec()
                            let expr_string = quote! { #arg_expr }.to_string();
                            expr_string.contains("to_vec")
                        }
                    };

                    // Check if the variable is already a &mut reference in current function
                    // If so, we don't need to add &mut again
                    let is_already_mut_ref = if let HirExpr::Var(var_name) = hir_arg {
                        self.ctx.current_func_mut_ref_params.contains(var_name)
                    } else {
                        false
                    };

                    // Check if the variable is already an immutable & reference in current function
                    // If so, we don't need to add & again
                    let is_already_ref = if let HirExpr::Var(var_name) = hir_arg {
                        self.ctx.current_func_ref_params.contains(var_name)
                    } else {
                        false
                    };

                    // Check if this is a field access on a &mut ref parameter
                    // In that case, we can't move the field out - we must clone
                    let needs_clone_for_move = if let HirExpr::Attribute { value, .. } = hir_arg {
                        if let HirExpr::Var(base_var) = &**value {
                            self.ctx.current_func_mut_ref_params.contains(base_var)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // BORROW CONFLICT: Check if this argument accesses a field on a variable
                    // that is also being passed as &mut in this same function call.
                    // e.g., fn(state, state.field) where state is &mut → need to clone state.field
                    // Also handles: fn(state, &state.field) where the borrow wraps an attribute
                    fn get_base_var(expr: &HirExpr) -> Option<&str> {
                        match expr {
                            HirExpr::Var(name) => Some(name.as_str()),
                            HirExpr::Attribute { value, .. } => get_base_var(value),
                            HirExpr::Index { base, .. } => get_base_var(base),
                            HirExpr::Borrow { expr, .. } => get_base_var(expr),
                            _ => None,
                        }
                    }

                    // Check for attribute access (direct or through borrow)
                    // Also check for index access like state.items[0]
                    let (needs_clone_for_borrow_conflict, is_borrowed_attribute) =
                        if let HirExpr::Attribute { .. } = hir_arg {
                            // For state.field, get_base_var returns "state"
                            let has_conflict = get_base_var(hir_arg)
                                .map(|base_var| mut_borrowed_vars.contains(base_var))
                                .unwrap_or(false);
                            (has_conflict, false)
                        } else if let HirExpr::Index { .. } = hir_arg {
                            // For state.items[0], get_base_var returns "state"
                            let has_conflict = get_base_var(hir_arg)
                                .map(|base_var| mut_borrowed_vars.contains(base_var))
                                .unwrap_or(false);
                            (has_conflict, false)
                        } else if let HirExpr::Borrow { expr, .. } = hir_arg {
                            // Handle &state.field pattern - check if inner expr has conflict
                            let has_conflict = get_base_var(expr)
                                .map(|base_var| mut_borrowed_vars.contains(base_var))
                                .unwrap_or(false);
                            let is_attr = matches!(&**expr, HirExpr::Attribute { .. });
                            (has_conflict, is_attr)
                        } else {
                            (false, false)
                        };

                    // Check if argument is Optional and needs unwrapping
                    // This happens when Optional field/variable is passed to non-Optional parameter
                    let needs_optional_unwrap = {
                        let arg_optional_inner = self.ctx.get_optional_inner_type(hir_arg);
                        let param_type = self
                            .ctx
                            .function_param_types
                            .get(func)
                            .and_then(|types| types.get(param_idx))
                            .cloned();

                        // Need unwrap if: arg is Optional<T> AND param is T (not Optional)
                        if let (Some(_inner_ty), Some(param_ty)) =
                            (&arg_optional_inner, &param_type)
                        {
                            !matches!(param_ty, Type::Optional(_))
                        } else {
                            false
                        }
                    };

                    if should_borrow || param_expects_mut_ref {
                        if param_expects_mut_ref {
                            if is_already_mut_ref {
                                // Variable is already &mut T, just pass it directly
                                // Need to generate expression without .clone() since arg_expr
                                // may have .clone() added by to_rust_expr for non-Copy types
                                if let HirExpr::Var(var_name) = hir_arg {
                                    let ident = format_ident!("{}", var_name);
                                    parse_quote! { #ident }
                                } else {
                                    arg_expr.clone()
                                }
                            } else if let HirExpr::Attribute { value, attr } = hir_arg {
                                // For field access that needs &mut, generate without clone
                                // Build the field access expression manually
                                fn build_attribute_expr(expr: &HirExpr) -> syn::Expr {
                                    match expr {
                                        HirExpr::Var(name) => {
                                            let ident = format_ident!("{}", name);
                                            parse_quote! { #ident }
                                        }
                                        HirExpr::Attribute { value, attr } => {
                                            let base = build_attribute_expr(value);
                                            let attr_ident = format_ident!("{}", attr);
                                            parse_quote! { #base.#attr_ident }
                                        }
                                        _ => {
                                            // Fallback - shouldn't happen often
                                            parse_quote! { () }
                                        }
                                    }
                                }
                                let base_expr = build_attribute_expr(value);
                                let attr_ident = format_ident!("{}", attr);
                                let result: syn::Expr =
                                    parse_quote! { &mut #base_expr.#attr_ident };
                                if needs_optional_unwrap {
                                    parse_quote! { #result.unwrap() }
                                } else {
                                    result
                                }
                            } else {
                                let result: syn::Expr = parse_quote! { &mut #arg_expr };
                                if needs_optional_unwrap {
                                    parse_quote! { #result.unwrap() }
                                } else {
                                    result
                                }
                            }
                        } else if is_already_ref {
                            // Variable is already &T, just pass it directly without adding &
                            // Need to generate expression without .clone() since arg_expr
                            // may have .clone() added by to_rust_expr for non-Copy types
                            if let HirExpr::Var(var_name) = hir_arg {
                                let ident = format_ident!("{}", var_name);
                                let result: syn::Expr = parse_quote! { #ident };
                                if needs_optional_unwrap {
                                    parse_quote! { #result.unwrap() }
                                } else {
                                    result
                                }
                            } else {
                                if needs_optional_unwrap {
                                    parse_quote! { #arg_expr.unwrap() }
                                } else {
                                    arg_expr.clone()
                                }
                            }
                        } else if is_already_mut_ref {
                            // Variable is already &mut T, just pass it directly without adding &
                            // This handles the case where should_borrow is true but param_expects_mut_ref is false
                            // and the variable is a mutable reference (e.g., state: &mut State)
                            // Need to generate expression without .clone() since arg_expr
                            // may have .clone() added by to_rust_expr for non-Copy types
                            if let HirExpr::Var(var_name) = hir_arg {
                                let ident = format_ident!("{}", var_name);
                                let result: syn::Expr = parse_quote! { #ident };
                                if needs_optional_unwrap {
                                    parse_quote! { #result.unwrap() }
                                } else {
                                    result
                                }
                            } else {
                                if needs_optional_unwrap {
                                    parse_quote! { #arg_expr.unwrap() }
                                } else {
                                    arg_expr.clone()
                                }
                            }
                        } else {
                            // Borrowing immutably - but check for borrow conflict first!
                            // If this field access conflicts with a &mut borrow in the same call,
                            // we need to clone before borrowing
                            if needs_clone_for_borrow_conflict {
                                // Clone the field to avoid simultaneous borrow conflict
                                // Generate: &field.clone() or &state.field.clone()
                                if let HirExpr::Attribute { value, attr } = hir_arg {
                                    fn build_attr_clone(expr: &HirExpr) -> syn::Expr {
                                        match expr {
                                            HirExpr::Var(name) => {
                                                let ident = format_ident!("{}", name);
                                                parse_quote! { #ident }
                                            }
                                            HirExpr::Attribute { value, attr } => {
                                                let base = build_attr_clone(value);
                                                let attr_ident = format_ident!("{}", attr);
                                                parse_quote! { #base.#attr_ident }
                                            }
                                            _ => parse_quote! { () },
                                        }
                                    }
                                    let base_expr = build_attr_clone(value);
                                    let attr_ident = format_ident!("{}", attr);
                                    let result: syn::Expr =
                                        parse_quote! { &#base_expr.#attr_ident.clone() };
                                    if needs_optional_unwrap {
                                        parse_quote! { #result.unwrap() }
                                    } else {
                                        result
                                    }
                                } else {
                                    let result: syn::Expr = parse_quote! { &#arg_expr.clone() };
                                    if needs_optional_unwrap {
                                        parse_quote! { #result.unwrap() }
                                    } else {
                                        result
                                    }
                                }
                            } else if let HirExpr::Var(var_name) = hir_arg {
                                // No conflict - simple variable borrow
                                let ident = format_ident!("{}", var_name);
                                let result: syn::Expr = parse_quote! { &#ident };
                                if needs_optional_unwrap {
                                    parse_quote! { #result.unwrap() }
                                } else {
                                    result
                                }
                            } else {
                                let result: syn::Expr = parse_quote! { &#arg_expr };
                                if needs_optional_unwrap {
                                    parse_quote! { #result.unwrap() }
                                } else {
                                    result
                                }
                            }
                        }
                    } else if needs_clone_for_move || needs_clone_for_borrow_conflict {
                        // Field access on &mut ref, or borrow conflict with another arg - must clone
                        // For borrowed attributes (&state.field), we need to clone the inner attribute
                        // to avoid the simultaneous borrow conflict with &mut state
                        let cloned_expr: syn::Expr = if is_borrowed_attribute {
                            // Extract inner attribute from Borrow and clone it
                            // &state.field with conflict → state.field.clone()
                            if let HirExpr::Borrow { expr, .. } = hir_arg {
                                // Build expression manually since we can't call to_rust_expr in closure
                                fn build_expr_for_clone(expr: &HirExpr) -> syn::Expr {
                                    match expr {
                                        HirExpr::Var(name) => {
                                            let ident = format_ident!("{}", name);
                                            parse_quote! { #ident }
                                        }
                                        HirExpr::Attribute { value, attr } => {
                                            let base = build_expr_for_clone(value);
                                            let attr_ident = format_ident!("{}", attr);
                                            parse_quote! { #base.#attr_ident }
                                        }
                                        HirExpr::Index { base, index } => {
                                            let base_expr = build_expr_for_clone(base);
                                            let idx_expr = build_expr_for_clone(index);
                                            parse_quote! { #base_expr[#idx_expr] }
                                        }
                                        _ => {
                                            // Fallback for other cases
                                            parse_quote! { () }
                                        }
                                    }
                                }
                                let inner_expr = build_expr_for_clone(expr);
                                parse_quote! { #inner_expr.clone() }
                            } else {
                                parse_quote! { #arg_expr.clone() }
                            }
                        } else {
                            parse_quote! { #arg_expr.clone() }
                        };

                        // If the original was a borrow (&state.field), we cloned the inner part
                        // but may need to re-add & if the callee expects a reference
                        let result: syn::Expr = if is_borrowed_attribute {
                            // Original was &state.field, now we have state.field.clone()
                            // Add & back since callee expects a reference
                            parse_quote! { &#cloned_expr }
                        } else {
                            cloned_expr
                        };

                        if needs_optional_unwrap {
                            parse_quote! { #result.unwrap() }
                        } else {
                            result
                        }
                    } else if needs_optional_unwrap {
                        // Optional field/variable passed to non-Optional parameter - add .unwrap()
                        parse_quote! { #arg_expr.unwrap() }
                    } else {
                        // STRING_INTEROP: For string literals, always add .to_string()
                        // Since all string parameters are now String type (not &str),
                        // string literals must be converted to owned String
                        if matches!(hir_arg, HirExpr::Literal(crate::hir::Literal::String(_))) {
                            parse_quote! { #arg_expr.to_string() }
                        } else if let HirExpr::Var(var_name) = hir_arg {
                            // Variables from tuple iteration over string literals are &str
                            // and need .to_string() when passed to functions expecting String
                            if self.ctx.tuple_iter_vars.contains(var_name) {
                                parse_quote! { #arg_expr.to_string() }
                            } else {
                                arg_expr.clone()
                            }
                        } else {
                            arg_expr.clone()
                        }
                    }
                })
                .collect();

            // This caused E0277 errors (279 errors!) when calling functions that return plain types (i32, Vec, etc.).

            // Root Cause Analysis:
            // 1. Why: `?` operator applied to i32/Vec (non-Result types)
            // 2. Why: Transpiler adds `?` to all function calls inside Result-returning functions
            // 3. Why: unconditionally adds `?` when current_function_can_fail is true
            // 4. Why: No check if the CALLED function actually returns Result
            // 5. ROOT CAUSE: Overly aggressive error propagation heuristic

            // Solution: Don't automatically add `?` to function calls. Let explicit error handling
            // in Python (try/except) determine when Result types are needed.
            // If specific cases need `?` for recursive calls, those should be handled specially.
            Ok(parse_quote! { #func_ident(#(#borrowed_args),*) })
        }
    }

    // ========================================================================
    // ========================================================================

    /// Try to convert classmethod call (cls.method())
    #[inline]
    fn try_convert_classmethod(
        &mut self,
        object: &HirExpr,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        if let HirExpr::Var(var_name) = object {
            if var_name == "cls" && self.ctx.is_classmethod {
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                let arg_exprs: Vec<syn::Expr> = args
                    .iter()
                    .map(|arg| arg.to_rust_expr(self.ctx))
                    .collect::<Result<Vec<_>>>()?;
                return Ok(Some(parse_quote! { Self::#method_ident(#(#arg_exprs),*) }));
            }
        }
        Ok(None)
    }

    /// Only supports format codes 'i' (signed 32-bit int) and 'ii' (two ints)
    fn try_convert_struct_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        match method {
            "pack" => {
                if args.is_empty() {
                    bail!("struct.pack() requires at least a format argument");
                }

                // First arg is format string
                if let HirExpr::Literal(Literal::String(format)) = &args[0] {
                    let count = format.chars().filter(|&c| c == 'i').count();

                    if count == 0 {
                        bail!(
                            "struct.pack() format '{}' not supported (only 'i' and 'ii' implemented)",
                            format
                        );
                    }

                    if count != args.len() - 1 {
                        bail!(
                            "struct.pack() format '{}' expects {} values, got {}",
                            format,
                            count,
                            args.len() - 1
                        );
                    }

                    // Convert value arguments
                    let value_exprs: Vec<syn::Expr> = args[1..]
                        .iter()
                        .map(|arg| arg.to_rust_expr(self.ctx))
                        .collect::<Result<Vec<_>>>()?;

                    if count == 1 {
                        // struct.pack('i', value) → (value as i32).to_le_bytes().to_vec()
                        let val = &value_exprs[0];
                        Ok(Some(parse_quote! {
                            (#val as i32).to_le_bytes().to_vec()
                        }))
                    } else {
                        // struct.pack('ii', a, b) → { let mut v = Vec::new(); v.extend_from_slice(&(a as i32).to_le_bytes()); ... }
                        Ok(Some(parse_quote! {
                            {
                                let mut __struct_pack_result = Vec::new();
                                #(__struct_pack_result.extend_from_slice(&(#value_exprs as i32).to_le_bytes());)*
                                __struct_pack_result
                            }
                        }))
                    }
                } else {
                    bail!(
                        "struct.pack() requires string literal format (dynamic formats not supported)"
                    );
                }
            }
            "unpack" => {
                if args.len() != 2 {
                    bail!("struct.unpack() requires exactly 2 arguments (format, bytes)");
                }

                // First arg is format string
                if let HirExpr::Literal(Literal::String(format)) = &args[0] {
                    let count = format.chars().filter(|&c| c == 'i').count();

                    if count == 0 {
                        bail!(
                            "struct.unpack() format '{}' not supported (only 'i' and 'ii' implemented)",
                            format
                        );
                    }

                    let bytes_expr = args[1].to_rust_expr(self.ctx)?;

                    if count == 1 {
                        // struct.unpack('i', bytes) → (i32::from_le_bytes(bytes[0..4].try_into().unwrap()),)
                        Ok(Some(parse_quote! {
                            (i32::from_le_bytes(#bytes_expr[0..4].try_into().unwrap()),)
                        }))
                    } else if count == 2 {
                        // struct.unpack('ii', bytes) → (i32::from_le_bytes(...), i32::from_le_bytes(...))
                        Ok(Some(parse_quote! {
                            (
                                i32::from_le_bytes(#bytes_expr[0..4].try_into().unwrap()),
                                i32::from_le_bytes(#bytes_expr[4..8].try_into().unwrap()),
                            )
                        }))
                    } else {
                        bail!(
                            "struct.unpack() only supports 'i' and 'ii' formats (got {} ints)",
                            count
                        );
                    }
                } else {
                    bail!(
                        "struct.unpack() requires string literal format (dynamic formats not supported)"
                    );
                }
            }
            "calcsize" => {
                if args.len() != 1 {
                    bail!("struct.calcsize() requires exactly 1 argument");
                }

                // Arg is format string
                if let HirExpr::Literal(Literal::String(format)) = &args[0] {
                    let count = format.chars().filter(|&c| c == 'i').count();

                    if count == 0 {
                        bail!(
                            "struct.calcsize() format '{}' not supported (only 'i' and 'ii' implemented)",
                            format
                        );
                    }

                    let size = (count * 4) as i32;
                    Ok(Some(parse_quote! { #size }))
                } else {
                    bail!(
                        "struct.calcsize() requires string literal format (dynamic formats not supported)"
                    );
                }
            }
            _ => {
                bail!("struct.{} not implemented", method);
            }
        }
    }

    /// Try to convert json module method calls
    ///
    #[inline]
    fn try_convert_json_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need serde_json crate
        self.ctx.needs_serde_json = true;

        let result = match method {
            // String serialization/deserialization
            "dumps" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("json.dumps() requires 1 or 2 arguments");
                }
                let obj = &arg_exprs[0];

                // json.dumps(result, indent=2) has 2 arguments after HIR conversion
                // (keyword args become positional args in HIR)
                if arg_exprs.len() >= 2 {
                    // json.dumps(obj, indent=n) → serde_json::to_string_pretty(&obj).unwrap()
                    parse_quote! { serde_json::to_string_pretty(&#obj).unwrap() }
                } else {
                    // json.dumps(obj) → serde_json::to_string(&obj).unwrap()
                    parse_quote! { serde_json::to_string(&#obj).unwrap() }
                }
            }

            "loads" => {
                if arg_exprs.len() != 1 {
                    bail!("json.loads() requires exactly 1 argument");
                }
                let s = &arg_exprs[0];
                // json.loads(s) → serde_json::from_str(&s).unwrap()
                // Returns serde_json::Value (dynamic JSON value)
                parse_quote! { serde_json::from_str::<serde_json::Value>(&#s).unwrap() }
            }

            // File-based serialization/deserialization
            "dump" => {
                if arg_exprs.len() != 2 {
                    bail!("json.dump() requires exactly 2 arguments (obj, file)");
                }
                let obj = &arg_exprs[0];
                let file = &arg_exprs[1];
                // json.dump(obj, file) → serde_json::to_writer(file, &obj).unwrap()
                parse_quote! { serde_json::to_writer(#file, &#obj).unwrap() }
            }

            "load" => {
                if arg_exprs.len() != 1 {
                    bail!("json.load() requires exactly 1 argument (file)");
                }
                let file = &arg_exprs[0];
                // json.load(file) → serde_json::from_reader(file).unwrap()
                parse_quote! { serde_json::from_reader::<_, serde_json::Value>(#file).unwrap() }
            }

            _ => {
                bail!("json.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert unicodedata module method calls
    ///
    /// Maps Python unicodedata functions to Rust unicode-normalization crate:
    /// - unicodedata.normalize("NFC", s) → s.nfc().collect::<String>()
    /// - unicodedata.normalize("NFD", s) → s.nfd().collect::<String>()
    /// - unicodedata.normalize("NFKC", s) → s.nfkc().collect::<String>()
    /// - unicodedata.normalize("NFKD", s) → s.nfkd().collect::<String>()
    ///
    #[inline]
    fn try_convert_unicodedata_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need unicode-normalization crate
        self.ctx.needs_unicode_normalization = true;

        let result = match method {
            "normalize" => {
                if arg_exprs.len() != 2 {
                    bail!("unicodedata.normalize() requires exactly 2 arguments (form, string)");
                }

                // Extract the normalization form (should be a string literal like "NFC", "NFD", etc.)
                let form_arg = &args[0];
                let string_arg = &arg_exprs[1];

                // Try to extract the string literal value for the form
                let form = match form_arg {
                    HirExpr::Literal(Literal::String(s)) => s.as_str(),
                    _ => bail!("unicodedata.normalize() first argument must be a string literal"),
                };

                // Map to the appropriate unicode-normalization method
                match form {
                    "NFC" => {
                        // unicodedata.normalize("NFC", s) → s.nfc().collect::<String>()
                        // The UnicodeNormalization trait is imported at the top of the file
                        parse_quote! { (#string_arg).nfc().collect::<String>() }
                    }
                    "NFD" => {
                        parse_quote! { (#string_arg).nfd().collect::<String>() }
                    }
                    "NFKC" => {
                        parse_quote! { (#string_arg).nfkc().collect::<String>() }
                    }
                    "NFKD" => {
                        parse_quote! { (#string_arg).nfkd().collect::<String>() }
                    }
                    _ => bail!(
                        "Unsupported normalization form: {}. Supported forms are NFC, NFD, NFKC, NFKD",
                        form
                    ),
                }
            }

            "category" | "name" | "lookup" => {
                // These functions don't have direct equivalents in unicode-normalization crate
                // They would need a different crate like unicode-width or a custom implementation
                bail!(
                    "unicodedata.{} is not yet supported - no direct Rust equivalent in unicode-normalization crate",
                    method
                )
            }

            _ => {
                bail!("unicodedata.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert re (regular expressions) module method calls
    ///
    ///
    /// Maps Python re module functions to Rust regex crate:
    /// - re.search() → Regex::new().find()
    /// - re.match() → Regex::new().is_match() with ^ anchor
    /// - re.findall() → Regex::new().find_iter()
    /// - re.sub() → Regex::new().replace_all()
    /// - re.split() → Regex::new().split()
    /// - re.compile() → Regex::new()
    /// - re.escape() → regex::escape()
    ///
    /// # Complexity
    /// 10 (match with 10 branches)
    #[inline]
    fn try_convert_re_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need regex crate
        self.ctx.needs_regex = true;

        let result = match method {
            // Pattern matching functions
            "search" => {
                if arg_exprs.len() < 2 {
                    bail!("re.search() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // Handle optional flags (simplified - just check for IGNORECASE)
                if arg_exprs.len() >= 3 {
                    // With flags: use RegexBuilder
                    parse_quote! {
                        regex::RegexBuilder::new(#pattern)
                            .case_insensitive(true)
                            .build()
                            .unwrap()
                            .find(#text)
                    }
                } else {
                    // No flags: direct Regex::new()
                    // re.search(pattern, text) → Regex::new(pattern).unwrap().find(text)
                    parse_quote! { regex::Regex::new(#pattern).unwrap().find(#text) }
                }
            }

            "match" => {
                if arg_exprs.len() < 2 {
                    bail!("re.match() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // Returns Option<Match> to support .group() calls
                // NOTE: Add start-of-string constraint in future (check match.start() == 0 or prepend ^) ()
                // For now, using .find() like search() - compatible with Match object usage
                parse_quote! { regex::Regex::new(#pattern).unwrap().find(#text) }
            }

            "fullmatch" => {
                if arg_exprs.len() < 2 {
                    bail!("re.fullmatch() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // re.fullmatch(pattern, text) → matches entire string
                // In Rust regex, we use is_match with anchors or capture the whole string
                parse_quote! {
                    regex::Regex::new(#pattern).unwrap().find(#text).filter(|m| m.start() == 0 && m.end() == #text.len())
                }
            }

            "findall" => {
                if arg_exprs.len() < 2 {
                    bail!("re.findall() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // re.findall(pattern, text) → Regex::new(pattern).unwrap().find_iter(text).map(|m| m.as_str()).collect::<Vec<_>>()
                parse_quote! {
                    regex::Regex::new(#pattern)
                        .unwrap()
                        .find_iter(#text)
                        .map(|m| m.as_str().to_string())
                        .collect::<Vec<_>>()
                }
            }

            "finditer" => {
                if arg_exprs.len() < 2 {
                    bail!("re.finditer() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // re.finditer(pattern, text) → Regex::new(pattern).unwrap().find_iter(text)
                parse_quote! {
                    regex::Regex::new(#pattern)
                        .unwrap()
                        .find_iter(#text)
                        .map(|m| m.as_str().to_string())
                        .collect::<Vec<_>>()
                }
            }

            // String substitution
            "sub" => {
                if arg_exprs.len() < 3 {
                    bail!("re.sub() requires at least 3 arguments (pattern, repl, string)");
                }
                let pattern = &arg_exprs[0];
                let repl = &arg_exprs[1];
                let text = &arg_exprs[2];

                // re.sub(pattern, repl, text) → Regex::new(pattern).unwrap().replace_all(text, repl)
                parse_quote! {
                    regex::Regex::new(#pattern)
                        .unwrap()
                        .replace_all(#text, #repl)
                        .to_string()
                }
            }

            "subn" => {
                if arg_exprs.len() < 3 {
                    bail!("re.subn() requires at least 3 arguments (pattern, repl, string)");
                }
                let pattern = &arg_exprs[0];
                let repl = &arg_exprs[1];
                let text = &arg_exprs[2];

                // re.subn(pattern, repl, text) → returns (result, count)
                parse_quote! {
                    {
                        let re = regex::Regex::new(#pattern).unwrap();
                        let count = re.find_iter(#text).count();
                        let result = re.replace_all(#text, #repl).to_string();
                        (result, count)
                    }
                }
            }

            // Pattern compilation
            "compile" => {
                if arg_exprs.is_empty() {
                    bail!("re.compile() requires at least 1 argument (pattern)");
                }
                let pattern = &arg_exprs[0];

                // Check for flags
                if arg_exprs.len() >= 2 {
                    // With flags: use RegexBuilder
                    // For now, simplified handling of common flags
                    parse_quote! {
                        regex::RegexBuilder::new(#pattern)
                            .case_insensitive(true)
                            .build()
                            .unwrap()
                    }
                } else {
                    // No flags: direct Regex::new()
                    // re.compile(pattern) → Regex::new(pattern).unwrap()
                    parse_quote! { regex::Regex::new(#pattern).unwrap() }
                }
            }

            // String splitting
            "split" => {
                if arg_exprs.len() < 2 {
                    bail!("re.split() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // Check for maxsplit argument
                if arg_exprs.len() >= 3 {
                    let maxsplit = &arg_exprs[2];
                    // re.split(pattern, text, maxsplit) → Regex::new(pattern).unwrap().splitn(maxsplit + 1, text)
                    parse_quote! {
                        regex::Regex::new(#pattern)
                            .unwrap()
                            .splitn(#text, #maxsplit + 1)
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    }
                } else {
                    // re.split(pattern, text) → Regex::new(pattern).unwrap().split(text)
                    parse_quote! {
                        regex::Regex::new(#pattern)
                            .unwrap()
                            .split(#text)
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    }
                }
            }

            // Escaping special characters
            "escape" => {
                if arg_exprs.len() != 1 {
                    bail!("re.escape() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // re.escape(text) → regex::escape(text)
                parse_quote! { regex::escape(#text).to_string() }
            }

            _ => {
                bail!("re.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert string module method calls
    ///
    ///
    /// Maps Python string module functions to Rust equivalents:
    /// - string.capwords() → split/capitalize/join
    /// - string.Template → String formatting
    ///
    /// # Complexity
    /// 2 (match with 2 branches)
    #[inline]
    fn try_convert_string_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // String utilities
            "capwords" => {
                if arg_exprs.is_empty() {
                    bail!("string.capwords() requires at least 1 argument (text)");
                }
                let text = &arg_exprs[0];

                // string.capwords(text) → text.split_whitespace().map(|w| {
                //     let mut c = w.chars();
                //     match c.next() {
                //         None => String::new(),
                //         Some(f) => f.to_uppercase().collect::<String>() + c.as_str()
                //     }
                // }).collect::<Vec<_>>().join(" ")
                parse_quote! {
                    #text.split_whitespace()
                        .map(|w| {
                            let mut chars = w.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    let mut result = first.to_uppercase().collect::<String>();
                                    result.push_str(&chars.as_str().to_lowercase());
                                    result
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            }

            _ => {
                bail!("string.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert time module method calls
    ///
    ///
    /// Maps Python time module functions to Rust equivalents:
    /// - time.time() → SystemTime::now()
    /// - time.sleep() → thread::sleep()
    /// - time.monotonic() → Instant::now()
    ///
    /// # Complexity
    /// 7 (match with 7+ branches)
    #[inline]
    fn try_convert_time_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Basic time measurement
            "time" => {
                // time.time() → SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64()
                parse_quote! {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                }
            }

            "monotonic" | "perf_counter" => {
                // time.monotonic() → Instant::now() (returns Instant, need elapsed)
                // For now, simplified: just generate the call
                // In real usage, user would call .elapsed() later
                parse_quote! { std::time::Instant::now() }
            }

            "process_time" => {
                // time.process_time() → CPU time (requires platform-specific code)
                // Simplified: use Instant as approximation
                parse_quote! { std::time::Instant::now() }
            }

            "thread_time" => {
                // time.thread_time() → thread-specific time
                // Simplified: use Instant
                parse_quote! { std::time::Instant::now() }
            }

            // Sleep function
            "sleep" => {
                if arg_exprs.len() != 1 {
                    bail!("time.sleep() requires exactly 1 argument (seconds)");
                }
                let seconds = &arg_exprs[0];

                // time.sleep(seconds) → thread::sleep(Duration::from_secs_f64(seconds))
                parse_quote! {
                    std::thread::sleep(std::time::Duration::from_secs_f64(#seconds))
                }
            }

            // Time formatting (requires chrono for full support)
            "ctime" => {
                self.ctx.needs_chrono = true;
                if arg_exprs.len() != 1 {
                    bail!("time.ctime() requires exactly 1 argument (timestamp)");
                }
                let timestamp = &arg_exprs[0];

                // time.ctime(timestamp) → chrono formatting
                // Simplified: convert timestamp to DateTime
                parse_quote! {
                    {
                        let secs = #timestamp as i64;
                        let nanos = ((#timestamp - secs as f64) * 1_000_000_000.0) as u32;
                        chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nanos)
                            .unwrap()
                            .to_string()
                    }
                }
            }

            "strftime" => {
                self.ctx.needs_chrono = true;
                if arg_exprs.len() < 2 {
                    bail!("time.strftime() requires at least 2 arguments (format, time_tuple)");
                }
                let format = &arg_exprs[0];
                let _time_tuple = &arg_exprs[1];

                // time.strftime(format, time_tuple) → chrono formatting
                // Simplified: assume current time for now
                parse_quote! {
                    chrono::Local::now().format(#format).to_string()
                }
            }

            "strptime" => {
                self.ctx.needs_chrono = true;
                if arg_exprs.len() < 2 {
                    bail!("time.strptime() requires at least 2 arguments (string, format)");
                }
                let time_str = &arg_exprs[0];
                let format = &arg_exprs[1];

                // time.strptime(string, format) → chrono parsing
                parse_quote! {
                    chrono::NaiveDateTime::parse_from_str(#time_str, #format).unwrap()
                }
            }

            // Time conversion
            "gmtime" => {
                self.ctx.needs_chrono = true;
                let timestamp = if arg_exprs.is_empty() {
                    parse_quote! { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64() }
                } else {
                    arg_exprs[0].clone()
                };

                // time.gmtime(timestamp) → chrono UTC conversion
                parse_quote! {
                    {
                        let secs = #timestamp as i64;
                        let nanos = ((#timestamp - secs as f64) * 1_000_000_000.0) as u32;
                        chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nanos).unwrap()
                    }
                }
            }

            "localtime" => {
                self.ctx.needs_chrono = true;
                let timestamp = if arg_exprs.is_empty() {
                    parse_quote! { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64() }
                } else {
                    arg_exprs[0].clone()
                };

                // time.localtime(timestamp) → chrono Local conversion
                parse_quote! {
                    {
                        let secs = #timestamp as i64;
                        let nanos = ((#timestamp - secs as f64) * 1_000_000_000.0) as u32;
                        chrono::DateTime::<chrono::Local>::from_timestamp(secs, nanos).unwrap()
                    }
                }
            }

            "mktime" => {
                self.ctx.needs_chrono = true;
                if arg_exprs.len() != 1 {
                    bail!("time.mktime() requires exactly 1 argument (time_tuple)");
                }
                let time_tuple = &arg_exprs[0];

                // time.mktime(time_tuple) → timestamp conversion
                // Simplified: assume time_tuple is a chrono DateTime
                parse_quote! { #time_tuple.timestamp() as f64 }
            }

            "asctime" => {
                self.ctx.needs_chrono = true;
                if arg_exprs.len() != 1 {
                    bail!("time.asctime() requires exactly 1 argument (time_tuple)");
                }
                let time_tuple = &arg_exprs[0];

                // time.asctime(time_tuple) → ASCII time string
                parse_quote! { #time_tuple.to_string() }
            }

            _ => {
                bail!("time.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert csv module method calls
    ///
    ///
    /// Maps Python csv module to Rust csv crate:
    /// - csv.reader() → csv::Reader::from_reader()
    /// - csv.writer() → csv::Writer::from_writer()
    /// - csv.DictReader → csv with headers
    /// - csv.DictWriter → csv with headers
    ///
    /// # Complexity
    /// 4 (match with 4 branches - simplified for core operations)
    #[inline]
    fn try_convert_csv_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need csv crate
        self.ctx.needs_csv = true;

        let result = match method {
            // CSV Reader
            "reader" => {
                if arg_exprs.is_empty() {
                    bail!("csv.reader() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // csv.reader(file) → csv::Reader::from_reader(file)
                // Note: Real implementation needs more context for delimiter, etc.
                parse_quote! { csv::Reader::from_reader(#file) }
            }

            // CSV Writer
            "writer" => {
                if arg_exprs.is_empty() {
                    bail!("csv.writer() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // csv.writer(file) → csv::Writer::from_writer(file)
                parse_quote! { csv::Writer::from_writer(#file) }
            }

            // DictReader (simplified - actual implementation more complex)
            "DictReader" => {
                if arg_exprs.is_empty() {
                    bail!("csv.DictReader() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // csv.DictReader(file) → csv::ReaderBuilder::new().has_headers(true).from_reader(file)
                parse_quote! {
                    csv::ReaderBuilder::new()
                        .has_headers(true)
                        .from_reader(#file)
                }
            }

            // DictWriter (simplified)
            // csv.DictWriter(file, fieldnames=[...]) or csv.DictWriter(file, fieldnames=...)
            "DictWriter" => {
                // Get file argument (first positional arg required)
                if arg_exprs.is_empty() {
                    bail!("csv.DictWriter() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // Get fieldnames from either positional arg or kwargs
                let _fieldnames = if arg_exprs.len() >= 2 {
                    // Positional: csv.DictWriter(file, ['col1', 'col2'])
                    Some(&arg_exprs[1])
                } else {
                    // Keyword: csv.DictWriter(file, fieldnames=['col1', 'col2'])
                    kwargs
                        .iter()
                        .find(|(key, _)| key == "fieldnames")
                        .map(|(_, value)| value.to_rust_expr(self.ctx))
                        .transpose()?
                        .as_ref()
                        .map(|_| &arg_exprs[0]) // Placeholder, we don't use fieldnames yet
                };

                if _fieldnames.is_none() {
                    bail!("csv.DictWriter() requires fieldnames argument (positional or keyword)");
                }

                // csv.DictWriter(file, fieldnames) → csv::Writer::from_writer(file)
                // Note: fieldnames handling requires more context
                parse_quote! { csv::Writer::from_writer(#file) }
            }

            _ => {
                bail!("csv.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert os module method calls
    ///
    /// Maps Python os module to Rust std::env:
    /// - os.getenv(key) → std::env::var(key)?
    /// - os.getenv(key, default) → std::env::var(key).unwrap_or_else(|_| default.to_string())
    ///
    /// # Complexity
    /// ≤10 (match with few branches)
    #[inline]
    fn try_convert_os_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "getenv" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("os.getenv() requires 1 or 2 arguments");
                }

                if arg_exprs.len() == 1 {
                    // os.getenv("KEY") → std::env::var("KEY")?
                    let key = &arg_exprs[0];
                    parse_quote! { std::env::var(#key)? }
                } else {
                    // os.getenv("KEY", "default") → std::env::var("KEY").unwrap_or_else(|_| "default".to_string())
                    let key = &arg_exprs[0];
                    let default = &arg_exprs[1];

                    // Python's os.getenv() always returns str, so the default must be converted to String.
                    // The unwrap_or_else closure must return String, so we always add .to_string()
                    // to ensure the default value is owned.
                    parse_quote! {
                        std::env::var(#key).unwrap_or_else(|_| #default.to_string())
                    }
                }
            }
            _ => {
                return Ok(None);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert os.environ method calls
    ///
    /// Maps Python os.environ methods to Rust std::env:
    /// - os.environ.get(key) → std::env::var(key).ok()
    /// - os.environ.get(key, default) → std::env::var(key).unwrap_or_else(|_| default.to_string())
    ///
    /// # Complexity
    /// ≤10 (match with few branches)
    #[inline]
    fn try_convert_os_environ_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "get" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("os.environ.get() requires 1 or 2 arguments");
                }

                if arg_exprs.len() == 1 {
                    // os.environ.get("KEY") → std::env::var("KEY").ok()
                    // Returns Option<String>: Some(value) if exists, None otherwise
                    let key = &arg_exprs[0];
                    parse_quote! { std::env::var(#key).ok() }
                } else {
                    // os.environ.get("KEY", "default") → std::env::var("KEY").unwrap_or_else(|_| "default".to_string())
                    // Returns String: value if exists, default otherwise
                    let key = &arg_exprs[0];
                    let default = &arg_exprs[1];
                    parse_quote! {
                        std::env::var(#key).unwrap_or_else(|_| #default.to_string())
                    }
                }
            }
            _ => {
                return Ok(None);
            }
        };

        Ok(Some(result))
    }

    /// Convert subprocess.run() to std::process::Command
    ///
    /// Maps Python subprocess.run() to Rust std::process::Command:
    /// - subprocess.run(cmd) → Command::new(cmd[0]).args(&cmd[1..]).status()
    /// - capture_output=True → .output() instead of .status()
    /// - cwd=path → .current_dir(path)
    /// - check=True → verify exit status (NOTE: add error handling )
    ///
    /// Returns anonymous struct with: returncode, stdout, stderr
    ///
    /// # Complexity
    /// ≤10 (linear processing of kwargs)
    #[inline]
    fn convert_subprocess_run(
        &mut self,
        args: &[HirExpr],
        kwargs: &[(Symbol, HirExpr)],
    ) -> Result<syn::Expr> {
        if args.is_empty() {
            bail!("subprocess.run() requires at least 1 argument (command list)");
        }

        // First argument is the command list
        let cmd_expr = args[0].to_rust_expr(self.ctx)?;

        // Parse keyword arguments
        let mut capture_output = false;
        let mut _text = false;
        let mut cwd_expr: Option<syn::Expr> = None;
        let mut _check = false;

        for (key, value) in kwargs {
            match key.as_str() {
                "capture_output" => {
                    if let HirExpr::Literal(Literal::Bool(b)) = value {
                        capture_output = *b;
                    }
                }
                "text" => {
                    if let HirExpr::Literal(Literal::Bool(b)) = value {
                        _text = *b;
                    }
                }
                "cwd" => {
                    cwd_expr = Some(value.to_rust_expr(self.ctx)?);
                }
                "check" => {
                    if let HirExpr::Literal(Literal::Bool(b)) = value {
                        _check = *b;
                    }
                }
                _ => {} // Ignore unknown kwargs for now
            }
        }

        // Build the Command construction
        // Python: subprocess.run(["echo", "hello"], capture_output=True, cwd="/tmp")
        // Rust: {
        //   let mut cmd = std::process::Command::new(&cmd_list[0]);
        //   cmd.args(&cmd_list[1..]);
        //   if cwd { cmd.current_dir(cwd); }
        //   let output = cmd.output()?;
        //   // Create result struct
        //   SubprocessResult {
        //     returncode: output.status.code().unwrap_or(-1),
        //     stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        //     stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        //   }
        // }

        let result = if capture_output {
            // Use .output() to capture stdout/stderr
            if let Some(cwd) = cwd_expr {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        cmd.current_dir(#cwd);
                        let output = cmd.output().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: output.status.code().unwrap_or(-1),
                            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        }
                    }
                }
            } else {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        let output = cmd.output().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: output.status.code().unwrap_or(-1),
                            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        }
                    }
                }
            }
        } else {
            // Use .status() for exit code only (no capture)
            if let Some(cwd) = cwd_expr {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        cmd.current_dir(#cwd);
                        let status = cmd.status().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: status.code().unwrap_or(-1),
                            stdout: String::new(),
                            stderr: String::new(),
                        }
                    }
                }
            } else {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        let status = cmd.status().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: status.code().unwrap_or(-1),
                            stdout: String::new(),
                            stderr: String::new(),
                        }
                    }
                }
            }
        };

        Ok(result)
    }

    /// Try to convert os.path module method calls
    ///
    ///
    /// Maps Python os.path module to Rust std::path + std::fs:
    /// - os.path.join() → PathBuf::new().join()
    /// - os.path.basename() → Path::file_name()
    /// - os.path.exists() → Path::exists()
    ///
    /// # Complexity
    /// 10 (match with 10 primary branches - split into helper methods as needed)
    #[inline]
    fn try_convert_os_path_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Path construction
            "join" => {
                if arg_exprs.is_empty() {
                    bail!("os.path.join() requires at least 1 argument");
                }

                // os.path.join(a, b, c, ...) → PathBuf::from(a).join(b).join(c)...
                let first = &arg_exprs[0];
                if arg_exprs.len() == 1 {
                    parse_quote! { std::path::PathBuf::from(#first) }
                } else {
                    let mut result: syn::Expr = parse_quote! { std::path::PathBuf::from(#first) };
                    for part in &arg_exprs[1..] {
                        result = parse_quote! { #result.join(#part) };
                    }
                    parse_quote! { #result.to_string_lossy().to_string() }
                }
            }

            // Path decomposition
            "basename" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.basename() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.basename(path) → Path::new(path).file_name()
                parse_quote! {
                    std::path::Path::new(#path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                }
            }

            "dirname" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.dirname() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.dirname(path) → Path::new(path).parent()
                parse_quote! {
                    std::path::Path::new(#path)
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                        .to_string()
                }
            }

            "split" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.split() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.split(path) → (dirname, basename) tuple
                parse_quote! {
                    {
                        let p = std::path::Path::new(#path);
                        let dirname = p.parent().and_then(|p| p.to_str()).unwrap_or("").to_string();
                        let basename = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                        (dirname, basename)
                    }
                }
            }

            "splitext" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.splitext() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.splitext(path) → (stem, extension) tuple
                parse_quote! {
                    {
                        let p = std::path::Path::new(#path);
                        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                        let ext = p.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
                        (stem, ext)
                    }
                }
            }

            // Path predicates
            "exists" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.exists() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.exists(path) → Path::new(path).exists()
                parse_quote! { std::path::Path::new(#path).exists() }
            }

            "isfile" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.isfile() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.isfile(path) → Path::new(path).is_file()
                parse_quote! { std::path::Path::new(#path).is_file() }
            }

            "isdir" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.isdir() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.isdir(path) → Path::new(path).is_dir()
                parse_quote! { std::path::Path::new(#path).is_dir() }
            }

            "isabs" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.isabs() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.isabs(path) → Path::new(path).is_absolute()
                parse_quote! { std::path::Path::new(#path).is_absolute() }
            }

            // Path normalization
            "abspath" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.abspath() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.abspath(path) → std::fs::canonicalize() or manual absolute path
                // Using canonicalize (resolves symlinks too, like realpath)
                parse_quote! {
                    std::fs::canonicalize(#path)
                        .unwrap_or_else(|_| std::path::PathBuf::from(#path))
                        .to_string_lossy()
                        .to_string()
                }
            }

            "normpath" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.normpath() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.normpath(path) → normalize path components
                // Rust Path doesn't have direct normpath, but we can use PathBuf operations
                parse_quote! {
                    {
                        let p = std::path::Path::new(#path);
                        let mut components = Vec::new();
                        for component in p.components() {
                            match component {
                                std::path::Component::CurDir => {},
                                std::path::Component::ParentDir => {
                                    components.pop();
                                }
                                _ => components.push(component),
                            }
                        }
                        components.iter()
                            .map(|c| c.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(std::path::MAIN_SEPARATOR_STR)
                    }
                }
            }

            "realpath" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.realpath() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.realpath(path) → std::fs::canonicalize()
                parse_quote! {
                    std::fs::canonicalize(#path)
                        .unwrap_or_else(|_| std::path::PathBuf::from(#path))
                        .to_string_lossy()
                        .to_string()
                }
            }

            // Path properties
            "getsize" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.getsize() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.getsize(path) → std::fs::metadata().len()
                parse_quote! {
                    std::fs::metadata(#path).unwrap().len() as i64
                }
            }

            "getmtime" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.getmtime() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.getmtime(path) → std::fs::metadata().modified()
                parse_quote! {
                    std::fs::metadata(#path)
                        .unwrap()
                        .modified()
                        .unwrap()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                }
            }

            "getctime" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.getctime() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.getctime(path) → std::fs::metadata().created()
                // Note: On Unix, this is ctime (change time), but Rust only has created()
                parse_quote! {
                    std::fs::metadata(#path)
                        .unwrap()
                        .created()
                        .unwrap()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                }
            }

            // Path expansion
            "expanduser" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.expanduser() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.expanduser(path) → expand ~ to home directory
                parse_quote! {
                    {
                        let p = #path;
                        if p.starts_with("~") {
                            if let Some(home) = std::env::var_os("HOME") {
                                format!("{}{}", home.to_string_lossy(), &p[1..])
                            } else {
                                p.to_string()
                            }
                        } else {
                            p.to_string()
                        }
                    }
                }
            }

            "expandvars" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.expandvars() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.expandvars(path) → expand environment variables
                // Simplified: just return path as-is for now (full implementation complex)
                parse_quote! { #path.to_string() }
            }

            //
            "relpath" => {
                if arg_exprs.len() != 2 {
                    bail!("os.path.relpath() requires exactly 2 arguments");
                }
                let path = &arg_exprs[0];
                let start = &arg_exprs[1];

                // os.path.relpath(path, start) → compute relative path from start to path
                parse_quote! {
                    {
                        let path_obj = std::path::Path::new(#path);
                        let start_obj = std::path::Path::new(#start);
                        path_obj
                            .strip_prefix(start_obj)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| #path.to_string())
                    }
                }
            }

            _ => {
                // For functions not yet implemented, return None to allow fallback
                return Ok(None);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert base64 module method calls
    ///
    ///
    /// Maps Python base64 module to Rust base64 crate:
    /// - base64.b64encode() → base64::encode()
    /// - base64.b64decode() → base64::decode()
    /// - base64.urlsafe_b64encode() → URL-safe encoding
    ///
    /// # Complexity
    /// 10 (match with 10 branches for different encodings)
    #[inline]
    fn try_convert_base64_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need base64 crate
        self.ctx.needs_base64 = true;

        let result = match method {
            // Standard Base64
            "b64encode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b64encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b64encode(data) → base64::engine::general_purpose::STANDARD.encode(data)
                parse_quote! {
                    base64::engine::general_purpose::STANDARD.encode(#data)
                }
            }

            "b64decode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b64decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b64decode(data) → base64::engine::general_purpose::STANDARD.decode(data).unwrap()
                parse_quote! {
                    base64::engine::general_purpose::STANDARD.decode(#data).unwrap()
                }
            }

            // URL-safe Base64
            "urlsafe_b64encode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.urlsafe_b64encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.urlsafe_b64encode(data) → base64::engine::general_purpose::URL_SAFE.encode(data)
                parse_quote! {
                    base64::engine::general_purpose::URL_SAFE.encode(#data)
                }
            }

            "urlsafe_b64decode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.urlsafe_b64decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.urlsafe_b64decode(data) → base64::engine::general_purpose::URL_SAFE.decode(data).unwrap()
                parse_quote! {
                    base64::engine::general_purpose::URL_SAFE.decode(#data).unwrap()
                }
            }

            // Base32 using data-encoding crate
            "b32encode" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 1 {
                    bail!("base64.b32encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b32encode(data) → data_encoding::BASE32.encode(data).into_bytes()
                parse_quote! {
                    data_encoding::BASE32.encode(#data).into_bytes()
                }
            }

            "b32decode" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 1 {
                    bail!("base64.b32decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b32decode(data) → data_encoding::BASE32.decode(data).unwrap()
                parse_quote! {
                    data_encoding::BASE32.decode(#data).unwrap()
                }
            }

            // Base16 (Hex)
            "b16encode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b16encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b16encode(data) → hex::encode_upper(data)
                parse_quote! {
                    hex::encode_upper(#data)
                }
            }

            "b16decode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b16decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b16decode(data) → hex::decode(data).unwrap()
                parse_quote! {
                    hex::decode(#data).unwrap()
                }
            }

            // Base85 (also needs additional crate)
            "b85encode" | "b85decode" => {
                // Simplified: note that full implementation needs additional crate
                bail!(
                    "base64.{} requires base85 encoding crate (not yet integrated)",
                    method
                );
            }

            _ => {
                bail!("base64.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert secrets module method calls
    ///
    ///
    /// Maps Python secrets module to Rust rand crate (cryptographic RNG):
    /// - secrets.randbelow() → rand::thread_rng().gen_range()
    /// - secrets.token_bytes() → Cryptographically secure random bytes
    ///
    /// # Complexity
    /// 5 (match with 5 branches)
    #[inline]
    fn try_convert_secrets_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need rand crate (ThreadRng is cryptographically secure)
        self.ctx.needs_rand = true;
        self.ctx.needs_base64 = true; // For token_urlsafe

        let result = match method {
            // Random number generation
            "randbelow" => {
                if arg_exprs.len() != 1 {
                    bail!("secrets.randbelow() requires exactly 1 argument");
                }
                let n = &arg_exprs[0];

                // secrets.randbelow(n) → rand::thread_rng().gen_range(0..n)
                parse_quote! { rand::thread_rng().gen_range(0..#n) }
            }

            "choice" => {
                if arg_exprs.len() != 1 {
                    bail!("secrets.choice() requires exactly 1 argument");
                }
                let seq = &arg_exprs[0];
                self.ctx.needs_indexed_random = true;

                // secrets.choice(seq) → seq.choose(&mut rand::thread_rng()).cloned().unwrap()
                parse_quote! { #seq.choose(&mut rand::thread_rng()).cloned().unwrap() }
            }

            // Token generation
            "token_bytes" => {
                let nbytes = if arg_exprs.is_empty() {
                    parse_quote! { 32 } // Default 32 bytes
                } else {
                    arg_exprs[0].clone()
                };

                // secrets.token_bytes(n) → generate n random bytes
                parse_quote! {
                    {
                        let mut bytes = vec![0u8; #nbytes];
                        rand::thread_rng().fill(&mut bytes[..]);
                        bytes
                    }
                }
            }

            "token_hex" => {
                let nbytes = if arg_exprs.is_empty() {
                    parse_quote! { 32 }
                } else {
                    arg_exprs[0].clone()
                };

                // secrets.token_hex(n) → generate n random bytes and encode as hex
                parse_quote! {
                    {
                        let mut bytes = vec![0u8; #nbytes];
                        rand::thread_rng().fill(&mut bytes[..]);
                        hex::encode(&bytes)
                    }
                }
            }

            "token_urlsafe" => {
                let nbytes = if arg_exprs.is_empty() {
                    parse_quote! { 32 }
                } else {
                    arg_exprs[0].clone()
                };

                // secrets.token_urlsafe(n) → generate n random bytes and encode as URL-safe base64
                parse_quote! {
                    {
                        let mut bytes = vec![0u8; #nbytes];
                        rand::thread_rng().fill(&mut bytes[..]);
                        base64::engine::general_purpose::URL_SAFE.encode(&bytes)
                    }
                }
            }

            _ => {
                bail!("secrets.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert hashlib module method calls
    ///
    ///
    /// Supports: md5, sha1, sha224, sha256, sha384, sha512, sha3_256, blake2b, blake2s
    /// Returns hex digest directly (one-shot hashing pattern)
    ///
    /// # Complexity
    /// Cyclomatic: 9 (match with 8 algorithms + default)
    #[inline]
    fn try_convert_hashlib_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // All hash functions need hex encoding
        self.ctx.needs_hex = true;

        let result = match method {
            // MD5 hash
            "md5" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.md5() accepts 0 or 1 arguments");
                }
                self.ctx.needs_md5 = true;

                // hashlib.md5(data) → hex::encode(md5::compute(data))
                // If no arguments, use empty bytes (Python default: data=b'')
                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use md5::Digest;
                            let mut hasher = md5::Md5::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use md5::Digest;
                            let mut hasher = md5::Md5::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // SHA-1 hash
            "sha1" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.sha1() requires exactly 1 argument");
                }
                self.ctx.needs_sha2 = true;
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use sha1::Digest;
                        let mut hasher = sha1::Sha1::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // SHA-224 hash
            "sha224" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.sha224() requires exactly 1 argument");
                }
                self.ctx.needs_sha2 = true;
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use sha2::Digest;
                        let mut hasher = sha2::Sha224::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // SHA-256 hash
            "sha256" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.sha256() accepts 0 or 1 arguments");
                }
                self.ctx.needs_sha2 = true;

                // If no arguments, use empty bytes (Python default: data=b'')
                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha256::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha256::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // SHA-384 hash
            "sha384" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.sha384() accepts 0 or 1 arguments");
                }
                self.ctx.needs_sha2 = true;

                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha384::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha384::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // SHA-512 hash
            "sha512" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.sha512() accepts 0 or 1 arguments");
                }
                self.ctx.needs_sha2 = true;

                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha512::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha512::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // BLAKE2b hash
            "blake2b" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.blake2b() requires exactly 1 argument");
                }
                self.ctx.needs_blake2 = true;
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use blake2::Digest;
                        let mut hasher = blake2::Blake2b512::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // BLAKE2s hash
            "blake2s" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.blake2s() requires exactly 1 argument");
                }
                self.ctx.needs_blake2 = true;
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use blake2::Digest;
                        let mut hasher = blake2::Blake2s256::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // SHA3-256 hash
            "sha3_256" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.sha3_256() requires exactly 1 argument");
                }
                self.ctx.needs_sha3 = true;
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use sha3::Digest;
                        let mut hasher = sha3::Sha3_256::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            _ => {
                bail!(
                    "hashlib.{} not implemented yet (try: md5, sha1, sha224, sha256, sha384, sha512, sha3_256, blake2b, blake2s)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert uuid module method calls
    ///
    ///
    /// Supports: uuid1 (time-based), uuid3 (MD5), uuid4 (random), uuid5 (SHA1)
    /// Returns string representation of UUID
    ///
    /// # Complexity
    /// Cyclomatic: 5 (match with 4 functions + default)
    #[inline]
    fn try_convert_uuid_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need uuid crate
        self.ctx.needs_uuid = true;

        let result = match method {
            // UUID v1 - time-based
            "uuid1" => {
                if !arg_exprs.is_empty() {
                    bail!("uuid.uuid1() takes no arguments (node/clock_seq not yet supported)");
                }

                // uuid.uuid1() → Uuid::new_v1(...).to_string()
                // Note: Requires context (timestamp + node ID)
                parse_quote! {
                    {
                        use uuid::Uuid;
                        // Generate time-based UUID v1
                        // Note: Using placeholder implementation (actual v1 needs timestamp context)
                        Uuid::new_v4().to_string()  // NOTE: Implement proper UUID v1 with timestamp ()
                    }
                }
            }

            // UUID v3 - MD5 hash-based
            "uuid3" => {
                if arg_exprs.len() != 2 {
                    bail!("uuid.uuid3() requires exactly 2 arguments (namespace, name)");
                }
                let namespace = &arg_exprs[0];
                let name = &arg_exprs[1];

                // uuid.uuid3(namespace, name) → Uuid::new_v3(&namespace, name.as_bytes()).to_string()
                parse_quote! {
                    {
                        use uuid::Uuid;
                        Uuid::new_v3(&#namespace, #name.as_bytes()).to_string()
                    }
                }
            }

            // UUID v4 - random (most common)
            "uuid4" => {
                if !arg_exprs.is_empty() {
                    bail!("uuid.uuid4() takes no arguments");
                }

                // uuid.uuid4() → Uuid::new_v4().to_string()
                parse_quote! {
                    {
                        use uuid::Uuid;
                        Uuid::new_v4().to_string()
                    }
                }
            }

            // UUID v5 - SHA1 hash-based
            "uuid5" => {
                if arg_exprs.len() != 2 {
                    bail!("uuid.uuid5() requires exactly 2 arguments (namespace, name)");
                }
                let namespace = &arg_exprs[0];
                let name = &arg_exprs[1];

                // uuid.uuid5(namespace, name) → Uuid::new_v5(&namespace, name.as_bytes()).to_string()
                parse_quote! {
                    {
                        use uuid::Uuid;
                        Uuid::new_v5(&#namespace, #name.as_bytes()).to_string()
                    }
                }
            }

            _ => {
                bail!(
                    "uuid.{} not implemented yet (try: uuid1, uuid3, uuid4, uuid5)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert hmac module method calls
    ///
    ///
    /// Supports: new() with SHA256, compare_digest()
    /// Returns hex digest for one-shot HMAC
    ///
    /// # Complexity
    /// Cyclomatic: 3 (match with 2 functions + default)
    #[inline]
    fn try_convert_hmac_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need hmac and related crates
        self.ctx.needs_hmac = true;
        self.ctx.needs_sha2 = true; // For SHA256
        self.ctx.needs_hex = true;

        let result = match method {
            // HMAC creation - simplified to SHA256
            "new" => {
                if arg_exprs.is_empty() {
                    bail!("hmac.new() requires at least 1 argument (key)");
                }
                let key = &arg_exprs[0];

                // Check if we have a message (2nd positional arg) or just key + digestmod
                // hmac.new(key, msg, digestmod) or hmac.new(key, digestmod=hashlib.sha256)
                if arg_exprs.len() >= 2 {
                    let msg = &arg_exprs[1];
                    // hmac.new(key, msg, hashlib.sha256) → HMAC-SHA256 hex digest
                    parse_quote! {
                        {
                            use hmac::{Hmac, Mac};
                            use sha2::Sha256;

                            type HmacSha256 = Hmac<Sha256>;
                            let mut mac = HmacSha256::new_from_slice(#key).expect("HMAC key error");
                            mac.update(#msg);
                            hex::encode(mac.finalize().into_bytes())
                        }
                    }
                } else {
                    // hmac.new(key, digestmod=...) - returns HMAC object for incremental updates
                    // We'll return a tuple of (mac_type, key) that can be used with .update()
                    parse_quote! {
                        {
                            use hmac::{Hmac, Mac};
                            use sha2::Sha256;

                            type HmacSha256 = Hmac<Sha256>;
                            HmacSha256::new_from_slice(#key).expect("HMAC key error")
                        }
                    }
                }
            }

            // Timing-safe comparison
            "compare_digest" => {
                if arg_exprs.len() != 2 {
                    bail!("hmac.compare_digest() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];

                // hmac.compare_digest(a, b) → constant-time comparison
                parse_quote! {
                    {
                        use subtle::ConstantTimeEq;
                        #a.ct_eq(#b).into()
                    }
                }
            }

            _ => {
                bail!(
                    "hmac.{} not implemented yet (try: new, compare_digest)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert platform module method calls
    ///
    /// Maps Python platform module to Rust std::env::consts:
    /// - platform.system() → std::env::consts::OS
    /// - platform.machine() → std::env::consts::ARCH
    /// - platform.python_version() → "3.11.0" (hardcoded constant)
    ///
    /// # Complexity
    /// ≤10 (simple match with few branches)
    #[inline]
    fn try_convert_platform_method(
        &mut self,
        method: &str,
        _args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        let result = match method {
            "system" => {
                // platform.system() → std::env::consts::OS
                // Returns "linux", "macos", "windows", etc.
                parse_quote! { std::env::consts::OS.to_string() }
            }

            "machine" => {
                // platform.machine() → std::env::consts::ARCH
                // Returns "x86_64", "aarch64", etc.
                parse_quote! { std::env::consts::ARCH.to_string() }
            }

            "python_version" => {
                // platform.python_version() → "3.11.0"
                // Hardcoded to Python 3.11 for compatibility
                parse_quote! { "3.11.0".to_string() }
            }

            "release" => {
                // platform.release() → OS release version
                // Note: This is OS-specific and may require additional logic
                parse_quote! { std::env::consts::OS.to_string() }
            }

            _ => {
                bail!(
                    "platform.{} not implemented yet (try: system, machine, python_version, release)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert binascii module method calls
    ///
    ///
    /// Supports: hexlify, unhexlify, b2a_hex, a2b_hex, b2a_base64, a2b_base64, crc32
    /// Common encoding/decoding operations
    ///
    /// # Complexity
    /// Cyclomatic: 8 (match with 7 functions + default)
    #[inline]
    fn try_convert_binascii_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Hex conversions
            "hexlify" | "b2a_hex" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.{}() requires exactly 1 argument", method);
                }
                self.ctx.needs_hex = true;
                let data = &arg_exprs[0];

                // binascii.hexlify(data) → hex::encode(data) as bytes
                parse_quote! {
                    hex::encode(#data).into_bytes()
                }
            }

            "unhexlify" | "a2b_hex" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.{}() requires exactly 1 argument", method);
                }
                self.ctx.needs_hex = true;
                let data = &arg_exprs[0];

                // binascii.unhexlify(data) → hex::decode(data)
                parse_quote! {
                    hex::decode(#data).expect("Invalid hex string")
                }
            }

            // Base64 conversions
            "b2a_base64" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.b2a_base64() requires exactly 1 argument");
                }
                self.ctx.needs_base64 = true;
                let data = &arg_exprs[0];

                // binascii.b2a_base64(data) → base64::encode(data) with newline
                parse_quote! {
                    {
                        let mut result = base64::engine::general_purpose::STANDARD.encode(#data);
                        result.push('\n');
                        result.into_bytes()
                    }
                }
            }

            "a2b_base64" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.a2b_base64() requires exactly 1 argument");
                }
                self.ctx.needs_base64 = true;
                let data = &arg_exprs[0];

                // binascii.a2b_base64(data) → base64::decode(data)
                parse_quote! {
                    base64::engine::general_purpose::STANDARD.decode(#data).expect("Invalid base64 string")
                }
            }

            // Quoted-printable encoding
            "b2a_qp" => {
                if arg_exprs.is_empty() {
                    bail!("binascii.b2a_qp() requires at least 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified implementation - basic quoted-printable
                // NOTE: Full RFC 1521 quoted-printable implementation ()
                parse_quote! {
                    {
                        // Simple QP: replace special chars, preserve printable ASCII
                        let bytes: &[u8] = #data;
                        let mut result = Vec::new();
                        for &b in bytes {
                            if b >= 33 && b <= 126 && b != b'=' {
                                result.push(b);
                            } else {
                                result.extend(format!("={:02X}", b).as_bytes());
                            }
                        }
                        result
                    }
                }
            }

            "a2b_qp" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.a2b_qp() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified QP decoder
                // NOTE: Full RFC 1521 quoted-printable implementation ()
                parse_quote! {
                    {
                        let s = std::str::from_utf8(#data).expect("Invalid UTF-8");
                        let mut result = Vec::new();
                        let mut chars = s.chars().peekable();
                        while let Some(c) = chars.next() {
                            if c == '=' {
                                let h1 = chars.next().unwrap_or('0');
                                let h2 = chars.next().unwrap_or('0');
                                let hex = format!("{}{}", h1, h2);
                                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                                    result.push(b);
                                }
                            } else {
                                result.push(c as u8);
                            }
                        }
                        result
                    }
                }
            }

            // UU encoding
            "b2a_uu" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.b2a_uu() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified UU encoding (basic implementation)
                // NOTE: Full UU encoding with proper line wrapping ()
                parse_quote! {
                    {
                        let bytes: &[u8] = #data;
                        let len = bytes.len();
                        let mut result = vec![(len as u8 + 32)]; // Length byte
                        for chunk in bytes.chunks(3) {
                            let mut val = 0u32;
                            for (i, &b) in chunk.iter().enumerate() {
                                val |= (b as u32) << (16 - i * 8);
                            }
                            for i in 0..4 {
                                let b = ((val >> (18 - i * 6)) & 0x3F) as u8;
                                result.push(b + 32);
                            }
                        }
                        result.push(b'\n');
                        result
                    }
                }
            }

            "a2b_uu" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.a2b_uu() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified UU decoding (basic implementation)
                // NOTE: Full UU decoding implementation ()
                parse_quote! {
                    {
                        let bytes: &[u8] = #data;
                        if bytes.is_empty() {
                            Vec::new()
                        } else {
                            let len = (bytes[0].wrapping_sub(32)) as usize;
                            let mut result = Vec::with_capacity(len);
                            for chunk in bytes[1..].chunks(4) {
                                if chunk.len() < 4 { break; }
                                let mut val = 0u32;
                                for (i, &b) in chunk.iter().enumerate() {
                                    val |= ((b.wrapping_sub(32) & 0x3F) as u32) << (18 - i * 6);
                                }
                                for i in 0..3 {
                                    if result.len() < len {
                                        result.push((val >> (16 - i * 8)) as u8);
                                    }
                                }
                            }
                            result
                        }
                    }
                }
            }

            // CRC32 checksum
            "crc32" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("binascii.crc32() requires 1 or 2 arguments");
                }
                self.ctx.needs_crc32 = true;
                let data = &arg_exprs[0];

                if arg_exprs.len() == 1 {
                    // binascii.crc32(data) → crc32 checksum as u32
                    parse_quote! {
                        {
                            use crc32fast::Hasher;
                            let mut hasher = Hasher::new();
                            hasher.update(#data);
                            hasher.finalize() as i32
                        }
                    }
                } else {
                    // binascii.crc32(data, crc) → update existing crc
                    let crc = &arg_exprs[1];
                    parse_quote! {
                        {
                            use crc32fast::Hasher;
                            let mut hasher = Hasher::new_with_initial(#crc as u32);
                            hasher.update(#data);
                            hasher.finalize() as i32
                        }
                    }
                }
            }

            _ => {
                bail!(
                    "binascii.{} not implemented yet (available: hexlify, unhexlify, b2a_hex, a2b_hex, b2a_base64, a2b_base64, b2a_qp, a2b_qp, b2a_uu, a2b_uu, crc32)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert urllib.parse module method calls
    ///
    ///
    /// Supports: quote, unquote, quote_plus, unquote_plus, urlencode, parse_qs
    /// Common URL encoding/decoding operations
    ///
    /// # Complexity
    /// Cyclomatic: 7 (match with 6 functions + default)
    #[inline]
    fn try_convert_urllib_parse_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need URL encoding support
        self.ctx.needs_url_encoding = true;

        let result = match method {
            // Percent encoding
            "quote" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.quote() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // quote(text) → percent-encode URL component
                parse_quote! {
                    {
                        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                        utf8_percent_encode(#text, NON_ALPHANUMERIC).to_string()
                    }
                }
            }

            "unquote" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.unquote() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // unquote(text) → percent-decode URL component
                parse_quote! {
                    {
                        use percent_encoding::percent_decode_str;
                        percent_decode_str(#text).decode_utf8_lossy().to_string()
                    }
                }
            }

            // Percent encoding with + for spaces (form encoding)
            "quote_plus" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.quote_plus() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // quote_plus(text) → percent-encode with + for spaces
                parse_quote! {
                    {
                        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                        utf8_percent_encode(#text, NON_ALPHANUMERIC)
                            .to_string()
                            .replace("%20", "+")
                    }
                }
            }

            "unquote_plus" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.unquote_plus() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // unquote_plus(text) → percent-decode with + as space
                parse_quote! {
                    {
                        use percent_encoding::percent_decode_str;
                        let replaced = (#text).replace("+", " ");
                        percent_decode_str(&replaced).decode_utf8_lossy().to_string()
                    }
                }
            }

            // Query string encoding
            "urlencode" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.urlencode() requires exactly 1 argument");
                }
                let params = &arg_exprs[0];

                // urlencode(dict) → key1=value1&key2=value2
                parse_quote! {
                    {
                        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                        #params.iter()
                            .map(|(k, v)| {
                                let key = utf8_percent_encode(&k.to_string(), NON_ALPHANUMERIC).to_string();
                                let val = utf8_percent_encode(&v.to_string(), NON_ALPHANUMERIC).to_string();
                                format!("{}={}", key, val)
                            })
                            .collect::<Vec<_>>()
                            .join("&")
                    }
                }
            }

            // Query string parsing
            "parse_qs" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.parse_qs() requires exactly 1 argument");
                }
                let qs = &arg_exprs[0];

                // parse_qs(qs) → HashMap<String, Vec<String>>
                parse_quote! {
                    {
                        use percent_encoding::percent_decode_str;
                        use std::collections::HashMap;

                        let mut result: HashMap<String, Vec<String>> = HashMap::new();
                        for pair in (#qs).split('&') {
                            if let Some((key, value)) = pair.split_once('=') {
                                let decoded_key = percent_decode_str(key).decode_utf8_lossy().to_string();
                                let decoded_value = percent_decode_str(value).decode_utf8_lossy().to_string();
                                result.entry(decoded_key).or_insert_with(Vec::new).push(decoded_value);
                            }
                        }
                        result
                    }
                }
            }

            _ => {
                bail!(
                    "urllib.parse.{} not implemented yet (available: quote, unquote, quote_plus, unquote_plus, urlencode, parse_qs)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert fnmatch module method calls
    ///
    ///
    /// Supports: fnmatch, fnmatchcase, filter, translate
    /// Shell wildcard patterns: *, ?, [seq], [!seq]
    ///
    /// # Complexity
    /// Cyclomatic: 5 (match with 4 functions + default)
    #[inline]
    fn try_convert_fnmatch_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // fnmatch needs regex crate for pattern matching
        self.ctx.needs_regex = true;

        let result = match method {
            // Basic pattern matching
            "fnmatch" | "fnmatchcase" => {
                if arg_exprs.len() != 2 {
                    bail!("fnmatch.{}() requires exactly 2 arguments", method);
                }
                let name = &arg_exprs[0];
                let pattern = &arg_exprs[1];

                // Simplified implementation: convert pattern to regex and match
                // NOTE: Proper fnmatch pattern translation with case sensitivity ()
                parse_quote! {
                    {
                        // Convert fnmatch pattern to regex
                        let pattern_str = #pattern;
                        let regex_pattern = pattern_str
                            .replace(".", "\\.")
                            .replace("*", ".*")
                            .replace("?", ".")
                            .replace("[!", "[^");

                        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))
                            .unwrap_or_else(|_| regex::Regex::new("^$").unwrap());

                        regex.is_match(#name)
                    }
                }
            }

            // Filter list by pattern
            "filter" => {
                if arg_exprs.len() != 2 {
                    bail!("fnmatch.filter() requires exactly 2 arguments");
                }
                let names = &arg_exprs[0];
                let pattern = &arg_exprs[1];

                // filter(names, pattern) → names matching pattern
                parse_quote! {
                    {
                        let pattern_str = #pattern;
                        let regex_pattern = pattern_str
                            .replace(".", "\\.")
                            .replace("*", ".*")
                            .replace("?", ".")
                            .replace("[!", "[^");

                        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))
                            .unwrap_or_else(|_| regex::Regex::new("^$").unwrap());

                        (#names).into_iter()
                            .filter(|name| regex.is_match(&name.to_string()))
                            .collect::<Vec<_>>()
                    }
                }
            }

            // Translate pattern to regex
            "translate" => {
                if arg_exprs.len() != 1 {
                    bail!("fnmatch.translate() requires exactly 1 argument");
                }
                let pattern = &arg_exprs[0];

                // translate(pattern) → regex string
                parse_quote! {
                    {
                        let pattern_str = #pattern;
                        let regex_pattern = pattern_str
                            .replace(".", "\\.")
                            .replace("*", ".*")
                            .replace("?", ".")
                            .replace("[!", "[^");

                        format!("(?ms)^{}$", regex_pattern)
                    }
                }
            }

            _ => {
                bail!(
                    "fnmatch.{} not implemented yet (available: fnmatch, fnmatchcase, filter, translate)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert shlex module method calls
    ///
    ///
    /// Supports: split, quote, join
    /// Security-critical: prevents shell injection
    ///
    /// # Complexity
    /// Cyclomatic: 4 (match with 3 functions + default)
    #[inline]
    fn try_convert_shlex_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Shell-like split (respects quotes and escapes)
            "split" => {
                if arg_exprs.len() != 1 {
                    bail!("shlex.split() requires exactly 1 argument");
                }
                let s = &arg_exprs[0];

                // Simplified shell split (handles basic quotes)
                // NOTE: Use shell-words crate for full POSIX shell compliance ()
                parse_quote! {
                    {
                        let input = #s;
                        let mut result = Vec::new();
                        let mut current = String::new();
                        let mut in_single_quote = false;
                        let mut in_double_quote = false;
                        let mut escaped = false;

                        for c in input.chars() {
                            if escaped {
                                current.push(c);
                                escaped = false;
                            } else if c == '\\' && !in_single_quote {
                                escaped = true;
                            } else if c == '\'' && !in_double_quote {
                                in_single_quote = !in_single_quote;
                            } else if c == '"' && !in_single_quote {
                                in_double_quote = !in_double_quote;
                            } else if c.is_whitespace() && !in_single_quote && !in_double_quote {
                                if !current.is_empty() {
                                    result.push(current.clone());
                                    current.clear();
                                }
                            } else {
                                current.push(c);
                            }
                        }

                        if !current.is_empty() {
                            result.push(current);
                        }

                        result
                    }
                }
            }

            // Shell-safe quoting
            "quote" => {
                if arg_exprs.len() != 1 {
                    bail!("shlex.quote() requires exactly 1 argument");
                }
                let s = &arg_exprs[0];

                // Quote string for safe shell usage
                parse_quote! {
                    {
                        let input = #s;
                        // Check if needs quoting
                        let needs_quoting = input.chars().any(|c| {
                            matches!(c, ' ' | '\t' | '\n' | '\'' | '"' | '\\' | '|' | '&' | ';' |
                                     '(' | ')' | '<' | '>' | '`' | '$' | '*' | '?' | '[' | ']' |
                                     '{' | '}' | '!' | '#' | '~')
                        });

                        if needs_quoting || input.is_empty() {
                            // Use single quotes and escape any single quotes
                            format!("'{}'", input.replace("'", "'\"'\"'"))
                        } else {
                            input.to_string()
                        }
                    }
                }
            }

            // Join list with shell-safe quoting
            "join" => {
                if arg_exprs.len() != 1 {
                    bail!("shlex.join() requires exactly 1 argument");
                }
                let args_list = &arg_exprs[0];

                // Join args with proper quoting
                parse_quote! {
                    {
                        let args = #args_list;
                        args.iter()
                            .map(|arg| {
                                let s = arg.to_string();
                                let needs_quoting = s.chars().any(|c| {
                                    matches!(c, ' ' | '\t' | '\n' | '\'' | '"' | '\\' | '|' | '&' | ';' |
                                             '(' | ')' | '<' | '>' | '`' | '$' | '*' | '?' | '[' | ']' |
                                             '{' | '}' | '!' | '#' | '~')
                                });

                                if needs_quoting || s.is_empty() {
                                    format!("'{}'", s.replace("'", "'\"'\"'"))
                                } else {
                                    s
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                }
            }

            _ => {
                bail!(
                    "shlex.{} not implemented yet (available: split, quote, join)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert textwrap module method calls
    ///
    ///
    /// Supports: wrap, fill, dedent, indent, shorten
    /// Text formatting for display and documentation
    ///
    /// # Complexity
    /// Cyclomatic: 6 (match with 5 functions + default)
    #[inline]
    fn try_convert_textwrap_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Wrap text into list of lines
            "wrap" => {
                if arg_exprs.len() < 2 {
                    bail!("textwrap.wrap() requires at least 2 arguments (text, width)");
                }
                let text = &arg_exprs[0];
                let width = &arg_exprs[1];

                // Simple word-wrapping algorithm
                parse_quote! {
                    {
                        let text = #text;
                        let width = #width as usize;
                        let mut lines = Vec::new();
                        let mut current_line = String::new();
                        let mut current_len = 0;

                        for word in text.split_whitespace() {
                            let word_len = word.len();
                            if current_len == 0 {
                                current_line = word.to_string();
                                current_len = word_len;
                            } else if current_len + 1 + word_len <= width {
                                current_line.push(' ');
                                current_line.push_str(word);
                                current_len += 1 + word_len;
                            } else {
                                lines.push(current_line);
                                current_line = word.to_string();
                                current_len = word_len;
                            }
                        }

                        if !current_line.is_empty() {
                            lines.push(current_line);
                        }

                        lines
                    }
                }
            }

            // Wrap and join into single string
            "fill" => {
                if arg_exprs.len() < 2 {
                    bail!("textwrap.fill() requires at least 2 arguments (text, width)");
                }
                let text = &arg_exprs[0];
                let width = &arg_exprs[1];

                // fill = wrap + join
                parse_quote! {
                    {
                        let text = #text;
                        let width = #width as usize;
                        let mut lines = Vec::new();
                        let mut current_line = String::new();
                        let mut current_len = 0;

                        for word in text.split_whitespace() {
                            let word_len = word.len();
                            if current_len == 0 {
                                current_line = word.to_string();
                                current_len = word_len;
                            } else if current_len + 1 + word_len <= width {
                                current_line.push(' ');
                                current_line.push_str(word);
                                current_len += 1 + word_len;
                            } else {
                                lines.push(current_line);
                                current_line = word.to_string();
                                current_len = word_len;
                            }
                        }

                        if !current_line.is_empty() {
                            lines.push(current_line);
                        }

                        lines.join("\n")
                    }
                }
            }

            // Remove common leading whitespace
            "dedent" => {
                if arg_exprs.len() != 1 {
                    bail!("textwrap.dedent() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                parse_quote! {
                    {
                        let text = #text;
                        let lines: Vec<&str> = text.lines().collect();

                        // Find minimum indentation (excluding empty lines)
                        let min_indent = lines.iter()
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
                            .min()
                            .unwrap_or(0);

                        // Remove that many spaces from each line
                        lines.iter()
                            .map(|line| {
                                if line.len() >= min_indent {
                                    &line[min_indent..]
                                } else {
                                    line
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }

            // Add prefix to each line
            "indent" => {
                if arg_exprs.len() != 2 {
                    bail!("textwrap.indent() requires exactly 2 arguments (text, prefix)");
                }
                let text = &arg_exprs[0];
                let prefix = &arg_exprs[1];

                parse_quote! {
                    {
                        let text = #text;
                        let prefix = #prefix;
                        text.lines()
                            .map(|line| format!("{}{}", prefix, line))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }

            // Shorten text with ellipsis
            "shorten" => {
                if arg_exprs.len() < 2 {
                    bail!("textwrap.shorten() requires at least 2 arguments (text, width)");
                }
                let text = &arg_exprs[0];
                let width = &arg_exprs[1];

                parse_quote! {
                    {
                        let text = #text;
                        let width = #width as usize;
                        let placeholder = " [...]";

                        if text.len() <= width {
                            text.to_string()
                        } else if width < placeholder.len() {
                            text.chars().take(width).collect()
                        } else {
                            let max_len = width - placeholder.len();
                            let truncated: String = text.chars().take(max_len).collect();
                            format!("{}{}", truncated, placeholder)
                        }
                    }
                }
            }

            _ => {
                bail!(
                    "textwrap.{} not implemented yet (available: wrap, fill, dedent, indent, shorten)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert bisect module method calls
    ///
    ///
    /// Supports: bisect_left, bisect_right, insort_left, insort_right
    /// Efficient O(log n) search and insertion
    ///
    /// # Complexity
    /// Cyclomatic: 5 (match with 4 functions + default)
    #[inline]
    fn try_convert_bisect_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Find leftmost insertion point
            "bisect_left" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.bisect_left() requires at least 2 arguments");
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = #a;
                        let val = &#x;
                        match arr.binary_search(val) {
                            Ok(mut pos) => {
                                while pos > 0 && &arr[pos - 1] == val {
                                    pos -= 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        }
                    }
                }
            }

            // Find rightmost insertion point
            "bisect_right" | "bisect" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.{}() requires at least 2 arguments", method);
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = #a;
                        let val = &#x;
                        match arr.binary_search(val) {
                            Ok(mut pos) => {
                                pos += 1;
                                while pos < arr.len() && &arr[pos] == val {
                                    pos += 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        }
                    }
                }
            }

            // Insert at leftmost position
            "insort_left" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.insort_left() requires at least 2 arguments");
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = &mut (#a);
                        let val = #x;
                        let pos = match arr.binary_search(&val) {
                            Ok(mut pos) => {
                                while pos > 0 && arr[pos - 1] == val {
                                    pos -= 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        };
                        arr.insert(pos, val);
                    }
                }
            }

            // Insert at rightmost position
            "insort_right" | "insort" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.{}() requires at least 2 arguments", method);
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = &mut (#a);
                        let val = #x;
                        let pos = match arr.binary_search(&val) {
                            Ok(mut pos) => {
                                pos += 1;
                                while pos < arr.len() && arr[pos] == val {
                                    pos += 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        };
                        arr.insert(pos, val);
                    }
                }
            }

            _ => {
                bail!(
                    "bisect.{} not implemented yet (available: bisect_left, bisect_right, insort_left, insort_right)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert heapq module method calls
    ///
    ///
    /// Supports: heapify, heappush, heappop, nlargest, nsmallest
    /// Python heapq is a MIN heap (smallest item first)
    ///
    /// # Complexity
    /// Cyclomatic: 6 (match with 5 functions + default)
    #[inline]
    fn try_convert_heapq_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Transform list into min-heap in-place
            "heapify" => {
                if arg_exprs.is_empty() {
                    bail!("heapq.heapify() requires at least 1 argument");
                }
                let x = &arg_exprs[0];

                parse_quote! {
                    {
                        let heap = &mut (#x);
                        // Build min-heap using bottom-up heapify
                        let len = heap.len();
                        if len > 1 {
                            for i in (0..len/2).rev() {
                                let mut pos = i;
                                loop {
                                    let left = 2 * pos + 1;
                                    let right = 2 * pos + 2;
                                    let mut smallest = pos;

                                    if left < len && heap[left] < heap[smallest] {
                                        smallest = left;
                                    }
                                    if right < len && heap[right] < heap[smallest] {
                                        smallest = right;
                                    }

                                    if smallest == pos {
                                        break;
                                    }

                                    heap.swap(pos, smallest);
                                    pos = smallest;
                                }
                            }
                        }
                    }
                }
            }

            // Push item onto min-heap
            "heappush" => {
                if arg_exprs.len() < 2 {
                    bail!("heapq.heappush() requires at least 2 arguments");
                }
                let heap = &arg_exprs[0];
                let item = &arg_exprs[1];

                parse_quote! {
                    {
                        let heap = &mut (#heap);
                        let item = #item;
                        heap.push(item);

                        // Bubble up to maintain min-heap property
                        let mut pos = heap.len() - 1;
                        while pos > 0 {
                            let parent = (pos - 1) / 2;
                            if heap[pos] >= heap[parent] {
                                break;
                            }
                            heap.swap(pos, parent);
                            pos = parent;
                        }
                    }
                }
            }

            // Pop and return smallest item from min-heap
            "heappop" => {
                if arg_exprs.is_empty() {
                    bail!("heapq.heappop() requires at least 1 argument");
                }
                let heap = &arg_exprs[0];

                parse_quote! {
                    {
                        let heap = &mut (#heap);
                        if heap.is_empty() {
                            panic!("heappop from empty heap");
                        }

                        let result = heap[0].clone();
                        let last = heap.pop().unwrap();

                        if !heap.is_empty() {
                            heap[0] = last;

                            // Bubble down to maintain min-heap property
                            let mut pos = 0;
                            loop {
                                let left = 2 * pos + 1;
                                let right = 2 * pos + 2;
                                let mut smallest = pos;

                                if left < heap.len() && heap[left] < heap[smallest] {
                                    smallest = left;
                                }
                                if right < heap.len() && heap[right] < heap[smallest] {
                                    smallest = right;
                                }

                                if smallest == pos {
                                    break;
                                }

                                heap.swap(pos, smallest);
                                pos = smallest;
                            }
                        }

                        result
                    }
                }
            }

            // Return n largest elements
            "nlargest" => {
                if arg_exprs.len() < 2 {
                    bail!("heapq.nlargest() requires at least 2 arguments");
                }
                let n = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let n = #n as usize;
                        let mut items = #iterable;
                        items.sort_by(|a, b| b.cmp(a));  // Sort descending
                        items.into_iter().take(n).collect::<Vec<_>>()
                    }
                }
            }

            // Return n smallest elements
            "nsmallest" => {
                if arg_exprs.len() < 2 {
                    bail!("heapq.nsmallest() requires at least 2 arguments");
                }
                let n = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let n = #n as usize;
                        let mut items = #iterable;
                        items.sort();  // Sort ascending
                        items.into_iter().take(n).collect::<Vec<_>>()
                    }
                }
            }

            _ => {
                bail!(
                    "heapq.{} not implemented yet (available: heapify, heappush, heappop, nlargest, nsmallest)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert copy module method calls
    ///
    ///
    /// Supports: copy, deepcopy
    /// Maps to Rust's .clone() for both (Rust clone is deep by default)
    ///
    /// # Complexity
    /// Cyclomatic: 3 (match with 2 functions + default)
    #[inline]
    fn try_convert_copy_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Shallow copy - in Rust, clone() is typically deep for owned data
            "copy" => {
                if arg_exprs.is_empty() {
                    bail!("copy.copy() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                parse_quote! {
                    (#obj).clone()
                }
            }

            // Deep copy - in Rust, clone() already performs deep copy
            "deepcopy" => {
                if arg_exprs.is_empty() {
                    bail!("copy.deepcopy() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                parse_quote! {
                    (#obj).clone()
                }
            }

            _ => {
                bail!(
                    "copy.{} not implemented yet (available: copy, deepcopy)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert itertools module method calls
    ///
    ///
    /// Supports: count, cycle, repeat, chain, islice, takewhile
    /// Maps to Rust's iterator adapters and std::iter methods
    ///
    /// # Complexity
    /// Cyclomatic: 7 (match with 6 functions + default)
    #[inline]
    fn try_convert_itertools_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Infinite counter with optional step
            "count" => {
                let start = if !arg_exprs.is_empty() {
                    &arg_exprs[0]
                } else {
                    &parse_quote!(0)
                };
                let step = if arg_exprs.len() >= 2 {
                    &arg_exprs[1]
                } else {
                    &parse_quote!(1)
                };

                parse_quote! {
                    {
                        let start = #start;
                        let step = #step;
                        std::iter::successors(Some(start), move |&n| Some(n + step))
                    }
                }
            }

            // Cycle through iterable infinitely
            "cycle" => {
                if arg_exprs.is_empty() {
                    bail!("itertools.cycle() requires at least 1 argument");
                }
                let iterable = &arg_exprs[0];

                parse_quote! {
                    {
                        let items = #iterable;
                        items.into_iter().cycle()
                    }
                }
            }

            // Repeat value n times (or infinitely if no count)
            "repeat" => {
                if arg_exprs.is_empty() {
                    bail!("itertools.repeat() requires at least 1 argument");
                }
                let value = &arg_exprs[0];

                if arg_exprs.len() >= 2 {
                    let times = &arg_exprs[1];
                    parse_quote! {
                        {
                            let val = #value;
                            let n = #times as usize;
                            std::iter::repeat(val).take(n)
                        }
                    }
                } else {
                    parse_quote! {
                        {
                            let val = #value;
                            std::iter::repeat(val)
                        }
                    }
                }
            }

            // Chain multiple iterables together
            "chain" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.chain() requires at least 2 arguments");
                }

                // Chain first two, then fold the rest
                let first = &arg_exprs[0];
                let second = &arg_exprs[1];

                if arg_exprs.len() == 2 {
                    parse_quote! {
                        {
                            let a = #first;
                            let b = #second;
                            a.into_iter().chain(b.into_iter())
                        }
                    }
                } else {
                    // For more than 2, we need to chain them all
                    let mut chain_expr: syn::Expr = parse_quote! {
                        #first.into_iter().chain(#second.into_iter())
                    };

                    for item in &arg_exprs[2..] {
                        chain_expr = parse_quote! {
                            #chain_expr.chain(#item.into_iter())
                        };
                    }

                    chain_expr
                }
            }

            // Slice iterator with start, stop, step
            "islice" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.islice() requires at least 2 arguments");
                }
                let iterable = &arg_exprs[0];

                if arg_exprs.len() == 2 {
                    // islice(iterable, stop)
                    let stop = &arg_exprs[1];
                    parse_quote! {
                        {
                            let items = #iterable;
                            let n = #stop as usize;
                            items.into_iter().take(n)
                        }
                    }
                } else {
                    // islice(iterable, start, stop)
                    let start = &arg_exprs[1];
                    let stop = &arg_exprs[2];
                    parse_quote! {
                        {
                            let items = #iterable;
                            let start_idx = #start as usize;
                            let stop_idx = #stop as usize;
                            items.into_iter().skip(start_idx).take(stop_idx - start_idx)
                        }
                    }
                }
            }

            // Take while predicate is true
            "takewhile" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.takewhile() requires at least 2 arguments");
                }
                let predicate = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let pred = #predicate;
                        let items = #iterable;
                        items.into_iter().take_while(pred)
                    }
                }
            }

            // Drop while predicate is true
            "dropwhile" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.dropwhile() requires at least 2 arguments");
                }
                let predicate = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let pred = #predicate;
                        let items = #iterable;
                        items.into_iter().skip_while(pred)
                    }
                }
            }

            // Accumulate (running sum/product)
            "accumulate" => {
                if arg_exprs.is_empty() {
                    bail!("itertools.accumulate() requires at least 1 argument");
                }
                let iterable = &arg_exprs[0];

                // accumulate with default + operation
                parse_quote! {
                    {
                        let items = #iterable;
                        let mut acc = None;
                        items.into_iter().map(|x| {
                            acc = Some(match acc {
                                None => x,
                                Some(a) => a + x,
                            });
                            acc.unwrap()
                        }).collect::<Vec<_>>()
                    }
                }
            }

            // Compress - filter by selector booleans
            "compress" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.compress() requires at least 2 arguments");
                }
                let data = &arg_exprs[0];
                let selectors = &arg_exprs[1];

                parse_quote! {
                    {
                        let items = #data;
                        let sels = #selectors;
                        items.into_iter()
                            .zip(sels.into_iter())
                            .filter_map(|(item, sel)| if sel { Some(item) } else { None })
                            .collect::<Vec<_>>()
                    }
                }
            }

            // Combinations - generate k-length combinations from iterable
            "combinations" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.combinations() requires 2 arguments (iterable, r)");
                }
                let iterable = &arg_exprs[0];
                let r = &arg_exprs[1];

                parse_quote! {
                    {
                        use itertools::Itertools;
                        let items: Vec<_> = #iterable.into_iter().collect();
                        items.into_iter().combinations(#r as usize)
                    }
                }
            }

            // Permutations - generate k-length permutations from iterable
            "permutations" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.permutations() requires 2 arguments (iterable, r)");
                }
                let iterable = &arg_exprs[0];
                let r = &arg_exprs[1];

                parse_quote! {
                    {
                        use itertools::Itertools;
                        let items: Vec<_> = #iterable.into_iter().collect();
                        items.into_iter().permutations(#r as usize)
                    }
                }
            }

            // Groupby - group consecutive elements by key function
            "groupby" => {
                // First arg is the iterable
                if arg_exprs.is_empty() {
                    bail!("itertools.groupby() requires at least 1 argument (iterable)");
                }
                let iterable = &arg_exprs[0];

                // Try to find key in kwargs
                let key_func_expr =
                    if let Some((_, key_expr)) = kwargs.iter().find(|(k, _)| k == "key") {
                        key_expr.to_rust_expr(self.ctx)?
                    } else if arg_exprs.len() >= 2 {
                        arg_exprs[1].clone()
                    } else {
                        // Default key: identity function
                        parse_quote! { |x| x }
                    };

                parse_quote! {
                    {
                        use itertools::Itertools;
                        let items = #iterable;
                        let key_fn = #key_func_expr;
                        items.into_iter().group_by(key_fn)
                    }
                }
            }

            _ => {
                bail!(
                    "itertools.{} not implemented yet (available: count, cycle, repeat, chain, islice, takewhile, dropwhile, accumulate, compress, combinations, permutations, groupby)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert functools module method calls
    ///
    ///
    /// Supports: reduce
    /// Maps to Rust's Iterator::fold() method
    ///
    /// # Complexity
    /// Cyclomatic: 2 (match with 1 function + default)
    #[inline]
    fn try_convert_functools_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Reduce/fold operation
            "reduce" => {
                if arg_exprs.len() < 2 {
                    bail!("functools.reduce() requires at least 2 arguments");
                }
                let function = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                if arg_exprs.len() >= 3 {
                    // With initial value
                    let initial = &arg_exprs[2];
                    parse_quote! {
                        {
                            let func = #function;
                            let items = #iterable;
                            let init = #initial;
                            items.into_iter().fold(init, func)
                        }
                    }
                } else {
                    // Without initial value - use first element
                    parse_quote! {
                        {
                            let func = #function;
                            let mut items = (#iterable).into_iter();
                            let init = items.next().expect("reduce() of empty sequence with no initial value");
                            items.fold(init, func)
                        }
                    }
                }
            }

            _ => {
                bail!(
                    "functools.{} not implemented yet (available: reduce)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert warnings module method calls
    ///
    ///
    /// Supports: warn
    /// Maps to Rust's eprintln! macro for stderr output
    ///
    /// # Complexity
    /// Cyclomatic: 2 (match with 1 function + default)
    #[inline]
    fn try_convert_warnings_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "warn" => {
                if arg_exprs.is_empty() {
                    bail!("warnings.warn() requires at least 1 argument");
                }
                let message = &arg_exprs[0];

                parse_quote! {
                    eprintln!("Warning: {}", #message)
                }
            }

            _ => {
                bail!("warnings.{} not implemented yet (available: warn)", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert sys module method calls
    ///
    ///
    /// Supports: exit
    /// Maps to Rust's std::process::exit
    ///
    /// # Complexity
    /// Cyclomatic: 2 (match with 1 function + default)
    #[inline]
    fn try_convert_sys_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "exit" => {
                let code = if !arg_exprs.is_empty() {
                    &arg_exprs[0]
                } else {
                    &parse_quote!(0)
                };

                parse_quote! {
                    std::process::exit(#code)
                }
            }

            _ => {
                bail!("sys.{} not implemented yet (available: exit)", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert pickle module method calls
    ///
    ///
    /// Supports: dumps, loads
    /// Maps to serde/bincode for serialization (placeholder)
    ///
    /// # Complexity
    /// Cyclomatic: 3 (match with 2 functions + default)
    #[inline]
    fn try_convert_pickle_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "dumps" => {
                if arg_exprs.is_empty() {
                    bail!("pickle.dumps() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                // Placeholder: In real implementation, would use serde + bincode
                parse_quote! {
                    {
                        // Note: Actual pickle serialization requires serde support
                        format!("{:?}", #obj).into_bytes()
                    }
                }
            }

            "loads" => {
                if arg_exprs.is_empty() {
                    bail!("pickle.loads() requires at least 1 argument");
                }
                let data = &arg_exprs[0];

                // Placeholder: In real implementation, would use serde + bincode
                parse_quote! {
                    {
                        // Note: Actual pickle deserialization requires serde support
                        String::from_utf8_lossy(#data).to_string()
                    }
                }
            }

            _ => {
                bail!(
                    "pickle.{} not implemented yet (available: dumps, loads)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert pprint module method calls
    ///
    ///
    /// Supports: pprint
    /// Maps to Rust's Debug formatting
    ///
    /// # Complexity
    /// Cyclomatic: 2 (match with 1 function + default)
    #[inline]
    fn try_convert_pprint_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "pprint" => {
                if arg_exprs.is_empty() {
                    bail!("pprint.pprint() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                parse_quote! {
                    println!("{:#?}", #obj)
                }
            }

            _ => {
                bail!("pprint.{} not implemented yet (available: pprint)", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert fractions module method calls
    ///
    #[inline]
    fn try_convert_fractions_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Mark that we need the num-rational crate
        self.ctx.needs_num_rational = true;

        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Fraction methods
            "limit_denominator" => {
                if arg_exprs.len() != 2 {
                    bail!(
                        "Fraction.limit_denominator() requires exactly 2 arguments (self, max_denominator)"
                    );
                }
                let frac = &arg_exprs[0];
                let max_denom = &arg_exprs[1];
                // Simplified: if denominator within limit, return as-is
                parse_quote! {
                    {
                        let f = #frac;
                        let max_d = #max_denom as i32;
                        if *f.denom() <= max_d {
                            f
                        } else {
                            // Approximate by converting to float and back
                            num::rational::Ratio::approximate_float(f.to_f64().unwrap()).unwrap_or(f)
                        }
                    }
                }
            }

            "as_integer_ratio" => {
                if arg_exprs.len() != 1 {
                    bail!("Fraction.as_integer_ratio() requires exactly 1 argument (self)");
                }
                let frac = &arg_exprs[0];
                parse_quote! { (*#frac.numer(), *#frac.denom()) }
            }

            _ => return Ok(None), // Not a recognized fractions method
        };

        Ok(Some(result))
    }

    /// Try to convert pathlib module method calls
    ///
    #[inline]
    fn try_convert_pathlib_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Path queries
            "exists" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.exists() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.exists() }
            }

            "is_file" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.is_file() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.is_file() }
            }

            "is_dir" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.is_dir() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.is_dir() }
            }

            "is_absolute" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.is_absolute() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.is_absolute() }
            }

            // Path transformations
            "absolute" | "resolve" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.{}() requires exactly 1 argument (self)", method);
                }
                let path = &arg_exprs[0];
                // Both absolute() and resolve() → canonicalize()
                parse_quote! { #path.canonicalize().unwrap() }
            }

            "with_name" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.with_name() requires exactly 2 arguments (self, name)");
                }
                let path = &arg_exprs[0];
                let name = &arg_exprs[1];
                parse_quote! { #path.with_file_name(#name) }
            }

            "with_suffix" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.with_suffix() requires exactly 2 arguments (self, suffix)");
                }
                let path = &arg_exprs[0];
                let suffix = &arg_exprs[1];
                parse_quote! { #path.with_extension(#suffix.trim_start_matches('.')) }
            }

            // Directory operations
            "mkdir" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("Path.mkdir() requires 1-2 arguments");
                }
                let path = &arg_exprs[0];

                // Check if parents=True was passed (simplified - assumes second arg is parents)
                if arg_exprs.len() == 2 {
                    // mkdir(parents=True) → create_dir_all
                    parse_quote! { std::fs::create_dir_all(#path).unwrap() }
                } else {
                    // mkdir() → create_dir
                    parse_quote! { std::fs::create_dir(#path).unwrap() }
                }
            }

            "rmdir" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.rmdir() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::remove_dir(#path).unwrap() }
            }

            "iterdir" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.iterdir() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! {
                    std::fs::read_dir(#path)
                        .unwrap()
                        .map(|e| e.unwrap().path())
                        .collect::<Vec<_>>()
                }
            }

            // File operations
            "read_text" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.read_text() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::read_to_string(#path).unwrap() }
            }

            "read_bytes" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.read_bytes() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::read(#path).unwrap() }
            }

            "write_text" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.write_text() requires exactly 2 arguments (self, content)");
                }
                let path = &arg_exprs[0];
                let content = &arg_exprs[1];
                parse_quote! { std::fs::write(#path, #content).unwrap() }
            }

            "write_bytes" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.write_bytes() requires exactly 2 arguments (self, content)");
                }
                let path = &arg_exprs[0];
                let content = &arg_exprs[1];
                parse_quote! { std::fs::write(#path, #content).unwrap() }
            }

            "unlink" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.unlink() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::remove_file(#path).unwrap() }
            }

            "rename" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.rename() requires exactly 2 arguments (self, target)");
                }
                let path = &arg_exprs[0];
                let target = &arg_exprs[1];
                parse_quote! { { std::fs::rename(&#path, #target).unwrap(); std::path::PathBuf::from(#target) } }
            }

            // Conversions
            "as_posix" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.as_posix() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.to_str().unwrap().to_string() }
            }

            _ => return Ok(None), // Not a recognized pathlib method
        };

        Ok(Some(result))
    }

    /// Try to convert datetime module method calls
    ///
    #[inline]
    fn try_convert_datetime_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Mark that we need the chrono crate
        self.ctx.needs_chrono = true;

        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // datetime.datetime.now() → Local::now()
            "now" => {
                if arg_exprs.is_empty() {
                    parse_quote! { chrono::Local::now().naive_local() }
                } else {
                    bail!("datetime.now() takes no arguments");
                }
            }

            // datetime.datetime.utcnow() → Utc::now()
            "utcnow" => {
                if arg_exprs.is_empty() {
                    parse_quote! { chrono::Utc::now().naive_utc() }
                } else {
                    bail!("datetime.utcnow() takes no arguments");
                }
            }

            // datetime.datetime.today() → Local::now().date()
            "today" => {
                if arg_exprs.is_empty() {
                    parse_quote! { chrono::Local::now().date_naive() }
                } else {
                    bail!("datetime.today() takes no arguments");
                }
            }

            // datetime.datetime.strftime(format) → dt.format(format).to_string()
            "strftime" => {
                if arg_exprs.len() != 2 {
                    bail!("strftime() requires exactly 2 arguments (self, format)");
                }
                let dt = &arg_exprs[0];
                let fmt = &arg_exprs[1];
                parse_quote! { #dt.format(#fmt).to_string() }
            }

            // datetime.datetime.strptime(string, format) → NaiveDateTime::parse_from_str(string, format)
            "strptime" => {
                if arg_exprs.len() != 2 {
                    bail!("strptime() requires exactly 2 arguments (string, format)");
                }
                let s = &arg_exprs[0];
                let fmt = &arg_exprs[1];
                parse_quote! {
                    chrono::NaiveDateTime::parse_from_str(#s, #fmt).unwrap()
                }
            }

            // datetime.datetime.isoformat() → dt.to_rfc3339()
            "isoformat" => {
                if arg_exprs.len() != 1 {
                    bail!("isoformat() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.to_string() }
            }

            // datetime.datetime.timestamp() → dt.timestamp()
            "timestamp" => {
                if arg_exprs.len() != 1 {
                    bail!("timestamp() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.and_utc().timestamp() as f64 }
            }

            // datetime.datetime.fromtimestamp(ts) → NaiveDateTime::from_timestamp(ts, 0)
            "fromtimestamp" => {
                if arg_exprs.len() != 1 {
                    bail!("fromtimestamp() requires exactly 1 argument (timestamp)");
                }
                let ts = &arg_exprs[0];
                parse_quote! {
                    chrono::DateTime::from_timestamp(#ts as i64, 0)
                        .unwrap()
                        .naive_local()
                }
            }

            // date.weekday() → dt.weekday().num_days_from_monday()
            "weekday" => {
                if arg_exprs.len() != 1 {
                    bail!("weekday() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.weekday().num_days_from_monday() as i32 }
            }

            // date.isoweekday() → dt.weekday().number_from_monday()
            "isoweekday" => {
                if arg_exprs.len() != 1 {
                    bail!("isoweekday() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                // ISO weekday: Monday=1, Sunday=7
                parse_quote! { (#dt.weekday().num_days_from_monday() + 1) as i32 }
            }

            // timedelta.total_seconds() → duration.num_seconds() as f64
            "total_seconds" => {
                if arg_exprs.len() != 1 {
                    bail!("total_seconds() requires exactly 1 argument (self)");
                }
                let td = &arg_exprs[0];
                parse_quote! { #td.num_seconds() as f64 }
            }

            // datetime.date() → extract date part
            "date" => {
                if arg_exprs.len() != 1 {
                    bail!("date() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.date() }
            }

            // datetime.time() → extract time part
            "time" => {
                if arg_exprs.len() != 1 {
                    bail!("time() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.time() }
            }

            // datetime.replace(year=..., month=..., day=..., ...)
            "replace" => {
                if arg_exprs.len() != 2 {
                    bail!("replace() not fully implemented (requires keyword args)");
                }
                // Simplified: assume single year replacement
                let dt = &arg_exprs[0];
                let new_year = &arg_exprs[1];
                parse_quote! { #dt.with_year(#new_year as i32).unwrap() }
            }

            _ => return Ok(None), // Not a recognized datetime method
        };

        Ok(Some(result))
    }

    /// Try to convert statistics module method calls
    ///
    #[inline]
    fn try_convert_decimal_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Mark that we need the rust_decimal crate
        self.ctx.needs_rust_decimal = true;

        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Mathematical operations
            "sqrt" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.sqrt() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.sqrt().unwrap() }
            }

            "exp" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.exp() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.exp() }
            }

            "ln" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.ln() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.ln() }
            }

            "log10" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.log10() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.log10() }
            }

            // Rounding and quantization
            "quantize" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.quantize() requires exactly 1 argument");
                }
                let value = &arg_exprs[0];
                // quantize(Decimal("0.01")) → round to 2 decimal places
                // For now, we'll use round_dp(2) as a simple approximation
                // NOTE: More sophisticated Decimal quantization based on quantum value ()
                parse_quote! { #value.round_dp(2) }
            }

            "to_integral" | "to_integral_value" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.to_integral() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.trunc() }
            }

            // Predicates
            "is_nan" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_nan() requires exactly 1 argument");
                }
                let _arg = &arg_exprs[0];
                // rust_decimal doesn't have NaN, always returns false
                parse_quote! { false }
            }

            "is_infinite" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_infinite() requires exactly 1 argument");
                }
                let _arg = &arg_exprs[0];
                // rust_decimal doesn't have infinity, always returns false
                parse_quote! { false }
            }

            "is_finite" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_finite() requires exactly 1 argument");
                }
                let _arg = &arg_exprs[0];
                // rust_decimal doesn't have infinity/NaN, always returns true
                parse_quote! { true }
            }

            "is_signed" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_signed() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.is_sign_negative() }
            }

            "is_zero" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_zero() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.is_zero() }
            }

            // Sign operations
            "copy_sign" | "copysign" => {
                if arg_exprs.len() != 2 {
                    bail!("Decimal.copy_sign() requires exactly 2 arguments");
                }
                let value = &arg_exprs[0];
                let other = &arg_exprs[1];
                // Copy sign: if other is negative, return -abs(value), else abs(value)
                parse_quote! {
                    if #other.is_sign_negative() {
                        -#value.abs()
                    } else {
                        #value.abs()
                    }
                }
            }

            // Comparison
            "compare" => {
                if arg_exprs.len() != 2 {
                    bail!("Decimal.compare() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // compare() returns -1, 0, or 1
                parse_quote! {
                    match #a.cmp(&#b) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    }
                }
            }

            _ => return Ok(None), // Not a recognized decimal method
        };

        Ok(Some(result))
    }

    fn try_convert_statistics_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Averages and central tendency
            "mean" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.mean() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.mean(data) → data.iter().sum::<f64>() / data.len() as f64
                parse_quote! {
                    {
                        let data = #data;
                        data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64
                    }
                }
            }

            "median" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.median() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.median(data) → sorted median calculation
                parse_quote! {
                    {
                        let mut sorted = #data.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let len = sorted.len();
                        if len % 2 == 0 {
                            let mid = len / 2;
                            ((sorted[mid - 1] as f64) + (sorted[mid] as f64)) / 2.0
                        } else {
                            sorted[len / 2] as f64
                        }
                    }
                }
            }

            "mode" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.mode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.mode(data) → find most common element
                self.ctx.needs_hashmap = true;
                parse_quote! {
                    {
                        let mut counts: HashMap<_, usize> = HashMap::new();
                        for &item in #data.iter() {
                            *counts.entry(item).or_insert(0) += 1;
                        }
                        *counts.iter().max_by_key(|(_, &count)| count).unwrap().0
                    }
                }
            }

            // Measures of spread
            "variance" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.variance() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.variance(data) → sample variance (n-1 denominator)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        sum_sq_diff / ((data.len() - 1) as f64)
                    }
                }
            }

            "pvariance" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.pvariance() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.pvariance(data) → population variance (n denominator)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        sum_sq_diff / (data.len() as f64)
                    }
                }
            }

            "stdev" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.stdev() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.stdev(data) → sqrt(variance)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        let variance = sum_sq_diff / ((data.len() - 1) as f64);
                        variance.sqrt()
                    }
                }
            }

            "pstdev" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.pstdev() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.pstdev(data) → sqrt(pvariance)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        let pvariance = sum_sq_diff / (data.len() as f64);
                        pvariance.sqrt()
                    }
                }
            }

            // Additional means
            "harmonic_mean" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.harmonic_mean() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.harmonic_mean(data) → n / sum(1/x for x in data)
                parse_quote! {
                    {
                        let data = #data;
                        let sum_reciprocals: f64 = data.iter()
                            .map(|&x| 1.0 / (x as f64))
                            .sum();
                        (data.len() as f64) / sum_reciprocals
                    }
                }
            }

            "geometric_mean" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.geometric_mean() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.geometric_mean(data) → (product of all values) ^ (1/n)
                parse_quote! {
                    {
                        let data = #data;
                        let product: f64 = data.iter()
                            .map(|&x| x as f64)
                            .product();
                        product.powf(1.0 / (data.len() as f64))
                    }
                }
            }

            // Quantiles (simplified implementation)
            "quantiles" => {
                // quantiles can take n= parameter, but we'll support basic case
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("statistics.quantiles() requires 1-2 arguments");
                }
                let data = &arg_exprs[0];
                let n = if arg_exprs.len() == 2 {
                    &arg_exprs[1]
                } else {
                    // Default n=4 (quartiles)
                    &parse_quote! { 4 }
                };
                // Simplified quantiles implementation
                parse_quote! {
                    {
                        let mut sorted = #data.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let n = #n as usize;
                        let mut result = Vec::new();
                        for i in 1..n {
                            let pos = (i as f64) * (sorted.len() as f64) / (n as f64);
                            let idx = pos.floor() as usize;
                            if idx < sorted.len() {
                                result.push(sorted[idx] as f64);
                            }
                        }
                        result
                    }
                }
            }

            _ => {
                bail!("statistics.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert random module method calls
    /// Uses SmallRng with thread-local initialization for performance.
    /// RNG is initialized once per module via DEPYLER_RNG thread_local static.
    #[inline]
    fn try_convert_random_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need rand crate and SmallRng
        // The module-level DEPYLER_RNG thread_local will be generated in generate_conditional_imports
        self.ctx.needs_rand = true;
        self.ctx.needs_small_rng = true;

        // All random operations use the module-level DEPYLER_RNG thread_local
        // which is initialized once per thread when first accessed

        let result = match method {
            // Random class constructor: random.Random(seed) → SmallRng::seed_from_u64(seed)
            "Random" => {
                if arg_exprs.is_empty() {
                    // No seed - use OS entropy
                    parse_quote! { SmallRng::from_os_rng() }
                } else if arg_exprs.len() == 1 {
                    let seed = &arg_exprs[0];
                    parse_quote! { SmallRng::seed_from_u64(#seed as u64) }
                } else {
                    bail!("random.Random() takes 0 or 1 argument");
                }
            }

            // SystemRandom class constructor: random.SystemRandom() → OsRng
            "SystemRandom" => {
                if !arg_exprs.is_empty() {
                    bail!("random.SystemRandom() takes no arguments");
                }
                parse_quote! { rand::rngs::OsRng }
            }

            // Basic random generation
            "random" => {
                if !arg_exprs.is_empty() {
                    bail!("random.random() takes no arguments");
                }
                // random.random() → use module-level SmallRng
                parse_quote! {
                    DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>())
                }
            }

            // Integer range functions
            "randint" => {
                if arg_exprs.len() != 2 {
                    bail!("random.randint() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // random.randint(a, b) → SmallRng.gen_range(a..=b)
                // Python's randint is inclusive on both ends
                parse_quote! {
                    DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(#a..=#b))
                }
            }

            "randrange" => {
                // randrange can take 1, 2, or 3 arguments (like range)
                if arg_exprs.is_empty() || arg_exprs.len() > 3 {
                    bail!("random.randrange() requires 1-3 arguments");
                }

                if arg_exprs.len() == 1 {
                    // randrange(stop) → gen_range(0..stop)
                    let stop = &arg_exprs[0];
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(0..#stop))
                    }
                } else if arg_exprs.len() == 2 {
                    // randrange(start, stop) → gen_range(start..stop)
                    let start = &arg_exprs[0];
                    let stop = &arg_exprs[1];
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(#start..#stop))
                    }
                } else {
                    // randrange(start, stop, step) - stepped range selection
                    let start = &arg_exprs[0];
                    let stop = &arg_exprs[1];
                    let step = &arg_exprs[2];
                    parse_quote! {
                        {
                            let start = #start;
                            let stop = #stop;
                            let step = #step;
                            let num_steps = ((stop - start) / step).max(0);
                            let offset = DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(0..num_steps));
                            start + offset * step
                        }
                    }
                }
            }

            // Float range function
            "uniform" => {
                if arg_exprs.len() != 2 {
                    bail!("random.uniform() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                parse_quote! {
                    DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range((#a as f64)..=(#b as f64)))
                }
            }

            // Sequence functions
            "choice" => {
                if arg_exprs.len() != 1 {
                    bail!("random.choice() requires exactly 1 argument");
                }
                let seq = &arg_exprs[0];
                self.ctx.needs_indexed_random = true;
                parse_quote! {
                    DEPYLER_RNG.with(|rng| #seq.choose(&mut *rng.borrow_mut()).cloned().unwrap())
                }
            }

            "shuffle" => {
                if arg_exprs.len() != 1 {
                    bail!("random.shuffle() requires exactly 1 argument");
                }
                let seq = &arg_exprs[0];
                self.ctx.needs_slice_random = true;
                parse_quote! {
                    DEPYLER_RNG.with(|rng| #seq.shuffle(&mut *rng.borrow_mut()))
                }
            }

            "sample" => {
                if arg_exprs.len() != 2 {
                    bail!("random.sample() requires exactly 2 arguments");
                }
                let seq = &arg_exprs[0];
                let k = &arg_exprs[1];
                self.ctx.needs_indexed_random = true;
                parse_quote! {
                    DEPYLER_RNG.with(|rng| {
                        #seq.choose_multiple(&mut *rng.borrow_mut(), #k as usize)
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                }
            }

            "choices" => {
                if arg_exprs.is_empty() {
                    bail!("random.choices() requires at least 1 argument");
                }
                let seq = &arg_exprs[0];
                let k = if arg_exprs.len() > 1 {
                    &arg_exprs[1]
                } else {
                    &parse_quote! { 1 }
                };
                self.ctx.needs_indexed_random = true;
                parse_quote! {
                    DEPYLER_RNG.with(|rng| {
                        let mut rng = rng.borrow_mut();
                        (0..#k)
                            .map(|_| #seq.choose(&mut *rng).cloned().unwrap())
                            .collect::<Vec<_>>()
                    })
                }
            }

            // Distribution functions
            "gauss" | "normalvariate" => {
                if arg_exprs.len() != 2 {
                    bail!("random.{}() requires exactly 2 arguments", method);
                }
                let mu = &arg_exprs[0];
                let sigma = &arg_exprs[1];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let normal = rand_distr::Normal::new(#mu as f64, #sigma as f64).unwrap();
                        DEPYLER_RNG.with(|rng| normal.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "expovariate" => {
                if arg_exprs.len() != 1 {
                    bail!("random.expovariate() requires exactly 1 argument");
                }
                let lambd = &arg_exprs[0];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let exp = rand_distr::Exp::new(#lambd as f64).unwrap();
                        DEPYLER_RNG.with(|rng| exp.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "betavariate" => {
                if arg_exprs.len() != 2 {
                    bail!("random.betavariate() requires exactly 2 arguments");
                }
                let alpha = &arg_exprs[0];
                let beta = &arg_exprs[1];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let beta_dist = rand_distr::Beta::new(#alpha as f64, #beta as f64).unwrap();
                        DEPYLER_RNG.with(|rng| beta_dist.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "gammavariate" => {
                if arg_exprs.len() != 2 {
                    bail!("random.gammavariate() requires exactly 2 arguments");
                }
                let alpha = &arg_exprs[0];
                let beta = &arg_exprs[1];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let gamma = rand_distr::Gamma::new(#alpha as f64, #beta as f64).unwrap();
                        DEPYLER_RNG.with(|rng| gamma.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            // Seed function - resets the module-level RNG
            "seed" => {
                if arg_exprs.len() > 1 {
                    bail!("random.seed() requires 0 or 1 argument");
                }
                if arg_exprs.is_empty() {
                    // seed() with no args - use OS entropy
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| *rng.borrow_mut() = SmallRng::from_os_rng())
                    }
                } else {
                    let seed_val = &arg_exprs[0];
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| *rng.borrow_mut() = SmallRng::seed_from_u64(#seed_val as u64))
                    }
                }
            }

            // Get/Set state (complex, simplified implementation)
            "getstate" => {
                bail!(
                    "random.getstate() not supported - Rust RNG state management differs from Python"
                );
            }
            "setstate" => {
                bail!(
                    "random.setstate() not supported - Rust RNG state management differs from Python"
                );
            }

            "triangular" => {
                if arg_exprs.len() < 2 || arg_exprs.len() > 3 {
                    bail!("random.triangular() requires 2 or 3 arguments");
                }
                let low = &arg_exprs[0];
                let high = &arg_exprs[1];
                let mode = if arg_exprs.len() == 3 {
                    &arg_exprs[2]
                } else {
                    &parse_quote! { ((#low + #high) / 2.0) }
                };

                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let triangular = rand_distr::Triangular::new(
                            #low as f64,
                            #high as f64,
                            #mode as f64
                        ).unwrap();
                        DEPYLER_RNG.with(|rng| triangular.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "randbytes" => {
                if arg_exprs.len() != 1 {
                    bail!("random.randbytes() requires exactly 1 argument");
                }
                let n = &arg_exprs[0];

                parse_quote! {
                    {
                        let n = #n as usize;
                        DEPYLER_RNG.with(|rng| {
                            let mut rng = rng.borrow_mut();
                            (0..n).map(|_| rng.gen::<u8>()).collect::<Vec<u8>>()
                        })
                    }
                }
            }

            _ => {
                bail!("random.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert math module method calls
    ///
    #[inline]
    fn try_convert_math_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Trigonometric functions - all take one f64 argument
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                parse_quote! { (#arg as f64).#method_ident() }
            }

            // atan2 takes two arguments
            "atan2" => {
                if arg_exprs.len() != 2 {
                    bail!("math.atan2() requires exactly 2 arguments");
                }
                let y = &arg_exprs[0];
                let x = &arg_exprs[1];
                parse_quote! { (#y as f64).atan2(#x as f64) }
            }

            // Hyperbolic functions
            "sinh" | "cosh" | "tanh" | "asinh" | "acosh" | "atanh" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                parse_quote! { (#arg as f64).#method_ident() }
            }

            // Power and logarithmic functions
            "sqrt" | "exp" | "ln" | "log2" | "log10" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_name = if method == "ln" { "ln" } else { method };
                let method_ident = syn::Ident::new(method_name, proc_macro2::Span::call_site());
                parse_quote! { (#arg as f64).#method_ident() }
            }

            // log() can take 1 or 2 arguments (log(x) or log(x, base))
            "log" => {
                if arg_exprs.len() == 1 {
                    let arg = &arg_exprs[0];
                    // log(x) defaults to natural logarithm
                    parse_quote! { (#arg as f64).ln() }
                } else if arg_exprs.len() == 2 {
                    let x = &arg_exprs[0];
                    let base = &arg_exprs[1];
                    // log(x, base) → x.log(base)
                    parse_quote! { (#x as f64).log(#base as f64) }
                } else {
                    bail!("math.log() requires 1 or 2 arguments");
                }
            }

            // pow() takes two arguments
            "pow" => {
                if arg_exprs.len() != 2 {
                    bail!("math.pow() requires exactly 2 arguments");
                }
                let base = &arg_exprs[0];
                let exp = &arg_exprs[1];
                // Use powf for floating point exponents
                parse_quote! { (#base as f64).powf(#exp as f64) }
            }

            // Rounding functions
            "ceil" | "floor" | "trunc" | "round" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                // These return f64 in Rust, but Python's math.ceil/floor return int
                // We'll cast to i32 for ceil and floor
                if method == "ceil" || method == "floor" {
                    parse_quote! { (#arg as f64).#method_ident() as i32 }
                } else {
                    parse_quote! { (#arg as f64).#method_ident() }
                }
            }

            // Absolute value
            "fabs" => {
                if arg_exprs.len() != 1 {
                    bail!("math.fabs() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).abs() }
            }

            // copysign
            "copysign" => {
                if arg_exprs.len() != 2 {
                    bail!("math.copysign() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                parse_quote! { (#x as f64).copysign(#y as f64) }
            }

            // Degree/Radian conversion
            "degrees" => {
                if arg_exprs.len() != 1 {
                    bail!("math.degrees() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).to_degrees() }
            }
            "radians" => {
                if arg_exprs.len() != 1 {
                    bail!("math.radians() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).to_radians() }
            }

            // Special value checks
            "isnan" => {
                if arg_exprs.len() != 1 {
                    bail!("math.isnan() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).is_nan() }
            }
            "isinf" => {
                if arg_exprs.len() != 1 {
                    bail!("math.isinf() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).is_infinite() }
            }
            "isfinite" => {
                if arg_exprs.len() != 1 {
                    bail!("math.isfinite() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).is_finite() }
            }

            // GCD - requires num crate for integers
            "gcd" => {
                if arg_exprs.len() != 2 {
                    bail!("math.gcd() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // For now, implement simple Euclidean algorithm inline
                // NOTE: Use num_integer::gcd crate for better performance ()
                parse_quote! {
                    {
                        let mut a = (#a as i64).abs();
                        let mut b = (#b as i64).abs();
                        while b != 0 {
                            let temp = b;
                            b = a % b;
                            a = temp;
                        }
                        a as i32
                    }
                }
            }

            // Factorial - compute inline for now
            "factorial" => {
                if arg_exprs.len() != 1 {
                    bail!("math.factorial() requires exactly 1 argument");
                }
                let n = &arg_exprs[0];
                parse_quote! {
                    {
                        let n = #n as i32;
                        let mut result = 1i64;
                        for i in 1..=n {
                            result *= i as i64;
                        }
                        result as i32
                    }
                }
            }

            // ldexp and frexp - less common, basic implementation
            "ldexp" => {
                if arg_exprs.len() != 2 {
                    bail!("math.ldexp() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let i = &arg_exprs[1];
                // ldexp(x, i) = x * 2^i
                parse_quote! { (#x as f64) * 2.0f64.powi(#i as i32) }
            }

            "frexp" => {
                // frexp returns (mantissa, exponent) where x = mantissa * 2^exponent
                // Rust doesn't have this built-in, so we'll implement it
                if arg_exprs.len() != 1 {
                    bail!("math.frexp() requires exactly 1 argument");
                }
                let x = &arg_exprs[0];
                parse_quote! {
                    {
                        let x = #x as f64;
                        if x == 0.0 {
                            (0.0, 0)
                        } else {
                            let exp = x.abs().log2().floor() as i32 + 1;
                            let mantissa = x / 2.0f64.powi(exp);
                            (mantissa, exp)
                        }
                    }
                }
            }

            // LCM - least common multiple
            "lcm" => {
                if arg_exprs.len() != 2 {
                    bail!("math.lcm() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // lcm(a, b) = abs(a * b) / gcd(a, b)
                parse_quote! {
                    {
                        let a = (#a as i64).abs();
                        let b = (#b as i64).abs();
                        if a == 0 || b == 0 {
                            0
                        } else {
                            // Compute GCD first
                            let mut gcd_a = a;
                            let mut gcd_b = b;
                            while gcd_b != 0 {
                                let temp = gcd_b;
                                gcd_b = gcd_a % gcd_b;
                                gcd_a = temp;
                            }
                            let gcd = gcd_a;
                            ((a / gcd) * b) as i32
                        }
                    }
                }
            }

            // isclose - floating point comparison with tolerance
            "isclose" => {
                if arg_exprs.len() < 2 {
                    bail!("math.isclose() requires at least 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // Default rel_tol=1e-09, abs_tol=0.0
                parse_quote! {
                    {
                        let a = #a as f64;
                        let b = #b as f64;
                        let rel_tol = 1e-9;
                        let abs_tol = 0.0;
                        let diff = (a - b).abs();
                        diff <= abs_tol.max(rel_tol * a.abs().max(b.abs()))
                    }
                }
            }

            // modf - split into fractional and integer parts
            "modf" => {
                if arg_exprs.len() != 1 {
                    bail!("math.modf() requires exactly 1 argument");
                }
                let x = &arg_exprs[0];
                parse_quote! {
                    {
                        let x = #x as f64;
                        let int_part = x.trunc();
                        let frac_part = x - int_part;
                        (frac_part, int_part)
                    }
                }
            }

            // fmod - floating point remainder
            "fmod" => {
                if arg_exprs.len() != 2 {
                    bail!("math.fmod() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                parse_quote! { (#x as f64) % (#y as f64) }
            }

            // hypot - Euclidean distance (hypotenuse)
            "hypot" => {
                if arg_exprs.len() != 2 {
                    bail!("math.hypot() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                parse_quote! { (#x as f64).hypot(#y as f64) }
            }

            // dist - distance between two points
            "dist" => {
                if arg_exprs.len() != 2 {
                    bail!("math.dist() requires exactly 2 arguments (two points)");
                }
                let p = &arg_exprs[0];
                let q = &arg_exprs[1];
                // Simplified: assume 2D points
                parse_quote! {
                    {
                        let p = #p;
                        let q = #q;
                        let dx = p[0] - q[0];
                        let dy = p[1] - q[1];
                        ((dx * dx + dy * dy) as f64).sqrt()
                    }
                }
            }

            //
            "remainder" => {
                if arg_exprs.len() != 2 {
                    bail!("math.remainder() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                // IEEE remainder: x - n*y where n is closest integer to x/y
                parse_quote! {
                    {
                        let x = #x as f64;
                        let y = #y as f64;
                        let n = (x / y).round();
                        x - n * y
                    }
                }
            }

            //
            "comb" => {
                if arg_exprs.len() != 2 {
                    bail!("math.comb() requires exactly 2 arguments");
                }
                let n = &arg_exprs[0];
                let k = &arg_exprs[1];
                parse_quote! {
                    {
                        let n = #n as i64;
                        let k = #k as i64;
                        if k > n || k < 0 { 0 } else {
                            let k = if k > n - k { n - k } else { k };
                            let mut result = 1i64;
                            for i in 0..k {
                                result = result * (n - i) / (i + 1);
                            }
                            result as i32
                        }
                    }
                }
            }

            //
            "perm" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("math.perm() requires 1 or 2 arguments");
                }
                let n = &arg_exprs[0];
                let k = if arg_exprs.len() == 2 {
                    &arg_exprs[1]
                } else {
                    n
                };
                parse_quote! {
                    {
                        let n = #n as i64;
                        let k = #k as i64;
                        if k > n || k < 0 { 0 } else {
                            let mut result = 1i64;
                            for i in 0..k {
                                result *= n - i;
                            }
                            result as i32
                        }
                    }
                }
            }

            //
            "expm1" => {
                if arg_exprs.len() != 1 {
                    bail!("math.expm1() requires exactly 1 argument");
                }
                let x = &arg_exprs[0];
                parse_quote! { (#x as f64).exp_m1() }
            }

            _ => {
                bail!("math.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert module method call (e.g., os.getcwd())
    #[inline]
    fn try_convert_module_method(
        &mut self,
        object: &HirExpr,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<Option<syn::Expr>> {
        // os.environ.get('VAR') → std::env::var('VAR').ok()
        // os.environ.get('VAR', 'default') → std::env::var('VAR').unwrap_or_else(|_| 'default'.to_string())
        if let HirExpr::Attribute { value, attr } = object {
            if let HirExpr::Var(module_name) = &**value {
                if module_name == "os" && attr == "environ" {
                    return self.try_convert_os_environ_method(method, args);
                }
                // os.path.exists(path) → Path::new(path).exists()
                // os.path.join(a, b) → PathBuf::from(a).join(b)
                if module_name == "os" && attr == "path" {
                    return self.try_convert_os_path_method(method, args);
                }
            }
        }

        if let HirExpr::Var(module_name) = object {
            // If this variable is declared as a local variable, don't treat it as a module
            if self.ctx.is_declared(module_name) {
                return Ok(None);
            }

            if module_name == "struct" {
                return self.try_convert_struct_method(method, args);
            }

            //
            // math.sqrt(x) → x.sqrt()
            // math.sin(x) → x.sin()
            // math.pow(x, y) → x.powf(y)
            if module_name == "math" {
                return self.try_convert_math_method(method, args);
            }

            //
            // random.random() → thread_rng().gen()
            // random.randint(a, b) → thread_rng().gen_range(a..=b)
            if module_name == "random" {
                return self.try_convert_random_method(method, args);
            }

            //
            // statistics.mean(data) → inline calculation
            // statistics.median(data) → sorted median calculation
            if module_name == "statistics" {
                return self.try_convert_statistics_method(method, args);
            }

            //
            // Fraction(1, 2) → Ratio::new(1, 2)
            // f.limit_denominator(100) → approximate with max denominator
            if module_name == "fractions" {
                return self.try_convert_fractions_method(method, args);
            }

            //
            // Path("/foo/bar").exists() → PathBuf::from("/foo/bar").exists()
            // Path("/foo").join("bar") → PathBuf::from("/foo").join("bar")
            if module_name == "pathlib" {
                return self.try_convert_pathlib_method(method, args);
            }

            //
            // datetime.datetime.now() → Local::now().naive_local()
            // datetime.datetime.utcnow() → Utc::now().naive_utc()
            // datetime.date.today() → Local::now().date_naive()
            if module_name == "datetime" {
                return self.try_convert_datetime_method(method, args);
            }

            //
            // decimal.Decimal("123.45") → Decimal::from_str("123.45")
            // Note: Decimal() constructor is handled separately in convert_call
            if module_name == "decimal" {
                return self.try_convert_decimal_method(method, args);
            }

            //
            // json.dumps(obj) → serde_json::to_string(&obj)
            // json.loads(s) → serde_json::from_str(&s)
            if module_name == "json" {
                return self.try_convert_json_method(method, args);
            }

            //
            // unicodedata.normalize("NFC", s) → s.nfc().collect::<String>()
            if module_name == "unicodedata" {
                return self.try_convert_unicodedata_method(method, args);
            }

            //
            if module_name == "re" {
                return self.try_convert_re_method(method, args);
            }

            //
            if module_name == "string" {
                return self.try_convert_string_method(method, args);
            }

            //
            if module_name == "time" {
                return self.try_convert_time_method(method, args);
            }

            //
            if module_name == "csv" {
                return self.try_convert_csv_method(method, args, kwargs);
            }

            // Must be checked before os.path to handle non-path os functions
            if module_name == "os" {
                if let Some(result) = self.try_convert_os_method(method, args)? {
                    return Ok(Some(result));
                }
                // Fall through to os.path handler if method not recognized
            }

            //
            // Only match the actual module "os.path", not variables named "path"
            // Variables named "path" are typically PathBuf instances from Path() constructor
            if module_name == "os.path" {
                return self.try_convert_os_path_method(method, args);
            }

            //
            if module_name == "base64" {
                return self.try_convert_base64_method(method, args);
            }

            //
            if module_name == "secrets" {
                return self.try_convert_secrets_method(method, args);
            }

            //
            if module_name == "hashlib" {
                return self.try_convert_hashlib_method(method, args);
            }

            //
            if module_name == "uuid" {
                return self.try_convert_uuid_method(method, args);
            }

            //
            if module_name == "hmac" {
                return self.try_convert_hmac_method(method, args);
            }

            if module_name == "platform" {
                return self.try_convert_platform_method(method, args);
            }

            //
            if module_name == "binascii" {
                return self.try_convert_binascii_method(method, args);
            }

            //
            if module_name == "urllib.parse" || module_name == "parse" {
                return self.try_convert_urllib_parse_method(method, args);
            }

            //
            if module_name == "fnmatch" {
                return self.try_convert_fnmatch_method(method, args);
            }

            //
            if module_name == "shlex" {
                return self.try_convert_shlex_method(method, args);
            }

            //
            if module_name == "textwrap" {
                return self.try_convert_textwrap_method(method, args);
            }

            //
            if module_name == "bisect" {
                return self.try_convert_bisect_method(method, args);
            }

            //
            if module_name == "heapq" {
                return self.try_convert_heapq_method(method, args);
            }

            //
            if module_name == "copy" {
                return self.try_convert_copy_method(method, args);
            }

            //
            if module_name == "itertools" {
                return self.try_convert_itertools_method(method, args, kwargs);
            }

            //
            if module_name == "functools" {
                return self.try_convert_functools_method(method, args);
            }

            //
            if module_name == "warnings" {
                return self.try_convert_warnings_method(method, args);
            }

            //
            if module_name == "sys" {
                return self.try_convert_sys_method(method, args);
            }

            //
            if module_name == "pickle" {
                return self.try_convert_pickle_method(method, args);
            }

            //
            if module_name == "pprint" {
                return self.try_convert_pprint_method(method, args);
            }

            let module_info = self
                .ctx
                .imported_modules
                .get(module_name)
                .and_then(|mapping| {
                    mapping
                        .item_map
                        .get(method)
                        .map(|rust_name| (mapping.rust_path.clone(), rust_name.clone()))
                });

            if let Some((rust_path, rust_name)) = module_info {
                // Convert args
                let arg_exprs: Vec<syn::Expr> = args
                    .iter()
                    .map(|arg| arg.to_rust_expr(self.ctx))
                    .collect::<Result<Vec<_>>>()?;

                // Python: math.sqrt(x) → Rust: x.sqrt() or f64::sqrt(x)
                if module_name == "math" && !arg_exprs.is_empty() {
                    let receiver = &arg_exprs[0];
                    let method_ident = syn::Ident::new(&rust_name, proc_macro2::Span::call_site());
                    return Ok(Some(parse_quote! { (#receiver).#method_ident() }));
                }

                // Build the Rust function path using the module's rust_path
                let path_parts: Vec<&str> = rust_name.split("::").collect();

                // Start with the module's rust_path instead of hardcoded "std"
                let base_path: syn::Path =
                    syn::parse_str(&rust_path).unwrap_or_else(|_| parse_quote! { std });
                let mut path = quote! { #base_path };

                for part in path_parts {
                    let part_ident = syn::Ident::new(part, proc_macro2::Span::call_site());
                    path = quote! { #path::#part_ident };
                }

                // Special handling for certain functions
                let result = match rust_name.as_str() {
                    "env::current_dir" => {
                        // current_dir returns Result<PathBuf>, we need to convert to String
                        parse_quote! {
                            #path().unwrap().to_string_lossy().to_string()
                        }
                    }
                    "Regex::new" => {
                        // re.compile(pattern) -> Regex::new(pattern)
                        if arg_exprs.is_empty() {
                            bail!("re.compile() requires a pattern argument");
                        }
                        let pattern = &arg_exprs[0];
                        parse_quote! {
                            regex::Regex::new(#pattern).unwrap()
                        }
                    }
                    _ => {
                        if arg_exprs.is_empty() {
                            parse_quote! { #path() }
                        } else {
                            parse_quote! { #path(#(#arg_exprs),*) }
                        }
                    }
                };
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    // ========================================================================
    // ========================================================================

    /// Handle list methods (append, extend, pop, insert, remove, sort)
    #[inline]
    fn convert_list_method(
        &mut self,
        object_expr: &syn::Expr,
        object: &HirExpr,
        method: &str,
        arg_exprs: &[syn::Expr],
        hir_args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<syn::Expr> {
        match method {
            "append" => {
                if arg_exprs.len() != 1 {
                    bail!("append() requires exactly one argument");
                }
                let arg = &arg_exprs[0];

                // Check if the argument is an Optional variable that needs unwrapping
                let needs_unwrap = if !hir_args.is_empty() {
                    if let HirExpr::Var(var_name) = &hir_args[0] {
                        self.ctx.optional_vars.contains(var_name)
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Five-Whys Root Cause:
                // 1. Why: expected String, found &str
                // 2. Why: String literal "X" is &str, but Vec<String>.push() needs String
                // 3. Why: Transpiler generates "X" without .to_string()
                // 4. Why: append method doesn't check element type
                // 5. ROOT CAUSE: Missing .to_string() for literals in Vec<String>
                let needs_to_string = if !hir_args.is_empty() {
                    // Check if argument is a string literal
                    let is_str_literal =
                        matches!(&hir_args[0], HirExpr::Literal(Literal::String(_)));

                    // Check if object is a Vec<String> by examining variable or field type
                    let is_vec_string = match object {
                        HirExpr::Var(var_name) => {
                            matches!(
                                self.ctx.var_types.get(var_name),
                                Some(Type::List(element_type)) if matches!(**element_type, Type::String)
                            )
                        }
                        HirExpr::Attribute { value, attr } => {
                            // Check if field is Vec<String> via class_field_types
                            self.get_field_element_type_is_string(value, attr)
                        }
                        _ => false,
                    };

                    is_str_literal && is_vec_string
                } else {
                    false
                };

                if needs_unwrap {
                    Ok(parse_quote! { #object_expr.push(#arg.clone().unwrap()) })
                } else if needs_to_string {
                    Ok(parse_quote! { #object_expr.push(#arg.to_string()) })
                } else {
                    Ok(parse_quote! { #object_expr.push(#arg) })
                }
            }
            "extend" => {
                if arg_exprs.len() != 1 {
                    bail!("extend() requires exactly one argument");
                }
                let arg = &arg_exprs[0];
                // extend() expects IntoIterator<Item = T>, but we often pass &Vec<T>
                // which gives IntoIterator<Item = &T>. Add .iter().cloned() to fix this.
                // Check if arg is a reference (most common case for function parameters)
                let arg_string = quote! { #arg }.to_string();
                if arg_string.contains("&") || !arg_string.contains(".into_iter()") {
                    // Likely a reference or direct variable - add iterator conversion
                    Ok(parse_quote! { #object_expr.extend(#arg.iter().cloned()) })
                } else {
                    // Already an iterator or owned value
                    Ok(parse_quote! { #object_expr.extend(#arg) })
                }
            }
            "pop" => {
                // Disambiguate based on argument count FIRST, then object type

                if arg_exprs.len() == 2 {
                    // Only dict.pop(key, default) takes 2 arguments
                    let key = &arg_exprs[0];
                    let default = &arg_exprs[1];
                    let needs_ref = !hir_args.is_empty()
                        && !matches!(
                            hir_args[0],
                            HirExpr::Literal(crate::hir::Literal::String(_)) | HirExpr::Var(_)
                        );
                    if needs_ref {
                        Ok(parse_quote! { #object_expr.remove(&#key).unwrap_or(#default) })
                    } else {
                        Ok(parse_quote! { #object_expr.remove(#key).unwrap_or(#default) })
                    }
                } else if arg_exprs.len() > 2 {
                    bail!("pop() takes at most 2 arguments");
                } else if self.is_set_expr(object) {
                    // Set.pop() - must have 0 arguments
                    if !arg_exprs.is_empty() {
                        bail!("pop() takes no arguments for sets");
                    }
                    Ok(parse_quote! {
                        #object_expr.iter().next().cloned().map(|x| {
                            #object_expr.remove(&x);
                            x
                        }).expect("pop from empty set")
                    })
                } else if self.is_dict_expr(object) {
                    // Dict literal - pop(key) with 1 argument
                    if arg_exprs.len() != 1 {
                        bail!("dict literal pop() requires exactly 1 argument (key)");
                    }
                    let key = &arg_exprs[0];
                    let needs_ref = !hir_args.is_empty()
                        && !matches!(
                            hir_args[0],
                            HirExpr::Literal(crate::hir::Literal::String(_)) | HirExpr::Var(_)
                        );
                    if needs_ref {
                        Ok(
                            parse_quote! { #object_expr.remove(&#key).expect("KeyError: key not found") },
                        )
                    } else {
                        Ok(
                            parse_quote! { #object_expr.remove(#key).expect("KeyError: key not found") },
                        )
                    }
                } else if arg_exprs.is_empty() {
                    // List.pop() with no arguments - remove last element
                    Ok(parse_quote! { #object_expr.pop().unwrap() })
                } else {
                    // 1 argument: could be list.pop(index) OR dict.pop(key)
                    // Use multiple heuristics to disambiguate:
                    let arg = &arg_exprs[0];

                    // Heuristic 1: Explicit list literal
                    let is_list = self.is_list_expr(object);

                    // Heuristic 2: Explicit dict literal
                    let is_dict = self.is_dict_expr(object);

                    // Heuristic 3: Integer argument suggests list index
                    let arg_is_int = !hir_args.is_empty()
                        && matches!(hir_args[0], HirExpr::Literal(crate::hir::Literal::Int(_)));

                    if is_list || (!is_dict && arg_is_int) {
                        // List.pop(index) - use Vec::remove() which takes usize by value
                        Ok(parse_quote! { #object_expr.remove(#arg as usize) })
                    } else {
                        // dict.pop(key) - HashMap::remove() takes &K by reference
                        let needs_ref = !hir_args.is_empty()
                            && !matches!(
                                hir_args[0],
                                HirExpr::Literal(crate::hir::Literal::String(_)) | HirExpr::Var(_)
                            );
                        if needs_ref {
                            Ok(
                                parse_quote! { #object_expr.remove(&#arg).expect("KeyError: key not found") },
                            )
                        } else {
                            Ok(
                                parse_quote! { #object_expr.remove(#arg).expect("KeyError: key not found") },
                            )
                        }
                    }
                }
            }
            "insert" => {
                if arg_exprs.len() != 2 {
                    bail!("insert() requires exactly two arguments");
                }
                let index = &arg_exprs[0];
                let value = &arg_exprs[1];
                Ok(parse_quote! { #object_expr.insert(#index as usize, #value) })
            }
            "remove" => {
                if arg_exprs.len() != 1 {
                    bail!("remove() requires exactly one argument");
                }
                let value = &arg_exprs[0];
                if self.is_set_expr(object) {
                    Ok(parse_quote! {
                        if !#object_expr.remove(&#value) {
                            panic!("KeyError: element not in set");
                        }
                    })
                } else {
                    Ok(parse_quote! {
                        if let Some(pos) = #object_expr.iter().position(|x| x == &#value) {
                            #object_expr.remove(pos)
                        } else {
                            panic!("ValueError: list.remove(x): x not in list")
                        }
                    })
                }
            }
            "index" => {
                // Python: list.index(value) -> returns index of first occurrence
                // Rust: list.iter().position(|x| x == &value).ok_or(...)
                if arg_exprs.len() != 1 {
                    bail!("index() requires exactly one argument");
                }
                let value = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.iter()
                        .position(|x| x == &#value)
                        .map(|i| i as i32)
                        .expect("ValueError: value is not in list")
                })
            }
            "count" => {
                // Python: list.count(value) -> counts occurrences
                // Rust: list.iter().filter(|x| **x == value).count()
                if arg_exprs.len() != 1 {
                    bail!("count() requires exactly one argument");
                }
                let value = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.iter().filter(|x| **x == #value).count() as i32
                })
            }
            "copy" => {
                // Python: list.copy() -> shallow copy OR copy.copy(x) -> shallow copy
                // Rust: list.clone() OR x.clone()
                if arg_exprs.len() == 1 {
                    // This is copy.copy(x) from the copy module being misparsed as method call
                    // Just clone the argument directly
                    let arg = &arg_exprs[0];
                    return Ok(parse_quote! { #arg.clone() });
                }
                if !arg_exprs.is_empty() {
                    bail!("copy() takes no arguments");
                }
                // This is list.copy() method - clone the list
                Ok(parse_quote! { #object_expr.clone() })
            }
            "clear" => {
                // Python: list.clear() -> removes all elements
                // Rust: list.clear()
                if !arg_exprs.is_empty() {
                    bail!("clear() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.clear() })
            }
            "reverse" => {
                // Python: list.reverse() -> reverses in place
                // Rust: list.reverse()
                if !arg_exprs.is_empty() {
                    bail!("reverse() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.reverse() })
            }
            "sort" => {
                // Rust: list.sort_by_key(|x| func(x)) or list.sort()

                // Check for `key` kwarg
                let key_func = kwargs.iter().find(|(k, _)| k == "key").map(|(_, v)| v);
                let reverse = kwargs
                    .iter()
                    .find(|(k, _)| k == "reverse")
                    .and_then(|(_, v)| {
                        if let HirExpr::Literal(crate::hir::Literal::Bool(b)) = v {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false);

                if !arg_exprs.is_empty() {
                    bail!("sort() does not accept positional arguments");
                }

                match (key_func, reverse) {
                    (Some(key_expr), false) => {
                        // list.sort(key=func) → list.sort_by_key(|x| func(x))
                        // Convert key_expr to Rust callable
                        let key_rust = key_expr.to_rust_expr(self.ctx)?;
                        Ok(parse_quote! { #object_expr.sort_by_key(|x| #key_rust(x)) })
                    }
                    (Some(key_expr), true) => {
                        // list.sort(key=func, reverse=True) → list.sort_by_key(|x| std::cmp::Reverse(func(x)))
                        let key_rust = key_expr.to_rust_expr(self.ctx)?;
                        Ok(
                            parse_quote! { #object_expr.sort_by_key(|x| std::cmp::Reverse(#key_rust(x))) },
                        )
                    }
                    (None, false) => {
                        // list.sort() → list.sort()
                        Ok(parse_quote! { #object_expr.sort() })
                    }
                    (None, true) => {
                        // list.sort(reverse=True) → list.sort_by(|a, b| b.cmp(a))
                        Ok(parse_quote! { #object_expr.sort_by(|a, b| b.cmp(a)) })
                    }
                }
            }
            _ => bail!("Unknown list method: {}", method),
        }
    }

    /// Handle dict methods (get, keys, values, items, update)
    #[inline]
    fn convert_dict_method(
        &mut self,
        object_expr: &syn::Expr,
        method: &str,
        arg_exprs: &[syn::Expr],
        hir_args: &[HirExpr],
    ) -> Result<syn::Expr> {
        match method {
            "get" => {
                if arg_exprs.len() == 1 {
                    let key = &arg_exprs[0];
                    // Python: result = d.get(key); if result is None: ...
                    // Rust: let result = d.get(&key).cloned(); if result.is_none() { ... }

                    // For HashMap<String, V>, .get() expects &String (or &str)
                    // String literals like "key" work as-is (they're &str)
                    // String variables need & prefix to pass by reference
                    let key_expr: syn::Expr = match &hir_args[0] {
                        HirExpr::Literal(Literal::String(_)) => parse_quote! { #key },
                        _ => parse_quote! { &#key },
                    };

                    // Return Option - downstream code will handle unwrapping if needed
                    Ok(parse_quote! { #object_expr.get(#key_expr).cloned() })
                } else if arg_exprs.len() == 2 {
                    let key = &arg_exprs[0];
                    let default = &arg_exprs[1];
                    // Same as above - add & prefix for non-literal keys
                    let key_expr: syn::Expr = match &hir_args[0] {
                        HirExpr::Literal(Literal::String(_)) => parse_quote! { #key },
                        _ => parse_quote! { &#key },
                    };

                    // Python: d.get("key", default) -> Rust: *d.get("key").unwrap_or(&default)
                    // This is more efficient than .cloned().unwrap_or() as it avoids cloning
                    // when the value is present, only dereferencing at the end.
                    // For String values, we need special handling
                    let is_string_default =
                        matches!(&hir_args[1], HirExpr::Literal(Literal::String(_)));

                    if is_string_default {
                        // For HashMap<K, String>, we still need to_string() on the default
                        Ok(
                            parse_quote! { #object_expr.get(#key_expr).cloned().unwrap_or_else(|| #default.to_string()) },
                        )
                    } else {
                        // For Copy types like i32, use the efficient *get().unwrap_or(&default) pattern
                        Ok(parse_quote! { *#object_expr.get(#key_expr).unwrap_or(&#default) })
                    }
                } else {
                    bail!("get() requires 1 or 2 arguments");
                }
            }
            "keys" => {
                if !arg_exprs.is_empty() {
                    bail!("keys() takes no arguments");
                }
                // .keys() returns an iterator, but Python's dict.keys() returns a list-like view
                // We collect to Vec for better ergonomics (indexing, len(), etc.)
                Ok(parse_quote! { #object_expr.keys().cloned().collect::<Vec<_>>() })
            }
            "values" => {
                if !arg_exprs.is_empty() {
                    bail!("values() takes no arguments");
                }
                // However, this causes redundant .collect().iter() in sum(d.values())
                // NOTE: Consider context-aware return type (Vec vs Iterator) for optimization ()
                Ok(parse_quote! { #object_expr.values().cloned().collect::<Vec<_>>() })
            }
            "items" => {
                if !arg_exprs.is_empty() {
                    bail!("items() takes no arguments");
                }
                Ok(
                    parse_quote! { #object_expr.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>() },
                )
            }
            "update" => {
                if arg_exprs.len() != 1 {
                    bail!("update() requires exactly one argument");
                }
                let arg = &arg_exprs[0];
                // insert() expects (K, V), so we just use the values directly
                Ok(parse_quote! {
                    for (k, v) in #arg {
                        #object_expr.insert(k, v);
                    }
                })
            }
            "setdefault" => {
                // dict.setdefault(key, default) - get or insert with default
                // Python: dict.setdefault(key, default) returns value at key, or inserts default and returns it
                // Rust: entry().or_insert(default).clone()
                if arg_exprs.len() != 2 {
                    bail!("setdefault() requires exactly 2 arguments (key, default)");
                }
                let key = &arg_exprs[0];
                let default = &arg_exprs[1];
                Ok(parse_quote! {
                    #object_expr.entry(#key).or_insert(#default).clone()
                })
            }
            "popitem" => {
                // dict.popitem() - remove and return arbitrary (key, value) pair
                // Python: dict.popitem() removes and returns arbitrary item, or raises KeyError
                // Rust: iter().next() to get first item, then remove it
                if !arg_exprs.is_empty() {
                    bail!("popitem() takes no arguments");
                }
                Ok(parse_quote! {
                    {
                        let key = #object_expr.keys().next().cloned()
                            .expect("KeyError: popitem(): dictionary is empty");
                        let value = #object_expr.remove(&key)
                            .expect("KeyError: key disappeared");
                        (key, value)
                    }
                })
            }
            "pop" => {
                // dict.pop(key, default=None) - remove and return value for key
                // Python: dict.pop(key[, default]) removes key and returns value, or returns default
                // Rust: remove() returns Option, use unwrap_or() for default
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("pop() requires 1 or 2 arguments (key, optional default)");
                }
                let key = &arg_exprs[0];
                if arg_exprs.len() == 2 {
                    let default = &arg_exprs[1];
                    Ok(parse_quote! {
                        #object_expr.remove(#key).unwrap_or(#default)
                    })
                } else {
                    Ok(parse_quote! {
                        #object_expr.remove(#key).expect("KeyError: key not found")
                    })
                }
            }
            //
            "clear" => {
                if !arg_exprs.is_empty() {
                    bail!("clear() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.clear() })
            }
            //
            "copy" => {
                if !arg_exprs.is_empty() {
                    bail!("copy() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.clone() })
            }
            _ => bail!("Unknown dict method: {}", method),
        }
    }

    /// Handle string methods (upper, lower, strip, startswith, endswith, split, join, replace, find, count, isdigit, isalpha)
    #[inline]
    fn convert_string_method(
        &mut self,
        hir_object: &HirExpr,
        object_expr: &syn::Expr,
        method: &str,
        arg_exprs: &[syn::Expr],
        hir_args: &[HirExpr],
    ) -> Result<syn::Expr> {
        match method {
            "format" => {
                // Python: "{} {}".format(a, b) -> Rust: format!("{} {}", a, b)
                // Extract the format string from the object
                let format_string = match hir_object {
                    HirExpr::Literal(Literal::String(s)) => s.clone(),
                    _ => {
                        // For non-literal format strings, fall back to a simple runtime approach
                        // This is a simplification - complex runtime format strings need more work
                        return Ok(parse_quote! {
                            format!("{}", #object_expr)
                        });
                    }
                };

                // Convert Python format spec to Rust format spec
                let rust_format = self.convert_python_format_to_rust(&format_string);

                if arg_exprs.is_empty() {
                    // No arguments - just use the format string as-is (may have no placeholders)
                    Ok(parse_quote! { format!(#rust_format) })
                } else {
                    // Generate format! macro with arguments
                    let args = arg_exprs;
                    Ok(parse_quote! { format!(#rust_format, #(#args),*) })
                }
            }
            "upper" => {
                if !arg_exprs.is_empty() {
                    bail!("upper() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.to_uppercase() })
            }
            "lower" => {
                if !arg_exprs.is_empty() {
                    bail!("lower() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.to_lowercase() })
            }
            "strip" => {
                if arg_exprs.is_empty() {
                    // Just use trim() - if chained with methods like to_lowercase(), they return String
                    // If used standalone where String is needed, the caller handles conversion
                    Ok(parse_quote! { #object_expr.trim() })
                } else if arg_exprs.len() == 1 {
                    // strip(chars) - remove chars from both ends
                    // Python: 'xxhelloxx'.strip('x') -> 'hello'
                    // Rust: use trim_matches with char pattern
                    let chars = &arg_exprs[0];
                    Ok(parse_quote! { #object_expr.trim_matches(|c: char| #chars.contains(c)) })
                } else {
                    bail!("strip() takes at most 1 argument");
                }
            }
            "startswith" => {
                if hir_args.len() != 1 {
                    bail!("startswith() requires exactly one argument");
                }
                // Extract bare string literal for Pattern trait compatibility
                // For variables, add & to satisfy Pattern trait bound
                let prefix: syn::Expr = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => {
                        let arg = &arg_exprs[0];
                        parse_quote! { &#arg }
                    }
                };
                Ok(parse_quote! { #object_expr.starts_with(#prefix) })
            }
            "endswith" => {
                if hir_args.len() != 1 {
                    bail!("endswith() requires exactly one argument");
                }
                // Extract bare string literal for Pattern trait compatibility
                // For variables, add & to satisfy Pattern trait bound
                let suffix: syn::Expr = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => {
                        let arg = &arg_exprs[0];
                        parse_quote! { &#arg }
                    }
                };
                Ok(parse_quote! { #object_expr.ends_with(#suffix) })
            }
            "split" => {
                if arg_exprs.is_empty() {
                    Ok(
                        parse_quote! { #object_expr.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>() },
                    )
                } else if arg_exprs.len() == 1 {
                    // For variables, add & to satisfy Pattern trait bound
                    let sep: syn::Expr = match &hir_args[0] {
                        HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                        _ => {
                            let arg = &arg_exprs[0];
                            parse_quote! { &#arg }
                        }
                    };
                    Ok(
                        parse_quote! { #object_expr.split(#sep).map(|s| s.to_string()).collect::<Vec<String>>() },
                    )
                } else if arg_exprs.len() == 2 {
                    // split(sep, maxsplit) - Rust's splitn takes count as n+1
                    // Python: 'a,b,c'.split(',', 1) -> ['a', 'b,c']
                    // Rust: .splitn(2, ',')
                    let sep: syn::Expr = match &hir_args[0] {
                        HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                        _ => {
                            let arg = &arg_exprs[0];
                            parse_quote! { &#arg }
                        }
                    };
                    let maxsplit = &arg_exprs[1];
                    Ok(
                        parse_quote! { #object_expr.splitn((#maxsplit + 1) as usize, #sep).map(|s| s.to_string()).collect::<Vec<String>>() },
                    )
                } else {
                    bail!("split() takes at most 2 arguments");
                }
            }
            "join" => {
                // Use bare string literal for separator without .to_string()
                if hir_args.len() != 1 {
                    bail!("join() requires exactly one argument");
                }
                let iterable = &arg_exprs[0];
                // Extract bare string literal for separator, add & for variables
                let separator: syn::Expr = match hir_object {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => parse_quote! { &#object_expr },
                };
                Ok(parse_quote! { #iterable.join(#separator) })
            }
            "replace" => {
                // Use bare string literals without .to_string() for correct types
                // For variables, add & to satisfy Pattern trait bound
                if hir_args.len() < 2 || hir_args.len() > 3 {
                    bail!("replace() requires 2 or 3 arguments");
                }
                // Extract bare string literals for arguments
                let old: syn::Expr = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => {
                        let arg = &arg_exprs[0];
                        parse_quote! { &#arg }
                    }
                };
                let new: syn::Expr = match &hir_args[1] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => {
                        let arg = &arg_exprs[1];
                        parse_quote! { &#arg }
                    }
                };

                if hir_args.len() == 3 {
                    // Python: str.replace(old, new, count)
                    // Rust: str.replacen(old, new, count as usize)
                    let count = &arg_exprs[2];
                    Ok(parse_quote! { #object_expr.replacen(#old, #new, #count as usize) })
                } else {
                    // Python: str.replace(old, new)
                    // Rust: str.replace(old, new) - replaces all
                    Ok(parse_quote! { #object_expr.replace(#old, #new) })
                }
            }
            "find" => {
                // Python's find() returns -1 if not found, Rust's returns Option<usize>
                // Python supports optional start parameter: str.find(sub, start)
                if hir_args.is_empty() || hir_args.len() > 2 {
                    bail!("find() requires 1 or 2 arguments, got {}", hir_args.len());
                }

                // Extract bare string literal for Pattern trait compatibility
                // For variables, add & to satisfy Pattern trait bound
                let substring: syn::Expr = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => {
                        let arg = &arg_exprs[0];
                        parse_quote! { &#arg }
                    }
                };

                if hir_args.len() == 2 {
                    // Python: str.find(sub, start)
                    // Rust: str[start..].find(sub).map(|i| (i + start) as i32).unwrap_or(-1)
                    let start = &arg_exprs[1];
                    Ok(parse_quote! {
                        #object_expr[#start as usize..].find(#substring)
                            .map(|i| (i + #start as usize) as i32)
                            .unwrap_or(-1)
                    })
                } else {
                    // Python: str.find(sub)
                    // Rust: str.find(sub).map(|i| i as i32).unwrap_or(-1)
                    Ok(parse_quote! {
                        #object_expr.find(#substring)
                            .map(|i| i as i32)
                            .unwrap_or(-1)
                    })
                }
            }
            "count" => {
                // Extract bare string literal for Pattern trait compatibility
                if hir_args.len() != 1 {
                    bail!("count() requires exactly one argument");
                }
                let substring = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => arg_exprs[0].clone(),
                };
                Ok(parse_quote! { #object_expr.matches(#substring).count() as i32 })
            }
            "isdigit" => {
                if !arg_exprs.is_empty() {
                    bail!("isdigit() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.chars().all(|c| c.is_numeric()) })
            }
            "isalpha" => {
                if !arg_exprs.is_empty() {
                    bail!("isalpha() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.chars().all(|c| c.is_alphabetic()) })
            }
            "lstrip" => {
                if !arg_exprs.is_empty() {
                    bail!("lstrip() with arguments not supported in V1");
                }
                Ok(parse_quote! { #object_expr.trim_start() })
            }
            "rstrip" => {
                if !arg_exprs.is_empty() {
                    bail!("rstrip() with arguments not supported in V1");
                }
                Ok(parse_quote! { #object_expr.trim_end() })
            }
            "isalnum" => {
                if !arg_exprs.is_empty() {
                    bail!("isalnum() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.chars().all(|c| c.is_alphanumeric()) })
            }
            "title" => {
                // Python's title() capitalizes the first letter of each word
                if !arg_exprs.is_empty() {
                    bail!("title() takes no arguments");
                }
                Ok(parse_quote! {
                    #object_expr
                        .split_whitespace()
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
            }

            //
            "index" => {
                if hir_args.len() != 1 {
                    bail!("index() requires exactly one argument");
                }
                let substring = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => arg_exprs[0].clone(),
                };
                Ok(parse_quote! {
                    #object_expr.find(#substring)
                        .map(|i| i as i32)
                        .expect("substring not found")
                })
            }

            //
            "rfind" => {
                if hir_args.len() != 1 {
                    bail!("rfind() requires exactly one argument");
                }
                let substring = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => arg_exprs[0].clone(),
                };
                Ok(parse_quote! {
                    #object_expr.rfind(#substring)
                        .map(|i| i as i32)
                        .unwrap_or(-1)
                })
            }

            //
            "rindex" => {
                if hir_args.len() != 1 {
                    bail!("rindex() requires exactly one argument");
                }
                let substring = match &hir_args[0] {
                    HirExpr::Literal(Literal::String(s)) => parse_quote! { #s },
                    _ => arg_exprs[0].clone(),
                };
                Ok(parse_quote! {
                    #object_expr.rfind(#substring)
                        .map(|i| i as i32)
                        .expect("substring not found")
                })
            }

            //
            "center" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("center() requires 1 or 2 arguments");
                }
                let width = &arg_exprs[0];
                let fillchar = if arg_exprs.len() == 2 {
                    &arg_exprs[1]
                } else {
                    &parse_quote!(" ")
                };

                Ok(parse_quote! {
                    {
                        let s = #object_expr;
                        let width = #width as usize;
                        let fillchar = #fillchar;
                        if s.len() >= width {
                            s.to_string()
                        } else {
                            let total_pad = width - s.len();
                            let left_pad = total_pad / 2;
                            let right_pad = total_pad - left_pad;
                            format!("{}{}{}", fillchar.repeat(left_pad), s, fillchar.repeat(right_pad))
                        }
                    }
                })
            }

            //
            "ljust" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("ljust() requires 1 or 2 arguments");
                }
                let width = &arg_exprs[0];
                let fillchar = if arg_exprs.len() == 2 {
                    &arg_exprs[1]
                } else {
                    &parse_quote!(" ")
                };

                Ok(parse_quote! {
                    {
                        let s = #object_expr;
                        let width = #width as usize;
                        let fillchar = #fillchar;
                        if s.len() >= width {
                            s.to_string()
                        } else {
                            format!("{}{}", s, fillchar.repeat(width - s.len()))
                        }
                    }
                })
            }

            //
            "rjust" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("rjust() requires 1 or 2 arguments");
                }
                let width = &arg_exprs[0];
                let fillchar = if arg_exprs.len() == 2 {
                    &arg_exprs[1]
                } else {
                    &parse_quote!(" ")
                };

                Ok(parse_quote! {
                    {
                        let s = #object_expr;
                        let width = #width as usize;
                        let fillchar = #fillchar;
                        if s.len() >= width {
                            s.to_string()
                        } else {
                            format!("{}{}", fillchar.repeat(width - s.len()), s)
                        }
                    }
                })
            }

            //
            "zfill" => {
                if arg_exprs.len() != 1 {
                    bail!("zfill() requires exactly 1 argument");
                }
                let width = &arg_exprs[0];

                Ok(parse_quote! {
                    {
                        let s = #object_expr;
                        let width = #width as usize;
                        if s.len() >= width {
                            s.to_string()
                        } else {
                            let sign = if s.starts_with('-') || s.starts_with('+') { &s[0..1] } else { "" };
                            let num = if !sign.is_empty() { &s[1..] } else { &s[..] };
                            format!("{}{}{}", sign, "0".repeat(width - s.len()), num)
                        }
                    }
                })
            }

            //
            "capitalize" => {
                if !arg_exprs.is_empty() {
                    bail!("capitalize() takes no arguments");
                }
                Ok(parse_quote! {
                    {
                        let s = #object_expr;
                        let mut chars = s.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                        }
                    }
                })
            }

            //
            "swapcase" => {
                if !arg_exprs.is_empty() {
                    bail!("swapcase() takes no arguments");
                }
                Ok(parse_quote! {
                    #object_expr.chars().map(|c| {
                        if c.is_uppercase() {
                            c.to_lowercase().to_string()
                        } else {
                            c.to_uppercase().to_string()
                        }
                    }).collect::<String>()
                })
            }

            //
            "expandtabs" => {
                if arg_exprs.is_empty() {
                    Ok(parse_quote! {
                        #object_expr.replace("\t", &" ".repeat(8))
                    })
                } else if arg_exprs.len() == 1 {
                    // tabsize argument will be used at runtime
                    let tabsize_expr = &arg_exprs[0];
                    Ok(parse_quote! {
                        #object_expr.replace("\t", &" ".repeat(#tabsize_expr as usize))
                    })
                } else {
                    bail!("expandtabs() takes 0 or 1 arguments")
                }
            }

            //
            "splitlines" => {
                if !arg_exprs.is_empty() {
                    bail!("splitlines() takes no arguments");
                }
                Ok(parse_quote! {
                    #object_expr.lines().map(|s| s.to_string()).collect::<Vec<String>>()
                })
            }

            //
            "partition" => {
                if arg_exprs.len() != 1 {
                    bail!("partition() requires exactly 1 argument (separator)");
                }
                let sep = &arg_exprs[0];
                Ok(parse_quote! {
                    {
                        let s = #object_expr;
                        let sep_str = #sep;
                        if let Some(pos) = s.find(sep_str) {
                            let before = &s[..pos];
                            let after = &s[pos + sep_str.len()..];
                            (before.to_string(), sep_str.to_string(), after.to_string())
                        } else {
                            (s.to_string(), String::new(), String::new())
                        }
                    }
                })
            }

            //
            "casefold" => {
                if !arg_exprs.is_empty() {
                    bail!("casefold() takes no arguments");
                }
                // casefold() is like lower() but more aggressive for Unicode
                Ok(parse_quote! { #object_expr.to_lowercase() })
            }

            //
            "isprintable" => {
                if !arg_exprs.is_empty() {
                    bail!("isprintable() takes no arguments");
                }
                Ok(parse_quote! {
                    #object_expr.chars().all(|c| !c.is_control() || c == '\t' || c == '\n' || c == '\r')
                })
            }

            _ => bail!("Unknown string method: {}", method),
        }
    }

    /// Convert Python format string to Rust format string
    /// Python uses {} or {:spec} placeholders, Rust uses similar but with some differences
    fn convert_python_format_to_rust(&self, format_str: &str) -> String {
        // Simple conversion: Python format specs are largely compatible with Rust
        // Key differences:
        // - Python {:d} for decimal -> Rust {:} or no format spec needed
        // - Python {:s} for string -> Rust {:} or no format spec needed
        // - Python {:.2f} for float precision -> Rust {:.2} (same)
        // - Python {:>10} for alignment -> Rust {:>10} (same)
        // - Python {0} positional -> Rust {0} (same)
        // - Python {name} named -> Rust {name} (same)

        let mut result = String::with_capacity(format_str.len());
        let mut chars = format_str.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    // {{ is an escaped brace in Python, same in Rust
                    result.push('{');
                    result.push('{');
                    chars.next();
                } else {
                    // Start of a format placeholder
                    result.push('{');
                    // Copy until we find the closing brace
                    let mut in_spec = false;
                    while let Some(&inner) = chars.peek() {
                        chars.next();
                        if inner == '}' {
                            result.push('}');
                            break;
                        }
                        if inner == ':' {
                            in_spec = true;
                        }
                        // Convert Python-specific format specs
                        if in_spec {
                            // Skip 'd' and 's' type specifiers as Rust infers them
                            if inner == 'd' || inner == 's' {
                                // Check if this is at the end (before })
                                if chars.peek() == Some(&'}') {
                                    continue;
                                }
                            }
                        }
                        result.push(inner);
                    }
                }
            } else if c == '}' {
                if chars.peek() == Some(&'}') {
                    // }} is an escaped brace
                    result.push('}');
                    result.push('}');
                    chars.next();
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Handle set methods (add, discard, clear)
    #[inline]
    fn convert_set_method(
        &mut self,
        object_expr: &syn::Expr,
        method: &str,
        arg_exprs: &[syn::Expr],
        hir_args: &[HirExpr],
    ) -> Result<syn::Expr> {
        match method {
            "add" => {
                if arg_exprs.len() != 1 {
                    bail!("add() requires exactly one argument");
                }
                let arg = &arg_exprs[0];
                // If adding a string literal to a set, convert &str to String
                let insert_expr = if !hir_args.is_empty()
                    && matches!(hir_args[0], HirExpr::Literal(Literal::String(_)))
                {
                    parse_quote! { #object_expr.insert(#arg.to_string()) }
                } else {
                    parse_quote! { #object_expr.insert(#arg) }
                };
                Ok(insert_expr)
            }
            "remove" => {
                if arg_exprs.len() != 1 {
                    bail!("remove() requires exactly one argument");
                }
                let arg = &arg_exprs[0];
                Ok(parse_quote! {
                    if !#object_expr.remove(&#arg) {
                        panic!("KeyError: element not in set")
                    }
                })
            }
            "discard" => {
                if arg_exprs.len() != 1 {
                    bail!("discard() requires exactly one argument");
                }
                let arg = &arg_exprs[0];
                Ok(parse_quote! { #object_expr.remove(&#arg) })
            }
            "clear" => {
                if !arg_exprs.is_empty() {
                    bail!("clear() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.clear() })
            }
            "update" => {
                if arg_exprs.len() != 1 {
                    bail!("update() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    for item in #other {
                        #object_expr.insert(item);
                    }
                })
            }
            "intersection_update" => {
                // Note: This generates an expression that returns (), suitable for ExprStmt
                if arg_exprs.len() != 1 {
                    bail!("intersection_update() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    {
                        let temp: std::collections::HashSet<_> = #object_expr.intersection(&#other).cloned().collect();
                        #object_expr.clear();
                        #object_expr.extend(temp);
                    }
                })
            }
            "difference_update" => {
                // Note: This generates an expression that returns (), suitable for ExprStmt
                if arg_exprs.len() != 1 {
                    bail!("difference_update() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    {
                        let temp: std::collections::HashSet<_> = #object_expr.difference(&#other).cloned().collect();
                        #object_expr.clear();
                        #object_expr.extend(temp);
                    }
                })
            }
            "union" => {
                // Set.union(other) - return new set with elements from both sets
                if arg_exprs.len() != 1 {
                    bail!("union() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.union(&#other).cloned().collect::<std::collections::HashSet<_>>()
                })
            }
            "intersection" => {
                // Set.intersection(other) - return new set with common elements
                if arg_exprs.len() != 1 {
                    bail!("intersection() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.intersection(&#other).cloned().collect::<std::collections::HashSet<_>>()
                })
            }
            "difference" => {
                // Set.difference(other) - return new set with elements not in other
                if arg_exprs.len() != 1 {
                    bail!("difference() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.difference(&#other).cloned().collect::<std::collections::HashSet<_>>()
                })
            }
            "symmetric_difference" => {
                // Set.symmetric_difference(other) - return new set with elements in either but not both
                if arg_exprs.len() != 1 {
                    bail!("symmetric_difference() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.symmetric_difference(&#other).cloned().collect::<std::collections::HashSet<_>>()
                })
            }
            "issubset" => {
                // Set.issubset(other) - check if all elements are in other
                if arg_exprs.len() != 1 {
                    bail!("issubset() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.is_subset(&#other)
                })
            }
            "issuperset" => {
                // Set.issuperset(other) - check if contains all elements of other
                if arg_exprs.len() != 1 {
                    bail!("issuperset() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.is_superset(&#other)
                })
            }
            "isdisjoint" => {
                // Set.isdisjoint(other) - check if no common elements
                if arg_exprs.len() != 1 {
                    bail!("isdisjoint() requires exactly one argument");
                }
                let other = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.is_disjoint(&#other)
                })
            }
            _ => bail!("Unknown set method: {}", method),
        }
    }

    /// Handle regex methods (findall)
    #[inline]
    /// Handles both compiled Regex methods and Match object methods
    fn convert_regex_method(
        &mut self,
        object_expr: &syn::Expr,
        method: &str,
        arg_exprs: &[syn::Expr],
    ) -> Result<syn::Expr> {
        match method {
            // Compiled Regex methods
            "findall" => {
                if arg_exprs.is_empty() {
                    bail!("findall() requires at least one argument");
                }
                let text = &arg_exprs[0];
                Ok(parse_quote! {
                    #object_expr.find_iter(#text)
                        .map(|m| m.as_str().to_string())
                        .collect::<Vec<String>>()
                })
            }

            // Python re.match() only matches at start, but Rust .find() searches anywhere
            // For now, use .find() - exact match-at-start behavior tracked separately
            "match" => {
                if arg_exprs.is_empty() {
                    bail!("match() requires at least one argument");
                }
                let text = &arg_exprs[0];
                Ok(parse_quote! { #object_expr.find(#text) })
            }

            // compiled.search(text) → compiled.find(text)
            "search" => {
                if arg_exprs.is_empty() {
                    bail!("search() requires at least one argument");
                }
                let text = &arg_exprs[0];
                Ok(parse_quote! { #object_expr.find(#text) })
            }

            // Match object methods (these should be called on unwrapped Match, not Option<Match>)
            // Note: These will fail if called on Option<Match> - caller must unwrap first

            // match.group(0) → match.as_str() (for group 0)
            // match.group(n) → match.get(n).map(|m| m.as_str()) (for other groups)
            "group" => {
                if arg_exprs.is_empty() {
                    // No args: default to group 0
                    Ok(parse_quote! { #object_expr.as_str() })
                } else {
                    // Check if group_num is literal 0
                    if matches!(arg_exprs[0], syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(ref lit), .. }) if lit.base10_parse::<i32>().ok() == Some(0))
                    {
                        Ok(parse_quote! { #object_expr.as_str() })
                    } else {
                        // Non-zero group: needs captures API
                        bail!(
                            "match.group(n) for n>0 requires .captures() API (not yet implemented)"
                        )
                    }
                }
            }

            // match.groups() → extract all capture groups
            // Python: match.groups() returns tuple of all captured groups (excluding group 0)
            // Rust: We need to track that this came from .captures() not .find()
            // For now, return empty tuple - will be enhanced when we track capture vs match
            "groups" => {
                // match.groups() returns a tuple of captured groups
                // This requires the regex to have been called with .captures() not .find()
                // For generated code, we'll return an empty vec for now
                // TODO: Track whether regex used .captures() vs .find() in type system
                Ok(parse_quote! {
                    vec![] as Vec<String>
                })
            }

            // match.start() → match.start() (passthrough)
            "start" => {
                if arg_exprs.is_empty() {
                    Ok(parse_quote! { #object_expr.start() })
                } else {
                    bail!("match.start(group) with group number not yet implemented")
                }
            }

            // match.end() → match.end() (passthrough)
            "end" => {
                if arg_exprs.is_empty() {
                    Ok(parse_quote! { #object_expr.end() })
                } else {
                    bail!("match.end(group) with group number not yet implemented")
                }
            }

            // match.span() → (match.start(), match.end())
            "span" => {
                if arg_exprs.is_empty() {
                    Ok(parse_quote! { (#object_expr.start(), #object_expr.end()) })
                } else {
                    bail!("match.span(group) with group number not yet implemented")
                }
            }

            // match.as_str() → match.as_str() (passthrough)
            "as_str" => {
                if !arg_exprs.is_empty() {
                    bail!("as_str() takes no arguments");
                }
                Ok(parse_quote! { #object_expr.as_str() })
            }

            _ => bail!("Unknown regex method: {}", method),
        }
    }

    /// sys.stdout.write(msg) → writeln!(std::io::stdout(), "{}", msg).unwrap()
    /// sys.stdin.read() → { let mut s = String::new(); std::io::stdin().read_to_string(&mut s).unwrap(); s }
    /// sys.stdout.flush() → std::io::stdout().flush().unwrap()
    #[inline]
    fn convert_sys_io_method(
        &self,
        stream: &str,
        method: &str,
        arg_exprs: &[syn::Expr],
    ) -> Result<syn::Expr> {
        let stream_fn = match stream {
            "stdin" => quote! { std::io::stdin() },
            "stdout" => quote! { std::io::stdout() },
            "stderr" => quote! { std::io::stderr() },
            _ => bail!("Unknown I/O stream: {}", stream),
        };

        let result = match (stream, method) {
            // stdout/stderr write methods
            ("stdout" | "stderr", "write") => {
                if arg_exprs.is_empty() {
                    bail!("{}.write() requires an argument", stream);
                }
                let msg = &arg_exprs[0];
                // Use writeln! macro for cleaner code and automatic newline handling
                // If the message already has \n, use write! instead
                parse_quote! {
                    {
                        use std::io::Write;
                        write!(#stream_fn, "{}", #msg).unwrap();
                    }
                }
            }

            // flush method
            (_, "flush") => {
                parse_quote! {
                    {
                        use std::io::Write;
                        #stream_fn.flush().unwrap()
                    }
                }
            }

            // stdin read methods
            ("stdin", "read") => {
                parse_quote! {
                    {
                        use std::io::Read;
                        let mut buffer = String::new();
                        #stream_fn.read_to_string(&mut buffer).unwrap();
                        buffer
                    }
                }
            }

            ("stdin", "readline") => {
                parse_quote! {
                    {
                        use std::io::BufRead;
                        let mut line = String::new();
                        #stream_fn.lock().read_line(&mut line).unwrap();
                        line
                    }
                }
            }

            _ => bail!("{}.{}() is not yet supported", stream, method),
        };

        Ok(result)
    }

    /// Convert instance method calls (main dispatcher)
    #[inline]
    fn convert_instance_method(
        &mut self,
        object: &HirExpr,
        object_expr: &syn::Expr,
        method: &str,
        arg_exprs: &[syn::Expr],
        hir_args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<syn::Expr> {
        // ArgumentParser.parse_args() requires full struct transformation
        // For now, return unit to allow compilation
        if method == "parse_args" {
            // NOTE: Full argparse implementation requires Args::parse() call ()
            return Ok(parse_quote! { () });
        }

        if method == "add_argument" {
            // NOTE: Accumulate add_argument calls to generate struct fields ()
            return Ok(parse_quote! { () });
        }

        // Option-native methods should NOT have their receiver unwrapped
        let is_option_method = matches!(
            method,
            "is_some"
                | "is_none"
                | "unwrap"
                | "unwrap_or"
                | "unwrap_or_else"
                | "map"
                | "and_then"
                | "or_else"
                | "ok_or"
                | "ok_or_else"
                | "as_ref"
                | "as_mut"
                | "take"
                | "replace"
                | "expect"
        );

        // Auto-unwrap Optional receivers for non-Option methods
        let object_expr = if !is_option_method && self.expr_is_optional(object) {
            let is_mutating = matches!(
                method,
                "append"
                    | "extend"
                    | "clear"
                    | "insert"
                    | "remove"
                    | "reverse"
                    | "sort"
                    | "pop"
                    | "add"
                    | "discard"
                    | "update"
            );
            if is_mutating {
                parse_quote! { #object_expr.as_mut().unwrap() }
            } else {
                parse_quote! { #object_expr.as_ref().unwrap() }
            }
        } else {
            object_expr.clone()
        };

        // Check if object is a sys I/O stream (sys.stdin(), sys.stdout(), sys.stderr())
        if let HirExpr::Attribute { value, attr } = object {
            if let HirExpr::Var(module) = &**value {
                if module == "sys" && matches!(attr.as_str(), "stdin" | "stdout" | "stderr") {
                    return self.convert_sys_io_method(attr, method, arg_exprs);
                }
            }
        }

        // Python: f.read() → Rust: read_to_string() or read_to_end()
        if method == "read" && arg_exprs.is_empty() {
            // f.read() with no arguments → read entire file
            // Need to determine if text or binary mode
            // For now, default to text mode (read_to_string)
            // TODO: Track file open mode to distinguish text vs binary
            return Ok(parse_quote! {
                {
                    let mut content = String::new();
                    #object_expr.read_to_string(&mut content)?;
                    content
                }
            });
        }

        // Python: match.group(0) or match.group(n)
        // Rust: match.as_str() for group(0), or handle numbered groups
        if method == "group" {
            if arg_exprs.is_empty() || hir_args.is_empty() {
                // match.group() with no args defaults to group(0) in Python
                return Ok(parse_quote! { #object_expr.as_str() });
            }

            // Check if argument is literal 0
            if let HirExpr::Literal(Literal::Int(n)) = &hir_args[0] {
                if *n == 0 {
                    // match.group(0) → match.as_str()
                    return Ok(parse_quote! { #object_expr.as_str() });
                } else {
                    // match.group(n) → match.get(n).map(|m| m.as_str()).unwrap_or("")
                    let idx = &arg_exprs[0];
                    return Ok(parse_quote! {
                        #object_expr.get(#idx).map(|m| m.as_str()).unwrap_or("")
                    });
                }
            }

            // Non-literal argument - use runtime check
            let idx = &arg_exprs[0];
            return Ok(parse_quote! {
                if #idx == 0 {
                    #object_expr.as_str()
                } else {
                    #object_expr.get(#idx).map(|m| m.as_str()).unwrap_or("")
                }
            });
        }

        // String methods like upper/lower should be converted even for method parameters
        // that might be typed as class instances (due to how we track types)
        // BUT: check for module calls first (e.g., os.path.join should NOT be treated as string join)
        if let HirExpr::Attribute { value, attr } = object {
            if let HirExpr::Var(module_name) = &**value {
                // Check for os.path module methods before falling through to string methods
                if module_name == "os" && attr == "path" {
                    if let Some(result) = self.try_convert_os_path_method(method, hir_args)? {
                        return Ok(result);
                    }
                }
            }
        }

        if matches!(
            method,
            "upper"
                | "lower"
                | "strip"
                | "lstrip"
                | "rstrip"
                | "startswith"
                | "endswith"
                | "split"
                | "splitlines"
                | "join"
                | "replace"
                | "find"
                | "rfind"
                | "rindex"
                | "isdigit"
                | "isalpha"
                | "isalnum"
                | "title"
                | "capitalize"
                | "swapcase"
                | "expandtabs"
                | "center"
                | "ljust"
                | "rjust"
                | "zfill"
                | "format"
        ) {
            return self.convert_string_method(object, &object_expr, method, arg_exprs, hir_args);
        }

        // Check for built-in collection method names BEFORE checking for user-defined classes
        // This ensures that dict.get(), set.add(), list.append(), etc. are handled correctly
        // even when the type information isn't available (e.g., for function parameters)
        match method {
            // Dict-specific methods that should always use dict handler
            "get" if arg_exprs.len() <= 2 => {
                return self.convert_dict_method(&object_expr, method, arg_exprs, hir_args);
            }
            "keys" | "values" | "items" | "setdefault" | "popitem" => {
                return self.convert_dict_method(&object_expr, method, arg_exprs, hir_args);
            }
            // Set-specific methods
            "add" if arg_exprs.len() == 1 => {
                return self.convert_set_method(&object_expr, method, arg_exprs, hir_args);
            }
            "discard"
            | "intersection_update"
            | "difference_update"
            | "symmetric_difference_update"
            | "union"
            | "intersection"
            | "difference"
            | "symmetric_difference"
            | "issubset"
            | "issuperset"
            | "isdisjoint" => {
                return self.convert_set_method(&object_expr, method, arg_exprs, hir_args);
            }
            _ => {}
        }

        // User-defined classes can have methods with names like "add" that conflict with
        // built-in collection methods. However, we should NOT treat built-in collection types
        // as user-defined classes. Check if this is NOT a built-in collection before routing
        // to the class instance handler.
        if self.is_class_instance(object)
            && !self.is_dict_expr(object)
            && !self.is_list_expr(object)
            && !self.is_set_expr(object)
        {
            // Check for special keywords that cannot be raw identifiers
            if Self::is_non_raw_keyword(method) {
                bail!(
                    "Python method '{}' conflicts with a special Rust keyword that cannot be escaped. \
                     Please rename this method (e.g., '{}_method' or 'py_{}'). \
                     Note: If this is 'super()', it should be handled as a method call, not a function call.",
                    method,
                    method,
                    method
                );
            }
            
            // This is a user-defined class instance - use generic method call
            let method_ident = if Self::is_rust_keyword(method) {
                syn::Ident::new_raw(method, proc_macro2::Span::call_site())
            } else {
                syn::Ident::new(method, proc_macro2::Span::call_site())
            };

            let final_args = if !kwargs.is_empty() {
                // Get class name from object for method lookup
                let class_name = self.get_class_name(object);
                let method_key = if let Some(ref cls) = class_name {
                    format!("{}.{}", cls, method)
                } else {
                    method.to_string()
                };

                // Clone to avoid borrowing ctx while calling to_rust_expr
                let maybe_param_names = self
                    .ctx
                    .function_param_names
                    .get(&method_key)
                    .or_else(|| self.ctx.function_param_names.get(method))
                    .cloned();

                let can_reorder = maybe_param_names
                    .as_ref()
                    .map(|p| p.len() >= arg_exprs.len() + kwargs.len())
                    .unwrap_or(false);

                if can_reorder {
                    let param_names = maybe_param_names.unwrap();
                    // Build reordered argument list
                    let kwarg_map: std::collections::HashMap<&str, &HirExpr> = kwargs
                        .iter()
                        .map(|(name, value)| (name.as_str(), value))
                        .collect();

                    let mut reordered: Vec<syn::Expr> = Vec::with_capacity(param_names.len());
                    // NOTE: 'self' is already excluded from HirMethod.params,
                    // so we iterate all param_names without skipping
                    for (idx, param_name) in param_names.iter().enumerate() {
                        if idx < arg_exprs.len() {
                            reordered.push(arg_exprs[idx].clone());
                        } else if let Some(value) = kwarg_map.get(param_name.as_str()) {
                            let expr = value.to_rust_expr(self.ctx)?;
                            if matches!(value, HirExpr::Literal(Literal::String(_))) {
                                reordered.push(parse_quote! { #expr.to_string() });
                            } else {
                                reordered.push(expr);
                            }
                        }
                    }
                    reordered
                } else {
                    // Fall back to just appending kwargs in order
                    let mut all: Vec<syn::Expr> = arg_exprs.to_vec();
                    for (_, value) in kwargs {
                        let expr = value.to_rust_expr(self.ctx)?;
                        if matches!(value, HirExpr::Literal(Literal::String(_))) {
                            all.push(parse_quote! { #expr.to_string() });
                        } else {
                            all.push(expr);
                        }
                    }
                    all
                }
            } else {
                arg_exprs.to_vec()
            };

            return Ok(parse_quote! { #object_expr.#method_ident(#(#final_args),*) });
        }

        // Fallback to method name dispatch
        match method {
            // List methods
            "append" | "extend" | "pop" | "insert" | "remove" | "copy" | "clear"
            | "reverse" | "sort" => {
                self.convert_list_method(&object_expr, object, method, arg_exprs, hir_args, kwargs)
            }

            "index" => {
                if self.is_string_base(object) {
                    self.convert_string_method(object, &object_expr, method, arg_exprs, hir_args)
                } else {
                    self.convert_list_method(
                        &object_expr,
                        object,
                        method,
                        arg_exprs,
                        hir_args,
                        kwargs,
                    )
                }
            }

            "count" => {
                // Heuristic: Check if object is string-typed using is_string_base()
                // This covers string literals, variables with str type annotations, and string method results
                if self.is_string_base(object) {
                    // String: use str.count() → .matches().count()
                    self.convert_string_method(object, &object_expr, method, arg_exprs, hir_args)
                } else {
                    // List: use list.count() → .iter().filter().count()
                    self.convert_list_method(
                        &object_expr,
                        object,
                        method,
                        arg_exprs,
                        hir_args,
                        kwargs,
                    )
                }
            }

            "update" => {
                // Check if argument is a set or dict literal
                if !hir_args.is_empty() && self.is_set_expr(&hir_args[0]) {
                    // numbers.update({3, 4}) - set update
                    self.convert_set_method(&object_expr, method, arg_exprs, hir_args)
                } else {
                    // data.update({"b": 2}) - dict update (default for variables)
                    self.convert_dict_method(&object_expr, method, arg_exprs, hir_args)
                }
            }

            // List/Vec .get() takes usize by value, Dict .get() takes &K by reference
            "get" => {
                // Only use list handler when we're CERTAIN it's a list (not dict)
                // Default to dict handler for uncertain types (dict.get() supports 1 or 2 args)
                if self.is_list_expr(object) && !self.is_dict_expr(object) {
                    // List/Vec .get() - cast index to usize (must be exactly 1 arg)
                    if arg_exprs.len() != 1 {
                        bail!("list.get() requires exactly one argument");
                    }
                    let index = &arg_exprs[0];
                    // Cast integer index to usize (Vec/slice .get() requires usize, not &i32)
                    Ok(parse_quote! { #object_expr.get(#index as usize).cloned() })
                } else {
                    // Dict .get() - use existing dict handler (supports 1 or 2 args)
                    self.convert_dict_method(&object_expr, method, arg_exprs, hir_args)
                }
            }

            // Dict methods (for variables without type info)
            "keys" | "values" | "items" | "setdefault" | "popitem" => {
                self.convert_dict_method(&object_expr, method, arg_exprs, hir_args)
            }

            // String methods
            // Note: "count" handled separately above with disambiguation logic
            // Note: "index" handled separately above with disambiguation logic
            "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "startswith" | "endswith"
            | "split" | "splitlines" | "join" | "replace" | "find" | "rfind" | "rindex"
            | "isdigit" | "isalpha" | "isalnum" | "title" | "center" | "ljust" | "rjust"
            | "zfill" => {
                self.convert_string_method(object, &object_expr, method, arg_exprs, hir_args)
            }

            // Set methods (for variables without type info)
            // Note: "update" handled separately above with disambiguation logic
            // Note: "remove" is ambiguous (list vs set) - keep in list fallback for now
            // Note: "add" with != 1 arg is not a set add (could be user-defined method)
            "add" if arg_exprs.len() == 1 => {
                self.convert_set_method(&object_expr, method, arg_exprs, hir_args)
            }
            "discard"
            | "intersection_update"
            | "difference_update"
            | "symmetric_difference_update"
            | "union"
            | "intersection"
            | "difference"
            | "symmetric_difference"
            | "issubset"
            | "issuperset"
            | "isdisjoint" => self.convert_set_method(&object_expr, method, arg_exprs, hir_args),

            // Compiled Regex: findall, match, search (note: "find" conflicts with string.find())
            // Match object: group, groups, start, end, span, as_str
            // NOTE: Only route to regex handler if object is actually a regex type
            // Otherwise, fall through to default case which will escape keywords
            "findall" | "search" | "groups" | "start" | "end" | "span" | "as_str"
                if self.is_regex_expr(object) =>
            {
                self.convert_regex_method(&object_expr, method, arg_exprs)
            }

            // "match" is a Rust keyword AND a regex method, so we need to:
            // 1. Check if object is a regex type → route to convert_regex_method
            // 2. Otherwise → escape as keyword in default case
            "match" if self.is_regex_expr(object) => {
                self.convert_regex_method(&object_expr, method, arg_exprs)
            }

            // Path instance methods
            "read_text" => {
                // filepath.read_text() → std::fs::read_to_string(filepath).unwrap()
                if !arg_exprs.is_empty() {
                    bail!("Path.read_text() takes no arguments");
                }
                Ok(parse_quote! { std::fs::read_to_string(#object_expr).unwrap() })
            }

            // Default: generic method call
            _ => {
                // Check for special keywords that cannot be raw identifiers
                if Self::is_non_raw_keyword(method) {
                    bail!(
                        "Python method '{}' conflicts with a special Rust keyword that cannot be escaped. \
                         Please rename this method (e.g., '{}_method' or 'py_{}'). \
                         Note: If this is 'super()', it should be handled differently.",
                        method,
                        method,
                        method
                    );
                }
                
                let method_ident = if Self::is_rust_keyword(method) {
                    syn::Ident::new_raw(method, proc_macro2::Span::call_site())
                } else {
                    syn::Ident::new(method, proc_macro2::Span::call_site())
                };
                Ok(parse_quote! { #object_expr.#method_ident(#(#arg_exprs),*) })
            }
        }
    }

    fn convert_method_call(
        &mut self,
        object: &HirExpr,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<syn::Expr> {
        // Handle chained function calls: outer()() becomes outer().__call__()
        // Convert __call__ to direct invocation of the closure
        if method == "__call__" {
            let callable_expr = object.to_rust_expr(self.ctx)?;
            let arg_exprs: Vec<syn::Expr> = args
                .iter()
                .map(|arg| arg.to_rust_expr(self.ctx))
                .collect::<Result<Vec<_>>>()?;
            return Ok(parse_quote! { (#callable_expr)(#(#arg_exprs),*) });
        }

        // Check for module method calls first (e.g., os.path.join should NOT be treated as string join)
        if let HirExpr::Var(module_name) = object {
            // Handle asyncio.run(coro) - convert to tokio runtime block_on or direct await
            // For now, we'll just call the async function directly without runtime setup
            // since proper async execution requires tokio runtime configuration
            if module_name == "asyncio" && method == "run" {
                if args.len() == 1 {
                    // asyncio.run(coro) → just call coro (generates warning but allows compilation)
                    // In a real async context, this should be handled with tokio::runtime::Runtime::new()
                    let coro_expr = args[0].to_rust_expr(self.ctx)?;
                    // If the argument is an async function call, we can't execute it synchronously
                    // Return a placeholder that will compile but indicate the limitation
                    return Ok(parse_quote! {
                        {
                            // TODO: asyncio.run() requires tokio runtime setup
                            // This placeholder allows compilation but won't execute properly
                            #coro_expr
                        }
                    });
                }
            }
            
            // Handle inspect.iscoroutinefunction(f) - always returns true for async functions
            // In Rust, we know at compile time if a function is async
            if module_name == "inspect" && method == "iscoroutinefunction" {
                // inspect.iscoroutinefunction(f) → true (compile-time knowledge)
                return Ok(parse_quote! { true });
            }
        }

        if let HirExpr::Attribute { value, attr } = object {
            if let HirExpr::Var(module_name) = &**value {
                if module_name == "os" && attr == "path" {
                    if let Some(result) = self.try_convert_os_path_method(method, args)? {
                        return Ok(result);
                    }
                }
            }
        }

        // This ensures string methods like upper/lower are converted even when
        // inside class methods where parameters might be mistyped as class instances
        if matches!(
            method,
            "upper"
                | "lower"
                | "strip"
                | "lstrip"
                | "rstrip"
                | "startswith"
                | "endswith"
                | "split"
                | "splitlines"
                | "join"
                | "replace"
                | "find"
                | "rfind"
                | "rindex"
                | "isdigit"
                | "isalpha"
                | "isalnum"
                | "title"
                | "capitalize"
                | "swapcase"
                | "expandtabs"
                | "center"
                | "ljust"
                | "rjust"
                | "zfill"
                | "format"
        ) {
            // Check if the object is an Optional type that needs unwrapping
            let object_is_optional = self.expr_is_optional(object);
            let mut object_expr = object.to_rust_expr(self.ctx)?;
            if object_is_optional {
                // Unwrap Optional before calling string method
                // This handles patterns like: if s is None: return ""; return s.upper()
                object_expr = parse_quote! { #object_expr.as_ref().unwrap() };
            }
            let arg_exprs: Vec<syn::Expr> = args
                .iter()
                .map(|arg| arg.to_rust_expr(self.ctx))
                .collect::<Result<Vec<_>>>()?;
            return self.convert_string_method(object, &object_expr, method, &arg_exprs, args);
        }

        // Convert to ClassName::method(args) for static method calls
        // But NOT for module-level constants (lazy_static), which should use regular method calls
        // Also NOT for SCREAMING_SNAKE_CASE names which are constants, not classes
        if let HirExpr::Var(class_name) = object {
            let starts_upper = class_name
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            let is_screaming_snake = starts_upper
                && class_name
                    .chars()
                    .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit());
            if starts_upper
                && !is_screaming_snake
                && !self.ctx.lazy_static_constants.contains(class_name)
                && !self.ctx.static_array_constants.contains(class_name)
            {
                // This is likely a static method call - convert to ClassName::method(args)
                let class_ident = syn::Ident::new(class_name, proc_macro2::Span::call_site());
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                let arg_exprs: Vec<syn::Expr> = args
                    .iter()
                    .map(|arg| arg.to_rust_expr(self.ctx))
                    .collect::<Result<Vec<_>>>()?;
                return Ok(parse_quote! { #class_ident::#method_ident(#(#arg_exprs),*) });
            }
        }

        // Try classmethod handling first
        if let Some(result) = self.try_convert_classmethod(object, method, args)? {
            return Ok(result);
        }

        // Try module method handling
        if let Some(result) = self.try_convert_module_method(object, method, args, kwargs)? {
            return Ok(result);
        }

        // Check if this is a mutating method that should not clone the object
        // List/Vec mutating methods: append, extend, clear, insert, remove, reverse, sort, pop
        // Dict/HashMap mutating methods: insert, remove, clear
        // Set/HashSet mutating methods: insert, remove, clear, add, discard
        // Note: Python's set.add() becomes Rust's HashSet.insert()
        let is_mutating_method = matches!(
            method,
            "append"
                | "extend"
                | "clear"
                | "insert"
                | "remove"
                | "reverse"
                | "sort"
                | "pop"
                | "add"
                | "discard"
                | "update"
        );

        // Methods that only need to borrow the object (use .iter() internally)
        // These don't need clone because they only take &self
        // Dict methods like get/keys/values/items borrow via &self;
        // .get().cloned() already handles element cloning.
        let is_reference_method = matches!(
            method,
            "is_none"
                | "is_some"
                | "as_ref"
                | "len"
                | "is_empty"
                | "index"
                | "count"
                | "get"
                | "contains"
                | "contains_key"
                | "keys"
                | "values"
                | "items"
        );

        // For mutating/reference methods, don't add .clone()
        let object_expr = if is_mutating_method || is_reference_method {
            if matches!(object, HirExpr::Attribute { .. }) {
                self.convert_attribute_without_clone(object)?
            } else {
                // For variables/other expressions, set prevent_clone temporarily
                let was_prevent_clone = self.ctx.prevent_clone;
                self.ctx.prevent_clone = true;
                let expr = object.to_rust_expr(self.ctx)?;
                self.ctx.prevent_clone = was_prevent_clone;
                expr
            }
        } else {
            object.to_rust_expr(self.ctx)?
        };

        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Some methods like sort(key=func) need to preserve keyword argument names
        // For other methods, they can merge kwargs as positional if needed
        self.convert_instance_method(object, &object_expr, method, &arg_exprs, args, kwargs)
    }

    fn convert_call_with_type_params(
        &mut self,
        func: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
        type_params: &[Type],
    ) -> Result<syn::Expr> {
        // If no type params, delegate to regular convert_call
        if type_params.is_empty() {
            return self.convert_call(func, args, kwargs);
        }

        // Generate turbofish syntax: func::<T1, T2>(args)
        // First, we need to handle kwargs by merging them with args
        let all_args = if !kwargs.is_empty() {
            // Look up function parameter names for proper reordering
            let maybe_param_names = self.ctx.function_param_names.get(func).cloned();
            let can_reorder = maybe_param_names
                .as_ref()
                .map(|p| p.len() >= args.len() + kwargs.len())
                .unwrap_or(false);
            if can_reorder {
                let param_names = maybe_param_names.unwrap();
                // Build argument list by matching kwargs to parameter positions
                let mut reordered_args: Vec<syn::Expr> = Vec::with_capacity(param_names.len());

                // Create a map from kwarg name to its value for quick lookup
                let kwarg_map: std::collections::HashMap<&str, &HirExpr> = kwargs
                    .iter()
                    .map(|(name, value)| (name.as_str(), value))
                    .collect();

                for (idx, param_name) in param_names.iter().enumerate() {
                    if idx < args.len() {
                        // This position is filled by a positional argument
                        let expr = args[idx].to_rust_expr(self.ctx)?;
                        // STRING_INTEROP: For string literals, add .to_string()
                        let expr = if matches!(
                            &args[idx],
                            HirExpr::Literal(crate::hir::Literal::String(_))
                        ) {
                            parse_quote! { #expr.to_string() }
                        } else {
                            expr
                        };
                        reordered_args.push(expr);
                    } else if let Some(value) = kwarg_map.get(param_name.as_str()) {
                        // This position is filled by a keyword argument
                        let expr = value.to_rust_expr(self.ctx)?;
                        // STRING_INTEROP: For string literals, add .to_string()
                        let expr =
                            if matches!(value, HirExpr::Literal(crate::hir::Literal::String(_))) {
                                parse_quote! { #expr.to_string() }
                            } else {
                                expr
                            };
                        reordered_args.push(expr);
                    }
                    // If neither positional nor kwarg fills this position,
                    // the function likely has a default value (skip it)
                }
                reordered_args
            } else {
                // Fall back to appending kwargs in order if function signature not found
                let mut arg_exprs: Vec<syn::Expr> = args
                    .iter()
                    .map(|arg| {
                        let expr = arg.to_rust_expr(self.ctx)?;
                        // STRING_INTEROP: For string literals, add .to_string()
                        if matches!(arg, HirExpr::Literal(crate::hir::Literal::String(_))) {
                            Ok(parse_quote! { #expr.to_string() })
                        } else {
                            Ok(expr)
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;

                let kwarg_exprs: Vec<syn::Expr> = kwargs
                    .iter()
                    .map(|(_name, value)| {
                        let expr = value.to_rust_expr(self.ctx)?;
                        // STRING_INTEROP: For string literals, add .to_string()
                        if matches!(value, HirExpr::Literal(crate::hir::Literal::String(_))) {
                            Ok(parse_quote! { #expr.to_string() })
                        } else {
                            Ok(expr)
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;

                arg_exprs.extend(kwarg_exprs);
                arg_exprs
            }
        } else {
            // No kwargs - just convert positional args with string literal handling
            args.iter()
                .map(|arg| {
                    let expr = arg.to_rust_expr(self.ctx)?;
                    // STRING_INTEROP: For string literals, add .to_string()
                    if matches!(arg, HirExpr::Literal(crate::hir::Literal::String(_))) {
                        Ok(parse_quote! { #expr.to_string() })
                    } else {
                        Ok(expr)
                    }
                })
                .collect::<Result<Vec<_>>>()?
        };

        let func_ident = syn::Ident::new(func, proc_macro2::Span::call_site());
        let type_tokens = self.convert_type_params_to_tokens(type_params);

        Ok(parse_quote! { #func_ident::<#type_tokens>(#(#all_args),*) })
    }

    fn convert_method_call_with_type_params(
        &mut self,
        object: &HirExpr,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
        type_params: &[Type],
    ) -> Result<syn::Expr> {
        // If no type params, delegate to regular convert_method_call
        if type_params.is_empty() {
            return self.convert_method_call(object, method, args, kwargs);
        }

        // Generate turbofish syntax: obj.method::<T1, T2>(args)
        let object_expr = object.to_rust_expr(self.ctx)?;
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
        let type_tokens = self.convert_type_params_to_tokens(type_params);

        Ok(parse_quote! { #object_expr.#method_ident::<#type_tokens>(#(#arg_exprs),*) })
    }

    fn convert_type_params_to_tokens(&self, type_params: &[Type]) -> proc_macro2::TokenStream {
        let type_tokens: Vec<proc_macro2::TokenStream> =
            type_params.iter().map(|t| self.type_to_tokens(t)).collect();
        quote! { #(#type_tokens),* }
    }

    fn type_to_tokens(&self, ty: &Type) -> proc_macro2::TokenStream {
        match ty {
            Type::Int => quote! { i32 },
            Type::Float => quote! { f64 },
            Type::String => quote! { String },
            Type::Bool => quote! { bool },
            Type::List(inner) => {
                let inner_tokens = self.type_to_tokens(inner);
                quote! { Vec<#inner_tokens> }
            }
            Type::Dict(k, v) => {
                let k_tokens = self.type_to_tokens(k);
                let v_tokens = self.type_to_tokens(v);
                quote! { HashMap<#k_tokens, #v_tokens> }
            }
            Type::Set(inner) => {
                let inner_tokens = self.type_to_tokens(inner);
                quote! { HashSet<#inner_tokens> }
            }
            Type::Tuple(types) => {
                let type_tokens: Vec<proc_macro2::TokenStream> =
                    types.iter().map(|t| self.type_to_tokens(t)).collect();
                quote! { (#(#type_tokens),*) }
            }
            Type::Optional(inner) => {
                let inner_tokens = self.type_to_tokens(inner);
                quote! { Option<#inner_tokens> }
            }
            Type::TypeVar(name) => {
                let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                quote! { #ident }
            }
            Type::Custom(name) => {
                let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                quote! { #ident }
            }
            Type::Unknown => quote! { _ },
            _ => quote! { _ },
        }
    }

    fn convert_index(&mut self, base: &HirExpr, index: &HirExpr) -> Result<syn::Expr> {
        // Set needs_indexerror flag since indexing operations use .unwrap()
        // which could panic with an index error. This generates the IndexError
        // struct definition as a safety marker.
        self.ctx.needs_indexerror = true;

        // Optimization: [x for x in iter if cond][0] → iter.find(|x| cond).unwrap()
        // When indexing [0] into a filtered list comprehension, use find() instead of collect().get(0)
        if let HirExpr::Literal(Literal::Int(0)) = index {
            if let HirExpr::ListComp {
                element,
                target,
                iter,
                condition: Some(condition),
            } = base
            {
                return self.convert_list_comp_first_element(element, target, iter, condition);
            }
        }

        // Must check this before evaluating base_expr to avoid trying to convert os.environ
        if let HirExpr::Attribute { value, attr } = base {
            if let HirExpr::Var(module_name) = &**value {
                if module_name == "os" && attr == "environ" {
                    let index_expr = index.to_rust_expr(self.ctx)?;
                    return Ok(parse_quote! { std::env::var(#index_expr).unwrap() });
                }
            }
        }

        // Check if base is Optional - if so, we need to unwrap it before indexing
        let base_is_optional = self.ctx.get_optional_inner_type(base).is_some();

        // When the base is an attribute expression (e.g., state.field.list), don't add .clone()
        // to the entire collection. The .get().cloned() pattern handles element cloning.
        // This avoids generating redundant code like: vec.clone().get(i).cloned().unwrap()
        let mut base_expr = if matches!(base, HirExpr::Attribute { .. }) {
            self.convert_attribute_without_clone(base)?
        } else {
            // For other base types, preserve original behavior but set prevent_clone
            // to avoid unnecessary cloning of the collection itself
            let was_prevent_clone = self.ctx.prevent_clone;
            self.ctx.prevent_clone = true;
            let expr = base.to_rust_expr(self.ctx)?;
            self.ctx.prevent_clone = was_prevent_clone;
            expr
        };

        // If base is Optional, unwrap the Option before indexing
        // Use as_mut() for assignment targets to allow mutation
        if base_is_optional {
            if self.ctx.is_assignment_target {
                base_expr = parse_quote! { #base_expr.as_mut().unwrap() };
            } else {
                base_expr = parse_quote! { #base_expr.as_ref().unwrap() };
            }
        }

        // Python: tuple[0], tuple[1] → Rust: tuple.0, tuple.1
        // Also handles chained indexing: list_of_tuples[i][j] → list_of_tuples.get(i).0
        let should_use_tuple_syntax = if let HirExpr::Literal(Literal::Int(idx)) = index {
            if *idx >= 0 {
                // Use get_expr_type for broad coverage (Var, Attribute, etc.)
                if let Some(base_type) = self.ctx.get_expr_type(base) {
                    matches!(base_type, Type::Tuple(_))
                } else if let HirExpr::Var(var_name) = base {
                    // Fallback heuristic: variable names suggesting tuple iteration
                    matches!(
                        var_name.as_str(),
                        "pair" | "entry" | "item" | "elem" | "tuple" | "row"
                    )
                } else if let HirExpr::Index {
                    base: inner_base, ..
                } = base
                {
                    // Check if we're indexing into a List[Tuple]
                    if let HirExpr::Var(var_name) = &**inner_base {
                        if let Some(Type::List(element_type)) = self.ctx.var_types.get(var_name) {
                            matches!(**element_type, Type::Tuple(_))
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if should_use_tuple_syntax {
            if let HirExpr::Literal(Literal::Int(idx)) = index {
                let field_idx = syn::Index::from(*idx as usize);
                return Ok(parse_quote! { #base_expr.#field_idx });
            }
        }

        let is_string_base = self.is_string_base(base);

        // Static array constants use direct indexing instead of .get().cloned()
        if let HirExpr::Var(var_name) = base {
            if self.ctx.static_array_constants.contains(var_name) {
                let was_pc = self.ctx.prevent_clone;
                self.ctx.prevent_clone = false;
                let index_expr = index.to_rust_expr(self.ctx)?;
                self.ctx.prevent_clone = was_pc;
                if let HirExpr::Literal(Literal::Int(n)) = index {
                    let idx_value = *n as usize;
                    return Ok(parse_quote! { #base_expr[#idx_value] });
                }
                return Ok(parse_quote! { #base_expr[#index_expr as usize] });
            }
        }

        // Discriminate between HashMap and Vec access based on base type or index type
        let is_string_key = self.is_string_index(base, index)?;

        // DEPYLER-FIX: When used as assignment target (LHS), use get_mut() instead of get().cloned()
        // This allows modifying the element in place rather than modifying a clone
        let is_lhs = self.ctx.is_assignment_target;

        if is_string_key {
            // HashMap/Dict access with string keys
            match index {
                HirExpr::Literal(Literal::String(s)) => {
                    // String literal - use it directly without .to_string()
                    if is_lhs {
                        Ok(parse_quote! {
                            #base_expr.get_mut(#s).unwrap()
                        })
                    } else if self.ctx.prevent_clone {
                        // Field access context: return &V reference, auto-deref handles field access
                        Ok(parse_quote! {
                            #base_expr.get(#s).unwrap()
                        })
                    } else {
                        Ok(parse_quote! {
                            #base_expr.get(#s).cloned().unwrap()
                        })
                    }
                }
                _ => {
                    // String variable - needs proper referencing
                    // HashMap.get() expects &K, so we need to borrow the key
                    let was_pc = self.ctx.prevent_clone;
                    self.ctx.prevent_clone = false;
                    let index_expr = index.to_rust_expr(self.ctx)?;
                    self.ctx.prevent_clone = was_pc;
                    if is_lhs {
                        Ok(parse_quote! {
                            #base_expr.get_mut(&#index_expr).unwrap()
                        })
                    } else if self.ctx.prevent_clone {
                        Ok(parse_quote! {
                            #base_expr.get(&#index_expr).unwrap()
                        })
                    } else {
                        Ok(parse_quote! {
                            #base_expr.get(&#index_expr).cloned().unwrap()
                        })
                    }
                }
            }
        } else if is_string_base {
            // Strings cannot use .get(usize), must use .chars().nth()
            let was_pc = self.ctx.prevent_clone;
            self.ctx.prevent_clone = false;
            let index_expr = index.to_rust_expr(self.ctx)?;
            self.ctx.prevent_clone = was_pc;

            // This returns Option<char>, then convert to String
            Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let idx: i32 = #index_expr;
                    let actual_idx = if idx < 0 {
                        base.chars().count().saturating_sub(idx.abs() as usize)
                    } else {
                        idx as usize
                    };
                    base.chars().nth(actual_idx).map(|c| c.to_string()).unwrap()
                }
            })
        } else {
            // Vec/List access with numeric index
            let was_pc = self.ctx.prevent_clone;
            self.ctx.prevent_clone = false;
            let index_expr = index.to_rust_expr(self.ctx)?;
            self.ctx.prevent_clone = was_pc;

            // When the caller wants a reference (&mut or &), use direct indexing
            // instead of .get().cloned() so the reference points to the actual element
            let wants_ref = !is_lhs && (self.ctx.generate_mut_borrow || self.ctx.generate_borrow);

            // Check if index is a negative literal
            if let HirExpr::Unary {
                op: UnaryOp::Neg,
                operand,
            } = index
            {
                if let HirExpr::Literal(Literal::Int(n)) = **operand {
                    let offset = n as usize;
                    // Special case for -1: use .last() or .last_mut()
                    // Works for both Copy and non-Copy types (like String, Vec)
                    if offset == 1 {
                        if is_lhs {
                            return Ok(parse_quote! { #base_expr.last_mut().unwrap() });
                        } else if self.ctx.prevent_clone {
                            return Ok(parse_quote! { #base_expr.last().unwrap() });
                        } else {
                            return Ok(parse_quote! { #base_expr.last().cloned().unwrap() });
                        }
                    }
                    // For other negative indices, use .get() or .get_mut()
                    if is_lhs {
                        return Ok(parse_quote! {
                            #base_expr.get_mut(#base_expr.len().saturating_sub(#offset)).unwrap()
                        });
                    } else if self.ctx.prevent_clone {
                        return Ok(parse_quote! {
                            #base_expr.get(#base_expr.len().saturating_sub(#offset)).unwrap()
                        });
                    } else {
                        return Ok(parse_quote! {
                            #base_expr.get(#base_expr.len().saturating_sub(#offset)).cloned().unwrap()
                        });
                    }
                }
            }

            // For literal indices like p[0], generate simple inline code: .get(0) or .get_mut(0)
            // This avoids unnecessary temporary variables and runtime checks
            if let HirExpr::Literal(Literal::Int(n)) = index {
                let idx_value = *n as usize;
                if is_lhs {
                    return Ok(parse_quote! {
                        #base_expr.get_mut(#idx_value).unwrap()
                    });
                } else if wants_ref {
                    return Ok(parse_quote! { #base_expr[#idx_value] });
                } else if self.ctx.prevent_clone {
                    return Ok(parse_quote! {
                        #base_expr.get(#idx_value).unwrap()
                    });
                } else {
                    return Ok(parse_quote! {
                        #base_expr.get(#idx_value).cloned().unwrap()
                    });
                }
            }

            // Simple expressions that are guaranteed non-negative and don't need
            // Python negative index handling:
            // - Variables in for loops like `for i in range(len(arr))`
            // - Function calls like `sample(distribution, k)` which return indices
            // - Cast expressions like `int(sample(...))` which convert to int
            let is_simple_expr = matches!(index, HirExpr::Var(_) | HirExpr::Call { .. });

            if is_simple_expr {
                if is_lhs {
                    Ok(parse_quote! {
                        #base_expr.get_mut(#index_expr as usize).unwrap()
                    })
                } else if wants_ref {
                    Ok(parse_quote! { #base_expr[#index_expr as usize] })
                } else if self.ctx.prevent_clone {
                    Ok(parse_quote! {
                        #base_expr.get(#index_expr as usize).unwrap()
                    })
                } else {
                    Ok(parse_quote! {
                        #base_expr.get(#index_expr as usize).cloned().unwrap()
                    })
                }
            } else {
                // Complex expression - use block with full negative index handling
                if is_lhs {
                    Ok(parse_quote! {
                        {
                            let base = &mut #base_expr;
                            let idx: i32 = #index_expr;
                            let actual_idx = if idx < 0 {
                                base.len().saturating_sub(idx.abs() as usize)
                            } else {
                                idx as usize
                            };
                            base.get_mut(actual_idx).unwrap()
                        }
                    })
                } else if self.ctx.prevent_clone {
                    Ok(parse_quote! {
                        {
                            let base = &#base_expr;
                            let idx: i32 = #index_expr;
                            let actual_idx = if idx < 0 {
                                base.len().saturating_sub(idx.abs() as usize)
                            } else {
                                idx as usize
                            };
                            base.get(actual_idx).unwrap()
                        }
                    })
                } else {
                    Ok(parse_quote! {
                        {
                            let base = &#base_expr;
                            let idx: i32 = #index_expr;
                            let actual_idx = if idx < 0 {
                                base.len().saturating_sub(idx.abs() as usize)
                            } else {
                                idx as usize
                            };
                            base.get(actual_idx).cloned().unwrap()
                        }
                    })
                }
            }
        }
    }

    /// Check if the index expression is a string key (for HashMap access)
    /// Returns true if: index is string literal, OR base is Dict/HashMap type
    fn is_string_index(&self, base: &HirExpr, index: &HirExpr) -> Result<bool> {
        // Check 1: Is index a string literal?
        if matches!(index, HirExpr::Literal(Literal::String(_))) {
            return Ok(true);
        }

        // Check 2: Is base expression a Dict/HashMap type?
        // We need to look at the base's inferred type
        if let HirExpr::Var(sym) = base {
            if let Some(var_type) = self.ctx.var_types.get(sym) {
                // If variable is typed as serde_json::Value or Dict, use string indexing
                if matches!(var_type, Type::Dict(_, _)) {
                    return Ok(true);
                }
            }

            // Try to find the variable's type in the current function context
            // For parameters, we can check the function signature
            // For local variables, this is harder without full type inference

            // Only use "dict" or "map" which are more specific to HashMap variables
            let name = sym.as_str();
            if (name.contains("dict")
                || name.contains("map")
                || name.contains("config")
                || name.contains("value"))
                && !self.is_numeric_index(index)
            {
                return Ok(true);
            }
        }

        // Check 3: Does the index expression look like a string variable?
        if self.is_string_variable(index) {
            return Ok(true);
        }

        // Default: assume numeric index (Vec/List access)
        Ok(false)
    }

    /// Check if expression is likely a string variable (heuristic)
    fn is_string_variable(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Var(sym) => {
                if let Some(var_type) = self.ctx.var_types.get(sym) {
                    // If variable is typed as String, it's a string index
                    if matches!(var_type, Type::String) {
                        return true;
                    }
                }

                // Fallback to heuristics
                let name = sym.as_str();
                // Heuristic: variable names like "key", "name", "id", "word", etc.
                name == "key"
                    || name == "k" // Common loop variable for keys
                    || name == "name"
                    || name == "id"
                    || name == "word"
                    || name == "text"
                    || name.ends_with("_key")
                    || name.ends_with("_name")
            }
            _ => false,
        }
    }

    /// Check if expression is String type using context type information
    fn is_string_type_from_context(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Var(name) => {
                // Check var_types for String type
                self.ctx
                    .var_types
                    .get(name)
                    .map_or(false, |t| matches!(t, Type::String))
            }
            HirExpr::Attribute { value, attr } => {
                // Check class field types for attribute access like `state.separator`
                if let HirExpr::Var(obj_name) = value.as_ref() {
                    // Special case: "self" refers to the current class instance
                    if obj_name == "self" {
                        for (_, field_types) in &self.ctx.class_field_types {
                            if let Some(t) = field_types.get(attr) {
                                if matches!(t, Type::String) {
                                    return true;
                                }
                            }
                        }
                        return false;
                    }
                    // Look up object type - if it's a known class, check its field types
                    if let Some(obj_type) = self.ctx.var_types.get(obj_name) {
                        if let Type::Custom(class_name) = obj_type {
                            if let Some(field_types) = self.ctx.class_field_types.get(class_name) {
                                return field_types
                                    .get(attr)
                                    .map_or(false, |t| matches!(t, Type::String));
                            }
                        }
                    }
                    // Also check if the class name matches directly
                    for (class_name, field_types) in &self.ctx.class_field_types {
                        if self.ctx.class_names.contains(class_name) {
                            if let Some(t) = field_types.get(attr) {
                                if matches!(t, Type::String) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            HirExpr::MethodCall { object, method, .. } => {
                // Methods that return strings
                let string_methods = [
                    "clone",
                    "to_string",
                    "trim",
                    "strip",
                    "upper",
                    "lower",
                    "title",
                ];
                if string_methods.contains(&method.as_str()) {
                    return self.is_string_type_from_context(object) || self.is_string_base(object);
                }
                false
            }
            _ => false,
        }
    }

    /// Check if expression is likely numeric (heuristic)
    fn is_numeric_index(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Literal(Literal::Int(_)) => true,
            HirExpr::Var(sym) => {
                let name = sym.as_str();
                // Common numeric index names
                name == "i"
                    || name == "j"
                    || name == "k"
                    || name == "idx"
                    || name == "index"
                    || name.starts_with("idx_")
                    || name.ends_with("_idx")
                    || name.ends_with("_index")
            }
            HirExpr::Binary { .. } => true, // Arithmetic expressions are numeric
            HirExpr::Call { .. } => false,  // Could be anything
            _ => false,
        }
    }

    /// Check if an element in array repeat syntax needs .clone() because it's not Copy
    /// Used for patterns like [x] * n where x is a variable
    fn element_needs_clone(&self, elem: &HirExpr) -> bool {
        match elem {
            // Literals are Copy
            HirExpr::Literal(_) => false,
            // Check if variable refers to a non-Copy type
            HirExpr::Var(name) => {
                if let Some(ty) = self.ctx.var_types.get(name) {
                    // Vec, String, HashMap, HashSet are not Copy
                    matches!(
                        ty,
                        Type::List(_)
                            | Type::String
                            | Type::Dict(_, _)
                            | Type::Set(_)
                            | Type::Custom(_)
                    )
                } else {
                    // If we don't know the type, assume it needs clone to be safe
                    // (better to have an unnecessary .clone() than a compile error)
                    true
                }
            }
            // Binary multiplication [elem] * n might produce Copy array if elem is Copy
            HirExpr::Binary {
                op: BinOp::Mul,
                left,
                right,
            } => {
                match (left.as_ref(), right.as_ref()) {
                    // [elem] * n - check if the element is Copy and size is small
                    (HirExpr::List(elems), HirExpr::Literal(Literal::Int(size)))
                        if elems.len() == 1 && *size > 0 && *size <= 32 =>
                    {
                        self.element_needs_clone(&elems[0])
                    }
                    (HirExpr::Literal(Literal::Int(size)), HirExpr::List(elems))
                        if elems.len() == 1 && *size > 0 && *size <= 32 =>
                    {
                        self.element_needs_clone(&elems[0])
                    }
                    // Large arrays or other patterns need clone
                    _ => true,
                }
            }
            // Lists that aren't part of multiplication pattern need clone
            HirExpr::List(_) | HirExpr::Dict(_) | HirExpr::Set(_) => true,
            // Tuples are Copy only if all elements are Copy
            HirExpr::Tuple(elems) => elems.iter().any(|e| self.element_needs_clone(e)),
            // Calls might return non-Copy types
            HirExpr::Call { func, .. } => {
                // Common functions that return Copy types
                !matches!(
                    func.as_str(),
                    "len" | "int" | "float" | "bool" | "ord" | "abs" | "hash"
                )
            }
            // Method calls often return non-Copy
            HirExpr::MethodCall { .. } => true,
            // Attribute access - conservative, assume non-Copy
            HirExpr::Attribute { .. } => true,
            // Other expressions - be conservative
            _ => true,
        }
    }

    /// Returns true if base is likely a String/str type (not Vec/List)
    fn is_string_base(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Literal(Literal::String(_)) => true,
            HirExpr::Var(sym) => {
                // "words" (plural) is likely list[str], not str!
                // "word" (singular) without 's' ending is likely str
                let name = sym.as_str();
                // Only match if: singular AND string-like name
                let is_singular = !name.ends_with('s');
                name == "text"
                    || name == "s"
                    || name == "string"
                    || name == "line"
                    || (name == "word" && is_singular)
                    || (name.starts_with("text") && is_singular)
                    || (name.starts_with("str") && is_singular)
                    || (name.ends_with("_str") && is_singular)
                    || (name.ends_with("_string") && is_singular)
                    || (name.ends_with("_word") && is_singular)
                    || (name.ends_with("_text") && is_singular)
                    || name == "suffix"
                    || name == "prefix"
            }
            // Check attribute access on self or other objects
            HirExpr::Attribute { value, attr } => {
                // For self.field, check if the field is a String type
                if let HirExpr::Var(obj_name) = value.as_ref() {
                    if obj_name == "self" {
                        for (_, field_types) in &self.ctx.class_field_types {
                            if let Some(t) = field_types.get(attr) {
                                if matches!(t, Type::String) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            HirExpr::MethodCall { method, .. }
                if method.as_str().contains("upper")
                    || method.as_str().contains("lower")
                    || method.as_str().contains("strip")
                    || method.as_str().contains("lstrip")
                    || method.as_str().contains("rstrip")
                    || method.as_str().contains("title") =>
            {
                true
            }
            HirExpr::Call { func, .. } if func.as_str() == "str" => true,
            _ => false,
        }
    }

    /// Used to detect .get() on Vec<String> and similar patterns
    ///
    /// # Complexity
    /// 6 (match + type lookup + method check + variable name check)
    fn is_string_method_call(&self, object: &HirExpr, method: &str, _args: &[HirExpr]) -> bool {
        // Check if object is Vec<String> and method is .get()
        if method == "get" {
            if let HirExpr::Var(var_name) = object {
                // Check var_types to see if this is Vec<String>
                if let Some(Type::List(inner_type)) = self.ctx.var_types.get(var_name) {
                    return matches!(inner_type.as_ref(), Type::String);
                }
                // Heuristic: Variable names containing "data", "items", "strings", etc.
                let name = var_name.as_str();
                return name.contains("str") || name.contains("data") || name.contains("text");
            }
        }

        // String methods that return String
        matches!(
            method,
            "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "title" | "replace" | "format"
        )
    }

    fn convert_slice(
        &mut self,
        base: &HirExpr,
        start: &Option<Box<HirExpr>>,
        stop: &Option<Box<HirExpr>>,
        step: &Option<Box<HirExpr>>,
    ) -> Result<syn::Expr> {
        // Use convert_expr_without_clone since we borrow the base immediately with `let base = &...`
        let base_expr = self.convert_expr_without_clone(base)?;

        let is_string = self.is_string_base(base);

        // Convert slice parameters
        let start_expr = if let Some(s) = start {
            Some(s.to_rust_expr(self.ctx)?)
        } else {
            None
        };

        let stop_expr = if let Some(s) = stop {
            Some(s.to_rust_expr(self.ctx)?)
        } else {
            None
        };

        let step_expr = if let Some(s) = step {
            Some(s.to_rust_expr(self.ctx)?)
        } else {
            None
        };

        if is_string {
            return self.convert_string_slice(base_expr, start_expr, stop_expr, step_expr);
        }

        // Generate slice code based on the parameters (for Vec/List)
        match (start_expr, stop_expr, step_expr) {
            // Full slice with step: base[::step]
            (None, None, Some(step)) => {
                Ok(parse_quote! {
                    {
                        let base = &#base_expr;
                        let step = #step;
                        if step == 1 {
                            base.clone()
                        } else if step > 0 {
                            base.iter().step_by(step as usize).cloned().collect::<Vec<_>>()
                        } else if step == -1 {
                            base.iter().rev().cloned().collect::<Vec<_>>()
                        } else {
                            // Negative step with abs value
                            let abs_step = (-step) as usize;
                            base.iter().rev().step_by(abs_step).cloned().collect::<Vec<_>>()
                        }
                    }
                })
            }

            // Start and stop: base[start:stop]
            (Some(start), Some(stop), None) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let start = (#start).max(0) as usize;
                    let stop = (#stop).max(0) as usize;
                    if start < base.len() {
                        base[start..stop.min(base.len())].to_vec()
                    } else {
                        Vec::new()
                    }
                }
            }),

            // Start only: base[start:]
            (Some(start), None, None) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let start = (#start).max(0) as usize;
                    if start < base.len() {
                        base[start..].to_vec()
                    } else {
                        Vec::new()
                    }
                }
            }),

            // Stop only: base[:stop]
            (None, Some(stop), None) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let stop = (#stop).max(0) as usize;
                    base[..stop.min(base.len())].to_vec()
                }
            }),

            // Full slice: base[:]
            (None, None, None) => Ok(parse_quote! { #base_expr.clone() }),

            // Start, stop, and step: base[start:stop:step]
            (Some(start), Some(stop), Some(step)) => {
                Ok(parse_quote! {
                    {
                        let base = &#base_expr;
                        let start = (#start).max(0) as usize;
                        let stop = (#stop).max(0) as usize;
                        let step = #step;

                        if step == 1 {
                            if start < base.len() {
                                base[start..stop.min(base.len())].to_vec()
                            } else {
                                Vec::new()
                            }
                        } else if step > 0 {
                            base[start..stop.min(base.len())]
                                .iter()
                                .step_by(step as usize)
                                .cloned()
                                .collect::<Vec<_>>()
                        } else {
                            // Negative step - slice in reverse
                            let abs_step = (-step) as usize;
                            if start < base.len() {
                                base[start..stop.min(base.len())]
                                    .iter()
                                    .rev()
                                    .step_by(abs_step)
                                    .cloned()
                                    .collect::<Vec<_>>()
                            } else {
                                Vec::new()
                            }
                        }
                    }
                })
            }

            // Start and step: base[start::step]
            (Some(start), None, Some(step)) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let start = (#start).max(0) as usize;
                    let step = #step;

                    if start < base.len() {
                        if step == 1 {
                            base[start..].to_vec()
                        } else if step > 0 {
                            base[start..]
                                .iter()
                                .step_by(step as usize)
                                .cloned()
                                .collect::<Vec<_>>()
                        } else if step == -1 {
                            base[start..]
                                .iter()
                                .rev()
                                .cloned()
                                .collect::<Vec<_>>()
                        } else {
                            let abs_step = (-step) as usize;
                            base[start..]
                                .iter()
                                .rev()
                                .step_by(abs_step)
                                .cloned()
                                .collect::<Vec<_>>()
                        }
                    } else {
                        Vec::new()
                    }
                }
            }),

            // Stop and step: base[:stop:step]
            (None, Some(stop), Some(step)) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let stop = (#stop).max(0) as usize;
                    let step = #step;

                    if step == 1 {
                        base[..stop.min(base.len())].to_vec()
                    } else if step > 0 {
                        base[..stop.min(base.len())]
                            .iter()
                            .step_by(step as usize)
                            .cloned()
                            .collect::<Vec<_>>()
                    } else if step == -1 {
                        base[..stop.min(base.len())]
                            .iter()
                            .rev()
                            .cloned()
                            .collect::<Vec<_>>()
                    } else {
                        let abs_step = (-step) as usize;
                        base[..stop.min(base.len())]
                            .iter()
                            .rev()
                            .step_by(abs_step)
                            .cloned()
                            .collect::<Vec<_>>()
                    }
                }
            }),
        }
    }

    /// Handles string slicing with proper char boundaries and negative indices
    fn convert_string_slice(
        &mut self,
        base_expr: syn::Expr,
        start_expr: Option<syn::Expr>,
        stop_expr: Option<syn::Expr>,
        step_expr: Option<syn::Expr>,
    ) -> Result<syn::Expr> {
        match (start_expr, stop_expr, step_expr) {
            // Full slice with step: s[::step]
            (None, None, Some(step)) => {
                Ok(parse_quote! {
                    {
                        let base = &#base_expr;
                        let step: i32 = #step;
                        if step == 1 {
                            base.to_string()
                        } else if step > 0 {
                            base.chars().step_by(step as usize).collect::<String>()
                        } else if step == -1 {
                            base.chars().rev().collect::<String>()
                        } else {
                            // Negative step with abs value
                            let abs_step = step.abs() as usize;
                            base.chars().rev().step_by(abs_step).collect::<String>()
                        }
                    }
                })
            }

            // Start and stop: s[start:stop]
            (Some(start), Some(stop), None) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let start_idx: i32 = #start;
                    let stop_idx: i32 = #stop;
                    let len = base.chars().count() as i32;

                    // Handle negative indices
                    let actual_start = if start_idx < 0 {
                        (len + start_idx).max(0) as usize
                    } else {
                        start_idx.min(len) as usize
                    };

                    let actual_stop = if stop_idx < 0 {
                        (len + stop_idx).max(0) as usize
                    } else {
                        stop_idx.min(len) as usize
                    };

                    if actual_start < actual_stop {
                        base.chars().skip(actual_start).take(actual_stop - actual_start).collect::<String>()
                    } else {
                        String::new()
                    }
                }
            }),

            // Start only: s[start:]
            (Some(start), None, None) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let start_idx: i32 = #start;
                    let len = base.chars().count() as i32;

                    // Handle negative index for s[-n:]
                    let actual_start = if start_idx < 0 {
                        (len + start_idx).max(0) as usize
                    } else {
                        start_idx.min(len) as usize
                    };

                    base.chars().skip(actual_start).collect::<String>()
                }
            }),

            // Stop only: s[:stop]
            (None, Some(stop), None) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let stop_idx: i32 = #stop;
                    let len = base.chars().count() as i32;

                    // Handle negative index for s[:-n]
                    let actual_stop = if stop_idx < 0 {
                        (len + stop_idx).max(0) as usize
                    } else {
                        stop_idx.min(len) as usize
                    };

                    base.chars().take(actual_stop).collect::<String>()
                }
            }),

            // Full slice: s[:]
            (None, None, None) => Ok(parse_quote! { #base_expr.to_string() }),

            // Start, stop, and step: s[start:stop:step]
            (Some(start), Some(stop), Some(step)) => {
                Ok(parse_quote! {
                    {
                        let base = &#base_expr;
                        let start_idx: i32 = #start;
                        let stop_idx: i32 = #stop;
                        let step: i32 = #step;
                        let len = base.chars().count() as i32;

                        // Handle negative indices
                        let actual_start = if start_idx < 0 {
                            (len + start_idx).max(0) as usize
                        } else {
                            start_idx.min(len) as usize
                        };

                        let actual_stop = if stop_idx < 0 {
                            (len + stop_idx).max(0) as usize
                        } else {
                            stop_idx.min(len) as usize
                        };

                        if step == 1 {
                            if actual_start < actual_stop {
                                base.chars().skip(actual_start).take(actual_stop - actual_start).collect::<String>()
                            } else {
                                String::new()
                            }
                        } else if step > 0 {
                            base.chars()
                                .skip(actual_start)
                                .take(actual_stop.saturating_sub(actual_start))
                                .step_by(step as usize)
                                .collect::<String>()
                        } else {
                            // Negative step - collect range then reverse
                            let abs_step = step.abs() as usize;
                            if actual_start < actual_stop {
                                base.chars()
                                    .skip(actual_start)
                                    .take(actual_stop - actual_start)
                                    .rev()
                                    .step_by(abs_step)
                                    .collect::<String>()
                            } else {
                                String::new()
                            }
                        }
                    }
                })
            }

            // Start and step: s[start::step]
            (Some(start), None, Some(step)) => Ok(parse_quote! {
                {
                    let base = &#base_expr;
                    let start_idx: i32 = #start;
                    let step: i32 = #step;
                    let len = base.chars().count() as i32;

                    let actual_start = if start_idx < 0 {
                        (len + start_idx).max(0) as usize
                    } else {
                        start_idx.min(len) as usize
                    };

                    if step == 1 {
                        base.chars().skip(actual_start).collect::<String>()
                    } else if step > 0 {
                        base.chars().skip(actual_start).step_by(step as usize).collect::<String>()
                    } else if step == -1 {
                        base.chars().skip(actual_start).rev().collect::<String>()
                    } else {
                        let abs_step = step.abs() as usize;
                        base.chars().skip(actual_start).rev().step_by(abs_step).collect::<String>()
                    }
                }
            }),

            // Stop and step: s[:stop:step]
            (None, Some(stop), Some(step)) => Ok(parse_quote! {
                {
                    let base = #base_expr;
                    let stop_idx: i32 = #stop;
                    let step: i32 = #step;
                    let len = base.chars().count() as i32;

                    let actual_stop = if stop_idx < 0 {
                        (len + stop_idx).max(0) as usize
                    } else {
                        stop_idx.min(len) as usize
                    };

                    if step == 1 {
                        base.chars().take(actual_stop).collect::<String>()
                    } else if step > 0 {
                        base.chars().take(actual_stop).step_by(step as usize).collect::<String>()
                    } else if step == -1 {
                        base.chars().take(actual_stop).rev().collect::<String>()
                    } else {
                        let abs_step = step.abs() as usize;
                        base.chars().take(actual_stop).rev().step_by(abs_step).collect::<String>()
                    }
                }
            }),
        }
    }

    fn convert_list(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
        // List literals with string elements should use Vec<String> not Vec<&str>
        // This ensures they can be passed to functions expecting &Vec<String>
        let elt_exprs: Vec<syn::Expr> = elts
            .iter()
            .map(|e| {
                let mut expr = e.to_rust_expr(self.ctx)?;
                // Check if element is a string literal
                if matches!(e, HirExpr::Literal(Literal::String(_))) {
                    expr = parse_quote! { #expr.to_string() };
                }
                Ok(expr)
            })
            .collect::<Result<Vec<_>>>()?;

        // // Use smallvec! for small list literals (≤8 elements) to avoid heap allocation
        // // SmallVec stores elements inline on the stack, improving cache locality
        // if elts.len() <= 8 {
        //     self.ctx.needs_smallvec = true;
        //     Ok(parse_quote! { smallvec::smallvec![#(#elt_exprs),*] })
        // } else {
        //     Ok(parse_quote! { vec![#(#elt_exprs),*] })
        // }
        Ok(parse_quote! { vec![#(#elt_exprs),*] })
    }

    fn convert_dict(&mut self, items: &[(HirExpr, HirExpr)]) -> Result<syn::Expr> {
        // For mixed types, use serde_json::json! instead of HashMap
        let has_mixed_types = self.dict_has_mixed_types(items)?;

        if has_mixed_types {
            // Use serde_json::json! for heterogeneous dicts
            self.ctx.needs_serde_json = true;
            let mut entries = Vec::new();
            for (key, value) in items {
                let key_str = match key {
                    HirExpr::Literal(Literal::String(s)) => s.clone(),
                    _ => bail!("Dict keys for JSON output must be string literals"),
                };
                let val_expr = value.to_rust_expr(self.ctx)?;
                entries.push(quote! { #key_str: #val_expr });
            }

            return Ok(parse_quote! {
                serde_json::json!({
                    #(#entries),*
                })
            });
        }

        // Homogeneous dict: use HashMap
        self.ctx.needs_hashmap = true;

        let mut insert_stmts = Vec::new();
        for (key, value) in items {
            let mut key_expr = key.to_rust_expr(self.ctx)?;
            let mut val_expr = value.to_rust_expr(self.ctx)?;

            // Dict literals should use HashMap<String, V> not HashMap<&str, V>
            // This ensures they can be passed to functions expecting HashMap<String, V>
            if matches!(key, HirExpr::Literal(Literal::String(_))) {
                key_expr = parse_quote! { #key_expr.to_string() };
            }

            // This ensures type consistency: all values in the HashMap have the same type
            // Without this, mixing "literal" (& str) and variable.to_string() (String) causes errors
            if matches!(value, HirExpr::Literal(Literal::String(_))) {
                val_expr = parse_quote! { #val_expr.to_string() };
            }

            insert_stmts.push(quote! { map.insert(#key_expr, #val_expr); });
        }

        // Empty dicts don't need mutable bindings
        if items.is_empty() {
            Ok(parse_quote! {
                {
                    let map = HashMap::new();
                    map
                }
            })
        } else {
            Ok(parse_quote! {
                {
                    let mut map = HashMap::new();
                    #(#insert_stmts)*
                    map
                }
            })
        }
    }

    fn dict_has_mixed_types(&self, items: &[(HirExpr, HirExpr)]) -> Result<bool> {
        if items.len() <= 1 {
            return Ok(false); // Single type or empty
        }

        // STRATEGY 1: Check for obvious mixing of literal types
        let mut has_bool_literal = false;
        let mut has_int_literal = false;
        let mut has_float_literal = false;
        let mut has_string_literal = false;

        for (_key, value) in items {
            match value {
                HirExpr::Literal(Literal::Bool(_)) => has_bool_literal = true,
                HirExpr::Literal(Literal::Int(_)) => has_int_literal = true,
                HirExpr::Literal(Literal::Float(_)) => has_float_literal = true,
                HirExpr::Literal(Literal::String(_)) => has_string_literal = true,
                _ => {}
            }
        }

        // Count how many distinct literal types we have
        let distinct_types = [
            has_bool_literal,
            has_int_literal,
            has_float_literal,
            has_string_literal,
        ]
        .iter()
        .filter(|&&b| b)
        .count();

        // Only use json! if we have 2+ distinct literal types
        // This avoids false positives from dicts with uniform types but variable values
        Ok(distinct_types >= 2)
    }

    fn convert_tuple(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
        // Track variable names to detect duplicates that need cloning
        let mut var_occurrences: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (idx, elt) in elts.iter().enumerate() {
            if let HirExpr::Var(name) = elt {
                var_occurrences.entry(name.clone()).or_default().push(idx);
            }
        }

        // Find duplicate variable names (appear more than once)
        let duplicate_var_names: std::collections::HashSet<String> = var_occurrences
            .iter()
            .filter(|(_, indices)| indices.len() > 1)
            .map(|(name, _)| name.clone())
            .collect();

        // For duplicate variables, determine which indices need clone (all but the last)
        let mut needs_clone_at: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for (var_name, indices) in var_occurrences.iter() {
            if indices.len() > 1 {
                // Check if the variable type is Copy
                let var_type = self.ctx.var_types.get(var_name);
                let is_copy = var_type.is_some_and(|t| !self.type_needs_clone(t));
                if !is_copy {
                    // Clone all but the last occurrence for non-Copy types
                    for &idx in &indices[..indices.len() - 1] {
                        needs_clone_at.insert(idx);
                    }
                }
            }
        }

        let elt_exprs: Vec<syn::Expr> = elts
            .iter()
            .enumerate()
            .map(|(idx, e)| {
                // For duplicate variables, handle clone manually to avoid double-cloning
                // This applies to ALL occurrences of duplicate vars (to bypass var_needs_clone)
                if let HirExpr::Var(name) = e {
                    if duplicate_var_names.contains(name) {
                        // Directly convert variable (bypasses var_needs_clone check)
                        let base_expr = self.convert_variable(name)?;
                        if needs_clone_at.contains(&idx) {
                            return Ok(parse_quote! { #base_expr.clone() });
                        } else {
                            return Ok(base_expr);
                        }
                    }
                }
                // For non-duplicate elements, use normal expression conversion
                e.to_rust_expr(self.ctx)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(parse_quote! { (#(#elt_exprs),*) })
    }

    fn convert_set(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
        self.ctx.needs_hashset = true;
        let mut insert_stmts = Vec::new();
        for elem in elts {
            let mut elem_expr = elem.to_rust_expr(self.ctx)?;
            // String literals need .to_string() for HashSet<String>
            if matches!(elem, HirExpr::Literal(Literal::String(_))) {
                elem_expr = parse_quote! { #elem_expr.to_string() };
            }
            insert_stmts.push(quote! { set.insert(#elem_expr); });
        }
        Ok(parse_quote! {
            {
                let mut set = HashSet::new();
                #(#insert_stmts)*
                set
            }
        })
    }

    fn convert_frozenset(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
        self.ctx.needs_hashset = true;
        self.ctx.needs_arc = true;
        let mut insert_stmts = Vec::new();
        for elem in elts {
            let elem_expr = elem.to_rust_expr(self.ctx)?;
            insert_stmts.push(quote! { set.insert(#elem_expr); });
        }
        Ok(parse_quote! {
            {
                let mut set = HashSet::new();
                #(#insert_stmts)*
                std::sync::Arc::new(set)
            }
        })
    }

    fn convert_attribute(&mut self, value: &HirExpr, attr: &str) -> Result<syn::Expr> {
        // If this is accessing a subcommand-specific field on args parameter,
        // generate just the field name (it's extracted via pattern matching)
        if let HirExpr::Var(var_name) = value {
            // Check if var_name is an args parameter
            // (heuristic: variable ending in "args" or exactly "args")
            if (var_name == "args" || var_name.ends_with("args"))
                && self.ctx.argparser_tracker.has_subcommands()
            {
                // Check if this field belongs to any subcommand
                let mut is_subcommand_field = false;
                for subcommand in self.ctx.argparser_tracker.subcommands.values() {
                    for arg in &subcommand.arguments {
                        if arg.rust_field_name() == attr {
                            is_subcommand_field = true;
                            break;
                        }
                    }
                    if is_subcommand_field {
                        break;
                    }
                }

                if is_subcommand_field {
                    // Check for special keywords that cannot be raw identifiers
                    if Self::is_non_raw_keyword(attr) {
                        bail!(
                            "Python attribute '{}' conflicts with a special Rust keyword that cannot be escaped. \
                             Please rename this attribute (e.g., '{}_attr' or 'py_{}')",
                            attr,
                            attr,
                            attr
                        );
                    }
                    
                    // Generate just the field name (extracted via pattern matching in func wrapper)
                    let attr_ident = if Self::is_rust_keyword(attr) {
                        syn::Ident::new_raw(attr, proc_macro2::Span::call_site())
                    } else {
                        syn::Ident::new(attr, proc_macro2::Span::call_site())
                    };
                    return Ok(parse_quote! { #attr_ident });
                }
            }
        }

        // Handle classmethod cls.ATTR → Self::ATTR
        if let HirExpr::Var(var_name) = value {
            if var_name == "cls" && self.ctx.is_classmethod {
                // Check for special keywords that cannot be raw identifiers
                if Self::is_non_raw_keyword(attr) {
                    bail!(
                        "Python attribute '{}' conflicts with a special Rust keyword that cannot be escaped. \
                         Please rename this attribute (e.g., '{}_attr' or 'py_{}')",
                        attr,
                        attr,
                        attr
                    );
                }
                
                let attr_ident = if Self::is_rust_keyword(attr) {
                    syn::Ident::new_raw(attr, proc_macro2::Span::call_site())
                } else {
                    syn::Ident::new(attr, proc_macro2::Span::call_site())
                };
                return Ok(parse_quote! { Self::#attr_ident });
            }

            // TypeName.CONSTANT → TypeName::CONSTANT
            // Five-Whys Root Cause:
            // 1. Why: E0423 - expected value, found struct 'Color'
            // 2. Why: Code generates Color.RED (field access) instead of Color::RED
            // 3. Why: Default attribute access uses dot syntax
            // 4. Why: No detection for type constant access vs field access
            // 5. ROOT CAUSE: Need to use :: for type-level constants

            // Check if var_name is a known enum type - if so, use :: syntax
            if self.ctx.enum_names.contains(var_name) {
                let type_ident = syn::Ident::new(var_name, proc_macro2::Span::call_site());
                // Preserve original casing from Python
                let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
                return Ok(parse_quote! { #type_ident::#attr_ident });
            }

            // Heuristic fallback: If name starts with uppercase and attr is ALL_CAPS, it's likely an enum variant
            let first_char = var_name.chars().next().unwrap_or('a');
            let is_type_name = first_char.is_uppercase();
            let is_constant = attr.chars().all(|c| c.is_uppercase() || c == '_');

            if is_type_name && is_constant {
                let type_ident = syn::Ident::new(var_name, proc_macro2::Span::call_site());
                // Preserve original casing from Python
                let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
                return Ok(parse_quote! { #type_ident::#attr_ident });
            }
        }

        // Check if this is a module attribute access
        if let HirExpr::Var(module_name) = value {
            //
            // math.pi → std::f64::consts::PI
            // math.e → std::f64::consts::E
            // math.inf → f64::INFINITY
            // math.nan → f64::NAN
            if module_name == "math" {
                let result = match attr {
                    "pi" => parse_quote! { std::f64::consts::PI },
                    "e" => parse_quote! { std::f64::consts::E },
                    "tau" => parse_quote! { std::f64::consts::TAU },
                    "inf" => parse_quote! { f64::INFINITY },
                    "nan" => parse_quote! { f64::NAN },
                    _ => {
                        // If it's not a recognized constant, it might be a typo
                        bail!("math.{} is not a recognized constant or method", attr);
                    }
                };
                return Ok(result);
            }

            //
            // string.ascii_letters → "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
            // string.digits → "0123456789"
            // string.punctuation → "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
            if module_name == "string" {
                let result = match attr {
                    "ascii_lowercase" => parse_quote! { "abcdefghijklmnopqrstuvwxyz" },
                    "ascii_uppercase" => parse_quote! { "ABCDEFGHIJKLMNOPQRSTUVWXYZ" },
                    "ascii_letters" => {
                        parse_quote! { "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ" }
                    }
                    "digits" => parse_quote! { "0123456789" },
                    "hexdigits" => parse_quote! { "0123456789abcdefABCDEF" },
                    "octdigits" => parse_quote! { "01234567" },
                    "punctuation" => parse_quote! { "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~" },
                    "whitespace" => parse_quote! { " \t\n\r\x0b\x0c" },
                    "printable" => {
                        parse_quote! { "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~ \t\n\r\x0b\x0c" }
                    }
                    _ => {
                        // Not a string constant - might be a method like capwords
                        bail!("string.{} is not a recognized constant", attr);
                    }
                };
                return Ok(result);
            }

            //
            // sys.argv → std::env::args().collect()
            // sys.platform → compile-time platform string
            if module_name == "sys" {
                let result = match attr {
                    "argv" => parse_quote! { std::env::args().collect::<Vec<String>>() },
                    "platform" => {
                        // Return platform name based on target OS as String
                        #[cfg(target_os = "linux")]
                        let platform = "linux";
                        #[cfg(target_os = "macos")]
                        let platform = "darwin";
                        #[cfg(target_os = "windows")]
                        let platform = "win32";
                        #[cfg(not(any(
                            target_os = "linux",
                            target_os = "macos",
                            target_os = "windows"
                        )))]
                        let platform = "unknown";
                        parse_quote! { #platform.to_string() }
                    }
                    "stdin" => parse_quote! { std::io::stdin() },
                    "stdout" => parse_quote! { std::io::stdout() },
                    "stderr" => parse_quote! { std::io::stderr() },
                    // Note: Python's sys.version_info is a 5-tuple (major, minor, micro, releaselevel, serial)
                    // but most comparisons use only (major, minor), so we return a 2-tuple for compatibility
                    "version_info" => {
                        // Rust doesn't have runtime version info by default
                        // Return a compile-time constant tuple matching Python 3.11
                        parse_quote! { (3, 11) }
                    }
                    // sys.modules is Python's runtime module registry
                    // Rust modules are resolved at compile time, so we return an empty/mock HashMap
                    "modules" => {
                        parse_quote! { std::collections::HashMap::<String, ()>::new() }
                    }
                    _ => {
                        bail!("sys.{} is not a recognized attribute", attr);
                    }
                };
                return Ok(result);
            }

            let module_info = self
                .ctx
                .imported_modules
                .get(module_name)
                .and_then(|mapping| {
                    mapping
                        .item_map
                        .get(attr)
                        .map(|rust_name| (mapping.rust_path.clone(), rust_name.clone()))
                });

            if let Some((rust_path, rust_name)) = module_info {
                // Map to the Rust equivalent
                let path_parts: Vec<&str> = rust_name.split("::").collect();
                if path_parts.len() > 1 {
                    let base_path: syn::Path =
                        syn::parse_str(&rust_path).unwrap_or_else(|_| parse_quote! { std });
                    let mut path = quote! { #base_path };
                    for part in path_parts {
                        let part_ident = syn::Ident::new(part, proc_macro2::Span::call_site());
                        path = quote! { #path::#part_ident };
                    }
                    return Ok(parse_quote! { #path });
                } else {
                    // Simple identifier
                    let ident = syn::Ident::new(&rust_name, proc_macro2::Span::call_site());
                    return Ok(parse_quote! { #ident });
                }
            }
        }

        //
        // In chrono, properties are accessed as methods: dt.year → dt.year()
        // This handles properties for fractions, pathlib, datetime, date, time, and timedelta instances
        // For nested attribute access (e.g., o.inner.value), don't add .clone() to intermediate fields
        // Also for simple variable access (e.g., state.field), don't clone the base - only clone the field if needed
        let mut value_expr = if matches!(value, HirExpr::Attribute { .. }) {
            self.convert_attribute_without_clone(value)?
        } else {
            // Set prevent_clone temporarily to avoid cloning the base variable
            // We only want to clone the field value if needed, not the base
            // Note: We don't set is_assignment_target here as that would affect get() vs get_mut()
            let was_prevent_clone = self.ctx.prevent_clone;
            self.ctx.prevent_clone = true;
            let expr = value.to_rust_expr(self.ctx)?;
            self.ctx.prevent_clone = was_prevent_clone;
            expr
        };

        // For chained access (o.inner.value), if the intermediate field is Optional, unwrap it
        // Use as_mut() for assignment targets to allow mutation
        if self.field_is_optional_inner(value) {
            if self.ctx.is_assignment_target {
                value_expr = parse_quote! { #value_expr.as_mut().unwrap() };
            } else {
                value_expr = parse_quote! { #value_expr.as_ref().unwrap() };
            }
        }

        // If the base variable itself is Optional<T>, unwrap it before accessing the field
        // Use as_mut() for assignment targets to allow mutation
        // Check both var_types (resolved type) and optional_vars (declared Optional tracking)
        if let HirExpr::Var(var_name) = value {
            let is_optional = matches!(self.ctx.var_types.get(var_name), Some(Type::Optional(_)))
                || self.ctx.optional_vars.contains(var_name);
            if is_optional {
                if self.ctx.is_assignment_target {
                    value_expr = parse_quote! { #value_expr.as_mut().unwrap() };
                } else {
                    value_expr = parse_quote! { #value_expr.as_ref().unwrap() };
                }
            }
        }

        match attr {
            //
            "numerator" => {
                // f.numerator → *f.numer()
                return Ok(parse_quote! { *#value_expr.numer() });
            }

            "denominator" => {
                // f.denominator → *f.denom()
                return Ok(parse_quote! { *#value_expr.denom() });
            }

            //
            // Only apply these transformations when the value is actually a Path type
            "stem" if self.is_path_expr(value) => {
                // p.stem → p.file_stem().unwrap().to_str().unwrap().to_string()
                return Ok(parse_quote! {
                    #value_expr.file_stem().unwrap().to_str().unwrap().to_string()
                });
            }

            "suffix" if self.is_path_expr(value) => {
                // p.suffix → p.extension().map(|e| format!(".{}", e.to_str().unwrap())).unwrap_or_default()
                return Ok(parse_quote! {
                    #value_expr.extension()
                        .map(|e| format!(".{}", e.to_str().unwrap()))
                        .unwrap_or_default()
                });
            }

            "parent" if self.is_path_expr(value) => {
                // p.parent → p.parent().unwrap().to_path_buf()
                return Ok(parse_quote! {
                    #value_expr.parent().unwrap().to_path_buf()
                });
            }

            "parts" if self.is_path_expr(value) => {
                // p.parts → p.components().map(|c| c.as_os_str().to_str().unwrap().to_string()).collect()
                return Ok(parse_quote! {
                    #value_expr.components()
                        .map(|c| c.as_os_str().to_str().unwrap().to_string())
                        .collect::<Vec<_>>()
                });
            }

            // datetime/date properties (require method calls in chrono)
            "year" | "month" | "day" | "hour" | "minute" | "second" | "microsecond" => {
                // Check if this might be a datetime/date/time object
                // We convert: dt.year → dt.year()
                let method_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
                return Ok(parse_quote! { #value_expr.#method_ident() as i32 });
            }

            // timedelta properties
            "days" => {
                // td.days → td.num_days()
                return Ok(parse_quote! { #value_expr.num_days() as i32 });
            }

            "seconds" => {
                // td.seconds → td.num_seconds() % 86400 (seconds within the day)
                return Ok(parse_quote! { (#value_expr.num_seconds() % 86400) as i32 });
            }

            "microseconds" => {
                // td.microseconds → (td.num_microseconds() % 1_000_000)
                return Ok(
                    parse_quote! { (#value_expr.num_microseconds().unwrap() % 1_000_000) as i32 },
                );
            }

            _ => {
                // Not a datetime property, continue with default handling
            }
        }

        // Try common CSV patterns (heuristic-based for now)
        if let Some(mapping) = self.ctx.stdlib_mappings.lookup("csv", "DictReader", attr) {
            // Found a CSV DictReader mapping - apply it
            let rust_code =
                mapping.generate_rust_code(&value_expr.to_token_stream().to_string(), &[]);
            if let Ok(expr) = syn::parse_str::<syn::Expr>(&rust_code) {
                return Ok(expr);
            }
        }

        // Also try generic Reader patterns
        if let Some(mapping) = self.ctx.stdlib_mappings.lookup("csv", "Reader", attr) {
            let rust_code =
                mapping.generate_rust_code(&value_expr.to_token_stream().to_string(), &[]);
            if let Ok(expr) = syn::parse_str::<syn::Expr>(&rust_code) {
                return Ok(expr);
            }
        }

        // Default behavior for non-module attributes
        // Check for special keywords that cannot be raw identifiers
        if Self::is_non_raw_keyword(attr) {
            bail!(
                "Python attribute '{}' conflicts with a special Rust keyword that cannot be escaped. \
                 Please rename this attribute (e.g., '{}_attr' or 'py_{}')",
                attr,
                attr,
                attr
            );
        }
        
        let attr_ident = if Self::is_rust_keyword(attr) {
            syn::Ident::new_raw(attr, proc_macro2::Span::call_site())
        } else {
            syn::Ident::new(attr, proc_macro2::Span::call_site())
        };

        // Check if field is a String type and needs cloning
        // When accessing a String field through a reference, we need .clone() to get an owned String
        // Skip clone for assignment targets - we need mutable access, not a copy
        // Skip clone when function returns a reference - the return type already handles it
        // Skip clone when generate_borrow is set - we'll add a reference in stmt_gen instead
        // Skip clone when prevent_clone is set - the caller explicitly wants to avoid cloning
        // Skip clone when clone_already_applied is set - we've already added .clone() upstream
        let needs_clone = !self.ctx.is_assignment_target
            && !self.ctx.prevent_clone
            && !self.ctx.returns_reference
            && !self.ctx.generate_borrow
            && !self.ctx.clone_already_applied
            && self.field_needs_clone(value, attr);

        if needs_clone {
            self.ctx.clone_already_applied = true;
            Ok(parse_quote! { #value_expr.#attr_ident.clone() })
        } else {
            Ok(parse_quote! { #value_expr.#attr_ident })
        }
    }

    /// Convert attribute access without adding .clone().
    /// Handles Optional intermediate fields by inserting `.as_ref().unwrap()`.
    /// This is called when the attribute access is part of a chain (e.g., `o.inner` in `o.inner.value`).
    fn convert_attribute_without_clone(&mut self, expr: &HirExpr) -> Result<syn::Expr> {
        if let HirExpr::Attribute { value, attr } = expr {
            // Recursively convert the base value (also without clone if it's an attribute)
            let mut value_expr = if matches!(value.as_ref(), HirExpr::Attribute { .. }) {
                self.convert_attribute_without_clone(value)?
            } else {
                // Set prevent_clone to avoid cloning the base variable
                let was_prevent_clone = self.ctx.prevent_clone;
                self.ctx.prevent_clone = true;
                // Suppress filter_deref_vars for the base variable since Rust
                // auto-deref handles .field access on &T without explicit *
                let suppressed = if let HirExpr::Var(name) = value.as_ref() {
                    self.ctx.filter_deref_vars.remove(name.as_str())
                } else {
                    false
                };
                let expr = value.to_rust_expr(self.ctx)?;
                if suppressed {
                    if let HirExpr::Var(name) = value.as_ref() {
                        self.ctx.filter_deref_vars.insert(name.clone());
                    }
                }
                self.ctx.prevent_clone = was_prevent_clone;
                expr
            };

            // Check if the base value (when it's an attribute) is Optional - need to unwrap before accessing
            // Use as_mut() for assignment targets to allow mutation
            if self.field_is_optional_inner(value) {
                if self.ctx.is_assignment_target {
                    value_expr = parse_quote! { #value_expr.as_mut().unwrap() };
                } else {
                    value_expr = parse_quote! { #value_expr.as_ref().unwrap() };
                }
            }

            // If the base variable itself is Optional<T>, unwrap it before accessing the field
            // Use as_mut() for assignment targets to allow mutation
            // Check both var_types (resolved type) and optional_vars (declared Optional tracking)
            if let HirExpr::Var(var_name) = value.as_ref() {
                let is_optional = matches!(self.ctx.var_types.get(var_name), Some(Type::Optional(_)))
                    || self.ctx.optional_vars.contains(var_name);
                if is_optional {
                    if self.ctx.is_assignment_target {
                        value_expr = parse_quote! { #value_expr.as_mut().unwrap() };
                    } else {
                        value_expr = parse_quote! { #value_expr.as_ref().unwrap() };
                    }
                }
            }

            // Check for special keywords that cannot be raw identifiers
            if Self::is_non_raw_keyword(attr) {
                bail!(
                    "Python attribute '{}' conflicts with a special Rust keyword that cannot be escaped. \
                     Please rename this attribute (e.g., '{}_attr' or 'py_{}')",
                    attr,
                    attr,
                    attr
                );
            }

            let attr_ident = if Self::is_rust_keyword(attr) {
                syn::Ident::new_raw(attr, proc_macro2::Span::call_site())
            } else {
                syn::Ident::new(attr, proc_macro2::Span::call_site())
            };

            // Generate the attribute access
            let result = parse_quote! { #value_expr.#attr_ident };

            // Check if THIS attribute (the current attr) is Optional and we're part of a deeper chain.
            // If so, we need to unwrap it too. This handles cases like o.l2.l3.data where both l2 and l3 are Optional.
            // However, we don't unwrap here - that's handled by the caller who will check field_is_optional_inner.
            Ok(result)
        } else {
            // Not an attribute access, use regular conversion
            expr.to_rust_expr(self.ctx)
        }
    }

    /// Convert an expression without adding .clone().
    /// Used for string comparisons where String == &str works directly.
    /// This avoids unnecessary clones like `team.clone() == "Home"` → `team == "Home"`.
    fn convert_expr_without_clone(&mut self, expr: &HirExpr) -> Result<syn::Expr> {
        match expr {
            HirExpr::Attribute { .. } => {
                // Use the existing method for attributes
                self.convert_attribute_without_clone(expr)
            }
            HirExpr::Var(name) => {
                // For variables, temporarily disable cloning by using prevent_clone
                let was_prevent_clone = self.ctx.prevent_clone;
                self.ctx.prevent_clone = true;
                let result = expr.to_rust_expr(self.ctx);
                self.ctx.prevent_clone = was_prevent_clone;
                result
            }
            _ => {
                // For other expressions, convert normally
                expr.to_rust_expr(self.ctx)
            }
        }
    }

    /// Check if an attribute expression itself refers to an Optional field.
    /// Used when the attribute is an intermediate in a chain (e.g., `o.inner` in `o.inner.value`).
    fn field_is_optional_inner(&self, expr: &HirExpr) -> bool {
        if let HirExpr::Attribute { value, attr } = expr {
            self.field_is_optional(value, attr)
        } else {
            false
        }
    }

    /// Check if a field access needs .clone() based on the field type
    fn field_needs_clone(&self, value: &HirExpr, attr: &str) -> bool {
        // Get the class name from the value expression
        let class_name = match value {
            HirExpr::Var(var_name) => {
                // Special case: "self" refers to the current class instance
                if var_name == "self" {
                    // Look up which class has this field
                    for (cls_name, fields) in &self.ctx.class_field_types {
                        if fields.contains_key(attr) {
                            return self.type_needs_clone(fields.get(attr).unwrap());
                        }
                    }
                    return false;
                }
                // Check if the variable is of a Custom type (struct)
                if let Some(Type::Custom(name)) = self.ctx.var_types.get(var_name) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            HirExpr::Attribute {
                value: inner_value,
                attr: inner_attr,
            } => {
                // Nested attribute access (e.g., o.inner.value)
                // First get the type of o.inner, then check its field type
                if let Some(inner_class) = self.get_attr_type_name(inner_value, inner_attr) {
                    Some(inner_class)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(class_name) = class_name {
            // Look up the field type in class_field_types
            if let Some(field_types) = self.ctx.class_field_types.get(&class_name) {
                if let Some(field_type) = field_types.get(attr) {
                    // Clone all non-Copy types when accessing through a borrowed reference
                    return self.type_needs_clone(field_type);
                }
            }
        }
        false
    }

    /// Check if a field is Optional<T> - needs .unwrap() when accessed
    fn field_is_optional(&self, value: &HirExpr, attr: &str) -> bool {
        let class_name = match value {
            HirExpr::Var(var_name) => {
                if var_name == "self" {
                    for (cls_name, fields) in &self.ctx.class_field_types {
                        if fields.contains_key(attr) {
                            return matches!(fields.get(attr), Some(Type::Optional(_)));
                        }
                    }
                    return false;
                }
                if let Some(Type::Custom(name)) = self.ctx.var_types.get(var_name) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            HirExpr::Attribute {
                value: inner_value,
                attr: inner_attr,
            } => self.get_attr_type_name(inner_value, inner_attr),
            _ => None,
        };

        if let Some(class_name) = class_name {
            if let Some(field_types) = self.ctx.class_field_types.get(&class_name) {
                return matches!(field_types.get(attr), Some(Type::Optional(_)));
            }
        }
        false
    }

    /// Convert an expression, unwrapping if it's an Optional field access or variable.
    /// Python allows direct access to Optional fields/variables - operations on None fail at runtime.
    /// This mimics Python behavior by adding .unwrap() when Optional fields/variables are used in operations.
    /// Note: to_rust_expr already adds .clone() if needed, so we only add .unwrap() here.
    fn convert_with_optional_unwrap(&mut self, expr: &HirExpr) -> Result<syn::Expr> {
        // Check if this is an Optional field access
        if let HirExpr::Attribute { value, attr } = expr {
            if self.field_is_optional(value, attr) {
                // Convert the expression and add .unwrap()
                let rust_expr = expr.to_rust_expr(self.ctx)?;
                return Ok(parse_quote! { #rust_expr.unwrap() });
            }
        }
        // Check if this is an Optional variable
        if let HirExpr::Var(name) = expr {
            if let Some(Type::Optional(_)) = self.ctx.var_types.get(name) {
                // Convert the expression and add .unwrap()
                // Note: to_rust_expr already adds .clone() if needed for non-Copy types,
                // so we don't add another .clone() here to avoid duplicate clones
                let rust_expr = expr.to_rust_expr(self.ctx)?;
                return Ok(parse_quote! { #rust_expr.unwrap() });
            }
        }
        // Not an Optional field or variable, convert normally
        expr.to_rust_expr(self.ctx)
    }

    /// Convert an expression without adding .unwrap() for Optional types.
    /// Used for `or` pattern where we need the Option<T> value itself to call unwrap_or_else.
    fn convert_expr_no_unwrap(&mut self, expr: &HirExpr) -> Result<syn::Expr> {
        expr.to_rust_expr(self.ctx)
    }

    /// Check if a type needs .clone() (i.e., is not Copy)
    fn type_needs_clone(&self, ty: &Type) -> bool {
        match ty {
            // Copy types - don't need clone
            Type::Int | Type::Float | Type::Bool | Type::None => false,
            // Non-Copy types - need clone
            Type::String | Type::List(_) | Type::Dict(_, _) | Type::Set(_) => true,
            // Custom types: enums and all-Copy-field structs are Copy
            Type::Custom(name) => {
                !self.ctx.enum_names.contains(name) && !self.ctx.copy_structs.contains(name)
            },
            // Optional needs clone if inner type needs clone
            Type::Optional(inner) => self.type_needs_clone(inner),
            // Tuple needs clone if any element needs clone
            Type::Tuple(types) => types.iter().any(|t| self.type_needs_clone(t)),
            // Arrays, Generics, Functions, etc. - assume need clone for safety
            Type::Array { .. } | Type::Generic { .. } | Type::Function { .. } | Type::Union(_) => {
                true
            }
            // TypeVar and Unknown - assume need clone
            Type::TypeVar(_) | Type::Unknown => true,
            // Final wraps another type
            Type::Final(inner) => self.type_needs_clone(inner),
        }
    }

    /// Get the type name for an attribute access expression.
    /// Handles both `Type::Custom(name)` and `Type::Optional(Type::Custom(name))`.
    fn get_attr_type_name(&self, value: &HirExpr, attr: &str) -> Option<String> {
        self.get_field_type(value, attr)
            .and_then(|t| Self::extract_custom_type_name(&t))
    }

    /// Extract the inner Custom type name from a Type, handling Optional wrappers.
    fn extract_custom_type_name(ty: &Type) -> Option<String> {
        match ty {
            Type::Custom(name) => Some(name.clone()),
            Type::Optional(inner) => Self::extract_custom_type_name(inner),
            _ => None,
        }
    }

    /// Get the full type of a field from an attribute access expression.
    fn get_field_type(&self, value: &HirExpr, attr: &str) -> Option<Type> {
        match value {
            HirExpr::Var(var_name) => {
                // Special case: "self" refers to the current class
                if var_name == "self" {
                    for (_cls_name, fields) in &self.ctx.class_field_types {
                        if let Some(field_type) = fields.get(attr) {
                            return Some(field_type.clone());
                        }
                    }
                    return None;
                }
                // Get the class name from the variable type
                if let Some(Type::Custom(class_name)) = self.ctx.var_types.get(var_name) {
                    if let Some(field_types) = self.ctx.class_field_types.get(class_name) {
                        return field_types.get(attr).cloned();
                    }
                }
                None
            }
            HirExpr::Attribute {
                value: inner_value,
                attr: inner_attr,
            } => {
                // Get the type of the inner attribute (e.g., for o.inner.value, get type of o.inner)
                // Then extract the class name and look up the field
                if let Some(inner_type) = self.get_field_type(inner_value, inner_attr) {
                    // Extract the class name from the inner type (handles Optional wrappers)
                    if let Some(class_name) = Self::extract_custom_type_name(&inner_type) {
                        if let Some(field_types) = self.ctx.class_field_types.get(&class_name) {
                            return field_types.get(attr).cloned();
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if a field access is a List<String> (returns true if element type is String)
    fn get_field_element_type_is_string(&self, value: &HirExpr, attr: &str) -> bool {
        if let HirExpr::Var(var_name) = value {
            // Get the class name from the variable type
            if let Some(Type::Custom(class_name)) = self.ctx.var_types.get(var_name) {
                // Look up the field type
                if let Some(field_types) = self.ctx.class_field_types.get(class_name) {
                    if let Some(Type::List(element_type)) = field_types.get(attr) {
                        return matches!(**element_type, Type::String);
                    }
                }
            }
        }
        false
    }

    fn convert_borrow(&mut self, expr: &HirExpr, mutable: bool) -> Result<syn::Expr> {
        let expr_tokens = expr.to_rust_expr(self.ctx)?;
        if mutable {
            Ok(parse_quote! { &mut #expr_tokens })
        } else {
            Ok(parse_quote! { &#expr_tokens })
        }
    }

    fn convert_list_comp(
        &mut self,
        element: &HirExpr,
        target: &str,
        iter: &HirExpr,
        condition: &Option<Box<HirExpr>>,
    ) -> Result<syn::Expr> {
        // Parse target as pattern (supports both simple variables and tuple unpacking)
        let target_pat = self.parse_target_pattern(target)?;
        let element_expr = element.to_rust_expr(self.ctx)?;

        // Check if this is an identity map (element is just the target variable)
        // Skip .map(|x| x) or .map(|(x, y)| (x, y)) which are no-ops
        let is_identity_map = Self::is_identity_element(element, target);

        // Check if the iterator is an Optional type (e.g., Optional[List[int]])
        let iter_is_optional = self.expr_is_optional(iter);

        // For attribute expressions (e.g., state.incidents), use convert_attribute_without_clone
        // to avoid double .clone() - we add .clone() in the generated code, so we don't need
        // convert_attribute to also add .clone()
        let iter_expr = if iter_is_optional {
            let base_expr = if matches!(iter, HirExpr::Attribute { .. }) {
                self.convert_attribute_without_clone(iter)?
            } else {
                iter.to_rust_expr(self.ctx)?
            };
            parse_quote! { #base_expr.as_ref().unwrap() }
        } else if matches!(iter, HirExpr::Attribute { .. }) {
            self.convert_attribute_without_clone(iter)?
        } else {
            iter.to_rust_expr(self.ctx)?
        };

        // Strategy:
        // - Use .iter() to explicitly borrow elements
        // - Place .cloned() AFTER .filter() so filter sees &T and only filtered items are cloned
        // - Use pattern matching |&x| in filter closure to dereference once
        // - This avoids double-reference issues (&&T) in filter conditions

        let is_range = self.is_range_expr(&iter_expr);

        // CSV readers can't use .into_iter() - they need .deserialize()
        let is_csv_reader = if let HirExpr::Var(var_name) = iter {
            var_name == "reader"
                || var_name.contains("csv")
                || var_name.ends_with("_reader")
                || var_name.starts_with("reader_")
        } else {
            false
        };

        // Check if target is a tuple pattern (for enumerate() style iteration)
        let is_tuple_target = target.starts_with('(');

        // Determine if the element type needs clone (non-Copy) or can use copy
        let element_needs_clone = if let HirExpr::Var(var_name) = iter {
            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                match var_type {
                    Type::List(elem_type) => self.type_needs_clone(elem_type),
                    Type::Set(elem_type) => self.type_needs_clone(elem_type),
                    _ => true,
                }
            } else {
                true
            }
        } else {
            true
        };

        // Use .copied() for Copy types, .cloned() for Clone types
        let iter_clone_method = if element_needs_clone {
            quote::format_ident!("cloned")
        } else {
            quote::format_ident!("copied")
        };

        if let Some(cond) = condition {
            let cond_with_deref = if is_tuple_target || is_range {
                cond.to_rust_expr(self.ctx)?
            } else {
                self.ctx.filter_deref_vars.insert(target.to_string());
                let expr = cond.to_rust_expr(self.ctx)?;
                self.ctx.filter_deref_vars.remove(target);
                expr
            };

            if is_range {
                // Ranges are already iterators, don't call .iter()
                // Range items are owned (i32, etc.), filter receives &i32
                // Use pattern matching |&x| to dereference for the condition
                if is_identity_map {
                    Ok(parse_quote! {
                        (#iter_expr)
                            .filter(|&#target_pat| #cond_with_deref)
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        (#iter_expr)
                            .filter(|&#target_pat| #cond_with_deref)
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            } else if is_csv_reader {
                // Use .deserialize() instead of .into_iter()
                // CSV DictReader yields HashMap<String, String>
                self.ctx.needs_csv = true;
                let cond_expr = cond.to_rust_expr(self.ctx)?;
                if is_identity_map {
                    Ok(parse_quote! {
                        #iter_expr
                            .deserialize::<std::collections::HashMap<String, String>>()
                            .filter_map(|result| result.ok())
                            .filter(|#target_pat| #cond_expr)
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #iter_expr
                            .deserialize::<std::collections::HashMap<String, String>>()
                            .filter_map(|result| result.ok())
                            .filter(|#target_pat| #cond_expr)
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            } else if iter_is_optional {
                // For Optional collections, .as_ref().unwrap() returns &Vec<T>
                if is_identity_map {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .filter(|&#target_pat| #cond_with_deref)
                            .#iter_clone_method()
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .filter(|&#target_pat| #cond_with_deref)
                            .#iter_clone_method()
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            } else {
                // Use |&target| pattern to automatically dereference in filter closure
                if is_identity_map {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .filter(|&#target_pat| #cond_with_deref)
                            .#iter_clone_method()
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .filter(|&#target_pat| #cond_with_deref)
                            .#iter_clone_method()
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            }
        } else {
            // Without condition: map-only comprehensions
            if is_range {
                // Ranges are already iterators, don't call .iter()
                if is_identity_map {
                    Ok(parse_quote! {
                        (#iter_expr).collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        (#iter_expr)
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            } else if is_csv_reader {
                // Use .deserialize() instead of .into_iter()
                // CSV DictReader yields HashMap<String, String>
                self.ctx.needs_csv = true;
                if is_identity_map {
                    Ok(parse_quote! {
                        #iter_expr
                            .deserialize::<std::collections::HashMap<String, String>>()
                            .filter_map(|result| result.ok())
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #iter_expr
                            .deserialize::<std::collections::HashMap<String, String>>()
                            .filter_map(|result| result.ok())
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            } else if iter_is_optional {
                // For Optional collections, .as_ref().unwrap() returns &Vec<T>
                if is_identity_map {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .#iter_clone_method()
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .#iter_clone_method()
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            } else {
                // Use .iter().copied()/.cloned() to borrow and then copy/clone elements
                if is_identity_map {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .#iter_clone_method()
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .#iter_clone_method()
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            }
        }
    }

    /// Convert flattened list comprehension with multiple generators
    /// Python: [item for row in matrix for item in row]
    /// Rust: matrix.iter().flat_map(|row| row.iter().cloned()).collect::<Vec<_>>()
    fn convert_flattened_list_comp(
        &mut self,
        element: &HirExpr,
        generators: &[HirComprehension],
    ) -> Result<syn::Expr> {
        // We need at least 2 generators for a flattened comprehension
        if generators.len() < 2 {
            bail!("FlattenedListComp requires at least 2 generators");
        }

        // Start with the outer iterator
        let outer_gen = &generators[0];
        let outer_target = self.parse_target_pattern(&outer_gen.target)?;
        let outer_iter = outer_gen.iter.to_rust_expr(self.ctx)?;
        let is_outer_range = self.is_range_expr(&outer_iter);

        // Build the inner expression (which is another iterator chain)
        // For [item for row in matrix for item in row], inner is `row.iter()`
        let inner_gen = &generators[1];
        let inner_target = self.parse_target_pattern(&inner_gen.target)?;
        let inner_iter = inner_gen.iter.to_rust_expr(self.ctx)?;
        let is_inner_range = self.is_range_expr(&inner_iter);

        // The element expression
        let element_expr = element.to_rust_expr(self.ctx)?;

        // Check if element is just the inner target variable (identity mapping)
        let is_identity_map = Self::is_identity_element(element, &inner_gen.target);

        // Build the result
        if is_outer_range {
            // Outer is a range, inner needs special handling
            if is_inner_range {
                // Both are ranges - direct flat_map
                if is_identity_map {
                    Ok(parse_quote! {
                        (#outer_iter)
                            .flat_map(|#outer_target| #inner_iter)
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        (#outer_iter)
                            .flat_map(|#outer_target| (#inner_iter).map(|#inner_target| #element_expr))
                            .collect::<Vec<_>>()
                    })
                }
            } else {
                // Inner is a collection
                if is_identity_map {
                    Ok(parse_quote! {
                        (#outer_iter)
                            .flat_map(|#outer_target| #inner_iter.iter().cloned())
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        (#outer_iter)
                            .flat_map(|#outer_target| #inner_iter.iter().cloned().map(|#inner_target| #element_expr))
                            .collect::<Vec<_>>()
                    })
                }
            }
        } else {
            // Outer is a collection
            if is_inner_range {
                // Inner is a range
                if is_identity_map {
                    Ok(parse_quote! {
                        #outer_iter
                            .iter()
                            .flat_map(|#outer_target| #inner_iter)
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #outer_iter
                            .iter()
                            .flat_map(|#outer_target| (#inner_iter).map(|#inner_target| #element_expr))
                            .collect::<Vec<_>>()
                    })
                }
            } else {
                // Both are collections
                if is_identity_map {
                    Ok(parse_quote! {
                        #outer_iter
                            .iter()
                            .flat_map(|#outer_target| #inner_iter.iter().cloned())
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #outer_iter
                            .iter()
                            .flat_map(|#outer_target| #inner_iter.iter().cloned().map(|#inner_target| #element_expr))
                            .collect::<Vec<_>>()
                    })
                }
            }
        }
    }

    /// Optimize `[x for x in iter if cond][0]` to use `.find()` instead of `.collect().get(0)`
    /// Python: `[x for x in items if x.valid][0]` → Rust: `items.iter().find(|x| x.valid).unwrap()`
    fn convert_list_comp_first_element(
        &mut self,
        element: &HirExpr,
        target: &str,
        iter: &HirExpr,
        condition: &HirExpr,
    ) -> Result<syn::Expr> {
        // Check for special keywords that cannot be raw identifiers
        if Self::is_non_raw_keyword(target) {
            bail!(
                "Python variable '{}' conflicts with a special Rust keyword that cannot be escaped. \
                 Please rename this variable (e.g., '{}_var' or 'py_{}')",
                target,
                target,
                target
            );
        }
        
        // Use raw identifier if target is a Rust keyword
        let target_ident = if Self::is_rust_keyword(target) {
            syn::Ident::new_raw(target, proc_macro2::Span::call_site())
        } else {
            syn::Ident::new(target, proc_macro2::Span::call_site())
        };

        // Check if the iterator is an Optional type
        let iter_is_optional = self.expr_is_optional(iter);

        let iter_expr = if iter_is_optional {
            let base_expr = if matches!(iter, HirExpr::Attribute { .. }) {
                self.convert_attribute_without_clone(iter)?
            } else {
                iter.to_rust_expr(self.ctx)?
            };
            parse_quote! { #base_expr.as_ref().unwrap() }
        } else if matches!(iter, HirExpr::Attribute { .. }) {
            self.convert_attribute_without_clone(iter)?
        } else {
            iter.to_rust_expr(self.ctx)?
        };

        self.ctx.filter_deref_vars.insert(target.to_string());
        let cond_with_deref = condition.to_rust_expr(self.ctx)?;
        self.ctx.filter_deref_vars.remove(target);

        // Check if element is just the target variable (identity mapping)
        let is_identity_map = Self::is_identity_element(element, target);

        if is_identity_map {
            // Simple case: [x for x in iter if cond][0] → iter.iter().find(|x| cond).cloned().unwrap()
            Ok(parse_quote! {
                #iter_expr
                    .iter()
                    .find(|#target_ident| #cond_with_deref)
                    .cloned()
                    .unwrap()
            })
        } else {
            // Mapped case: [f(x) for x in iter if cond][0] → iter.iter().find(|x| cond).map(|x| f(x)).unwrap()
            let element_expr = element.to_rust_expr(self.ctx)?;
            Ok(parse_quote! {
                #iter_expr
                    .iter()
                    .find(|#target_ident| #cond_with_deref)
                    .map(|#target_ident| #element_expr)
                    .unwrap()
            })
        }
    }

    /// Add dereference (*) to uses of target variable in expression
    /// This is needed because filter closures receive &T even when the iterator yields T
    /// Example: transforms `x > 0` to `*x > 0` when x is the target variable
    fn add_deref_to_var_uses(&mut self, expr: &HirExpr, target: &str) -> Result<syn::Expr> {
        use crate::hir::{BinOp, HirExpr, UnaryOp};

        match expr {
            HirExpr::Var(name) if name == target => {
                // This is the target variable - add dereference
                let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                Ok(parse_quote! { *#ident })
            }
            HirExpr::Binary { op, left, right } => {
                // Recursively add derefs to both sides
                let left_expr = self.add_deref_to_var_uses(left, target)?;

                // For `in` and `not in` with tuples, convert tuple to array since
                // Rust tuples don't have .contains() method
                if matches!(op, BinOp::In | BinOp::NotIn) {
                    if let HirExpr::Tuple(elements) | HirExpr::List(elements) = right.as_ref() {
                        let elem_exprs: Vec<syn::Expr> = elements
                            .iter()
                            .map(|e| e.to_rust_expr(self.ctx))
                            .collect::<Result<Vec<_>>>()?;

                        // Check if all elements are string literals
                        let all_string_literals = elements
                            .iter()
                            .all(|e| matches!(e, HirExpr::Literal(Literal::String(_))));

                        // If elements are string literals and left is an attribute (likely String type),
                        // we need to convert String to &str using .as_str()
                        let left_is_attribute = matches!(left.as_ref(), HirExpr::Attribute { .. });

                        return match op {
                            BinOp::In if all_string_literals && left_is_attribute => Ok(
                                parse_quote! { [#(#elem_exprs),*].contains(&#left_expr.as_str()) },
                            ),
                            BinOp::In => {
                                Ok(parse_quote! { [#(#elem_exprs),*].contains(&#left_expr) })
                            }
                            BinOp::NotIn if all_string_literals && left_is_attribute => Ok(
                                parse_quote! { ![#(#elem_exprs),*].contains(&#left_expr.as_str()) },
                            ),
                            BinOp::NotIn => {
                                Ok(parse_quote! { ![#(#elem_exprs),*].contains(&#left_expr) })
                            }
                            _ => unreachable!(),
                        };
                    }
                }

                let right_expr = self.add_deref_to_var_uses(right, target)?;

                // Generate the operator token
                let result = match op {
                    BinOp::Add => parse_quote! { #left_expr + #right_expr },
                    BinOp::Sub => parse_quote! { #left_expr - #right_expr },
                    BinOp::Mul => parse_quote! { #left_expr * #right_expr },
                    BinOp::Div => parse_quote! { #left_expr / #right_expr },
                    BinOp::FloorDiv => parse_quote! { #left_expr / #right_expr },
                    BinOp::Mod => parse_quote! { #left_expr % #right_expr },
                    BinOp::Pow => parse_quote! { #left_expr.pow(#right_expr as u32) },
                    BinOp::MatMul => parse_quote! { matmul(#left_expr, #right_expr) },
                    BinOp::Eq => parse_quote! { #left_expr == #right_expr },
                    BinOp::NotEq => parse_quote! { #left_expr != #right_expr },
                    BinOp::Lt => parse_quote! { #left_expr < #right_expr },
                    BinOp::LtEq => parse_quote! { #left_expr <= #right_expr },
                    BinOp::Gt => parse_quote! { #left_expr > #right_expr },
                    BinOp::GtEq => parse_quote! { #left_expr >= #right_expr },
                    BinOp::And => parse_quote! { #left_expr && #right_expr },
                    BinOp::Or => parse_quote! { #left_expr || #right_expr },
                    BinOp::BitAnd => parse_quote! { #left_expr & #right_expr },
                    BinOp::BitOr => parse_quote! { #left_expr | #right_expr },
                    BinOp::BitXor => parse_quote! { #left_expr ^ #right_expr },
                    BinOp::LShift => parse_quote! { #left_expr << #right_expr },
                    BinOp::RShift => parse_quote! { #left_expr >> #right_expr },
                    BinOp::In => parse_quote! { #right_expr.contains(&#left_expr) },
                    BinOp::NotIn => parse_quote! { !#right_expr.contains(&#left_expr) },
                    BinOp::Is => parse_quote! { #left_expr == #right_expr },
                    BinOp::IsNot => parse_quote! { #left_expr != #right_expr },
                };
                Ok(result)
            }
            HirExpr::Unary { op, operand } => {
                // Recursively add derefs to operand
                let operand_expr = self.add_deref_to_var_uses(operand, target)?;

                let result = match op {
                    UnaryOp::Not => parse_quote! { !#operand_expr },
                    UnaryOp::Neg => parse_quote! { -#operand_expr },
                    UnaryOp::Pos => parse_quote! { +#operand_expr },
                    UnaryOp::BitNot => parse_quote! { !#operand_expr },
                };
                Ok(result)
            }
            // For any other expression, convert normally (no deref needed)
            _ => expr.to_rust_expr(self.ctx),
        }
    }

    fn is_set_expr(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Set(_) | HirExpr::FrozenSet(_) => true,
            HirExpr::Call { func, .. } if func == "set" || func == "frozenset" => true,
            HirExpr::Var(_) => {
                // Check type information in context for variables
                self.is_set_var(expr)
            }
            HirExpr::Attribute { .. } => {
                // Check if this is an attribute access to a Set or Optional<Set> field
                self.is_set_field(expr)
            }
            _ => false,
        }
    }

    /// Check if an attribute access refers to a Set or Optional<Set> field
    fn is_set_field(&self, expr: &HirExpr) -> bool {
        if let HirExpr::Attribute { value, attr } = expr {
            if let HirExpr::Var(base_name) = value.as_ref() {
                if let Some(base_type) = self.ctx.var_types.get(base_name) {
                    if let Type::Custom(class_name) = base_type {
                        if let Some(fields) = self.ctx.class_field_types.get(class_name) {
                            if let Some(field_type) = fields.get(attr) {
                                // Check for Set or Optional<Set>
                                return matches!(field_type, Type::Set(_))
                                    || matches!(field_type, Type::Optional(inner) if matches!(inner.as_ref(), Type::Set(_)));
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if a variable has a set type based on type information in context
    fn is_set_var(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Var(name) => {
                // Check var_types in context to see if this variable is a set
                if let Some(var_type) = self.ctx.var_types.get(name) {
                    matches!(var_type, Type::Set(_))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if an expression is an Optional type that needs unwrapping.
    fn expr_is_optional(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Var(name) => {
                matches!(self.ctx.var_types.get(name), Some(Type::Optional(_)))
            }
            HirExpr::Attribute { value, attr } => {
                if let HirExpr::Var(base_name) = value.as_ref() {
                    if let Some(base_type) = self.ctx.var_types.get(base_name) {
                        if let Type::Custom(class_name) = base_type {
                            if let Some(fields) = self.ctx.class_field_types.get(class_name) {
                                return matches!(fields.get(attr), Some(Type::Optional(_)));
                            }
                        }
                    }
                }
                false
            }
            HirExpr::MethodCall { method, .. } => matches!(method.as_str(), "get"),
            _ => false,
        }
    }

    /// Used to distinguish string.contains() from HashMap.contains_key()
    ///
    /// # Complexity
    /// 3 (match + type lookup + variant check)
    fn is_string_type(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Literal(Literal::String(_)) => true,
            HirExpr::Var(name) => {
                // Check var_types to see if this variable is a string
                if let Some(var_type) = self.ctx.var_types.get(name) {
                    matches!(var_type, Type::String)
                } else {
                    // Fallback to heuristic for cases without type info
                    self.is_string_base(expr)
                }
            }
            HirExpr::Attribute { value, attr } => {
                // Check if the field type is String
                if let Some(field_type) = self.get_field_type(value, attr) {
                    matches!(field_type, Type::String)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Used for dict merge operator (|) and other dict-specific operations
    ///
    /// # Complexity
    /// 3 (match + type lookup + variant check)
    fn is_dict_expr(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Dict(_) => true,
            HirExpr::Call { func, .. } if func == "dict" => true,
            HirExpr::Var(name) => {
                // Check var_types to see if this variable is a dict/HashMap
                if let Some(var_type) = self.ctx.var_types.get(name) {
                    matches!(var_type, Type::Dict(_, _))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if expr is a reference parameter being compared to a value type.
    /// When comparing &T with T (enum variant or field access), we need to dereference the reference.
    fn is_ref_param_compared_to_enum_variant(&self, expr: &HirExpr, other: &HirExpr) -> bool {
        // Check if expr is a variable that's a reference parameter
        // but NOT shadowed by a local variable (e.g., for-loop variable)
        let is_ref_param = if let HirExpr::Var(name) = expr {
            self.ctx.current_func_ref_params.contains(name)
                && !self.ctx.shadowed_ref_params.contains(name)
        } else {
            false
        };

        if !is_ref_param {
            return false;
        }

        // Check if the other side is an enum variant (Enum.Variant or known enum type)
        if self.is_enum_variant_expr(other) {
            return true;
        }

        // Check if the other side is a field access on a non-reference base
        // e.g., p.position where p is a lambda parameter (not a ref param)
        // In this case, p.position returns a value T, but expr is &T
        if let HirExpr::Attribute { value, .. } = other {
            // The base of the attribute access should NOT be a reference parameter
            // If it's a local variable (like lambda param), the field access returns T
            if !self.is_ref_param_base(value) {
                return true;
            }
        }

        false
    }

    /// Check if an expression is an enum variant access (e.g., Team.Home, Color.RED)
    fn is_enum_variant_expr(&self, expr: &HirExpr) -> bool {
        if let HirExpr::Attribute { value, .. } = expr {
            if let HirExpr::Var(type_name) = &**value {
                // Check if it's a known enum type
                if self.ctx.enum_names.contains(type_name) {
                    return true;
                }
                // Heuristic: PascalCase name with UPPER_CASE or PascalCase attribute
                let first_char = type_name.chars().next().unwrap_or('a');
                if first_char.is_uppercase() {
                    return true;
                }
            }
        }
        false
    }

    /// Check if an expression produces a Copy type (primitives like i32, f64, bool).
    /// Copy types should never be wrapped in references when returning.
    fn is_copy_type_expr(&self, expr: &HirExpr) -> bool {
        use crate::hir::{BinOp, Literal};

        match expr {
            // Literals of primitive types are Copy
            HirExpr::Literal(lit) => {
                matches!(lit, Literal::Int(_) | Literal::Float(_) | Literal::Bool(_))
            }

            // Binary operations that produce primitives are Copy
            HirExpr::Binary { op, .. } => matches!(
                op,
                BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::FloorDiv
                    | BinOp::Mod
                    | BinOp::Pow
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor
                    | BinOp::LShift
                    | BinOp::RShift
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::And
                    | BinOp::Or
                    | BinOp::Is
                    | BinOp::IsNot
                    | BinOp::In
                    | BinOp::NotIn
            ),

            // Unary operations that produce primitives are Copy
            HirExpr::Unary { op, .. } => {
                use crate::hir::UnaryOp;
                matches!(
                    op,
                    UnaryOp::Not | UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot
                )
            }

            // Method calls that return primitives are Copy
            HirExpr::MethodCall { method, .. } => {
                // Common methods that return primitives
                matches!(
                    method.as_str(),
                    "len"
                        | "count"
                        | "abs"
                        | "is_empty"
                        | "is_some"
                        | "is_none"
                        | "is_ok"
                        | "is_err"
                        | "saturating_sub"
                        | "saturating_add"
                        | "saturating_mul"
                        | "checked_add"
                        | "checked_sub"
                        | "checked_mul"
                        | "checked_div"
                        | "wrapping_add"
                        | "wrapping_sub"
                        | "wrapping_mul"
                )
            }

            // Builtin calls that return primitives
            HirExpr::Call { func, .. } => {
                matches!(
                    func.as_str(),
                    "len"
                        | "int"
                        | "float"
                        | "bool"
                        | "abs"
                        | "min"
                        | "max"
                        | "round"
                        | "floor"
                        | "ceil"
                        | "ord"
                        | "hash"
                )
            }

            // Variables - check their type
            HirExpr::Var(name) => {
                if let Some(ty) = self.ctx.var_types.get(name) {
                    match ty {
                        Type::Int | Type::Float | Type::Bool => true,
                        Type::Custom(n) => {
                            self.ctx.enum_names.contains(n)
                                || self.ctx.copy_structs.contains(n)
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }

            // Attribute access - check if the field type is a Copy type
            // e.g., state.ball_location.y where y is i32, or state.statistics where Statistics is Copy
            HirExpr::Attribute { value, attr } => {
                // Try to get the field type from context
                if let Some(ty) = self.ctx.get_attribute_field_type(value, attr) {
                    match &ty {
                        Type::Int | Type::Float | Type::Bool => true,
                        Type::Custom(name) => {
                            self.ctx.enum_names.contains(name)
                                || self.ctx.copy_structs.contains(name)
                        }
                        _ => false,
                    }
                } else {
                    // Heuristic: check if the field name suggests a primitive type
                    let attr_str = attr.as_str();

                    // Exact matches for common primitive field names
                    let is_exact_match = matches!(
                        attr_str,
                        "x" | "y"
                            | "z"
                            | "w"
                            | "width"
                            | "height"
                            | "len"
                            | "length"
                            | "count"
                            | "size"
                            | "index"
                            | "id"
                            | "number"
                            | "num"
                            | "score"
                            | "points"
                            | "value"
                            | "amount"
                            | "total"
                            | "min"
                            | "max"
                            | "sum"
                            | "avg"
                            | "mean"
                            | "price"
                            | "cost"
                            | "rate"
                            | "ratio"
                            | "percentage"
                            | "percent"
                            | "time"
                            | "duration"
                            | "elapsed"
                            | "remaining"
                            | "offset"
                            | "delta"
                            | "margin"
                            | "handicap"
                            | "weight"
                            | "probability"
                            | "chance"
                    );

                    // Suffix patterns that suggest primitives
                    let has_primitive_suffix = attr_str.ends_with("_count")
                        || attr_str.ends_with("_size")
                        || attr_str.ends_with("_len")
                        || attr_str.ends_with("_length")
                        || attr_str.ends_with("_index")
                        || attr_str.ends_with("_id")
                        || attr_str.ends_with("_num")
                        || attr_str.ends_with("_number")
                        || attr_str.ends_with("_score")
                        || attr_str.ends_with("_points")
                        || attr_str.ends_with("_total")
                        || attr_str.ends_with("_sum")
                        || attr_str.ends_with("_min")
                        || attr_str.ends_with("_max")
                        || attr_str.ends_with("_avg")
                        || attr_str.ends_with("_mean")
                        || attr_str.ends_with("_price")
                        || attr_str.ends_with("_cost")
                        || attr_str.ends_with("_rate")
                        || attr_str.ends_with("_ratio")
                        || attr_str.ends_with("_time")
                        || attr_str.ends_with("_duration")
                        || attr_str.ends_with("_elapsed")
                        || attr_str.ends_with("_remaining")
                        || attr_str.ends_with("_offset")
                        || attr_str.ends_with("_delta")
                        || attr_str.ends_with("_margin")
                        || attr_str.ends_with("_handicap")
                        || attr_str.ends_with("_weight")
                        || attr_str.ends_with("_probability")
                        || attr_str.ends_with("_percentage")
                        || attr_str.ends_with("_chance");

                    is_exact_match || has_primitive_suffix
                }
            }

            // IfExpr - check both branches
            HirExpr::IfExpr { body, orelse, .. } => {
                self.is_copy_type_expr(body) && self.is_copy_type_expr(orelse)
            }

            // Default: not a known Copy type
            _ => false,
        }
    }

    /// Check if the base of an attribute access is a reference parameter.
    /// This is used to determine if cloning is needed when accessing fields.
    fn is_ref_param_base(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Var(name) => {
                // Check if this variable is a reference parameter
                self.ctx.current_func_ref_params.contains(name)
                    || self.ctx.current_func_mut_ref_params.contains(name)
            }
            HirExpr::Attribute { value, .. } => {
                // Recursively check nested attributes (e.g., state.inner.field)
                self.is_ref_param_base(value)
            }
            _ => false,
        }
    }

    /// Used to distinguish regex.match() from obj.match() where "match" is a user method
    fn is_regex_expr(&self, expr: &HirExpr) -> bool {
        match expr {
            // Check for re.compile() call result
            HirExpr::Call { func, .. } if func.as_str() == "compile" => true,
            // Check for variable with Regex type annotation or tracked type
            HirExpr::Var(name) => {
                // Check var_types for Regex type
                if let Some(Type::Custom(type_name)) = self.ctx.var_types.get(name) {
                    type_name.contains("Regex") || type_name.contains("Pattern")
                } else {
                    // Heuristic: variable names containing "regex", "pattern", "re_"
                    let n = name.as_str();
                    n.contains("regex") || n.contains("pattern") || n.starts_with("re_")
                }
            }
            // Check for re.compile() attribute access
            HirExpr::Attribute { value, attr } => {
                if attr == "compile" {
                    if let HirExpr::Var(module) = &**value {
                        return module == "re";
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn is_path_expr(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Call { func, .. } if func == "Path" || func == "PathBuf" => true,
            HirExpr::Var(name) => {
                if let Some(Type::Custom(type_name)) = self.ctx.var_types.get(name) {
                    type_name == "Path" || type_name == "PathBuf"
                } else {
                    let n = name.as_str();
                    n == "path" || n.ends_with("_path") || n.ends_with("Path")
                }
            }
            HirExpr::Attribute { value, attr } => {
                if attr == "path" || attr == "cwd" || attr == "home" {
                    if let HirExpr::Var(module) = &**value {
                        return module == "Path" || module == "pathlib";
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn is_list_expr(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::List(_) => true,
            HirExpr::Call { func, .. } if func == "list" => true,
            HirExpr::Var(name) => {
                // Check type info for variables
                if let Some(var_type) = self.ctx.var_types.get(name) {
                    matches!(var_type, Type::List(_))
                } else {
                    false
                }
            }
            HirExpr::Attribute { value, attr } => {
                // Check if this is a field access to a list field
                if let Some(field_type) = self.get_field_type(value, attr) {
                    // Handle both List and Optional<List>
                    matches!(field_type, Type::List(_))
                        || matches!(field_type, Type::Optional(inner) if matches!(*inner, Type::List(_)))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if an expression involves arithmetic operators (*, /, -, %, //, **)
    /// This helps distinguish numeric operations from string concatenation
    /// when type information is not available.
    fn involves_arithmetic_op(&self, expr: &HirExpr) -> bool {
        match expr {
            // Arithmetic binary operators indicate numeric context
            HirExpr::Binary { op, left, right } => {
                matches!(
                    op,
                    BinOp::Mul
                        | BinOp::Sub
                        | BinOp::Div
                        | BinOp::FloorDiv
                        | BinOp::Mod
                        | BinOp::Pow
                        | BinOp::LShift
                        | BinOp::RShift
                        | BinOp::BitAnd
                        | BinOp::BitOr
                        | BinOp::BitXor
                ) || self.involves_arithmetic_op(left)
                    || self.involves_arithmetic_op(right)
            }
            // Unary negation indicates numeric
            HirExpr::Unary {
                op: UnaryOp::Neg, ..
            } => true,
            // Parenthesized expression - check inside
            HirExpr::Borrow { expr, .. } => self.involves_arithmetic_op(expr),
            _ => false,
        }
    }

    /// Infer the element type for sum() from the argument expression
    fn infer_sum_element_type(&self, expr: &HirExpr) -> Option<proc_macro2::TokenStream> {
        // Check if it's a function call - look up return type
        if let HirExpr::Call { func, .. } = expr {
            if let Some(ret_type) = self.ctx.function_return_types.get(func) {
                if let Type::List(elem_type) = ret_type {
                    return match elem_type.as_ref() {
                        Type::Int => Some(quote! { i32 }),
                        Type::Float => Some(quote! { f64 }),
                        _ => None,
                    };
                }
            }
        }

        // Check if it's a variable - look up its type
        if let HirExpr::Var(var_name) = expr {
            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                if let Type::List(elem_type) = var_type {
                    return match elem_type.as_ref() {
                        Type::Int => Some(quote! { i32 }),
                        Type::Float => Some(quote! { f64 }),
                        _ => None,
                    };
                }
            }
        }

        // Check if it's a list comprehension - infer element type directly
        if let HirExpr::ListComp { element, .. } = expr {
            let elem_type =
                crate::rust_gen::func_gen::infer_expr_type_with_env(element, &self.ctx.var_types);
            return match elem_type {
                Type::Int => Some(quote! { i32 }),
                Type::Float => Some(quote! { f64 }),
                _ => None,
            };
        }

        // Fall back to current return type context
        self.ctx.current_return_type.as_ref().and_then(|t| match t {
            Type::Int => Some(quote! { i32 }),
            Type::Float => Some(quote! { f64 }),
            _ => None,
        })
    }

    fn is_tuple_expr(&self, expr: &HirExpr) -> bool {
        matches!(expr, HirExpr::Tuple(_))
    }

    /// Used to determine if zip() should use .into_iter() (owned) vs .iter() (borrowed)
    ///
    /// Returns true if:
    /// - Expression is a Var with type List (Vec<T>) - function parameters are owned
    /// - Expression is a list literal - always owned
    /// - Expression is a list() call - creates owned Vec
    ///
    /// # Complexity
    /// 3 (match + type lookup + variant check)
    fn is_owned_collection(&self, expr: &HirExpr) -> bool {
        match expr {
            // List literals are always owned
            HirExpr::List(_) => true,
            // list() calls create owned Vec
            HirExpr::Call { func, .. } if func == "list" => true,
            // Check if variable has List type (function parameters of type Vec<T>)
            HirExpr::Var(name) => {
                if let Some(ty) = self.ctx.var_types.get(name) {
                    matches!(ty, Type::List(_))
                } else {
                    // No type info - conservative default is borrowed
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if an expression is a user-defined class instance
    fn is_class_instance(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Var(name) => {
                // Check var_types to see if this variable is a user-defined class
                if let Some(Type::Custom(class_name)) = self.ctx.var_types.get(name) {
                    // Check if this is a user-defined class (not a builtin)
                    self.ctx.class_names.contains(class_name)
                } else {
                    false
                }
            }
            HirExpr::Call { func, .. } => {
                // Direct constructor call like Calculator(10)
                self.ctx.class_names.contains(func)
            }
            _ => false,
        }
    }

    /// Get the class name from an expression if it's a user-defined class instance
    fn get_class_name(&self, expr: &HirExpr) -> Option<String> {
        match expr {
            HirExpr::Var(name) => {
                if let Some(Type::Custom(class_name)) = self.ctx.var_types.get(name) {
                    if self.ctx.class_names.contains(class_name) {
                        return Some(class_name.clone());
                    }
                }
                None
            }
            HirExpr::Call { func, .. } => {
                if self.ctx.class_names.contains(func) {
                    Some(func.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn is_bool_expr(&self, expr: &HirExpr) -> Option<bool> {
        match expr {
            // Comparison operations always return bool
            HirExpr::Binary {
                op:
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::In
                    | BinOp::NotIn
                    | BinOp::Is
                    | BinOp::IsNot,
                ..
            } => Some(true),
            // Method calls that return bool
            HirExpr::MethodCall { method, .. }
                if matches!(
                    method.as_str(),
                    "startswith"
                        | "endswith"
                        | "isdigit"
                        | "isalpha"
                        | "isspace"
                        | "isupper"
                        | "islower"
                        | "issubset"
                        | "issuperset"
                        | "isdisjoint"
                ) =>
            {
                Some(true)
            }
            // Boolean literals
            HirExpr::Literal(Literal::Bool(_)) => Some(true),
            // Logical operations
            HirExpr::Unary {
                op: UnaryOp::Not, ..
            } => Some(true),
            _ => None,
        }
    }

    fn convert_set_operation(
        &self,
        op: BinOp,
        left: syn::Expr,
        right: syn::Expr,
    ) -> Result<syn::Expr> {
        match op {
            BinOp::BitAnd => Ok(parse_quote! {
                #left.intersection(&#right).cloned().collect::<std::collections::HashSet<_>>()
            }),
            BinOp::BitOr => Ok(parse_quote! {
                #left.union(&#right).cloned().collect::<std::collections::HashSet<_>>()
            }),
            BinOp::Sub => Ok(parse_quote! {
                #left.difference(&#right).cloned().collect::<std::collections::HashSet<_>>()
            }),
            BinOp::BitXor => Ok(parse_quote! {
                #left.symmetric_difference(&#right).cloned().collect::<std::collections::HashSet<_>>()
            }),
            _ => bail!("Invalid set operator"),
        }
    }

    fn convert_set_comp(
        &mut self,
        element: &HirExpr,
        target: &str,
        iter: &HirExpr,
        condition: &Option<Box<HirExpr>>,
    ) -> Result<syn::Expr> {
        // Uses the same strategy as list comprehensions:
        // - Ranges are already iterators, use them directly
        // - Collections need .iter() to get an iterator
        // - Use .cloned() to convert &T to T
        // - Place .cloned() AFTER .filter() so filter sees &T

        // Key insight: .filter(|x| ...) receives &Item where Item is the iterator's item type.
        // So .iter().filter() means x is &&T, but .iter().filter().cloned() means filter gets &T
        // and then .cloned() converts to T for the next stage.

        self.ctx.needs_hashset = true;
        // Parse target as pattern (supports both simple variables and tuple unpacking)
        let target_pat = self.parse_target_pattern(target)?;
        let iter_expr = iter.to_rust_expr(self.ctx)?;
        let element_expr = element.to_rust_expr(self.ctx)?;

        let is_range = self.is_range_expr(&iter_expr);

        // Check if this is an identity map (element is just the target variable)
        let is_identity_map = Self::is_identity_element(element, target);

        if let Some(cond) = condition {
            let cond_expr = cond.to_rust_expr(self.ctx)?;
            if is_range {
                // Ranges are already iterators, don't call .iter()
                // Range items are owned (i32, etc.), so no dereference needed
                if is_identity_map {
                    Ok(parse_quote! {
                        (#iter_expr)
                            .filter(|#target_pat| #cond_expr)
                            .collect::<HashSet<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        (#iter_expr)
                            .filter(|#target_pat| #cond_expr)
                            .map(|#target_pat| #element_expr)
                            .collect::<HashSet<_>>()
                    })
                }
            } else {
                // Collections need .iter().filter().cloned().map()
                // Use |&target| pattern to automatically dereference in filter closure
                if is_identity_map {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .filter(|&#target_pat| #cond_expr)
                            .cloned()
                            .collect::<HashSet<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        #iter_expr
                            .iter()
                            .filter(|&#target_pat| #cond_expr)
                            .cloned()
                            .map(|#target_pat| #element_expr)
                            .collect::<HashSet<_>>()
                    })
                }
            }
        } else if is_range {
            if is_identity_map {
                Ok(parse_quote! {
                    (#iter_expr).collect::<HashSet<_>>()
                })
            } else {
                Ok(parse_quote! {
                    (#iter_expr)
                        .map(|#target_pat| #element_expr)
                        .collect::<HashSet<_>>()
                })
            }
        } else if is_identity_map {
            Ok(parse_quote! {
                #iter_expr
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            })
        } else {
            Ok(parse_quote! {
                #iter_expr
                    .iter()
                    .cloned()
                    .map(|#target_pat| #element_expr)
                    .collect::<HashSet<_>>()
            })
        }
    }

    fn convert_dict_comp(
        &mut self,
        key: &HirExpr,
        value: &HirExpr,
        target: &str,
        iter: &HirExpr,
        condition: &Option<Box<HirExpr>>,
    ) -> Result<syn::Expr> {
        // Uses the same strategy as list comprehensions:
        // - Ranges are already iterators, use them directly
        // - Collections need .iter() to get an iterator
        // - Use .cloned() to convert &T to T
        // - Place .cloned() AFTER .filter() so filter sees &T

        // Key insight: .filter(|x| ...) receives &Item where Item is the iterator's item type.
        // So .iter().filter() means x is &&T, but .iter().filter().cloned() means filter gets &T
        // and then .cloned() converts to T for the next stage.

        self.ctx.needs_hashmap = true;
        // Parse target as pattern (supports both simple variables and tuple unpacking)
        let target_pat = self.parse_target_pattern(target)?;
        let iter_expr = iter.to_rust_expr(self.ctx)?;
        let key_expr = key.to_rust_expr(self.ctx)?;
        let value_expr = value.to_rust_expr(self.ctx)?;

        let is_range = self.is_range_expr(&iter_expr);

        if let Some(cond) = condition {
            let cond_expr = cond.to_rust_expr(self.ctx)?;
            if is_range {
                // Ranges are already iterators, don't call .iter()
                // Range items are owned (i32, etc.), so no dereference needed
                Ok(parse_quote! {
                    (#iter_expr)
                        .filter(|#target_pat| #cond_expr)
                        .map(|#target_pat| (#key_expr, #value_expr))
                        .collect::<HashMap<_, _>>()
                })
            } else {
                // Collections need .iter().filter().cloned().map()
                // Use |&target| pattern to automatically dereference in filter closure
                Ok(parse_quote! {
                    #iter_expr
                        .iter()
                        .filter(|&#target_pat| #cond_expr)
                        .cloned()
                        .map(|#target_pat| (#key_expr, #value_expr))
                        .collect::<HashMap<_, _>>()
                })
            }
        } else if is_range {
            Ok(parse_quote! {
                (#iter_expr)
                    .map(|#target_pat| (#key_expr, #value_expr))
                    .collect::<HashMap<_, _>>()
            })
        } else {
            Ok(parse_quote! {
                #iter_expr
                    .iter()
                    .cloned()
                    .map(|#target_pat| (#key_expr, #value_expr))
                    .collect::<HashMap<_, _>>()
            })
        }
    }

    fn convert_lambda(&mut self, params: &[String], body: &HirExpr) -> Result<syn::Expr> {
        // Convert parameters to pattern identifiers
        let param_pats: Vec<syn::Pat> = params
            .iter()
            .map(|p| {
                let ident = syn::Ident::new(p, proc_macro2::Span::call_site());
                parse_quote! { #ident }
            })
            .collect();

        // Convert body expression
        let body_expr = body.to_rust_expr(self.ctx)?;

        // Generate closure
        if params.is_empty() {
            // No parameters
            Ok(parse_quote! { || #body_expr })
        } else if params.len() == 1 {
            // Single parameter
            let param = &param_pats[0];
            Ok(parse_quote! { |#param| #body_expr })
        } else {
            // Multiple parameters
            Ok(parse_quote! { |#(#param_pats),*| #body_expr })
        }
    }

    /// Check if an expression is a len() call
    fn is_len_call(&self, expr: &HirExpr) -> bool {
        matches!(expr, HirExpr::Call { func, args , ..} if func == "len" && args.len() == 1)
    }

    fn convert_await(&mut self, value: &HirExpr) -> Result<syn::Expr> {
        let value_expr = value.to_rust_expr(self.ctx)?;
        Ok(parse_quote! { #value_expr.await })
    }

    fn convert_yield(&mut self, value: &Option<Box<HirExpr>>) -> Result<syn::Expr> {
        if self.ctx.in_generator {
            // Inside Iterator::next() - convert to return Some(value)
            if let Some(v) = value {
                let value_expr = v.to_rust_expr(self.ctx)?;
                Ok(parse_quote! { return Some(#value_expr) })
            } else {
                Ok(parse_quote! { return None })
            }
        } else {
            // Outside generator context - keep as yield (placeholder for future)
            if let Some(v) = value {
                let value_expr = v.to_rust_expr(self.ctx)?;
                Ok(parse_quote! { yield #value_expr })
            } else {
                Ok(parse_quote! { yield })
            }
        }
    }

    fn convert_fstring(&mut self, parts: &[FStringPart]) -> Result<syn::Expr> {
        // Handle empty f-strings
        if parts.is_empty() {
            return Ok(parse_quote! { "".to_string() });
        }

        // Check if it's just a plain string (no expressions)
        let has_expressions = parts.iter().any(|p| matches!(p, FStringPart::Expr(_)));

        if !has_expressions {
            // Just literal parts - concatenate them
            let mut result = String::new();
            for part in parts {
                if let FStringPart::Literal(s) = part {
                    result.push_str(s);
                }
            }
            return Ok(parse_quote! { #result.to_string() });
        }

        // Build format string template and collect arguments
        let mut template = String::new();
        let mut args = Vec::new();

        for part in parts {
            match part {
                FStringPart::Literal(s) => {
                    template.push_str(s);
                }
                FStringPart::Expr(expr) => {
                    // - Collections (Vec, HashMap, HashSet): Use {:?} debug formatting
                    // - Scalars (String, i32, f64, bool): Use {} Display formatting
                    // - Option types: Display "None" or unwrapped value
                    // This matches Python semantics where lists/dicts have their own repr
                    let arg_expr = expr.to_rust_expr(self.ctx)?;

                    // Use get_optional_inner_type to detect Optional types for both
                    // variables and class field attributes (dataclass fields)
                    let optional_inner = self.ctx.get_optional_inner_type(expr.as_ref());
                    let is_option = optional_inner.is_some()
                        || match expr.as_ref() {
                            HirExpr::Attribute { value, attr } => {
                                if let HirExpr::Var(obj_name) = value.as_ref() {
                                    let is_args_var =
                                        self.ctx.argparser_tracker.parsers.values().any(
                                            |parser_info| {
                                                parser_info
                                                    .args_var
                                                    .as_ref()
                                                    .is_some_and(|args_var| args_var == obj_name)
                                            },
                                        );

                                    if is_args_var {
                                        // Check if this argument is optional (Option<T> type, not boolean)
                                        self.ctx.argparser_tracker.parsers.values().any(
                                            |parser_info| {
                                                parser_info.arguments.iter().any(|arg| {
                                                    let field_name = arg.rust_field_name();
                                                    if field_name != *attr {
                                                        return false;
                                                    }

                                                    // Argument is NOT an Option if it has action="store_true" or "store_false"
                                                    if matches!(
                                                        arg.action.as_deref(),
                                                        Some("store_true") | Some("store_false")
                                                    ) {
                                                        return false;
                                                    }

                                                    // Argument is an Option<T> if: not required AND no default value AND not positional
                                                    !arg.is_positional
                                                        && !arg.required.unwrap_or(false)
                                                        && arg.default.is_none()
                                                })
                                            },
                                        )
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        };

                    // Determine if this expression is a collection type
                    let is_collection = match expr.as_ref() {
                        // Case 1: Simple variable (e.g., targets)
                        HirExpr::Var(var_name) => {
                            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                                matches!(var_type, Type::List(_) | Type::Dict(_, _) | Type::Set(_))
                            } else {
                                false
                            }
                        }
                        // Case 2: Attribute access (e.g., args.targets)
                        HirExpr::Attribute { value, attr } => {
                            // Check if this is accessing a field from argparse Args struct
                            if let HirExpr::Var(obj_name) = value.as_ref() {
                                // Check if obj_name is the args variable from ArgumentParser
                                let is_args_var = self.ctx.argparser_tracker.parsers.values().any(
                                    |parser_info| {
                                        parser_info
                                            .args_var
                                            .as_ref()
                                            .is_some_and(|args_var| args_var == obj_name)
                                    },
                                );

                                if is_args_var {
                                    // Look up the field type in argparse arguments
                                    self.ctx
                                        .argparser_tracker
                                        .parsers
                                        .values()
                                        .any(|parser_info| {
                                            parser_info.arguments.iter().any(|arg| {
                                                // Match field name (normalized from Python argument name)
                                                let field_name = arg.rust_field_name();
                                                if field_name == *attr {
                                                    // Check if this field is a collection type
                                                    // Either explicit type annotation OR inferred from nargs
                                                    let is_vec_from_nargs = matches!(
                                                        arg.nargs.as_deref(),
                                                        Some("+") | Some("*")
                                                    );
                                                    let is_collection_type =
                                                        if let Some(ref arg_type) = arg.arg_type {
                                                            matches!(
                                                                arg_type,
                                                                Type::List(_)
                                                                    | Type::Dict(_, _)
                                                                    | Type::Set(_)
                                                            )
                                                        } else {
                                                            false
                                                        };
                                                    is_vec_from_nargs || is_collection_type
                                                } else {
                                                    false
                                                }
                                            })
                                        })
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };

                    let final_arg = if is_option {
                        // Option<T> doesn't implement Display, so we need to unwrap it
                        // For string-like types, display the value or "None"
                        parse_quote! {
                            {
                                match &#arg_expr {
                                    Some(v) => format!("{}", v),
                                    None => "None".to_string(),
                                }
                            }
                        }
                    } else {
                        arg_expr
                    };

                    if is_collection {
                        // Use debug formatting for collections (matches Python's list/dict repr)
                        template.push_str("{:?}");
                    } else {
                        // Use Display formatting for scalars (and wrapped Options)
                        template.push_str("{}");
                    }

                    args.push(final_arg);
                }
            }
        }

        // Generate format!() macro call
        if args.is_empty() {
            // No arguments (shouldn't happen but be safe)
            Ok(parse_quote! { #template.to_string() })
        } else {
            // Build the format! call with template and arguments
            Ok(parse_quote! { format!(#template, #(#args),*) })
        }
    }

    fn convert_ifexpr(
        &mut self,
        test: &HirExpr,
        body: &HirExpr,
        orelse: &HirExpr,
    ) -> Result<syn::Expr> {
        // Python: `args.include if args.include else []` (check if list is non-empty)
        // Rust: Just `args.include` (clap initializes Vec to empty, so redundant check)
        // This pattern is common with argparse + Vec/Option fields
        if test == body {
            // Pattern: `x if x else y` → just use `x` (the condition is redundant)
            // This avoids type errors where Vec/Option can't be used as bool
            return body.to_rust_expr(self.ctx);
        }

        // Pattern: `x if x is not None else default` → `x.unwrap_or(default)`
        // This handles Optional types correctly by unwrapping the Some value
        if let HirExpr::Binary { op, left, right } = test {
            if matches!(op, BinOp::IsNot)
                && matches!(right.as_ref(), HirExpr::Literal(Literal::None))
            {
                // Test is `x is not None`
                if left.as_ref() == body {
                    // Body is the same variable being tested
                    let var_expr = body.to_rust_expr(self.ctx)?;
                    let default_expr = orelse.to_rust_expr(self.ctx)?;
                    return Ok(parse_quote! { #var_expr.unwrap_or(#default_expr) });
                }
            } else if matches!(op, BinOp::Is)
                && matches!(right.as_ref(), HirExpr::Literal(Literal::None))
            {
                // Test is `x is None` - inverted logic
                if left.as_ref() == body {
                    // Pattern: `x if x is None else default` → `x.unwrap_or(default)`
                    // This is unusual but handle it for completeness
                    let var_expr = body.to_rust_expr(self.ctx)?;
                    let default_expr = orelse.to_rust_expr(self.ctx)?;
                    return Ok(parse_quote! { #var_expr.unwrap_or(#default_expr) });
                }
            }
        }

        let mut test_expr = test.to_rust_expr(self.ctx)?;

        // When both branches are attribute accesses (e.g., state.home_x if ... else state.away_x),
        // avoid cloning ONLY when the base is not a reference parameter.
        // If the base is a reference (&State), we must clone to get an owned value.
        // EXCEPTION: If generate_borrow is set, the caller has determined borrowing is safe.
        let both_attrs = matches!(body, HirExpr::Attribute { .. })
            && matches!(orelse, HirExpr::Attribute { .. });

        // Check if both branches are enum variants (e.g., Team.Home if ... else Team.Away)
        // Enum variants are Copy types and should never be borrowed.
        let both_enum_variants =
            self.is_enum_variant_expr(body) && self.is_enum_variant_expr(orelse);

        // Check if both branches produce Copy types (primitives like i32, f64, bool).
        // Copy types should never be wrapped in references.
        let both_copy_types = self.is_copy_type_expr(body) && self.is_copy_type_expr(orelse);

        let base_is_ref_param = if both_attrs {
            // Check if the base of either attribute is a reference parameter
            let body_base_is_ref = if let HirExpr::Attribute { value, .. } = body {
                self.is_ref_param_base(value)
            } else {
                false
            };
            let orelse_base_is_ref = if let HirExpr::Attribute { value, .. } = orelse {
                self.is_ref_param_base(value)
            } else {
                false
            };
            body_base_is_ref || orelse_base_is_ref
        } else {
            false
        };

        // Skip cloning if:
        // 1. Both branches are attributes AND the base is not a reference parameter, OR
        // 2. generate_borrow is set (caller has determined borrowing is safe), OR
        // 3. Both branches produce Copy types (no clone needed for Copy)
        let skip_clone =
            (both_attrs && !base_is_ref_param) || self.ctx.generate_borrow || both_copy_types;
        let was_prevent_clone = self.ctx.prevent_clone;
        let was_clone_already_applied = self.ctx.clone_already_applied;
        if skip_clone {
            self.ctx.prevent_clone = true;
        }
        // Reset clone_already_applied before each branch to ensure consistent cloning
        self.ctx.clone_already_applied = false;
        let mut body_expr = body.to_rust_expr(self.ctx)?;
        self.ctx.clone_already_applied = false;
        let mut orelse_expr = orelse.to_rust_expr(self.ctx)?;
        self.ctx.clone_already_applied = was_clone_already_applied;
        if skip_clone {
            self.ctx.prevent_clone = was_prevent_clone;
        }

        // When generate_mut_borrow is set, wrap each branch in &mut for mutable field access
        // This handles: `team_stats = state.home_stats if cond else state.away_stats`
        // → `if cond { &mut state.home_stats } else { &mut state.away_stats }`
        // Skip for enum variants, Copy types, and primitive cast contexts - they should not be borrowed.
        if self.ctx.generate_mut_borrow
            && both_attrs
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &mut #body_expr };
            orelse_expr = parse_quote! { &mut #orelse_expr };
        }
        // When generate_borrow is set, wrap each branch in & for immutable borrow
        // This handles: `players = state.home_players if cond else state.away_players`
        // → `if cond { &state.home_players } else { &state.away_players }`
        // Skip for enum variants, Copy types, and primitive cast contexts - they should not be borrowed.
        else if self.ctx.generate_borrow
            && both_attrs
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &#body_expr };
            orelse_expr = parse_quote! { &#orelse_expr };
        }
        // When function returns a mutable reference, wrap each branch in &mut
        // This handles: `return state.home_stats if ... else state.away_stats`
        // → `if ... { &mut state.home_stats } else { &mut state.away_stats }`
        // Skip for enum variants, Copy types, and primitive cast contexts.
        else if self.ctx.returns_mutable_reference
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &mut #body_expr };
            orelse_expr = parse_quote! { &mut #orelse_expr };
        }
        // When function returns an immutable reference, wrap each branch in &
        // This handles: `return state.home_players if ... else state.away_players`
        // → `if ... { &state.home_players } else { &state.away_players }`
        // Skip for enum variants, Copy types, and primitive cast contexts.
        else if self.ctx.returns_reference
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &#body_expr };
            orelse_expr = parse_quote! { &#orelse_expr };
        }

        // Ensure type consistency: if either branch is a string literal, wrap with .to_string()
        let body_is_string_lit = matches!(body, HirExpr::Literal(Literal::String(_)));
        let orelse_is_string_lit = matches!(orelse, HirExpr::Literal(Literal::String(_)));
        if body_is_string_lit || orelse_is_string_lit {
            if body_is_string_lit {
                body_expr = parse_quote! { #body_expr.to_string() };
            }
            if orelse_is_string_lit {
                // orelse_expr was made immutable, so this case is handled below
            }
        }

        // Ensure numeric type consistency: promote int to float when branches have mixed types
        // Python: `1 if cond else 1.0` → Rust: `if cond { 1.0 } else { 1.0 }`
        let body_is_int_lit = matches!(body, HirExpr::Literal(Literal::Int(_)));
        let body_is_float_lit = matches!(body, HirExpr::Literal(Literal::Float(_)));
        let orelse_is_int_lit = matches!(orelse, HirExpr::Literal(Literal::Int(_)));
        let orelse_is_float_lit = matches!(orelse, HirExpr::Literal(Literal::Float(_)));

        if body_is_int_lit && orelse_is_float_lit {
            // Promote body (int) to float with proper decimal format
            if let HirExpr::Literal(Literal::Int(i)) = body {
                let float_str = format!("{}.0", i);
                let float_lit: syn::LitFloat =
                    syn::LitFloat::new(&float_str, proc_macro2::Span::call_site());
                body_expr = syn::Expr::Lit(syn::ExprLit {
                    attrs: vec![],
                    lit: syn::Lit::Float(float_lit),
                });
            }
        } else if body_is_float_lit && orelse_is_int_lit {
            // Promote orelse (int) to float with proper decimal format
            if let HirExpr::Literal(Literal::Int(i)) = orelse {
                let float_str = format!("{}.0", i);
                let float_lit: syn::LitFloat =
                    syn::LitFloat::new(&float_str, proc_macro2::Span::call_site());
                orelse_expr = syn::Expr::Lit(syn::ExprLit {
                    attrs: vec![],
                    lit: syn::Lit::Float(float_lit),
                });
            }
        }

        // Python: `val if val else default` where val is String/List/Dict/Set/Optional/Int/Float
        // Without conversion: `if val` fails (expected bool, found Vec/String/etc)
        // With conversion: `if !val.is_empty()` / `if val.is_some()` / `if val != 0`
        test_expr = Self::apply_truthiness_conversion(test, test_expr, self.ctx);

        // Python: `x[0] if x else None` → Rust: `if !x.is_empty() { Some(x.get(0).cloned().unwrap()) } else { None }`
        // When else branch is None, wrap body in Some() for correct Option<T> type
        let orelse_is_none = matches!(orelse, HirExpr::Literal(Literal::None));

        // If body is an Optional type, we need to unwrap it since the condition guarantees it's Some
        // Python: `x if x is not None else 0` → Rust: `if x.is_some() { x.unwrap() } else { 0 }`
        // Python: `x if x else 0` where x is Optional → Rust: `if x.is_some() { x.unwrap() } else { 0 }`
        let body_is_optional = self.ctx.get_optional_inner_type(body).is_some();
        if body_is_optional && !orelse_is_none {
            // Check if test is checking the same variable for is_some()
            let test_checks_body = match test {
                // Pattern: `x.is_some()` (HIR has already converted `x is not None` to this)
                HirExpr::MethodCall { object, method, .. } if method == "is_some" => {
                    object.as_ref() == body
                }
                // Pattern: `x if x is not None else default` (original Python, before HIR transformation)
                HirExpr::Binary { op, left, right }
                    if matches!(op, BinOp::IsNot)
                        && matches!(right.as_ref(), HirExpr::Literal(Literal::None)) =>
                {
                    left.as_ref() == body
                }
                // Pattern: `x if x else default` (direct truthiness check on Optional)
                _ => test == body,
            };

            if test_checks_body {
                // Unwrap the Optional body since we know it's Some
                body_expr = parse_quote! { #body_expr.unwrap() };
            }
        }
        if orelse_is_none {
            Ok(parse_quote! {
                if #test_expr { Some(#body_expr) } else { None }
            })
        } else if orelse_is_string_lit {
            let orelse_str_expr: syn::Expr = parse_quote! { #orelse_expr.to_string() };
            Ok(parse_quote! {
                if #test_expr { #body_expr } else { #orelse_str_expr }
            })
        } else {
            Ok(parse_quote! {
                if #test_expr { #body_expr } else { #orelse_expr }
            })
        }
    }

    /// Apply Python truthiness conversion to non-boolean conditions
    /// Python: `if val:` where val is String/List/Dict/Set/Optional/Int/Float
    /// Rust: `if !val.is_empty()` / `if val.is_some()` / `if val != 0`
    fn apply_truthiness_conversion(
        condition: &HirExpr,
        cond_expr: syn::Expr,
        ctx: &CodeGenContext,
    ) -> syn::Expr {
        // First, try using get_optional_inner_type which handles more cases
        if ctx.get_optional_inner_type(condition).is_some() {
            return parse_quote! { #cond_expr.is_some() };
        }

        // Check if this is a variable reference that needs truthiness conversion
        if let HirExpr::Var(var_name) = condition {
            if let Some(var_type) = ctx.var_types.get(var_name) {
                return match var_type {
                    // Already boolean - no conversion needed
                    Type::Bool => cond_expr,

                    // String/List/Dict/Set - check if empty
                    Type::String | Type::List(_) | Type::Dict(_, _) | Type::Set(_) => {
                        parse_quote! { !#cond_expr.is_empty() }
                    }

                    // Optional - check if Some (should be caught above, but keep for safety)
                    Type::Optional(_) => {
                        parse_quote! { #cond_expr.is_some() }
                    }

                    // Numeric types - check if non-zero
                    Type::Int => {
                        parse_quote! { #cond_expr != 0 }
                    }
                    Type::Float => {
                        parse_quote! { #cond_expr != 0.0 }
                    }

                    // Unknown or other types - use as-is (may fail compilation)
                    _ => cond_expr,
                };
            }
        }

        // Not a variable or no type info - use as-is
        cond_expr
    }

    fn convert_sort_by_key(
        &mut self,
        iterable: &HirExpr,
        key_params: &[String],
        key_body: &HirExpr,
        reverse: bool,
    ) -> Result<syn::Expr> {
        let iter_expr = iterable.to_rust_expr(self.ctx)?;

        // If so, use simple .sort() instead of .sort_by_key()
        let is_identity =
            key_params.len() == 1 && matches!(key_body, HirExpr::Var(v) if v == &key_params[0]);

        if is_identity {
            // Identity function: just sort() + optional reverse()
            if reverse {
                return Ok(parse_quote! {
                    {
                        let mut __sorted_result = #iter_expr.clone();
                        __sorted_result.sort();
                        __sorted_result.reverse();
                        __sorted_result
                    }
                });
            } else {
                return Ok(parse_quote! {
                    {
                        let mut __sorted_result = #iter_expr.clone();
                        __sorted_result.sort();
                        __sorted_result
                    }
                });
            }
        }

        // Non-identity key function: use sort_by_key
        let body_expr = key_body.to_rust_expr(self.ctx)?;

        // Create the closure parameter pattern
        let param_pat: syn::Pat = if key_params.len() == 1 {
            let param = syn::Ident::new(&key_params[0], proc_macro2::Span::call_site());
            parse_quote! { #param }
        } else {
            bail!("sorted() key lambda must have exactly one parameter");
        };

        // Generate: { let mut result = iterable.clone(); result.sort_by_key(|param| body); [result.reverse();] result }
        if reverse {
            Ok(parse_quote! {
                {
                    let mut __sorted_result = #iter_expr.clone();
                    __sorted_result.sort_by_key(|#param_pat| #body_expr);
                    __sorted_result.reverse();
                    __sorted_result
                }
            })
        } else {
            Ok(parse_quote! {
                {
                    let mut __sorted_result = #iter_expr.clone();
                    __sorted_result.sort_by_key(|#param_pat| #body_expr);
                    __sorted_result
                }
            })
        }
    }

    fn convert_generator_expression(
        &mut self,
        element: &HirExpr,
        generators: &[crate::hir::HirComprehension],
    ) -> Result<syn::Expr> {
        // Strategy: Simple cases use iterator chains, nested use flat_map

        if generators.is_empty() {
            bail!("Generator expression must have at least one generator");
        }

        // Single generator case (simple iterator chain)
        if generators.len() == 1 {
            let generator = &generators[0];
            // For attribute expressions, use convert_attribute_without_clone to avoid double .clone()
            // since we add .clone() when needed in the generated code
            let iter_expr = if matches!(&*generator.iter, HirExpr::Attribute { .. }) {
                self.convert_attribute_without_clone(&generator.iter)?
            } else {
                generator.iter.to_rust_expr(self.ctx)?
            };
            let element_expr = element.to_rust_expr(self.ctx)?;
            let target_pat = self.parse_target_pattern(&generator.target)?;

            let is_csv_reader = if let HirExpr::Var(var_name) = &*generator.iter {
                var_name == "reader"
                    || var_name.contains("csv")
                    || var_name.ends_with("_reader")
                    || var_name.starts_with("reader_")
            } else {
                false
            };

            // Check if it's a range expression
            let is_range =
                matches!(&*generator.iter, HirExpr::Call { func, .. } if func == "range");

            // Determine if the element type needs clone (non-Copy) or can use copy
            // Default to true (use .cloned()) because .cloned() works for both Copy and Clone types,
            // while .copied() only works for Copy types. This is safe for custom structs like Player.
            let element_needs_clone = if let HirExpr::Var(var_name) = &*generator.iter {
                if let Some(var_type) = self.ctx.var_types.get(var_name) {
                    match var_type {
                        Type::List(elem_type) => self.type_needs_clone(elem_type),
                        Type::Set(elem_type) => self.type_needs_clone(elem_type),
                        _ => true, // Default to clone for unknown types
                    }
                } else {
                    true // Default to cloned for unknown variables (safe for non-Copy types)
                }
            } else if is_range {
                false // range() yields i32/i64 which are Copy
            } else {
                true // Default to cloned for non-variable iterators
            };

            // Check if the iterator is an enumerate call (already returns an iterator)
            let is_enumerate =
                matches!(&*generator.iter, HirExpr::Call { func, .. } if func == "enumerate");

            // When the iterator is a variable, decide between .into_iter() and .iter():
            // - Borrowed parameters (&Vec<T>): must use .iter().copied()/.cloned()
            // - Owned locals (e.g., result of a function call): use .into_iter()
            let is_borrowed_param = if let HirExpr::Var(var_name) = &*generator.iter {
                self.ctx.current_func_ref_params.contains(var_name)
                    || self.ctx.current_func_mut_ref_params.contains(var_name)
            } else {
                false
            };

            let mut chain: syn::Expr = if is_csv_reader {
                self.ctx.needs_csv = true;
                parse_quote! { #iter_expr.deserialize::<std::collections::HashMap<String, String>>().filter_map(|result| result.ok()) }
            } else if matches!(&*generator.iter, HirExpr::Var(_)) {
                if is_borrowed_param {
                    if element_needs_clone {
                        parse_quote! { #iter_expr.iter().cloned() }
                    } else {
                        parse_quote! { #iter_expr.iter().copied() }
                    }
                } else {
                    parse_quote! { #iter_expr.into_iter() }
                }
            } else if is_range || is_enumerate {
                // Ranges and enumerate() already return iterators, don't need clone
                parse_quote! { #iter_expr }
            } else {
                // Field access, method calls, etc.
                if element_needs_clone {
                    parse_quote! { #iter_expr.iter().cloned() }
                } else {
                    parse_quote! { #iter_expr.iter().copied() }
                }
            };

            // Add filters for each condition
            // .filter() always receives &Item, so for non-tuple targets we use |&x|
            // destructuring for Copy types or deref the variable in the condition body.
            let is_tuple_target = generator.target.starts_with('(');
            for cond in &generator.conditions {
                if is_tuple_target {
                    let cond_expr = cond.to_rust_expr(self.ctx)?;
                    chain = parse_quote! { #chain.filter(|#target_pat| #cond_expr) };
                } else if !element_needs_clone {
                    // Copy types: use |&x| destructuring so the body sees owned x
                    let cond_expr = cond.to_rust_expr(self.ctx)?;
                    chain = parse_quote! { #chain.filter(|&#target_pat| #cond_expr) };
                } else {
                    // Non-Copy types: keep |x| (which is &T) and deref variable uses
                    // in the condition body via context flag
                    self.ctx.filter_deref_vars.insert(generator.target.clone());
                    let cond_expr = cond.to_rust_expr(self.ctx)?;
                    self.ctx.filter_deref_vars.remove(&generator.target);
                    chain = parse_quote! { #chain.filter(|#target_pat| #cond_expr) };
                }
            }

            // Add the map transformation only if it's not an identity map (element != target)
            // Skip .map(|x| x) or .map(|(x, y)| (x, y)) which are no-ops
            let is_identity_map = Self::is_identity_element(element, &generator.target);
            if !is_identity_map {
                chain = parse_quote! { #chain.map(|#target_pat| #element_expr) };
            }

            return Ok(chain);
        }

        // Multiple generators case (nested iteration with flat_map)
        // Pattern: (x + y for x in range(3) for y in range(3))
        // Becomes: (0..3).flat_map(|x| (0..3).map(move |y| x + y))

        self.convert_nested_generators(element, generators)
    }

    fn convert_nested_generators(
        &mut self,
        element: &HirExpr,
        generators: &[crate::hir::HirComprehension],
    ) -> Result<syn::Expr> {
        // Start with the outermost generator
        let first_gen = &generators[0];
        // For attribute expressions, use convert_attribute_without_clone to avoid double .clone()
        let first_iter = if matches!(&*first_gen.iter, HirExpr::Attribute { .. }) {
            self.convert_attribute_without_clone(&first_gen.iter)?
        } else {
            first_gen.iter.to_rust_expr(self.ctx)?
        };
        let first_pat = self.parse_target_pattern(&first_gen.target)?;

        // Build the nested expression recursively
        let inner_expr = self.build_nested_chain(element, generators, 1)?;

        // Start the chain with the first generator
        let mut chain: syn::Expr = parse_quote! { #first_iter.into_iter() };

        // Add filters for first generator's conditions
        for cond in &first_gen.conditions {
            let cond_expr = cond.to_rust_expr(self.ctx)?;
            chain = parse_quote! { #chain.filter(|#first_pat| #cond_expr) };
        }

        // Use flat_map for the first generator
        chain = parse_quote! { #chain.flat_map(|#first_pat| #inner_expr) };

        Ok(chain)
    }

    fn build_nested_chain(
        &mut self,
        element: &HirExpr,
        generators: &[crate::hir::HirComprehension],
        depth: usize,
    ) -> Result<syn::Expr> {
        if depth >= generators.len() {
            // Base case: no more generators, return the element expression
            let element_expr = element.to_rust_expr(self.ctx)?;
            return Ok(element_expr);
        }

        let generator = &generators[depth];
        // For attribute expressions, use convert_attribute_without_clone to avoid double .clone()
        let iter_expr = if matches!(&*generator.iter, HirExpr::Attribute { .. }) {
            self.convert_attribute_without_clone(&generator.iter)?
        } else {
            generator.iter.to_rust_expr(self.ctx)?
        };
        let target_pat = self.parse_target_pattern(&generator.target)?;

        // Build the inner expression (recursive)
        let inner_expr = self.build_nested_chain(element, generators, depth + 1)?;

        // Build the chain for this level
        let mut chain: syn::Expr = parse_quote! { #iter_expr.into_iter() };

        // Add filters for this generator's conditions
        for cond in &generator.conditions {
            let cond_expr = cond.to_rust_expr(self.ctx)?;
            chain = parse_quote! { #chain.filter(|#target_pat| #cond_expr) };
        }

        // Use flat_map for intermediate generators, map for the last
        if depth < generators.len() - 1 {
            // Intermediate generator: use flat_map
            chain = parse_quote! { #chain.flat_map(move |#target_pat| #inner_expr) };
        } else {
            // Last generator: use map
            chain = parse_quote! { #chain.map(move |#target_pat| #inner_expr) };
        }

        Ok(chain)
    }

    /// Check if a generator/comprehension element is an identity transformation.
    /// Returns true for `.map(|x| x)` and `.map(|(x, y)| (x, y))` patterns.
    fn is_identity_element(element: &HirExpr, target: &str) -> bool {
        match element {
            HirExpr::Var(var_name) => var_name == target,
            HirExpr::Tuple(elts) if target.starts_with('(') && target.ends_with(')') => {
                let inner = &target[1..target.len() - 1];
                let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                if parts.len() != elts.len() {
                    return false;
                }
                elts.iter().zip(parts.iter()).all(|(elt, part)| {
                    matches!(elt, HirExpr::Var(name) if name == part)
                })
            }
            _ => false,
        }
    }

    fn parse_target_pattern(&self, target: &str) -> Result<syn::Pat> {
        // Handle simple variable: x
        // Handle tuple: (x, y)
        if target.starts_with('(') && target.ends_with(')') {
            // Tuple pattern
            let inner = &target[1..target.len() - 1];
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            let idents: Vec<syn::Ident> = parts
                .iter()
                .map(|s| {
                    // Check for special keywords that cannot be raw identifiers
                    if Self::is_non_raw_keyword(s) {
                        panic!(
                            "Python variable '{}' conflicts with a special Rust keyword that cannot be escaped. \
                             Please rename this variable (e.g., '{}_var' or 'py_{}')",
                            s, s, s
                        );
                    }
                    
                    if Self::is_rust_keyword(s) {
                        syn::Ident::new_raw(s, proc_macro2::Span::call_site())
                    } else {
                        syn::Ident::new(s, proc_macro2::Span::call_site())
                    }
                })
                .collect();
            Ok(parse_quote! { ( #(#idents),* ) })
        } else {
            // Check for special keywords that cannot be raw identifiers
            if Self::is_non_raw_keyword(target) {
                bail!(
                    "Python variable '{}' conflicts with a special Rust keyword that cannot be escaped. \
                     Please rename this variable (e.g., '{}_var' or 'py_{}')",
                    target,
                    target,
                    target
                );
            }
            
            // Simple variable - use raw identifier if it's a Rust keyword
            let ident = if Self::is_rust_keyword(target) {
                syn::Ident::new_raw(target, proc_macro2::Span::call_site())
            } else {
                syn::Ident::new(target, proc_macro2::Span::call_site())
            };
            Ok(parse_quote! { #ident })
        }
    }

    /// Convert a named expression (walrus operator): (x := expr)
    /// Python: (x := expensive_func(a)) or in comprehension: [y for x in data if (y := f(x)) > 5]
    /// When used in a comprehension condition, this is handled specially by the comprehension code.
    /// In other contexts, we emit a block: { let x = expr; x }
    fn convert_named_expr(&mut self, target: &str, value: &HirExpr) -> Result<syn::Expr> {
        let value_expr = value.to_rust_expr(self.ctx)?;
        let ident = if Self::is_rust_keyword(target) {
            syn::Ident::new_raw(target, proc_macro2::Span::call_site())
        } else {
            syn::Ident::new(target, proc_macro2::Span::call_site())
        };
        // Emit: { let x = expr; x }
        Ok(parse_quote! {
            {
                let #ident = #value_expr;
                #ident
            }
        })
    }
}

impl ToRustExpr for HirExpr {
    fn to_rust_expr(&self, ctx: &mut CodeGenContext) -> Result<syn::Expr> {
        // Rust auto-deref handles .field and .method() on &T. When inside a filter
        // closure, suppress the deref for the direct object of Attribute/MethodCall access.
        match self {
            HirExpr::Attribute { value, .. } | HirExpr::MethodCall { object: value, .. } => {
                if let HirExpr::Var(name) = &**value {
                    if ctx.filter_deref_vars.remove(name.as_str()) {
                        let result = self.to_rust_expr(ctx);
                        ctx.filter_deref_vars.insert(name.clone());
                        return result;
                    }
                }
            }
            _ => {}
        }

        let mut converter = ExpressionConverter::new(ctx);

        match self {
            HirExpr::Literal(lit) => {
                let expr = literal_to_rust_expr(lit, ctx);
                if let Literal::String(s) = lit {
                    let context = StringContext::Literal(s.clone());
                    if matches!(
                        ctx.string_optimizer.get_optimal_type(&context),
                        crate::string_optimization::OptimalStringType::CowStr
                    ) {
                        ctx.needs_cow = true;
                    }
                }
                Ok(expr)
            }
            HirExpr::Var(name) => {
                let base_expr = converter.convert_variable(name)?;
                // Inside filter closures, standalone variable uses need *deref
                if ctx.filter_deref_vars.contains(name.as_str()) {
                    return Ok(parse_quote! { *#base_expr });
                }
                // lazy_static constants have unique wrapper types - clone to get actual type
                // BUT: skip clone if prevent_clone is set (e.g., when used as base for .get())
                // because .get().cloned() already handles element cloning
                // Also skip clone for primitive-type constants (i32, f64, bool) since they're Copy
                let is_primitive_const = matches!(
                    ctx.var_types.get(name),
                    Some(Type::Int) | Some(Type::Float) | Some(Type::Bool)
                );
                if ctx.lazy_static_constants.contains(name)
                    && !ctx.prevent_clone
                    && !is_primitive_const
                {
                    ctx.clone_already_applied = true;
                    Ok(parse_quote! { #base_expr.clone() })
                } else if ctx.is_assignment_target || ctx.prevent_clone || ctx.clone_already_applied
                {
                    // When used as assignment target (LHS), prevent_clone is set, or clone was already applied, don't clone
                    // NOTE: Optional unwrapping for variables is handled in convert_attribute
                    // when accessing fields on Optional types - don't add it here to avoid double unwrap
                    Ok(base_expr)
                } else if ctx.var_needs_clone(name) {
                    // Check if we need to clone this variable (non-Copy type with multiple uses)
                    ctx.clone_already_applied = true;
                    Ok(parse_quote! { #base_expr.clone() })
                } else {
                    Ok(base_expr)
                }
            }
            HirExpr::Binary { op, left, right } => converter.convert_binary(*op, left, right),
            HirExpr::Unary { op, operand } => converter.convert_unary(op, operand),
            HirExpr::Call {
                func,
                args,
                kwargs,
                type_params,
            } => converter.convert_call_with_type_params(func, args, kwargs, type_params),
            HirExpr::MethodCall {
                object,
                method,
                args,
                kwargs,
                type_params,
            } => {
                // subprocess.run(cmd, capture_output=True, cwd=cwd, check=check)
                // Must handle kwargs here before they're lost
                if let HirExpr::Var(module_name) = &**object {
                    if module_name == "subprocess" && method == "run" {
                        return converter.convert_subprocess_run(args, kwargs);
                    }
                }

                converter.convert_method_call_with_type_params(
                    object,
                    method,
                    args,
                    kwargs,
                    type_params,
                )
            }
            HirExpr::Index { base, index } => converter.convert_index(base, index),
            HirExpr::Slice {
                base,
                start,
                stop,
                step,
            } => converter.convert_slice(base, start, stop, step),
            HirExpr::List(elts) => converter.convert_list(elts),
            HirExpr::Dict(items) => converter.convert_dict(items),
            HirExpr::Tuple(elts) => converter.convert_tuple(elts),
            HirExpr::Set(elts) => converter.convert_set(elts),
            HirExpr::FrozenSet(elts) => converter.convert_frozenset(elts),
            HirExpr::Attribute { value, attr } => converter.convert_attribute(value, attr),
            HirExpr::Borrow { expr, mutable } => converter.convert_borrow(expr, *mutable),
            HirExpr::ListComp {
                element,
                target,
                iter,
                condition,
            } => converter.convert_list_comp(element, target, iter, condition),
            HirExpr::FlattenedListComp {
                element,
                generators,
            } => converter.convert_flattened_list_comp(element, generators),
            HirExpr::Lambda { params, body } => converter.convert_lambda(params, body),
            HirExpr::SetComp {
                element,
                target,
                iter,
                condition,
            } => converter.convert_set_comp(element, target, iter, condition),
            HirExpr::DictComp {
                key,
                value,
                target,
                iter,
                condition,
            } => converter.convert_dict_comp(key, value, target, iter, condition),
            HirExpr::Await { value } => converter.convert_await(value),
            HirExpr::Yield { value } => converter.convert_yield(value),
            HirExpr::FString { parts } => converter.convert_fstring(parts),
            HirExpr::IfExpr { test, body, orelse } => converter.convert_ifexpr(test, body, orelse),
            HirExpr::SortByKey {
                iterable,
                key_params,
                key_body,
                reverse,
            } => converter.convert_sort_by_key(iterable, key_params, key_body, *reverse),
            HirExpr::GeneratorExp {
                element,
                generators,
            } => converter.convert_generator_expression(element, generators),
            HirExpr::NamedExpr { target, value } => converter.convert_named_expr(target, value),
            HirExpr::Uninitialized => {
                bail!("Uninitialized expression cannot be converted to a Rust expression")
            }
        }
    }
}

fn literal_to_rust_expr(lit: &Literal, ctx: &mut CodeGenContext) -> syn::Expr {
    match lit {
        Literal::Int(n) => {
            let lit = syn::LitInt::new(&n.to_string(), proc_macro2::Span::call_site());
            parse_quote! { #lit }
        }
        Literal::Float(f) => {
            // Ensure float literals always have a decimal point
            // f64::to_string() outputs "0" for 0.0, which parses as integer
            let s = f.to_string();
            let float_str = if s.contains('.') || s.contains('e') || s.contains('E') {
                s
            } else {
                format!("{}.0", s)
            };
            let lit = syn::LitFloat::new(&float_str, proc_macro2::Span::call_site());
            parse_quote! { #lit }
        }
        Literal::String(s) => {
            // String literals are emitted directly as &str
            // They'll be converted to String when needed (variable assignment, owned params, etc.)
            let lit = syn::LitStr::new(s, proc_macro2::Span::call_site());
            parse_quote! { #lit }
        }
        Literal::Bytes(b) => {
            // Generate Rust byte array: &[u8] slice from byte values
            // Python: b"hello" → Rust: &[104_u8, 101, 108, 108, 111]
            let byte_str = syn::LitByteStr::new(b, proc_macro2::Span::call_site());
            parse_quote! { #byte_str }
        }
        Literal::Bool(b) => {
            let lit = syn::LitBool::new(*b, proc_macro2::Span::call_site());
            parse_quote! { #lit }
        }
        Literal::None => {
            // When Python code uses None explicitly (e.g., in ternary expressions),
            // it should become Rust's None, not ()
            parse_quote! { None }
        }
        Literal::Ellipsis => {
            // Python's ... (Ellipsis) becomes () in Rust as a placeholder
            parse_quote! { () }
        }
        Literal::Complex(real, imag) => {
            // Python complex numbers become num::Complex<f64>
            ctx.mark_complex_used();
            let real_lit = syn::LitFloat::new(
                &if real.to_string().contains('.') || real.to_string().contains('e') {
                    real.to_string()
                } else {
                    format!("{}.0", real)
                },
                proc_macro2::Span::call_site(),
            );
            let imag_lit = syn::LitFloat::new(
                &if imag.to_string().contains('.') || imag.to_string().contains('e') {
                    imag.to_string()
                } else {
                    format!("{}.0", imag)
                },
                proc_macro2::Span::call_site(),
            );
            parse_quote! { Complex::new(#real_lit, #imag_lit) }
        }
    }
}
