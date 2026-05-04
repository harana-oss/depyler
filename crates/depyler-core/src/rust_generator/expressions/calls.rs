//! Expression code generation - calls

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
    /// Convert functools.partial() to Rust closure
    /// partial(func, arg1, arg2, kwarg1=val1) → move |remaining...| func(arg1, arg2, remaining..., kwarg1=val1)
    pub(super) fn convert_partial_call(
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
    pub(super) fn infer_partial_remaining_params(&self, target_func: &HirExpr, bound_count: usize) -> usize {
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

    pub(super) fn convert_call(
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
            return crate::rust_generator::builtins::handle_max(args, self.ctx);
        }

        // Handle min() builtin
        if func == "min" {
            return crate::rust_generator::builtins::handle_min(args, self.ctx);
        }

        if func == "abs" {
            return crate::rust_generator::builtins::handle_abs(args, self.ctx);
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
            return crate::rust_generator::builtins::handle_round(args, self.ctx);
        }

        // Handle pow() builtin
        if func == "pow" {
            return crate::rust_generator::builtins::handle_pow(args, self.ctx);
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
            self.ctx.require(crate::rust_generator::context::Import::RustDecimal);
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
            self.ctx.require(crate::rust_generator::context::Import::NumRational);

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
            self.ctx.require(crate::rust_generator::context::Import::Chrono);

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
            self.ctx.require(crate::rust_generator::context::Import::Chrono);
            let year = args[0].to_rust_expr(self.ctx)?;
            let month = args[1].to_rust_expr(self.ctx)?;
            let day = args[2].to_rust_expr(self.ctx)?;
            return Ok(parse_quote! {
                chrono::NaiveDate::from_ymd_opt(#year as i32, #month as u32, #day as u32).unwrap()
            });
        }

        // time(hour, minute, second) → NaiveTime::from_hms_opt(h, m, s).unwrap()
        if func == "time" && args.len() >= 2 {
            self.ctx.require(crate::rust_generator::context::Import::Chrono);
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
            self.ctx.require(crate::rust_generator::context::Import::Chrono);

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

    pub(super) fn try_convert_map_with_zip(&mut self, args: &[HirExpr]) -> Result<Option<syn::Expr>> {
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

    pub(super) fn convert_len_call(&mut self, hir_args: &[HirExpr], _args: &[syn::Expr]) -> Result<syn::Expr> {
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
    pub(super) fn needs_parens_for_cast(expr: &syn::Expr) -> bool {
        !matches!(expr, syn::Expr::Path(_) | syn::Expr::Lit(_) | syn::Expr::Field(_) | syn::Expr::MethodCall(_) | syn::Expr::Call(_) | syn::Expr::Paren(_))
    }

    pub(super) fn convert_int_cast(&self, hir_args: &[HirExpr], arg_exprs: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_float_cast(&self, hir_args: &[HirExpr], args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("float() requires exactly one argument");
        }
        let arg = &args[0];

        // If the expression is already a float type, don't cast - just return it.
        // This avoids invalid casts like `(&mut f64) as f64`.
        if !hir_args.is_empty() && self.ctx.expr_has_type(&hir_args[0], &Type::Float) {
            return Ok(arg.clone());
        }

        if Self::needs_parens_for_cast(arg) {
            return Ok(parse_quote! { (#arg) as f64 });
        }
        Ok(parse_quote! { #arg as f64 })
    }

    pub(super) fn convert_str_conversion(
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

    pub(super) fn convert_bool_cast(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("bool() requires exactly one argument");
        }
        let arg = &args[0];
        // In Python, bool(x) checks truthiness
        // In Rust, we cast to bool or use appropriate conversion
        Ok(parse_quote! { (#arg) as bool })
    }

    pub(super) fn convert_range_call(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_range_with_step(
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

    pub(super) fn convert_range_negative_step(
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

    pub(super) fn convert_range_positive_step(
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

    pub(super) fn convert_array_init_call(
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

    pub(super) fn convert_array_small_literal(
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

    pub(super) fn convert_array_large_literal(&mut self, func: &str, args: &[HirExpr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_array_dynamic_size(&mut self, func: &str, args: &[HirExpr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_set_constructor(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.require(crate::rust_generator::context::Import::HashSet);
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

    pub(super) fn convert_frozenset_constructor(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.require(crate::rust_generator::context::Import::HashSet);
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

    pub(super) fn convert_counter_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.require(crate::rust_generator::context::Import::HashMap);
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

    pub(super) fn convert_dict_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.require(crate::rust_generator::context::Import::HashMap);
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

    pub(super) fn convert_deque_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
        self.ctx.require(crate::rust_generator::context::Import::VecDeque);
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

    pub(super) fn convert_list_builtin(&mut self, args: &[syn::Expr]) -> Result<syn::Expr> {
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
                self.ctx.require(crate::rust_generator::context::Import::Csv);
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

    pub(super) fn convert_all_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("all() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter().all(|x| x) })
    }

    pub(super) fn convert_any_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("any() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter().any(|x| x) })
    }

    pub(super) fn convert_divmod_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 2 {
            bail!("divmod() requires exactly 2 arguments");
        }
        let a = &args[0];
        let b = &args[1];
        Ok(parse_quote! { (#a / #b, #a % #b) })
    }

    pub(super) fn convert_enumerate_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_zip_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_reversed_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("reversed() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter().rev() })
    }

    pub(super) fn convert_sorted_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_filter_builtin(
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

    pub(super) fn convert_sum_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_round_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.is_empty() || args.len() > 2 {
            bail!("round() requires 1 or 2 arguments");
        }
        let value = &args[0];
        // Simplified: ignore ndigits parameter
        Ok(parse_quote! { (#value as f64).round() as i32 })
    }

    pub(super) fn convert_abs_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("abs() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { (#value).abs() })
    }

    pub(super) fn convert_min_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_max_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_pow_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() < 2 || args.len() > 3 {
            bail!("pow() requires 2 or 3 arguments");
        }
        let base = &args[0];
        let exp = &args[1];
        // Simplified: ignore modulo parameter
        Ok(parse_quote! { (#base as f64).powf(#exp as f64) as i32 })
    }

    pub(super) fn convert_hex_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("hex() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("0x{:x}", #value) })
    }

    pub(super) fn convert_bin_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("bin() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("0b{:b}", #value) })
    }

    pub(super) fn convert_oct_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("oct() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("0o{:o}", #value) })
    }

    pub(super) fn convert_chr_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("chr() requires exactly 1 argument");
        }
        let code = &args[0];
        Ok(parse_quote! {
            char::from_u32(#code as u32).unwrap().to_string()
        })
    }

    pub(super) fn convert_ord_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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
    pub(super) fn convert_open_builtin(&self, hir_args: &[HirExpr], args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_hash_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
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

    pub(super) fn convert_repr_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("repr() requires exactly 1 argument");
        }
        let value = &args[0];
        Ok(parse_quote! { format!("{:?}", #value) })
    }

    /// Convert next() builtin to Rust.
    /// Optimizes `next((x for x in items if cond), None)` to `items.iter().find(|x| cond).cloned()`
    /// Optimizes `next((i for i, x in enumerate(items) if cond), default)` to `items.iter().position(|x| cond).map(|i| i as i32).unwrap_or(default)`
    pub(super) fn convert_next_builtin(
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

                if !generator.conditions.is_empty() {
                    let iter_expr = if matches!(&*generator.iter, HirExpr::Attribute { .. }) {
                        self.convert_attribute_without_clone(&generator.iter)?
                    } else {
                        generator.iter.to_rust_expr(self.ctx)?
                    };
                    let target_pat = self.parse_target_pattern(&generator.target)?;

                    // Determine if element type is Copy (doesn't need clone)
                    let element_is_copy = if let HirExpr::Var(var_name) = &*generator.iter {
                        if let Some(var_type) = self.ctx.var_types.get(var_name) {
                            match var_type {
                                crate::hir::Type::List(elem_type)
                                | crate::hir::Type::Set(elem_type) => {
                                    !self.type_needs_clone(elem_type)
                                }
                                _ => false,
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // Build combined condition.
                    // For Copy types, use |&target| pattern so no * dereference is needed.
                    // For non-Copy types, use filter_deref_vars to add * in the body.
                    let is_tuple_target = generator.target.starts_with('(');
                    if !is_tuple_target && !element_is_copy {
                        self.ctx.filter_deref_vars.insert(generator.target.clone());
                    }
                    let conditions: Vec<syn::Expr> = generator
                        .conditions
                        .iter()
                        .map(|c| c.to_rust_expr(self.ctx))
                        .collect::<Result<Vec<_>>>()?;
                    if !is_tuple_target && !element_is_copy {
                        self.ctx.filter_deref_vars.remove(&generator.target);
                    }

                    let combined_condition: syn::Expr = if conditions.len() == 1 {
                        conditions.into_iter().next().unwrap()
                    } else {
                        conditions
                            .into_iter()
                            .reduce(|acc, c| parse_quote! { #acc && #c })
                            .unwrap()
                    };

                    let is_borrowed_param = if let HirExpr::Var(var_name) = &*generator.iter {
                        self.ctx.current_func_ref_params.contains(var_name)
                            || self.ctx.current_func_mut_ref_params.contains(var_name)
                            || self.ctx.borrowable_vars.contains(var_name)
                            || self.ctx.mut_borrowable_vars.contains(var_name)
                    } else {
                        matches!(&*generator.iter, HirExpr::Attribute { .. })
                    };

                    if is_identity {
                        // Identity pattern: next((x for x in items if cond), default)
                        // → items.iter().find(|x| cond).cloned()
                        let find_expr: syn::Expr = if element_is_copy {
                            // Copy types: .iter().copied() yields T, find(|&x|) destructures &T→T
                            parse_quote! { #iter_expr.iter().copied().find(|&#target_pat| #combined_condition) }
                        } else {
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

                            if needs_cloned {
                                parse_quote! { #iter_expr.iter().find(|#target_pat| #combined_condition).cloned() }
                            } else {
                                parse_quote! { #iter_expr.iter().find(|#target_pat| #combined_condition).copied() }
                            }
                        };

                        return self.wrap_find_with_default(find_expr, hir_args, args);
                    } else {
                        // Non-identity pattern: next((f(x) for x in items if cond), default)
                        // → items.into_iter().find(|x| cond).map(|x| f(x))
                        let element_expr = element.to_rust_expr(self.ctx)?;

                        let find_expr: syn::Expr = if element_is_copy {
                            if is_borrowed_param {
                                parse_quote! { #iter_expr.iter().copied().find(|&#target_pat| #combined_condition) }
                            } else {
                                parse_quote! { #iter_expr.into_iter().find(|&#target_pat| #combined_condition) }
                            }
                        } else if is_borrowed_param {
                            parse_quote! { #iter_expr.iter().find(|#target_pat| #combined_condition) }
                        } else {
                            parse_quote! { #iter_expr.into_iter().find(|#target_pat| #combined_condition) }
                        };

                        let find_map_expr: syn::Expr =
                            parse_quote! { #find_expr.map(|#target_pat| #element_expr) };

                        return self.wrap_find_with_default(find_map_expr, hir_args, args);
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

    pub(super) fn wrap_find_with_default(
        &self,
        find_expr: syn::Expr,
        hir_args: &[HirExpr],
        args: &[syn::Expr],
    ) -> Result<syn::Expr> {
        if hir_args.len() == 2 {
            if matches!(&hir_args[1], HirExpr::Literal(crate::hir::Literal::None)) {
                Ok(find_expr)
            } else {
                let default = args[1].clone();
                Ok(parse_quote! { #find_expr.unwrap_or(#default) })
            }
        } else {
            Ok(parse_quote! { #find_expr.expect("StopIteration: iterator is empty") })
        }
    }

    /// Optimize `next((i for i, x in enumerate(items) if cond), default)` to
    /// `items.iter().position(|x| cond).map(|i| i as i32).unwrap_or(default)`
    pub(super) fn try_optimize_enumerate_position(
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
        // .position() receives the item type directly from the iterator.
        // For Copy types, use |&elem| pattern to destructure the reference.
        let elem_ident = syn::Ident::new(&elem_var, proc_macro2::Span::call_site());
        let element_is_copy = !element_needs_clone;

        if !element_is_copy {
            self.ctx.filter_deref_vars.insert(elem_var.clone());
        }
        let conditions: Vec<syn::Expr> = comprehension
            .conditions
            .iter()
            .map(|c| c.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;
        if !element_is_copy {
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
        let position_expr: syn::Expr = if element_is_copy {
            parse_quote! {
                #collection_expr.iter().position(|&#elem_ident| #combined_condition).map(|i| i as i32)
            }
        } else {
            parse_quote! {
                #collection_expr.iter().position(|#elem_ident| #combined_condition).map(|i| i as i32)
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

    //
    /// getattr(obj, name) → obj.name
    /// getattr(obj, name, default) → obj.name (default is ignored in static Rust)
    ///
    /// Python's getattr() retrieves an attribute on an object dynamically. In Rust, we generate
    /// a direct field access when the attribute name is a string literal, or HashMap access
    /// when the attribute name is an f-string (dynamic attribute access).
    pub(super) fn convert_getattr_builtin(&mut self, hir_args: &[HirExpr]) -> Result<syn::Expr> {
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
    pub(super) fn convert_setattr_builtin(&mut self, hir_args: &[HirExpr]) -> Result<syn::Expr> {
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
    pub(super) fn convert_iter_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("iter() requires exactly 1 argument");
        }
        let iterable = &args[0];
        Ok(parse_quote! { #iterable.into_iter() })
    }

    //
    pub(super) fn convert_type_builtin(&self, args: &[syn::Expr]) -> Result<syn::Expr> {
        if args.len() != 1 {
            bail!("type() requires exactly 1 argument");
        }
        let value = &args[0];
        // Return a string representation of the type name
        // This is a simplified implementation - full Python type() is more complex
        Ok(parse_quote! { std::any::type_name_of_val(&#value) })
    }

    /// Check if expression already ends with .collect()
    pub(super) fn already_collected(&self, expr: &syn::Expr) -> bool {
        if let syn::Expr::MethodCall(method_call) = expr {
            method_call.method == "collect"
        } else {
            false
        }
    }

    /// Check if expression is a range (0..5, start..end, etc.)
    pub(super) fn is_range_expr(&self, expr: &syn::Expr) -> bool {
        matches!(expr, syn::Expr::Range(_))
    }

    /// Check if expression is an iterator-producing expression
    pub(super) fn is_iterator_expr(&self, expr: &syn::Expr) -> bool {
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
    pub(super) fn is_csv_reader_var(&self, expr: &syn::Expr) -> bool {
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

    pub(super) fn convert_generic_call(
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
                    // Exception: Copy types (enums, primitives) are implicitly copied
                    let needs_clone_for_move = if let HirExpr::Attribute { value, attr } = hir_arg {
                        if let HirExpr::Var(base_var) = &**value {
                            if self.ctx.current_func_mut_ref_params.contains(base_var) {
                                self.get_field_type(value, attr)
                                    .map(|ty| self.type_needs_clone(&ty))
                                    .unwrap_or(true)
                            } else {
                                false
                            }
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
                        if let HirExpr::Attribute { value, attr } = hir_arg {
                            // For state.field, get_base_var returns "state"
                            let has_conflict = get_base_var(hir_arg)
                                .map(|base_var| mut_borrowed_vars.contains(base_var))
                                .unwrap_or(false);
                            // Copy types (enums, primitives) don't need clone for borrow conflicts
                            let field_is_copy = self.get_field_type(value, attr)
                                .map(|ty| !self.type_needs_clone(&ty))
                                .unwrap_or(false);
                            (has_conflict && !field_is_copy, false)
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
                            // Copy types don't need clone for borrow conflicts
                            let field_is_copy = if let HirExpr::Attribute { value, attr } = &**expr {
                                self.get_field_type(value, attr)
                                    .map(|ty| !self.type_needs_clone(&ty))
                                    .unwrap_or(false)
                            } else {
                                false
                            };
                            (has_conflict && !field_is_copy, is_attr)
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
                                // Static array constants need .to_vec() when
                                // passed to functions expecting &Vec
                                if self.ctx.static_array_constants.contains(var_name) {
                                    let ident = format_ident!("{}", var_name);
                                    let result: syn::Expr =
                                        parse_quote! { &#ident.to_vec() };
                                    if needs_optional_unwrap {
                                        parse_quote! { #result.unwrap() }
                                    } else {
                                        result
                                    }
                                } else {
                                    // No conflict - simple variable borrow
                                    let ident = format_ident!("{}", var_name);
                                    let result: syn::Expr = parse_quote! { &#ident };
                                    if needs_optional_unwrap {
                                        parse_quote! { #result.unwrap() }
                                    } else {
                                        result
                                    }
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
    pub(super) fn try_convert_classmethod(
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
    pub(super) fn try_convert_struct_method(
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

}
