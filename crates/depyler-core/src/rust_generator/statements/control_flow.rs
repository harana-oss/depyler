//! Control Flow code generation

use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_generator::keywords::safe_ident;
use crate::rust_generator::type_gen::rust_type_to_syn;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};

use super::*;
pub(crate) fn codegen_if_stmt(
    condition: &HirExpr,
    then_body: &[HirStmt],
    else_body: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use std::collections::HashSet;

    if ctx.argparser_tracker.has_subcommands() {
        if let Some(match_stmt) =
            try_generate_subcommand_match(condition, then_body, else_body, ctx)?
        {
            return Ok(match_stmt);
        }
    }

    // Check for `if var is not None:` pattern - use `if let Some(var) = var` for type narrowing
    if let Some((var_name, is_not_none)) = extract_none_check(condition) {
        let is_known_optional = ctx.optional_vars.contains(&var_name)
            || matches!(ctx.var_types.get(&var_name), Some(Type::Optional(_)))
            || matches!(condition, HirExpr::MethodCall { method, .. } if method == "is_some");
        if is_not_none && is_known_optional {
            return codegen_if_let_some(var_name, then_body, else_body, ctx);
        }
    }

    // Extract walrus operator (NamedExpr) assignments from condition
    let (walrus_assignments, modified_condition) = extract_walrus_assignments(condition);

    // Generate assignment statements for walrus operators before the if
    let mut walrus_stmts = Vec::new();
    for (var_name, value_expr) in &walrus_assignments {
        let var_ident = safe_ident(var_name);
        let value_tokens = value_expr.to_rust_expr(ctx)?;
        walrus_stmts.push(quote! { let #var_ident = #value_tokens; });
        // Mark the variable as declared so it's accessible in the if body
        ctx.declare_var(var_name);
    }

    let mut cond = modified_condition.to_rust_expr(ctx)?;

    // Convert non-boolean expressions to boolean (e.g., `if val` where val: String)
    cond = apply_truthiness_conversion(condition, cond, ctx);

    let hoisted_vars: HashSet<String> = if let Some(else_stmts) = else_body {
        let then_vars = extract_assigned_symbols(then_body);
        let else_vars = extract_assigned_symbols(else_stmts);
        then_vars.intersection(&else_vars).cloned().collect()
    } else {
        HashSet::new()
    };

    let mut hoisted_decls = Vec::new();
    for var_name in &hoisted_vars {
        if ctx.is_declared(var_name) {
            continue;
        }

        // Find the variable's type from the first assignment in either branch
        let var_type = find_variable_type(var_name, then_body).or_else(|| {
            if let Some(else_stmts) = else_body {
                find_variable_type(var_name, else_stmts)
            } else {
                None
            }
        });

        let var_ident = safe_ident(var_name);
        let needs_mut = ctx.mutable_vars.contains(var_name);

        if let Some(ty) = var_type {
            let rust_type = ctx.type_mapper.map_type(&ty);
            let syn_type = rust_type_to_syn(&rust_type)?;
            if needs_mut {
                hoisted_decls.push(quote! { let mut #var_ident: #syn_type; });
            } else {
                hoisted_decls.push(quote! { let #var_ident: #syn_type; });
            }
        } else {
            // No type annotation - use type inference placeholder
            // Rust will infer the type from the assignments in the branches
            if needs_mut {
                hoisted_decls.push(quote! { let mut #var_ident; });
            } else {
                hoisted_decls.push(quote! { let #var_ident; });
            }
        }

        // Mark variable as declared so assignments use `var = value` not `let var = value`
        ctx.declare_var(var_name);
    }

    ctx.enter_scope();
    let then_stmts: Vec<_> = then_body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    if let Some(else_stmts) = else_body {
        ctx.enter_scope();
        let else_tokens: Vec<_> = else_stmts
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;
        ctx.exit_scope();
        Ok(quote! {
            #(#walrus_stmts)*
            #(#hoisted_decls)*
            if #cond {
                #(#then_stmts)*
            } else {
                #(#else_tokens)*
            }
        })
    } else {
        Ok(quote! {
            #(#walrus_stmts)*
            if #cond {
                #(#then_stmts)*
            }
        })
    }
}

pub(crate) fn codegen_while_stmt(
    condition: &HirExpr,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Extract walrus operators from the condition
    let (walrus_assignments, modified_condition) = extract_walrus_assignments(condition);

    // Return statements inside loops must use explicit `return` keyword
    let saved_is_final = ctx.is_final_statement;
    ctx.is_final_statement = false;

    ctx.enter_scope();

    // If there are walrus operators, we need to use loop {} with assignments at the start
    if !walrus_assignments.is_empty() {
        // Check which walrus variables are mutated in the loop body
        let mut mutated_walrus_vars = std::collections::HashSet::new();
        for (var_name, _) in &walrus_assignments {
            // Check if this variable is reassigned in the loop body
            if is_var_mutated_in_stmts(var_name, body) {
                mutated_walrus_vars.insert(var_name.clone());
            }
        }

        // Generate assignment statements for walrus operators
        let mut assignment_stmts = Vec::new();
        for (var_name, value_expr) in &walrus_assignments {
            let var_ident = quote::format_ident!("{}", var_name);
            let value_tokens = value_expr.to_rust_expr(ctx)?;

            // Mark variable as declared so it can be used in condition and body
            ctx.declare_var(var_name);

            // Mark as mutable if it's mutated in the loop body
            if mutated_walrus_vars.contains(var_name) {
                ctx.mutable_vars.insert(var_name.clone());
                assignment_stmts.push(quote! { let mut #var_ident = #value_tokens; });
            } else {
                assignment_stmts.push(quote! { let #var_ident = #value_tokens; });
            }
        }

        // Generate the condition check (with walrus operators replaced by variable references)
        let mut cond = modified_condition.to_rust_expr(ctx)?;
        cond = apply_truthiness_conversion(&modified_condition, cond, ctx);

        // Generate body statements
        let body_stmts: Vec<_> = body
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;

        ctx.exit_scope();
        ctx.is_final_statement = saved_is_final;

        // Use loop {} pattern with assignments at start and conditional break
        Ok(quote! {
            loop {
                #(#assignment_stmts)*
                if !(#cond) {
                    break;
                }
                #(#body_stmts)*
            }
        })
    } else {
        // No walrus operators - use standard while loop
        let mut cond = condition.to_rust_expr(ctx)?;
        cond = apply_truthiness_conversion(condition, cond, ctx);

        let body_stmts: Vec<_> = body
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;

        ctx.exit_scope();
        ctx.is_final_statement = saved_is_final;

        Ok(quote! {
            while #cond {
                #(#body_stmts)*
            }
        })
    }
}

pub(crate) fn codegen_for_stmt(
    target: &AssignTarget,
    iter: &HirExpr,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // If unused, prefix with _ to avoid unused variable warnings with -D warnings

    // Check if loop variable is mutated to determine if we need `mut` keyword
    let needs_mut_pattern = does_loop_body_mutate_items(target, body, &ctx.function_param_borrows);

    // Generate target pattern based on AssignTarget type
    let target_pattern: syn::Pat = match target {
        AssignTarget::Symbol(name) => {
            // Check if this variable is used in the loop body
            let is_used = body.iter().any(|stmt| is_var_used_in_stmt(name, stmt));

            // If unused, prefix with underscore
            let var_name = if is_used {
                name.clone()
            } else {
                format!("_{}", name)
            };

            let ident = safe_ident(&var_name);
            if needs_mut_pattern {
                parse_quote! { mut #ident }
            } else {
                parse_quote! { #ident }
            }
        }
        AssignTarget::Tuple(targets) => {
            // For tuple unpacking, recursively build pattern for nested tuples
            fn build_for_loop_pattern(
                targets: &[AssignTarget],
                body: &[HirStmt],
                needs_mut_pattern: bool,
            ) -> syn::Pat {
                let idents: Vec<syn::Pat> = targets
                    .iter()
                    .map(|t| match t {
                        AssignTarget::Symbol(s) => {
                            // Check if this specific element is used
                            let is_used = body.iter().any(|stmt| is_var_used_in_stmt(s, stmt));
                            let var_name = if is_used {
                                s.clone()
                            } else {
                                format!("_{}", s)
                            };
                            let ident = safe_ident(&var_name);
                            if needs_mut_pattern {
                                parse_quote! { mut #ident }
                            } else {
                                parse_quote! { #ident }
                            }
                        }
                        AssignTarget::Tuple(nested) => {
                            // Recursively build nested tuple pattern
                            build_for_loop_pattern(nested, body, needs_mut_pattern)
                        }
                        _ => {
                            // For index/attribute/slice in for loop, we'd need more complex handling
                            panic!("Complex assignment targets (index/attribute/slice) not supported in for loop unpacking")
                        }
                    })
                    .collect();
                parse_quote! { (#(#idents),*) }
            }

            build_for_loop_pattern(targets, body, needs_mut_pattern)
        }
        _ => bail!("Unsupported for loop target type"),
    };

    // When iterating over field accesses (e.g., state.items), we MUST use borrows
    // because Rust doesn't allow moving out of struct fields.
    // Determine whether to use & or &mut based on loop body mutations.
    let (needs_field_borrow, is_special_call) = if let Some((_root_var, is_field)) =
        is_field_access_iter(iter)
    {
        if is_field {
            // This is a field access - we need borrowing
            // Determine if we need mutable or immutable borrow
            let needs_mut_borrow =
                does_loop_body_mutate_items(target, body, &ctx.function_param_borrows);

            // Check if this is a special function call (enumerate, reversed)
            let is_special = matches!(iter, HirExpr::Call { func, .. } if func == "enumerate" || func == "reversed");

            (Some(needs_mut_borrow), is_special)
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    // Convert tuple to array for iteration (tuples aren't directly iterable in Rust)
    // For field accesses that will be borrowed, generate without .clone()
    let mut iter_expr = if let HirExpr::Tuple(elts) = iter {
        // Track loop variable as &str if iterating over string literals
        // This is needed to add .to_string() when passing to functions expecting String
        let is_string_tuple = elts
            .iter()
            .all(|e| matches!(e, HirExpr::Literal(crate::hir::Literal::String(_))));
        if is_string_tuple {
            if let AssignTarget::Symbol(var_name) = target {
                ctx.tuple_iter_vars.insert(var_name.clone());
            }
        }
        let elt_exprs: Vec<syn::Expr> = elts
            .iter()
            .map(|e| e.to_rust_expr(ctx))
            .collect::<Result<Vec<_>>>()?;
        parse_quote! { [#(#elt_exprs),*] }
    } else if needs_field_borrow.is_some() {
        // For field accesses being iterated, generate without .clone() since we'll borrow
        generate_field_access_without_clone(iter, ctx)?
    } else {
        // For simple variables that will get .iter().cloned(), prevent initial clone
        // to avoid redundant `var.clone().iter().cloned()` pattern
        let saved_prevent_clone = ctx.prevent_clone;
        if matches!(iter, HirExpr::Var(_)) {
            ctx.prevent_clone = true;
        }
        let expr = iter.to_rust_expr(ctx)?;
        ctx.prevent_clone = saved_prevent_clone;
        expr
    };

    // Check if the iterator is an Optional type (e.g., Optional[List[int]])
    // If so, unwrap it before iterating
    let iter_is_optional = expr_is_optional(iter, ctx);
    if iter_is_optional {
        iter_expr = parse_quote! { #iter_expr.as_ref().unwrap() };
    }

    // Python: for line in sys.stdin:
    // Rust: for line in std::io::stdin().lock().lines()
    let is_stdin_iter = matches!(iter, HirExpr::Attribute { value, attr }
        if matches!(&**value, HirExpr::Var(m) if m == "sys") && attr == "stdin");

    // Python: for line in f: (where f = open(...))
    // Rust: use BufReader for efficient line-by-line reading
    // Check if this variable might be a File object
    // Heuristic: variables named 'f', 'file', 'input', 'output', or ending in '_file'
    let is_file_iter = if let HirExpr::Var(var_name) = iter {
        var_name == "f"
            || var_name == "file"
            || var_name == "input"
            || var_name == "output"
            || var_name.ends_with("_file")
            || var_name.starts_with("file_")
    } else {
        false
    };

    if is_stdin_iter {
        // Wrap stdin with .lines() to get line iterator
        // Stdin::lines() method provides buffered line-by-line reading
        // Returns Iterator<Item = Result<String, io::Error>>
        // We map to unwrap_or_default() to handle errors gracefully
        iter_expr = parse_quote! { #iter_expr.lines().map(|l| l.unwrap_or_default()) };
    } else if is_file_iter {
        // This is the idiomatic Rust way to iterate over file lines
        // Method call syntax (.lines()) is preferred over trait syntax (BufRead::lines())
        iter_expr = parse_quote! {
            std::io::BufReader::new(#iter_expr).lines()
                .map(|l| l.unwrap_or_default())
        };
    }

    // Check if variable name suggests CSV reader (heuristic-based)
    let is_csv_reader = if let HirExpr::Var(var_name) = iter {
        var_name == "reader"
            || var_name.contains("csv")
            || var_name.ends_with("_reader")
            || var_name.starts_with("reader_")
    } else {
        false
    };

    // Track if CSV pattern yields Results (need to unwrap in loop)
    let mut csv_yields_results = false;

    if !is_stdin_iter && !is_file_iter && is_csv_reader {
        // Try to apply CSV iteration mapping from stdlib_mappings
        // This transforms: for row in reader
        // Into: for result in reader.deserialize::<HashMap<String, String>>()
        if let Some(pattern) = ctx
            .stdlib_mappings
            .get_iteration_pattern("csv", "DictReader")
        {
            // Check if pattern yields Results
            if let crate::mappings::stdlib_mappings::RustPattern::IterationPattern {
                yields_results, ..
            } = pattern
            {
                csv_yields_results = *yields_results;
            }

            let rust_code =
                pattern.generate_rust_code(&iter_expr.to_token_stream().to_string(), &[]);
            if let Ok(expr) = syn::parse_str::<syn::Expr>(&rust_code) {
                // Set needs_csv flag
                ctx.require(crate::rust_generator::context::Import::Csv);
                // Wrap in iteration that handles Results
                iter_expr = expr;
            }
        }
    }

    // If we determined that a borrow is needed (field access on a parameter),
    // wrap the iterator expression with & or &mut, OR use .iter()/.iter_mut() for special calls
    // Skip this if we already unwrapped an Optional, since .as_ref().unwrap() already returns a reference
    if let Some(needs_mut) = needs_field_borrow {
        if !iter_is_optional {
            if is_special_call {
                // For enumerate/reversed, we need to regenerate using .iter()/.iter_mut()
                // Extract the field access expression
                match iter {
                    HirExpr::Call { func, args, .. } if func == "enumerate" && !args.is_empty() => {
                        // Get the field access from inside enumerate - without clone
                        let field_expr = generate_field_access_without_clone(&args[0], ctx)?;
                        if needs_mut {
                            iter_expr = parse_quote! { #field_expr.iter_mut().enumerate() };
                        } else {
                            iter_expr = parse_quote! { #field_expr.iter().enumerate() };
                        }
                    }
                    HirExpr::Call { func, args, .. } if func == "reversed" && !args.is_empty() => {
                        // Get the field access from inside reversed - without clone
                        let field_expr = generate_field_access_without_clone(&args[0], ctx)?;
                        if needs_mut {
                            iter_expr = parse_quote! { #field_expr.iter_mut().rev() };
                        } else {
                            iter_expr = parse_quote! { #field_expr.iter().rev() };
                        }
                    }
                    _ => {}
                }
            } else if let HirExpr::Slice {
                base,
                start,
                stop,
                step,
            } = iter
            {
                // For conditional field access (e.g., (ternary).field[:N]), split at the IfExpr
                // and bind it to a local so field access goes through a place expression.
                if step.is_none() && expr_contains_ifexpr(base) {
                    if let Some(result) =
                        generate_ifexpr_slice_iter(base, start, stop, needs_mut, ctx)?
                    {
                        iter_expr = result;
                    }
                } else {
                    // For sliced field access (e.g., state.items[:N]), generate iter_mut/iter on the slice
                    let base_expr = if needs_mut {
                        generate_field_access_mut_ref(base, ctx)?
                    } else {
                        generate_field_access_without_clone(base, ctx)?
                    };
                    let iter_method = if needs_mut {
                        quote::quote! { iter_mut }
                    } else {
                        quote::quote! { iter }
                    };
                    match (start, stop, step) {
                        (None, Some(stop_val), None) => {
                            let stop_expr = stop_val.to_rust_expr(ctx)?;
                            iter_expr = parse_quote! {{
                                let base = &mut #base_expr;
                                let stop = (#stop_expr).max(0) as usize;
                                base[..stop.min(base.len())].#iter_method()
                            }};
                        }
                        (Some(start_val), Some(stop_val), None) => {
                            let start_expr = start_val.to_rust_expr(ctx)?;
                            let stop_expr = stop_val.to_rust_expr(ctx)?;
                            iter_expr = parse_quote! {{
                                let base = &mut #base_expr;
                                let start = (#start_expr).max(0) as usize;
                                let stop = (#stop_expr).max(0) as usize;
                                if start < base.len() { base[start..stop.min(base.len())].#iter_method() } else { [].#iter_method() }
                            }};
                        }
                        (Some(start_val), None, None) => {
                            let start_expr = start_val.to_rust_expr(ctx)?;
                            iter_expr = parse_quote! {{
                                let base = &mut #base_expr;
                                let start = (#start_expr).max(0) as usize;
                                if start < base.len() { base[start..].#iter_method() } else { [].#iter_method() }
                            }};
                        }
                        (None, None, None) => {
                            iter_expr = parse_quote! { #base_expr.#iter_method() };
                        }
                        _ => {
                            // For complex step-based slices, fall back to wrapping
                            if needs_mut {
                                iter_expr = parse_quote! { &mut #iter_expr };
                            } else {
                                iter_expr = parse_quote! { &#iter_expr };
                            }
                        }
                    }
                }
            } else {
                // For plain field access, just wrap with & or &mut
                if needs_mut {
                    iter_expr = parse_quote! { &mut #iter_expr };
                } else {
                    iter_expr = parse_quote! { &#iter_expr };
                }
            }
        }
    }

    // Check if we're iterating over a borrowed collection
    // If iter is a simple variable that refers to a borrowed collection (e.g., &Vec<T>),
    // we need to add .iter() to properly iterate over it
    // Skip this for stdin/file/csv iterators which are already properly wrapped
    // Also skip for field access iterators that we just added borrows to
    if !is_stdin_iter
        && !is_file_iter
        && !is_csv_reader
        && needs_field_borrow.is_none()
        && !is_special_call
    {
        if let HirExpr::Var(var_name) = iter {
            // This is more reliable than name heuristics
            let is_string_type = ctx
                .var_types
                .get(var_name)
                .is_some_and(|t| matches!(t, Type::String));

            // Strings use .chars() instead of .iter().cloned()
            let is_string_name = {
                let n = var_name.as_str();
                // Exact matches (singular forms only)
                (n == "s" || n == "string" || n == "text" || n == "word" || n == "line"
                || n == "char" || n == "character")
            // Prefixes (but not if followed by 's' for plural)
            || (n.starts_with("str") && !n.starts_with("strings"))
            || (n.starts_with("word") && !n.starts_with("words"))
            || (n.starts_with("text") && !n.starts_with("texts"))
            // Suffixes (but exclude plurals)
            || (n.ends_with("_str") && !n.ends_with("_strs"))
            || (n.ends_with("_string") && !n.ends_with("_strings"))
            || (n.ends_with("_word") && !n.ends_with("_words"))
            || (n.ends_with("_text") && !n.ends_with("_texts"))
            };

            if is_string_type || is_string_name {
                // For strings, use .chars() to iterate over characters
                iter_expr = parse_quote! { #iter_expr.chars() };
            } else {
                // Determine element type to choose .copied() (Copy) vs .cloned() (Clone)
                let element_needs_clone = ctx
                    .var_types
                    .get(var_name)
                    .map(|t| match t {
                        Type::List(elem_t) => ctx.type_needs_clone(elem_t),
                        Type::Set(elem_t) => ctx.type_needs_clone(elem_t),
                        _ => true,
                    })
                    .unwrap_or(true);
                if element_needs_clone {
                    iter_expr = parse_quote! { #iter_expr.iter().cloned() };
                } else {
                    iter_expr = parse_quote! { #iter_expr.iter().copied() };
                }
            }
        }
    }

    // Return statements inside loops must use explicit `return` keyword
    let saved_is_final = ctx.is_final_statement;
    ctx.is_final_statement = false;

    ctx.enter_scope();

    // Extract element type from iterator and add to var_types
    let element_type = match iter {
        HirExpr::Var(var_name) => {
            // Simple case: for x in items
            // Look up items type, extract element type
            ctx.var_types.get(var_name).and_then(|t| match t {
                Type::List(elem_t) => Some(*elem_t.clone()),
                Type::Set(elem_t) => Some(*elem_t.clone()),
                Type::Dict(key_t, _) => Some(*key_t.clone()), // dict iteration yields keys
                _ => None,
            })
        }
        HirExpr::Call { func, args, .. } if func == "enumerate" => {
            // enumerate(items) yields (int, elem_type)
            if let Some(HirExpr::Var(var_name)) = args.first() {
                ctx.var_types.get(var_name).and_then(|t| match t {
                    Type::List(elem_t) => Some(Type::Tuple(vec![Type::Int, *elem_t.clone()])),
                    Type::Set(elem_t) => Some(Type::Tuple(vec![Type::Int, *elem_t.clone()])),
                    _ => None,
                })
            } else {
                None
            }
        }
        HirExpr::Attribute { value, attr } => {
            // Field access: for x in obj.field — resolve field type
            ctx.get_attribute_field_type(value, attr).and_then(|t| match t {
                Type::List(elem_t) => Some(*elem_t),
                Type::Set(elem_t) => Some(*elem_t),
                Type::Dict(key_t, _) => Some(*key_t),
                _ => None,
            })
        }
        _ => None,
    };

    // Declare all variables from the target pattern and set their types
    // Also track if they shadow ref params (for proper dereference handling)
    match (target, element_type) {
        (AssignTarget::Symbol(name), Some(elem_type)) => {
            ctx.declare_var(name);
            ctx.var_types.insert(name.clone(), elem_type.clone());
            // Clear stale optional status when loop variable overrides a previous Optional assignment
            if !matches!(elem_type, Type::Optional(_)) {
                ctx.optional_vars.remove(name);
            }
            // Track if this for-loop variable shadows a ref param
            if ctx.current_func_ref_params.contains(name) {
                ctx.shadowed_ref_params.insert(name.clone());
            }
        }
        (AssignTarget::Symbol(name), None) => {
            ctx.declare_var(name);
            // Clear stale optional status - loop variable is the unwrapped element
            ctx.optional_vars.remove(name);
            // Track if this for-loop variable shadows a ref param
            if ctx.current_func_ref_params.contains(name) {
                ctx.shadowed_ref_params.insert(name.clone());
            }
        }
        (AssignTarget::Tuple(targets), Some(Type::Tuple(elem_types)))
            if targets.len() == elem_types.len() =>
        {
            // Tuple unpacking with type info: (i, val) from enumerate
            for (t, typ) in targets.iter().zip(elem_types.iter()) {
                if let AssignTarget::Symbol(s) = t {
                    ctx.declare_var(s);
                    ctx.var_types.insert(s.clone(), typ.clone());
                    if !matches!(typ, Type::Optional(_)) {
                        ctx.optional_vars.remove(s);
                    }
                    // Track if this for-loop variable shadows a ref param
                    if ctx.current_func_ref_params.contains(s) {
                        ctx.shadowed_ref_params.insert(s.clone());
                    }
                }
            }
        }
        (AssignTarget::Tuple(targets), _) => {
            // Tuple unpacking without type info
            for t in targets {
                if let AssignTarget::Symbol(s) = t {
                    ctx.declare_var(s);
                    ctx.optional_vars.remove(s);
                    // Track if this for-loop variable shadows a ref param
                    if ctx.current_func_ref_params.contains(s) {
                        ctx.shadowed_ref_params.insert(s.clone());
                    }
                }
            }
        }
        _ => {}
    }

    // Collect variables we added to shadowed_ref_params so we can remove them after scope exit
    let shadowed_in_this_scope: Vec<String> = match target {
        AssignTarget::Symbol(name) if ctx.current_func_ref_params.contains(name) => {
            vec![name.clone()]
        }
        AssignTarget::Tuple(targets) => targets
            .iter()
            .filter_map(|t| {
                if let AssignTarget::Symbol(s) = t {
                    if ctx.current_func_ref_params.contains(s) {
                        Some(s.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    };

    let body_stmts: Vec<_> = body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    // Remove shadowed variables when exiting scope
    for var in &shadowed_in_this_scope {
        ctx.shadowed_ref_params.remove(var);
    }

    ctx.is_final_statement = saved_is_final;

    // When iterating with enumerate(), the first element of the tuple is usize
    // If we're destructuring a tuple and the iterator is enumerate(), cast the first variable to i32
    let needs_enumerate_cast = matches!(iter, HirExpr::Call { func, .. } if func == "enumerate")
        && matches!(target, AssignTarget::Tuple(targets) if !targets.is_empty());

    // When iterating over strings with .chars(), convert char to String for HashMap<String, _> compatibility
    // Check if we're iterating over a string (will use .chars()) AND target is a simple symbol
    let needs_char_to_string = matches!(iter, HirExpr::Var(name) if {
        let n = name.as_str();
        (n == "s" || n == "string" || n == "text" || n == "word" || n == "line")
            || (n.starts_with("str") && !n.starts_with("strings"))
            || (n.starts_with("word") && !n.starts_with("words"))
            || (n.starts_with("text") && !n.starts_with("texts"))
            || (n.ends_with("_str") && !n.ends_with("_strs"))
            || (n.ends_with("_string") && !n.ends_with("_strings"))
            || (n.ends_with("_word") && !n.ends_with("_words"))
            || (n.ends_with("_text") && !n.ends_with("_texts"))
    }) && matches!(target, AssignTarget::Symbol(_));

    // When iterating over field accesses with .iter(), loop variables are references
    // We need to dereference them for use in value comparisons/assignments
    // This applies when: needs_field_borrow is Some(false) (immutable borrow)
    // For mutable iteration (Some(true)), we DON'T add clone - we modify in place
    let needs_deref = matches!(needs_field_borrow, Some(false));

    if needs_enumerate_cast {
        // Get the first variable name from the tuple pattern (the index from enumerate)
        if let AssignTarget::Tuple(targets) = target {
            if let Some(AssignTarget::Symbol(index_var)) = targets.first() {
                // If unused, it will be prefixed with _ in target_pattern, so no cast needed
                let is_index_used = body.iter().any(|stmt| is_var_used_in_stmt(index_var, stmt));

                // Also check if there's a value variable (second element) that needs dereferencing
                // Use *deref for Copy types, .clone() for non-Copy types
                let value_deref_stmt = if needs_deref && targets.len() >= 2 {
                    if let Some(AssignTarget::Symbol(value_var)) = targets.get(1) {
                        let is_value_used =
                            body.iter().any(|stmt| is_var_used_in_stmt(value_var, stmt));
                        if is_value_used {
                            let value_ident = safe_ident(value_var);
                            let is_copy = ctx
                                .var_types
                                .get(value_var)
                                .is_some_and(|t| !ctx.type_needs_clone(t));
                            if is_copy {
                                Some(quote! { let #value_ident = *#value_ident; })
                            } else {
                                Some(quote! { let #value_ident = #value_ident.clone(); })
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if is_index_used {
                    // Add a cast statement at the beginning of the loop body
                    let index_ident = safe_ident(index_var);
                    if let Some(deref_stmt) = value_deref_stmt {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                let #index_ident = #index_ident as i32;
                                #deref_stmt
                                #(#body_stmts)*
                            }
                        })
                    } else {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                let #index_ident = #index_ident as i32;
                                #(#body_stmts)*
                            }
                        })
                    }
                } else {
                    // Index is unused - don't generate cast statement
                    if let Some(deref_stmt) = value_deref_stmt {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                #deref_stmt
                                #(#body_stmts)*
                            }
                        })
                    } else {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                #(#body_stmts)*
                            }
                        })
                    }
                }
            } else {
                Ok(quote! {
                    for #target_pattern in #iter_expr {
                        #(#body_stmts)*
                    }
                })
            }
        } else {
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else if needs_char_to_string {
        // Python: for char in s: freq[char] = ...
        // Rust: for _char in s.chars() { let char = _char.to_string(); ... }
        if let AssignTarget::Symbol(var_name) = target {
            let var_ident = safe_ident(var_name);
            let temp_ident = safe_ident(&format!("_{}", var_name));
            Ok(quote! {
                for #temp_ident in #iter_expr {
                    let #var_ident = #temp_ident.to_string();
                    #(#body_stmts)*
                }
            })
        } else {
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else if csv_yields_results {
        // Python: for row in reader
        // Rust: for result in reader.deserialize() { let row = result?; ... }
        if let AssignTarget::Symbol(var_name) = target {
            let var_ident = safe_ident(var_name);
            let result_ident = safe_ident("result");
            Ok(quote! {
                for #result_ident in #iter_expr {
                    let #var_ident = #result_ident?;
                    #(#body_stmts)*
                }
            })
        } else {
            // Fallback if target is not a simple symbol
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else if needs_deref {
        // Iterating over field access with references - need to dereference
        if let AssignTarget::Symbol(var_name) = target {
            let is_used = body.iter().any(|stmt| is_var_used_in_stmt(var_name, stmt));
            if is_used {
                let var_ident = safe_ident(var_name);
                // Use *deref for Copy types, .clone() for non-Copy types
                let is_copy = ctx
                    .var_types
                    .get(var_name)
                    .is_some_and(|t| !ctx.type_needs_clone(t));
                let deref_stmt = if is_copy {
                    quote! { let #var_ident = *#var_ident; }
                } else {
                    quote! { let #var_ident = #var_ident.clone(); }
                };
                Ok(quote! {
                    for #target_pattern in #iter_expr {
                        #deref_stmt
                        #(#body_stmts)*
                    }
                })
            } else {
                Ok(quote! {
                    for #target_pattern in #iter_expr {
                        #(#body_stmts)*
                    }
                })
            }
        } else {
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else {
        Ok(quote! {
            for #target_pattern in #iter_expr {
                #(#body_stmts)*
            }
        })
    }
}

pub(crate) fn codegen_break_stmt(label: &Option<String>) -> Result<proc_macro2::TokenStream> {
    if let Some(label_name) = label {
        let label_ident =
            syn::Lifetime::new(&format!("'{}", label_name), proc_macro2::Span::call_site());
        Ok(quote! { break #label_ident; })
    } else {
        Ok(quote! { break; })
    }
}

pub(crate) fn codegen_continue_stmt(label: &Option<String>) -> Result<proc_macro2::TokenStream> {
    if let Some(label_name) = label {
        let label_ident =
            syn::Lifetime::new(&format!("'{}", label_name), proc_macro2::Span::call_site());
        Ok(quote! { continue #label_ident; })
    } else {
        Ok(quote! { continue; })
    }
}

pub(crate) fn extract_none_check(condition: &HirExpr) -> Option<(String, bool)> {
    if let HirExpr::Binary { op, left, right } = condition {
        // Check for: var is not None  OR  None is not var
        if *op == BinOp::IsNot {
            if let (HirExpr::Var(var_name), HirExpr::Literal(Literal::None)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), true));
            }
            if let (HirExpr::Literal(Literal::None), HirExpr::Var(var_name)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), true));
            }
        }
        // Check for: var is None  OR  None is var
        if *op == BinOp::Is {
            if let (HirExpr::Var(var_name), HirExpr::Literal(Literal::None)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), false));
            }
            if let (HirExpr::Literal(Literal::None), HirExpr::Var(var_name)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), false));
            }
        }
    }
    // Handle pre-transformed `var.is_some()` / `var.is_none()` method calls
    if let HirExpr::MethodCall { object, method, .. } = condition {
        if let HirExpr::Var(var_name) = object.as_ref() {
            if method == "is_some" {
                return Some((var_name.clone(), true));
            }
            if method == "is_none" {
                return Some((var_name.clone(), false));
            }
        }
    }
    None
}

pub(crate) fn codegen_if_let_some(
    var_name: String,
    then_body: &[HirStmt],
    else_body: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let var_ident = safe_ident(&var_name);

    // Temporarily remove from optional_vars so inner code doesn't double-unwrap
    ctx.optional_vars.remove(&var_name);

    // Also narrow var_types: Optional<T> → T so field access doesn't add unwrap
    let saved_var_type = ctx.var_types.remove(&var_name);
    if let Some(Type::Optional(inner)) = &saved_var_type {
        ctx.var_types
            .insert(var_name.clone(), inner.as_ref().clone());
    }

    ctx.enter_scope();
    // Declare the narrowed (unwrapped) variable in the inner scope
    ctx.declare_var(&var_name);

    let then_stmts: Vec<_> = then_body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    // Restore optional_vars and var_types
    ctx.optional_vars.insert(var_name.clone());
    if let Some(saved) = saved_var_type {
        ctx.var_types.insert(var_name.clone(), saved);
    }

    if let Some(else_stmts) = else_body {
        ctx.enter_scope();
        let else_tokens: Vec<_> = else_stmts
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;
        ctx.exit_scope();
        Ok(quote! {
            if let Some(mut #var_ident) = #var_ident {
                #(#then_stmts)*
            } else {
                #(#else_tokens)*
            }
        })
    } else {
        Ok(quote! {
            if let Some(mut #var_ident) = #var_ident {
                #(#then_stmts)*
            }
        })
    }
}

pub(crate) fn extract_walrus_assignments(expr: &HirExpr) -> (Vec<(String, Box<HirExpr>)>, HirExpr) {
    let mut assignments = Vec::new();
    let modified = extract_walrus_recursive(expr, &mut assignments);
    (assignments, modified)
}

pub(crate) fn extract_walrus_recursive(
    expr: &HirExpr,
    assignments: &mut Vec<(String, Box<HirExpr>)>,
) -> HirExpr {
    match expr {
        HirExpr::NamedExpr { target, value } => {
            // Extract the assignment
            let extracted_value = extract_walrus_recursive(value, assignments);
            assignments.push((target.clone(), Box::new(extracted_value.clone())));
            // Replace with just the variable reference
            HirExpr::Var(target.clone())
        }
        HirExpr::Binary { op, left, right } => {
            let new_left = Box::new(extract_walrus_recursive(left, assignments));
            let new_right = Box::new(extract_walrus_recursive(right, assignments));
            HirExpr::Binary {
                op: *op,
                left: new_left,
                right: new_right,
            }
        }
        HirExpr::Unary { op, operand } => {
            let new_operand = Box::new(extract_walrus_recursive(operand, assignments));
            HirExpr::Unary {
                op: *op,
                operand: new_operand,
            }
        }
        HirExpr::Call {
            func,
            args,
            type_params,
            kwargs,
        } => {
            let new_args = args
                .iter()
                .map(|arg| extract_walrus_recursive(arg, assignments))
                .collect();
            HirExpr::Call {
                func: func.clone(),
                args: new_args,
                type_params: type_params.clone(),
                kwargs: kwargs.clone(),
            }
        }
        HirExpr::MethodCall {
            object,
            method,
            args,
            kwargs,
            type_params,
        } => {
            let new_object = Box::new(extract_walrus_recursive(object, assignments));
            let new_args = args
                .iter()
                .map(|arg| extract_walrus_recursive(arg, assignments))
                .collect();
            HirExpr::MethodCall {
                object: new_object,
                method: method.clone(),
                args: new_args,
                kwargs: kwargs.clone(),
                type_params: type_params.clone(),
            }
        }
        HirExpr::IfExpr { test, body, orelse } => {
            let new_test = Box::new(extract_walrus_recursive(test, assignments));
            let new_body = Box::new(extract_walrus_recursive(body, assignments));
            let new_orelse = Box::new(extract_walrus_recursive(orelse, assignments));
            HirExpr::IfExpr {
                test: new_test,
                body: new_body,
                orelse: new_orelse,
            }
        }
        // For other expression types, just clone as-is (no walrus operators inside)
        _ => expr.clone(),
    }
}

pub(crate) fn extract_assigned_symbols(stmts: &[HirStmt]) -> std::collections::HashSet<String> {
    use std::collections::HashSet;
    let mut symbols = HashSet::new();

    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                ..
            } => {
                symbols.insert(name.clone());
            }
            // Recursively check nested if/else, while, for, try blocks
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                symbols.extend(extract_assigned_symbols(then_body));
                if let Some(else_stmts) = else_body {
                    symbols.extend(extract_assigned_symbols(else_stmts));
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                symbols.extend(extract_assigned_symbols(body));
            }
            HirStmt::Try {
                body,
                handlers,
                finalbody,
                ..
            } => {
                symbols.extend(extract_assigned_symbols(body));
                for handler in handlers {
                    symbols.extend(extract_assigned_symbols(&handler.body));
                }
                if let Some(finally) = finalbody {
                    symbols.extend(extract_assigned_symbols(finally));
                }
            }
            _ => {}
        }
    }

    symbols
}

pub(crate) fn find_variable_type(var_name: &str, stmts: &[HirStmt]) -> Option<Type> {
    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                type_annotation,
                ..
            } if name == var_name => {
                return type_annotation.clone();
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn generate_field_access_without_clone(
    iter: &HirExpr,
    ctx: &mut CodeGenContext,
) -> Result<syn::Expr> {
    generate_field_access_inner(iter, ctx, false)
}

/// Like `generate_field_access_without_clone` but wraps IfExpr branches
/// in `&mut` to avoid moving out of mutable references.
pub(crate) fn generate_field_access_mut_ref(
    iter: &HirExpr,
    ctx: &mut CodeGenContext,
) -> Result<syn::Expr> {
    generate_field_access_inner(iter, ctx, true)
}

fn generate_field_access_inner(
    iter: &HirExpr,
    ctx: &mut CodeGenContext,
    mut_ref_branches: bool,
) -> Result<syn::Expr> {
    match iter {
        HirExpr::Attribute { value, attr } => {
            let value_expr = generate_field_access_inner(value, ctx, mut_ref_branches)?;
            let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
            Ok(parse_quote! { #value_expr.#attr_ident })
        }
        HirExpr::Var(name) => {
            let ident = safe_ident(name);
            Ok(parse_quote! { #ident })
        }
        HirExpr::IfExpr {
            test, body, orelse, ..
        } => {
            let test_expr = test.to_rust_expr(ctx)?;
            let body_expr = generate_field_access_inner(body, ctx, mut_ref_branches)?;
            let orelse_expr = generate_field_access_inner(orelse, ctx, mut_ref_branches)?;
            if mut_ref_branches
                && matches!(**body, HirExpr::Attribute { .. })
                && matches!(**orelse, HirExpr::Attribute { .. })
            {
                Ok(parse_quote! { if #test_expr { &mut #body_expr } else { &mut #orelse_expr } })
            } else {
                Ok(parse_quote! { if #test_expr { #body_expr } else { #orelse_expr } })
            }
        }
        HirExpr::Call { func, args, .. } if func == "enumerate" || func == "reversed" => {
            if !args.is_empty() {
                let inner = generate_field_access_inner(&args[0], ctx, mut_ref_branches)?;
                if func == "enumerate" {
                    Ok(parse_quote! { #inner.iter().enumerate() })
                } else {
                    Ok(parse_quote! { #inner.iter().rev() })
                }
            } else {
                iter.to_rust_expr(ctx)
            }
        }
        _ => iter.to_rust_expr(ctx),
    }
}

fn expr_contains_ifexpr(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::IfExpr { .. } => true,
        HirExpr::Attribute { value, .. } | HirExpr::Index { base: value, .. } => {
            expr_contains_ifexpr(value)
        }
        _ => false,
    }
}

/// Splits an expression like `(IfExpr).attr1.attr2` into the IfExpr parts and
/// the remaining attribute chain. Returns `(test, body, orelse, attrs)` where
/// `attrs` is the list of attribute names after the IfExpr.
fn split_at_ifexpr<'a>(
    expr: &'a HirExpr,
    attrs: &mut Vec<String>,
) -> Option<(&'a HirExpr, &'a HirExpr, &'a HirExpr)> {
    match expr {
        HirExpr::IfExpr {
            test, body, orelse, ..
        } => Some((test, body, orelse)),
        HirExpr::Attribute { value, attr } => {
            attrs.push(attr.clone());
            split_at_ifexpr(value, attrs)
        }
        _ => None,
    }
}

/// Generates a sliced iteration over a conditional field access.
/// Produces: `{ let __ref = if cond { &mut body } else { &mut orelse }; let stop = ...; __ref.field[..stop].iter_mut() }`
fn generate_ifexpr_slice_iter(
    base: &HirExpr,
    start: &Option<Box<HirExpr>>,
    stop: &Option<Box<HirExpr>>,
    needs_mut: bool,
    ctx: &mut CodeGenContext,
) -> Result<Option<syn::Expr>> {
    let mut attrs = Vec::new();
    let Some((test, body, orelse)) = split_at_ifexpr(base, &mut attrs) else {
        return Ok(None);
    };
    // attrs were collected in reverse order (innermost first)
    attrs.reverse();

    let test_expr = test.to_rust_expr(ctx)?;
    let body_expr = generate_field_access_without_clone(body, ctx)?;
    let orelse_expr = generate_field_access_without_clone(orelse, ctx)?;

    let attr_idents: Vec<syn::Ident> = attrs
        .iter()
        .map(|a| syn::Ident::new(a, proc_macro2::Span::call_site()))
        .collect();

    let iter_method = if needs_mut {
        quote::quote! { iter_mut }
    } else {
        quote::quote! { iter }
    };

    // Build the field chain: __ref.attr1.attr2...
    let field_chain = quote::quote! { __ref #(.#attr_idents)* };

    let result = match (start, stop) {
        (None, Some(stop_val)) => {
            let stop_expr = stop_val.to_rust_expr(ctx)?;
            parse_quote! {{
                let __ref = if #test_expr { &mut #body_expr } else { &mut #orelse_expr };
                let __len = #field_chain.len();
                let stop = (#stop_expr).max(0) as usize;
                #field_chain[..stop.min(__len)].#iter_method()
            }}
        }
        (Some(start_val), Some(stop_val)) => {
            let start_expr = start_val.to_rust_expr(ctx)?;
            let stop_expr = stop_val.to_rust_expr(ctx)?;
            parse_quote! {{
                let __ref = if #test_expr { &mut #body_expr } else { &mut #orelse_expr };
                let __len = #field_chain.len();
                let start = (#start_expr).max(0) as usize;
                let stop = (#stop_expr).max(0) as usize;
                if start < __len { #field_chain[start..stop.min(__len)].#iter_method() } else { [].#iter_method() }
            }}
        }
        (Some(start_val), None) => {
            let start_expr = start_val.to_rust_expr(ctx)?;
            parse_quote! {{
                let __ref = if #test_expr { &mut #body_expr } else { &mut #orelse_expr };
                let __len = #field_chain.len();
                let start = (#start_expr).max(0) as usize;
                if start < __len { #field_chain[start..].#iter_method() } else { [].#iter_method() }
            }}
        }
        (None, None) => {
            parse_quote! {{
                let __ref = if #test_expr { &mut #body_expr } else { &mut #orelse_expr };
                #field_chain.#iter_method()
            }}
        }
    };
    Ok(Some(result))
}

pub(crate) fn is_field_access_iter(iter: &HirExpr) -> Option<(String, bool)> {
    match iter {
        // Direct field access: state.items or (ternary).field
        HirExpr::Attribute { value, .. } => {
            if let Some(root_var) = crate::ast_bridge::expr_utils::extract_root_var(value) {
                Some((root_var, true))
            } else {
                is_field_access_iter(value)
            }
        }
        // Slice of a field access: state.items[:N] or (ternary).field[:N]
        HirExpr::Slice { base, .. } => is_field_access_iter(base),
        // Conditional field access: (state.home if cond else state.away)
        HirExpr::IfExpr { body, orelse, .. } => {
            let body_result = is_field_access_iter(body);
            let orelse_result = is_field_access_iter(orelse);
            match (body_result, orelse_result) {
                (Some((_, true)), Some((_, true))) => Some((String::new(), true)),
                _ => None,
            }
        }
        // enumerate(state.items) or reversed(state.items)
        HirExpr::Call { func, args, .. }
            if (func == "enumerate" || func == "reversed") && !args.is_empty() =>
        {
            is_field_access_iter(&args[0])
        }
        _ => None,
    }
}

pub(crate) fn does_loop_body_mutate_items(
    target: &AssignTarget,
    body: &[HirStmt],
    function_param_borrows: &std::collections::HashMap<
        String,
        Vec<crate::rust_generator::context::ParamBorrowInfo>,
    >,
) -> bool {
    // Get the loop variable name
    let loop_var = match target {
        AssignTarget::Symbol(name) => name,
        AssignTarget::Tuple(targets) => {
            // For tuples like (i, item), check the second element
            if targets.len() >= 2 {
                if let AssignTarget::Symbol(name) = &targets[1] {
                    name
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        _ => return false,
    };

    // Check if the loop variable is mutated in the body
    for stmt in body {
        if is_loop_var_mutated(loop_var, stmt, function_param_borrows) {
            return true;
        }
    }
    false
}

pub(crate) fn is_loop_var_mutated(
    var_name: &str,
    stmt: &HirStmt,
    function_param_borrows: &std::collections::HashMap<
        String,
        Vec<crate::rust_generator::context::ParamBorrowInfo>,
    >,
) -> bool {
    match stmt {
        // Direct assignment to loop variable or its fields
        HirStmt::Assign { target, value, .. } => {
            let target_mutated = match target {
                AssignTarget::Symbol(name) if name == var_name => true,
                AssignTarget::Attribute { value, .. } => {
                    // Check if assigning to var_name.field (including nested like var_name.a.b.c)
                    crate::ast_bridge::expr_utils::extract_root_var(value).is_some_and(|root| root == var_name)
                }
                AssignTarget::Index { base, .. } => {
                    // Check if assigning to var_name[index] (including nested like var_name.a[i])
                    crate::ast_bridge::expr_utils::extract_root_var(base).is_some_and(|root| root == var_name)
                }
                _ => false,
            };
            target_mutated || is_var_passed_as_mut_in_expr(var_name, value, function_param_borrows)
        }
        // Check expression statements (e.g., bare function calls)
        HirStmt::Expr(expr) => is_var_passed_as_mut_in_expr(var_name, expr, function_param_borrows),
        // Check return statements
        HirStmt::Return(Some(expr)) => {
            is_var_passed_as_mut_in_expr(var_name, expr, function_param_borrows)
        }
        // Check nested statements
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            then_body
                .iter()
                .any(|s| is_loop_var_mutated(var_name, s, function_param_borrows))
                || else_body.as_ref().is_some_and(|body| {
                    body.iter()
                        .any(|s| is_loop_var_mutated(var_name, s, function_param_borrows))
                })
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => body
            .iter()
            .any(|s| is_loop_var_mutated(var_name, s, function_param_borrows)),
        _ => false,
    }
}

/// Check if a variable is passed as an argument to a function expecting `&mut`.
fn is_var_passed_as_mut_in_expr(
    var_name: &str,
    expr: &HirExpr,
    function_param_borrows: &std::collections::HashMap<
        String,
        Vec<crate::rust_generator::context::ParamBorrowInfo>,
    >,
) -> bool {
    match expr {
        HirExpr::Call { func, args, .. } => {
            if let Some(borrows) = function_param_borrows.get(func.as_str()) {
                for (idx, arg) in args.iter().enumerate() {
                    if let HirExpr::Var(name) = arg {
                        if name == var_name {
                            if let Some(info) = borrows.get(idx) {
                                if info.should_borrow && info.needs_mut {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            // Recurse into all sub-expressions
            args.iter()
                .any(|a| is_var_passed_as_mut_in_expr(var_name, a, function_param_borrows))
        }
        HirExpr::ListComp { element, .. } => {
            is_var_passed_as_mut_in_expr(var_name, element, function_param_borrows)
        }
        HirExpr::Binary { left, right, .. } => {
            is_var_passed_as_mut_in_expr(var_name, left, function_param_borrows)
                || is_var_passed_as_mut_in_expr(var_name, right, function_param_borrows)
        }
        HirExpr::Unary { operand, .. } => {
            is_var_passed_as_mut_in_expr(var_name, operand, function_param_borrows)
        }
        HirExpr::IfExpr { test, body, orelse } => {
            is_var_passed_as_mut_in_expr(var_name, test, function_param_borrows)
                || is_var_passed_as_mut_in_expr(var_name, body, function_param_borrows)
                || is_var_passed_as_mut_in_expr(var_name, orelse, function_param_borrows)
        }
        _ => false,
    }
}
