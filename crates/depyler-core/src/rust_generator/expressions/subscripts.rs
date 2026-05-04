//! Expression code generation - subscripts

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
    pub(super) fn convert_index(&mut self, base: &HirExpr, index: &HirExpr) -> Result<syn::Expr> {
        // Set needs_indexerror flag since indexing operations use .unwrap()
        // which could panic with an index error. This generates the IndexError
        // struct definition as a safety marker.
        self.ctx.require(crate::rust_generator::context::Import::IndexError);

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

        // Use .copied() for Copy element types, .cloned() otherwise
        let elem_is_copy = self.ctx.get_expr_type(base).is_some_and(|base_type| {
            match &base_type {
                Type::List(elem) | Type::Set(elem) => !self.type_needs_clone(elem),
                Type::Dict(_, val) => !self.type_needs_clone(val),
                _ => false,
            }
        });
        let clone_method = if elem_is_copy {
            quote::format_ident!("copied")
        } else {
            quote::format_ident!("cloned")
        };

        
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
                            #base_expr.get(#s).#clone_method().unwrap()
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
                            #base_expr.get(&#index_expr).#clone_method().unwrap()
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
                            return Ok(parse_quote! { #base_expr.last().#clone_method().unwrap() });
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
                            #base_expr.get(#base_expr.len().saturating_sub(#offset)).#clone_method().unwrap()
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
                        #base_expr.get(#idx_value).#clone_method().unwrap()
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
                        #base_expr.get(#index_expr as usize).#clone_method().unwrap()
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
                            base.get(actual_idx).#clone_method().unwrap()
                        }
                    })
                }
            }
        }
    }

    /// Check if the index expression is a string key (for HashMap access)
    /// Returns true if: index is string literal, OR base is Dict/HashMap type
    pub(super) fn is_string_index(&self, base: &HirExpr, index: &HirExpr) -> Result<bool> {
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

    pub(super) fn convert_slice(
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
    pub(super) fn convert_string_slice(
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

    pub(super) fn convert_attribute(&mut self, value: &HirExpr, attr: &str) -> Result<syn::Expr> {
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
        // Prefer var_types (resolved type) over optional_vars: if var_types has a known
        // non-Optional type, trust it even if optional_vars contains the variable (stale entry).
        if let HirExpr::Var(var_name) = value {
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
        // Skip clone when generate_borrow is set - we'll add a reference in statements instead
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

}
