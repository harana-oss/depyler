//! Expression code generation - collections

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
    // ========================================================================
    // ========================================================================

    /// Handle list methods (append, extend, pop, insert, remove, sort)
    #[inline]
    pub(super) fn convert_list_method(
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
                // Prefer var_types (resolved type) over optional_vars (may be stale).
                let needs_unwrap = if !hir_args.is_empty() {
                    if let HirExpr::Var(var_name) = &hir_args[0] {
                        match self.ctx.var_types.get(var_name) {
                            Some(Type::Optional(_)) => true,
                            Some(_) => false,
                            None => self.ctx.optional_vars.contains(var_name),
                        }
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
    pub(super) fn convert_dict_method(
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
    pub(super) fn convert_string_method(
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

    /// Handle set methods (add, discard, clear)
    #[inline]
    pub(super) fn convert_set_method(
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
    pub(super) fn convert_regex_method(
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
    pub(super) fn convert_sys_io_method(
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
    pub(super) fn convert_instance_method(
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

    pub(super) fn convert_method_call(
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

    pub(super) fn convert_call_with_type_params(
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

    pub(super) fn convert_method_call_with_type_params(
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

    pub(super) fn convert_type_params_to_tokens(&self, type_params: &[Type]) -> proc_macro2::TokenStream {
        let type_tokens: Vec<proc_macro2::TokenStream> =
            type_params.iter().map(|t| self.type_to_tokens(t)).collect();
        quote! { #(#type_tokens),* }
    }

    pub(super) fn type_to_tokens(&self, ty: &Type) -> proc_macro2::TokenStream {
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

    pub(super) fn convert_list(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
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
        //     self.ctx.require(crate::rust_generator::context::Import::SmallVec);
        //     Ok(parse_quote! { smallvec::smallvec![#(#elt_exprs),*] })
        // } else {
        //     Ok(parse_quote! { vec![#(#elt_exprs),*] })
        // }
        Ok(parse_quote! { vec![#(#elt_exprs),*] })
    }

    pub(super) fn convert_dict(&mut self, items: &[(HirExpr, HirExpr)]) -> Result<syn::Expr> {
        // For mixed types, use serde_json::json! instead of HashMap
        let has_mixed_types = self.dict_has_mixed_types(items)?;

        if has_mixed_types {
            // Use serde_json::json! for heterogeneous dicts
            self.ctx.require(crate::rust_generator::context::Import::SerdeJson);
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
        self.ctx.require(crate::rust_generator::context::Import::HashMap);

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

    pub(super) fn dict_has_mixed_types(&self, items: &[(HirExpr, HirExpr)]) -> Result<bool> {
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

    pub(super) fn convert_tuple(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_set(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
        self.ctx.require(crate::rust_generator::context::Import::HashSet);
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

    pub(super) fn convert_frozenset(&mut self, elts: &[HirExpr]) -> Result<syn::Expr> {
        self.ctx.require(crate::rust_generator::context::Import::HashSet);
        self.ctx.require(crate::rust_generator::context::Import::Arc);
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

    pub(super) fn convert_list_comp(
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
            if is_range {
                // Ranges yield Copy types — use |&target| pattern directly, no deref needed
                let cond_expr = if is_tuple_target {
                    cond.to_rust_expr(self.ctx)?
                } else {
                    cond.to_rust_expr(self.ctx)?
                };

                if is_identity_map {
                    Ok(parse_quote! {
                        (#iter_expr)
                            .filter(|&#target_pat| #cond_expr)
                            .collect::<Vec<_>>()
                    })
                } else {
                    Ok(parse_quote! {
                        (#iter_expr)
                            .filter(|&#target_pat| #cond_expr)
                            .map(|#target_pat| #element_expr)
                            .collect::<Vec<_>>()
                    })
                }
            } else {
            let cond_with_deref = if is_tuple_target {
                cond.to_rust_expr(self.ctx)?
            } else {
                self.ctx.filter_deref_vars.insert(target.to_string());
                let expr = cond.to_rust_expr(self.ctx)?;
                self.ctx.filter_deref_vars.remove(target);
                expr
            };

            if is_csv_reader {
                // Use .deserialize() instead of .into_iter()
                // CSV DictReader yields HashMap<String, String>
                self.ctx.require(crate::rust_generator::context::Import::Csv);
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
            } // close the non-range else block
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
                self.ctx.require(crate::rust_generator::context::Import::Csv);
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
    pub(super) fn convert_flattened_list_comp(
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
    pub(super) fn convert_list_comp_first_element(
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

        // Determine if the element type is Copy
        let element_is_copy = if let HirExpr::Var(var_name) = iter {
            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                match var_type {
                    crate::hir::Type::List(elem_type)
                    | crate::hir::Type::Set(elem_type) => !self.type_needs_clone(elem_type),
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        };

        // Check if element is just the target variable (identity mapping)
        let is_identity_map = Self::is_identity_element(element, target);

        if element_is_copy {
            // Copy types: .iter().copied() yields T, find(|&x|) destructures &T→T
            let cond = condition.to_rust_expr(self.ctx)?;
            if is_identity_map {
                Ok(parse_quote! {
                    #iter_expr
                        .iter()
                        .copied()
                        .find(|&#target_ident| #cond)
                        .unwrap()
                })
            } else {
                let element_expr = element.to_rust_expr(self.ctx)?;
                Ok(parse_quote! {
                    #iter_expr
                        .iter()
                        .copied()
                        .find(|&#target_ident| #cond)
                        .map(|#target_ident| #element_expr)
                        .unwrap()
                })
            }
        } else {
            self.ctx.filter_deref_vars.insert(target.to_string());
            let cond_with_deref = condition.to_rust_expr(self.ctx)?;
            self.ctx.filter_deref_vars.remove(target);

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
    }

    /// Add dereference (*) to uses of target variable in expression
    /// This is needed because filter closures receive &T even when the iterator yields T
    /// Example: transforms `x > 0` to `*x > 0` when x is the target variable
    pub(super) fn add_deref_to_var_uses(&mut self, expr: &HirExpr, target: &str) -> Result<syn::Expr> {
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

    pub(super) fn convert_set_operation(
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

    pub(super) fn convert_set_comp(
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

        self.ctx.require(crate::rust_generator::context::Import::HashSet);
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

    pub(super) fn convert_dict_comp(
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

        self.ctx.require(crate::rust_generator::context::Import::HashMap);
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

}
