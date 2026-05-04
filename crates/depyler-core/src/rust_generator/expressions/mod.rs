//! Expression code generation - main module

//! Expression code generation
//!
//! This module handles converting HIR expressions to Rust syn::Expr nodes.
//! It includes the ExpressionConverter for complex expression transformations
//! and the ToRustExpr trait implementation for HirExpr.

use crate::rust_generator::direct_rules::to_pascal_case;
use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, ToRustExpr};
use crate::rust_generator::return_type_expects_float;
use crate::rust_generator::type_gen::convert_binop;
use crate::optimizations::string_optimization::{StringContext, StringOptimizer};
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::{self, parse_quote};


mod arithmetic;
mod calls;
mod stdlib;
mod collections;
mod subscripts;
mod literals;
mod closures;
mod async_exprs;

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
            // Prefer var_types (resolved type) over optional_vars: if var_types has a known
            // non-Optional type, trust it even if optional_vars contains the variable (stale entry).
            if let HirExpr::Var(var_name) = value.as_ref() {
                let is_optional = match self.ctx.var_types.get(var_name) {
                    Some(Type::Optional(_)) => true,
                    Some(_) => false,
                    None => self.ctx.optional_vars.contains(var_name),
                };
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
                crate::rust_generator::func_gen::infer_expr_type_with_env(element, &self.ctx.var_types);
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

    /// Check if an expression is a len() call
    fn is_len_call(&self, expr: &HirExpr) -> bool {
        matches!(expr, HirExpr::Call { func, args , ..} if func == "len" && args.len() == 1)
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
                        crate::optimizations::string_optimization::OptimalStringType::CowStr
                    ) {
                        ctx.require(crate::rust_generator::context::Import::Cow);
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
