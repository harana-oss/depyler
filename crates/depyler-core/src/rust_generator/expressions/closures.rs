//! Expression code generation - closures

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
    pub(super) fn convert_lambda(&mut self, params: &[String], body: &HirExpr) -> Result<syn::Expr> {
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

    pub(super) fn convert_generator_expression(
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
                self.ctx.require(crate::rust_generator::context::Import::Csv);
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
            // .filter() always receives &Item.
            // For Copy types, use |&x| pattern to destructure the reference directly.
            // For non-Copy types, use |x| and deref in the body via filter_deref_vars.
            let is_tuple_target = generator.target.starts_with('(');
            let element_is_copy = !element_needs_clone;
            for cond in &generator.conditions {
                if is_tuple_target {
                    let cond_expr = cond.to_rust_expr(self.ctx)?;
                    chain = parse_quote! { #chain.filter(|#target_pat| #cond_expr) };
                } else if element_is_copy {
                    let cond_expr = cond.to_rust_expr(self.ctx)?;
                    chain = parse_quote! { #chain.filter(|&#target_pat| #cond_expr) };
                } else {
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

    pub(super) fn convert_nested_generators(
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

    pub(super) fn build_nested_chain(
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

}
