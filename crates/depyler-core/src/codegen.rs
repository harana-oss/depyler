use crate::hir::*;
use anyhow::{Result, bail};
use quote::{ToTokens, quote};
use std::collections::HashSet;
use syn;

pub fn generate_rust(file: syn::File) -> Result<String> {
    let tokens = file.to_token_stream();
    let rust_code = tokens.to_string();

    Ok(prettify_rust_code(rust_code))
}

pub fn hir_to_rust(hir: &HirModule) -> Result<String> {
    let mut rust_items = Vec::new();

    if needs_std_collections(hir) {
        rust_items.push(quote! { use std::collections::HashMap; });
    }

    for func in &hir.functions {
        let rust_func = convert_function_to_rust(func)?;
        rust_items.push(rust_func);
    }

    let file = quote! {
        #(#rust_items)*
    };

    Ok(prettify_rust_code(file.to_string()))
}

fn needs_std_collections(hir: &HirModule) -> bool {
    hir.functions.iter().any(|f| {
        f.params.iter().any(|param| uses_hashmap(&param.ty))
            || uses_hashmap(&f.ret_type)
            || function_body_uses_hashmap(&f.body)
    })
}

fn uses_hashmap(ty: &Type) -> bool {
    match ty {
        Type::Dict(_, _) => true,
        Type::List(inner) | Type::Optional(inner) => uses_hashmap(inner),
        Type::Tuple(types) => types.iter().any(uses_hashmap),
        Type::Function { params, ret } => params.iter().any(uses_hashmap) || uses_hashmap(ret),
        _ => false,
    }
}

fn function_body_uses_hashmap(body: &[HirStmt]) -> bool {
    body.iter().any(stmt_uses_hashmap)
}

fn stmt_uses_hashmap(stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Assign { value, .. } => expr_uses_hashmap(value),
        HirStmt::Return(Some(expr)) => expr_uses_hashmap(expr),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_uses_hashmap(condition)
                || function_body_uses_hashmap(then_body)
                || else_body
                    .as_ref()
                    .is_some_and(|body| function_body_uses_hashmap(body))
        }
        HirStmt::While { condition, body } => {
            expr_uses_hashmap(condition) || function_body_uses_hashmap(body)
        }
        HirStmt::For { iter, body, .. } => {
            expr_uses_hashmap(iter) || function_body_uses_hashmap(body)
        }
        HirStmt::Expr(expr) => expr_uses_hashmap(expr),
        _ => false,
    }
}

fn expr_uses_hashmap(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Dict(_) => true,
        HirExpr::Binary { left, right, .. } => expr_uses_hashmap(left) || expr_uses_hashmap(right),
        HirExpr::Unary { operand, .. } => expr_uses_hashmap(operand),
        HirExpr::Call { args, .. } => args.iter().any(expr_uses_hashmap),
        HirExpr::Index { base, index } => expr_uses_hashmap(base) || expr_uses_hashmap(index),
        HirExpr::List(items) | HirExpr::Tuple(items) => items.iter().any(expr_uses_hashmap),
        _ => false,
    }
}

struct ScopeTracker {
    declared_vars: Vec<HashSet<String>>,
}

impl ScopeTracker {
    fn new() -> Self {
        Self {
            declared_vars: vec![HashSet::new()],
        }
    }

    fn enter_scope(&mut self) {
        self.declared_vars.push(HashSet::new());
    }

    fn exit_scope(&mut self) {
        self.declared_vars.pop();
    }

    fn is_declared(&self, var_name: &str) -> bool {
        self.declared_vars
            .iter()
            .any(|scope| scope.contains(var_name))
    }

    fn declare_var(&mut self, var_name: &str) {
        if let Some(current_scope) = self.declared_vars.last_mut() {
            current_scope.insert(var_name.to_string());
        }
    }
}

fn convert_function_to_rust(func: &HirFunction) -> Result<proc_macro2::TokenStream> {
    let name = syn::Ident::new(&func.name, proc_macro2::Span::call_site());

    // Convert parameters
    let params: Vec<_> = func
        .params
        .iter()
        .map(|param| {
            let param_ident = syn::Ident::new(&param.name, proc_macro2::Span::call_site());
            let rust_type = type_to_rust_type(&param.ty);
            quote! { #param_ident: #rust_type }
        })
        .collect();

    // Convert return type
    let return_type = type_to_rust_type(&func.ret_type);

    // Convert body with scope tracking
    let mut scope_tracker = ScopeTracker::new();

    // Declare function parameters in the scope
    for param in &func.params {
        scope_tracker.declare_var(&param.name);
    }

    let body_stmts: Vec<_> = func
        .body
        .iter()
        .map(|stmt| stmt_to_rust_tokens_with_scope(stmt, &mut scope_tracker))
        .collect::<Result<Vec<_>>>()?;

    // Add async if needed
    let func_tokens = if func.properties.is_async {
        quote! {
            pub async fn #name(#(#params),*) -> #return_type {
                #(#body_stmts)*
            }
        }
    } else {
        quote! {
            pub fn #name(#(#params),*) -> #return_type {
                #(#body_stmts)*
            }
        }
    };

    Ok(func_tokens)
}

fn type_to_rust_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Int => quote! { i32 },
        Type::Float => quote! { f64 },
        Type::String => quote! { String },
        Type::Bool => quote! { bool },
        Type::None => quote! { () },
        Type::List(inner) => {
            let inner_type = type_to_rust_type(inner);
            quote! { Vec<#inner_type> }
        }
        Type::Dict(key, value) => {
            let key_type = type_to_rust_type(key);
            let value_type = type_to_rust_type(value);
            quote! { HashMap<#key_type, #value_type> }
        }
        Type::Tuple(types) => {
            let rust_types: Vec<_> = types.iter().map(type_to_rust_type).collect();
            quote! { (#(#rust_types),*) }
        }
        Type::Optional(inner) => {
            let inner_type = type_to_rust_type(inner);
            quote! { Option<#inner_type> }
        }
        Type::Final(inner) => type_to_rust_type(inner), // Unwrap Final to get the actual type
        Type::Function { params, ret } => {
            let param_types: Vec<_> = params.iter().map(type_to_rust_type).collect();
            let ret_type = type_to_rust_type(ret);
            quote! { fn(#(#param_types),*) -> #ret_type }
        }
        Type::Custom(name) => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! { #ident }
        }
        Type::Unknown => quote! { () },
        Type::TypeVar(name) => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! { #ident }
        }
        Type::Generic { base, params } => {
            let base_ident = syn::Ident::new(base, proc_macro2::Span::call_site());
            let param_types: Vec<_> = params.iter().map(type_to_rust_type).collect();
            quote! { #base_ident<#(#param_types),*> }
        }
        Type::Union(_) => quote! { UnionType }, // Placeholder, will be handled by enum generation
        Type::Array { element_type, size } => {
            let element = type_to_rust_type(element_type);
            match size {
                crate::hir::ConstGeneric::Literal(n) => {
                    let size_lit = syn::LitInt::new(&n.to_string(), proc_macro2::Span::call_site());
                    quote! { [#element; #size_lit] }
                }
                crate::hir::ConstGeneric::Parameter(name) => {
                    let param_ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                    quote! { [#element; #param_ident] }
                }
                crate::hir::ConstGeneric::Expression(expr) => {
                    // For expressions, parse them as token streams
                    let expr_tokens: proc_macro2::TokenStream = expr.parse().unwrap_or_else(|_| {
                        quote! { /* invalid const expression */ }
                    });
                    quote! { [#element; #expr_tokens] }
                }
            }
        }
        Type::Set(inner) => {
            let inner_type = type_to_rust_type(inner);
            quote! { HashSet<#inner_type> }
        }
    }
}

#[allow(dead_code)]
fn stmt_to_rust_tokens(stmt: &HirStmt) -> Result<proc_macro2::TokenStream> {
    // Legacy function - delegate to the new scope-aware version with a throwaway scope
    let mut scope_tracker = ScopeTracker::new();
    stmt_to_rust_tokens_with_scope(stmt, &mut scope_tracker)
}

fn handle_assign_target(
    target: &AssignTarget,
    value_tokens: proc_macro2::TokenStream,
    scope_tracker: &mut ScopeTracker,
) -> Result<proc_macro2::TokenStream> {
    match target {
        AssignTarget::Symbol(symbol) => {
            let target_ident = syn::Ident::new(symbol, proc_macro2::Span::call_site());
            if scope_tracker.is_declared(symbol) {
                Ok(quote! { #target_ident = #value_tokens; })
            } else {
                scope_tracker.declare_var(symbol);
                Ok(quote! { let mut #target_ident = #value_tokens; })
            }
        }
        AssignTarget::Index { base, index } => {
            let base_tokens = expr_to_rust_tokens(base)?;
            let index_tokens = expr_to_rust_tokens(index)?;
            Ok(quote! { #base_tokens.insert(#index_tokens, #value_tokens); })
        }
        AssignTarget::Attribute { value, attr } => {
            // Struct field assignment: obj.field = value
            let base_tokens = expr_to_rust_tokens(value)?;
            let attr_ident = syn::Ident::new(attr.as_str(), proc_macro2::Span::call_site());
            Ok(quote! { #base_tokens.#attr_ident = #value_tokens; })
        }
        AssignTarget::Tuple(targets) => {
            // Tuple unpacking
            let all_symbols: Option<Vec<&str>> = targets
                .iter()
                .map(|t| match t {
                    AssignTarget::Symbol(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect();

            match all_symbols {
                Some(symbols) => {
                    let all_declared = symbols.iter().all(|s| scope_tracker.is_declared(s));

                    if all_declared {
                        let idents: Vec<_> = symbols
                            .iter()
                            .map(|s| syn::Ident::new(s, proc_macro2::Span::call_site()))
                            .collect();
                        Ok(quote! { (#(#idents),*) = #value_tokens; })
                    } else {
                        symbols.iter().for_each(|s| scope_tracker.declare_var(s));
                        let idents: Vec<_> = symbols
                            .iter()
                            .map(|s| syn::Ident::new(s, proc_macro2::Span::call_site()))
                            .collect();
                        Ok(quote! { let (mut #(#idents),*) = #value_tokens; })
                    }
                }
                None => {
                    // Handle complex tuple unpacking with index targets
                    codegen_complex_tuple_unpack(targets, value_tokens)
                }
            }
        }
        AssignTarget::Slice { base, .. } => {
            // Slice assignment: x[:] = value -> clear and extend
            let base_tokens = expr_to_rust_tokens(base)?;
            Ok(quote! {
                #base_tokens.clear();
                #base_tokens.extend(#value_tokens);
            })
        }
    }
}

/// Generate code for complex tuple unpacking (with index targets)
fn codegen_complex_tuple_unpack(
    targets: &[AssignTarget],
    value_tokens: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    // Generate temporary variable names
    let temp_names: Vec<syn::Ident> = (0..targets.len())
        .map(|i| syn::Ident::new(&format!("_swap_tmp{}", i), proc_macro2::Span::call_site()))
        .collect();

    // Create tuple pattern for temporaries
    let temp_pattern: Vec<_> = temp_names.iter().map(|name| quote! { #name }).collect();
    let capture_stmt = quote! { let (#(#temp_pattern),*) = #value_tokens; };

    // Generate individual assignments
    let mut assignments = Vec::new();
    for (i, target) in targets.iter().enumerate() {
        let temp_name = &temp_names[i];
        let assign = match target {
            AssignTarget::Symbol(symbol) => {
                let ident = syn::Ident::new(symbol, proc_macro2::Span::call_site());
                quote! { #ident = #temp_name; }
            }
            AssignTarget::Index { base, index } => {
                let base_tokens = expr_to_rust_tokens(base)?;
                let index_tokens = expr_to_rust_tokens(index)?;
                quote! { #base_tokens[#index_tokens as usize] = #temp_name; }
            }
            AssignTarget::Attribute { value: base, attr } => {
                let base_tokens = expr_to_rust_tokens(base)?;
                let attr_ident = syn::Ident::new(attr.as_str(), proc_macro2::Span::call_site());
                quote! { #base_tokens.#attr_ident = #temp_name; }
            }
            AssignTarget::Tuple(_) => anyhow::bail!("Nested tuple unpacking not supported"),
            AssignTarget::Slice { .. } => {
                anyhow::bail!("Slice target in tuple unpacking not supported")
            }
        };
        assignments.push(assign);
    }

    Ok(quote! {
        {
            #capture_stmt
            #(#assignments)*
        }
    })
}

fn handle_if_stmt(
    condition: &HirExpr,
    then_body: &[HirStmt],
    else_body: &Option<Vec<HirStmt>>,
    scope_tracker: &mut ScopeTracker,
) -> Result<proc_macro2::TokenStream> {
    let cond_tokens = expr_to_rust_tokens(condition)?;

    scope_tracker.enter_scope();
    let then_stmts: Vec<_> = then_body
        .iter()
        .map(|stmt| stmt_to_rust_tokens_with_scope(stmt, scope_tracker))
        .collect::<Result<Vec<_>>>()?;
    scope_tracker.exit_scope();

    if let Some(else_stmts) = else_body {
        scope_tracker.enter_scope();
        let else_tokens: Vec<_> = else_stmts
            .iter()
            .map(|stmt| stmt_to_rust_tokens_with_scope(stmt, scope_tracker))
            .collect::<Result<Vec<_>>>()?;
        scope_tracker.exit_scope();
        Ok(quote! {
            if #cond_tokens {
                #(#then_stmts)*
            } else {
                #(#else_tokens)*
            }
        })
    } else {
        Ok(quote! {
            if #cond_tokens {
                #(#then_stmts)*
            }
        })
    }
}

fn handle_while_stmt(
    condition: &HirExpr,
    body: &[HirStmt],
    scope_tracker: &mut ScopeTracker,
) -> Result<proc_macro2::TokenStream> {
    let cond_tokens = expr_to_rust_tokens(condition)?;
    scope_tracker.enter_scope();
    let body_stmts: Vec<_> = body
        .iter()
        .map(|stmt| stmt_to_rust_tokens_with_scope(stmt, scope_tracker))
        .collect::<Result<Vec<_>>>()?;
    scope_tracker.exit_scope();
    Ok(quote! {
        while #cond_tokens {
            #(#body_stmts)*
        }
    })
}

fn handle_for_stmt(
    target: &AssignTarget,
    iter: &HirExpr,
    body: &[HirStmt],
    scope_tracker: &mut ScopeTracker,
) -> Result<proc_macro2::TokenStream> {
    // Convert tuple to array for iteration (tuples in Rust aren't directly iterable)
    let iter_tokens = if let HirExpr::Tuple(items) = iter {
        let item_tokens: Vec<_> = items
            .iter()
            .map(expr_to_rust_tokens)
            .collect::<Result<Vec<_>>>()?;
        quote! { [#(#item_tokens),*] }
    } else {
        expr_to_rust_tokens(iter)?
    };
    scope_tracker.enter_scope();

    // Generate target pattern and declare variables
    let target_pattern = match target {
        AssignTarget::Symbol(name) => {
            scope_tracker.declare_var(name);
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! { #ident }
        }
        AssignTarget::Tuple(targets) => {
            // Extract symbols and declare them
            let idents: Vec<_> = targets
                .iter()
                .map(|t| match t {
                    AssignTarget::Symbol(s) => {
                        scope_tracker.declare_var(s);
                        syn::Ident::new(s, proc_macro2::Span::call_site())
                    }
                    _ => panic!("Nested tuple unpacking not supported in for loops"),
                })
                .collect();
            quote! { (#(#idents),*) }
        }
        _ => bail!("Unsupported for loop target type"),
    };

    let body_stmts: Vec<_> = body
        .iter()
        .map(|stmt| stmt_to_rust_tokens_with_scope(stmt, scope_tracker))
        .collect::<Result<Vec<_>>>()?;
    scope_tracker.exit_scope();
    Ok(quote! {
        for #target_pattern in #iter_tokens {
            #(#body_stmts)*
        }
    })
}

fn handle_with_stmt(
    context: &HirExpr,
    target: &Option<String>,
    body: &[HirStmt],
) -> Result<proc_macro2::TokenStream> {
    let context_tokens = expr_to_rust_tokens(context)?;
    let body_tokens: Vec<_> = body
        .iter()
        .map(stmt_to_rust_tokens)
        .collect::<Result<_>>()?;

    if let Some(var_name) = target {
        let var_ident = syn::Ident::new(var_name, proc_macro2::Span::call_site());
        Ok(quote! {
            {
                let mut #var_ident = #context_tokens;
                #(#body_tokens)*
            }
        })
    } else {
        Ok(quote! {
            {
                let _context = #context_tokens;
                #(#body_tokens)*
            }
        })
    }
}

fn stmt_to_rust_tokens_with_scope(
    stmt: &HirStmt,
    scope_tracker: &mut ScopeTracker,
) -> Result<proc_macro2::TokenStream> {
    match stmt {
        HirStmt::Assign { target, value, .. } => {
            let value_tokens = expr_to_rust_tokens(value)?;
            handle_assign_target(target, value_tokens, scope_tracker)
        }
        HirStmt::Return(expr_opt) => {
            if let Some(expr) = expr_opt {
                let expr_tokens = expr_to_rust_tokens(expr)?;
                Ok(quote! { return #expr_tokens; })
            } else {
                Ok(quote! { return; })
            }
        }
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => handle_if_stmt(condition, then_body, else_body, scope_tracker),
        HirStmt::While { condition, body } => handle_while_stmt(condition, body, scope_tracker),
        HirStmt::For { target, iter, body } => handle_for_stmt(target, iter, body, scope_tracker),
        HirStmt::Expr(expr) => {
            let expr_tokens = expr_to_rust_tokens(expr)?;
            Ok(quote! { #expr_tokens; })
        }
        HirStmt::Raise {
            exception,
            cause: _,
        } => {
            // Simple error handling for codegen - just generate a panic for now
            if let Some(exc) = exception {
                let exc_tokens = expr_to_rust_tokens(exc)?;
                Ok(quote! { panic!("Exception: {}", #exc_tokens); })
            } else {
                Ok(quote! { panic!("Exception raised"); })
            }
        }
        HirStmt::Break { label } => {
            if let Some(label_name) = label {
                let label_ident =
                    syn::Lifetime::new(&format!("'{}", label_name), proc_macro2::Span::call_site());
                Ok(quote! { break #label_ident; })
            } else {
                Ok(quote! { break; })
            }
        }
        HirStmt::Continue { label } => {
            if let Some(label_name) = label {
                let label_ident =
                    syn::Lifetime::new(&format!("'{}", label_name), proc_macro2::Span::call_site());
                Ok(quote! { continue #label_ident; })
            } else {
                Ok(quote! { continue; })
            }
        }
        HirStmt::With {
            context,
            target,
            body,
        } => handle_with_stmt(context, target, body),
        HirStmt::Try {
            body,
            handlers,
            orelse: _,
            finalbody,
        } => {
            // Generate try body statements
            let try_stmts: Vec<_> = body
                .iter()
                .map(|s| stmt_to_rust_tokens_with_scope(s, scope_tracker))
                .collect::<Result<Vec<_>>>()?;

            // Generate finally statements if present
            let finally_stmts = if let Some(finally_body) = finalbody {
                let stmts: Vec<_> = finally_body
                    .iter()
                    .map(|s| stmt_to_rust_tokens_with_scope(s, scope_tracker))
                    .collect::<Result<Vec<_>>>()?;
                Some(quote! { #(#stmts)* })
            } else {
                None
            };

            // Generate handler statements (just use first handler for simplicity)
            if let Some(handler) = handlers.first() {
                let handler_stmts: Vec<_> = handler
                    .body
                    .iter()
                    .map(|s| stmt_to_rust_tokens_with_scope(s, scope_tracker))
                    .collect::<Result<Vec<_>>>()?;

                if let Some(finally_code) = finally_stmts {
                    Ok(quote! {
                        {
                            let _result = (|| -> Result<(), Box<dyn std::error::Error>> {
                                #(#try_stmts)*
                                Ok(())
                            })();
                            if let Err(_e) = _result {
                                #(#handler_stmts)*
                            }
                            #finally_code
                        }
                    })
                } else {
                    Ok(quote! {
                        {
                            let _result = (|| -> Result<(), Box<dyn std::error::Error>> {
                                #(#try_stmts)*
                                Ok(())
                            })();
                            if let Err(_e) = _result {
                                #(#handler_stmts)*
                            }
                        }
                    })
                }
            } else {
                // No handlers - try/finally without except
                if let Some(finally_code) = finally_stmts {
                    Ok(quote! {
                        {
                            #(#try_stmts)*
                            #finally_code
                        }
                    })
                } else {
                    Ok(quote! { #(#try_stmts)* })
                }
            }
        }
        HirStmt::Assert { test, msg } => {
            // Generate assert! macro call
            let test_expr = expr_to_rust_tokens(test)?;
            if let Some(message) = msg {
                let msg_expr = expr_to_rust_tokens(message)?;
                Ok(quote! { assert!(#test_expr, "{}", #msg_expr); })
            } else {
                Ok(quote! { assert!(#test_expr); })
            }
        }
        HirStmt::Pass => {
            // Pass statement generates no code
            Ok(quote! {})
        }
        HirStmt::FunctionDef { .. } => {
            // This is handled by the main rust_gen module
            // This codegen.rs module is a legacy simplified codegen path
            // For now, just return empty - nested functions use the main rust_gen path
            Ok(quote! {})
        }
        HirStmt::Global { .. } | HirStmt::Nonlocal { .. } => {
            // Declaration markers - no code generated
            Ok(quote! {})
        }
        HirStmt::Import { .. } | HirStmt::ImportFrom { .. } => {
            // Import statements inside functions are no-ops in Rust
            Ok(quote! {})
        }
        HirStmt::AsyncFor { iter, body, target } => {
            let iter_expr = expr_to_rust_tokens(iter)?;
            let body_stmts: Vec<_> = body
                .iter()
                .map(|s| stmt_to_rust_tokens_with_scope(s, scope_tracker))
                .collect::<Result<Vec<_>>>()?;
            let target_ident = match target {
                AssignTarget::Symbol(s) => syn::Ident::new(s, proc_macro2::Span::call_site()),
                _ => syn::Ident::new("item", proc_macro2::Span::call_site()),
            };
            Ok(quote! {
                while let Some(#target_ident) = #iter_expr.next().await {
                    #(#body_stmts)*
                }
            })
        }
        HirStmt::AsyncWith {
            context,
            body,
            target,
        } => {
            let context_expr = expr_to_rust_tokens(context)?;
            let body_stmts: Vec<_> = body
                .iter()
                .map(|s| stmt_to_rust_tokens_with_scope(s, scope_tracker))
                .collect::<Result<Vec<_>>>()?;
            if let Some(var_name) = target {
                let var_ident = syn::Ident::new(var_name, proc_macro2::Span::call_site());
                Ok(quote! {
                    {
                        let #var_ident = #context_expr;
                        #(#body_stmts)*
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let _ctx = #context_expr;
                        #(#body_stmts)*
                    }
                })
            }
        }
        HirStmt::Delete { targets } => {
            let delete_stmts: Vec<_> = targets
                .iter()
                .map(|target| match target {
                    AssignTarget::Symbol(s) => {
                        let ident = syn::Ident::new(s, proc_macro2::Span::call_site());
                        quote! { drop(#ident); }
                    }
                    AssignTarget::Index { base, index } => {
                        let base_tokens =
                            expr_to_rust_tokens(base).unwrap_or_else(|_| quote! { collection });
                        let index_tokens =
                            expr_to_rust_tokens(index).unwrap_or_else(|_| quote! { key });
                        quote! { #base_tokens.remove(&#index_tokens); }
                    }
                    AssignTarget::Attribute { value, attr } => {
                        let value_tokens =
                            expr_to_rust_tokens(value).unwrap_or_else(|_| quote! { obj });
                        let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
                        quote! { drop(#value_tokens.#attr_ident); }
                    }
                    _ => quote! { /* delete not supported for this target */ },
                })
                .collect();
            Ok(quote! { #(#delete_stmts)* })
        }
        HirStmt::AsyncFunctionDef {
            name,
            params,
            ret_type,
            body,
            ..
        } => {
            let fn_name = syn::Ident::new(name, proc_macro2::Span::call_site());
            let param_tokens: Vec<proc_macro2::TokenStream> = params
                .iter()
                .map(|p| {
                    let param_name = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
                    let param_type = type_to_rust_type(&p.ty);
                    quote! { #param_name: #param_type }
                })
                .collect();
            let return_type = type_to_rust_type(ret_type);
            let body_stmts: Vec<_> = body
                .iter()
                .map(|s| stmt_to_rust_tokens_with_scope(s, scope_tracker))
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! {
                async fn #fn_name(#(#param_tokens),*) -> #return_type {
                    #(#body_stmts)*
                }
            })
        }
        HirStmt::Match { subject, cases } => {
            let subject_tokens = expr_to_rust_tokens(subject)?;
            let arms: Vec<proc_macro2::TokenStream> = cases
                .iter()
                .map(|case| {
                    let pattern = pattern_to_rust_tokens(&case.pattern)?;
                    let body_stmts: Vec<_> = case
                        .body
                        .iter()
                        .map(|s| stmt_to_rust_tokens_with_scope(s, scope_tracker))
                        .collect::<Result<Vec<_>>>()?;
                    if let Some(guard) = &case.guard {
                        let guard_tokens = expr_to_rust_tokens(guard)?;
                        Ok(quote! { #pattern if #guard_tokens => { #(#body_stmts)* } })
                    } else {
                        Ok(quote! { #pattern => { #(#body_stmts)* } })
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! { match #subject_tokens { #(#arms)* } })
        }
    }
}

/// Convert a HIR pattern to Rust tokens for match arms
fn pattern_to_rust_tokens(pattern: &HirPattern) -> Result<proc_macro2::TokenStream> {
    match pattern {
        HirPattern::Value(expr) => {
            // For literal values in patterns
            match expr {
                HirExpr::Literal(lit) => match lit {
                    Literal::Int(i) => Ok(quote! { #i }),
                    Literal::String(s) => Ok(quote! { #s }),
                    Literal::Bool(b) => Ok(quote! { #b }),
                    Literal::None => Ok(quote! { None }),
                    _ => bail!("Unsupported literal in pattern"),
                },
                HirExpr::Var(name) => {
                    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                    Ok(quote! { #ident })
                }
                _ => bail!("Unsupported expression in pattern"),
            }
        }
        HirPattern::Singleton(lit) => match lit {
            Literal::None => Ok(quote! { None }),
            Literal::Bool(true) => Ok(quote! { true }),
            Literal::Bool(false) => Ok(quote! { false }),
            _ => bail!("Unsupported singleton in pattern"),
        },
        HirPattern::Sequence(patterns) => {
            let inner: Vec<proc_macro2::TokenStream> = patterns
                .iter()
                .map(pattern_to_rust_tokens)
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! { [#(#inner),*] })
        }
        HirPattern::Mapping { .. } => {
            bail!("Map pattern matching not supported in Rust")
        }
        HirPattern::Class {
            cls,
            patterns,
            kwd_attrs,
            kwd_patterns,
        } => {
            let cls_ident = syn::Ident::new(cls, proc_macro2::Span::call_site());
            if patterns.is_empty() && kwd_attrs.is_empty() {
                Ok(quote! { #cls_ident { .. } })
            } else if !kwd_attrs.is_empty() {
                let field_patterns: Vec<proc_macro2::TokenStream> = kwd_attrs
                    .iter()
                    .zip(kwd_patterns.iter())
                    .map(|(attr, pat)| {
                        let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
                        let pat_tokens = pattern_to_rust_tokens(pat)?;
                        Ok(quote! { #attr_ident: #pat_tokens })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(quote! { #cls_ident { #(#field_patterns),*, .. } })
            } else {
                let pos_patterns: Vec<proc_macro2::TokenStream> = patterns
                    .iter()
                    .map(pattern_to_rust_tokens)
                    .collect::<Result<Vec<_>>>()?;
                Ok(quote! { #cls_ident(#(#pos_patterns),*) })
            }
        }
        HirPattern::Star(name) => {
            if let Some(n) = name {
                let ident = syn::Ident::new(n, proc_macro2::Span::call_site());
                Ok(quote! { #ident @ .. })
            } else {
                Ok(quote! { .. })
            }
        }
        HirPattern::As { pattern, name } => match (pattern, name) {
            (Some(inner), Some(n)) => {
                let inner_pat = pattern_to_rust_tokens(inner)?;
                let ident = syn::Ident::new(n, proc_macro2::Span::call_site());
                Ok(quote! { #inner_pat @ #ident })
            }
            (None, Some(n)) => {
                let ident = syn::Ident::new(n, proc_macro2::Span::call_site());
                Ok(quote! { #ident })
            }
            (Some(inner), None) => pattern_to_rust_tokens(inner),
            (None, None) => Ok(quote! { _ }),
        },
        HirPattern::Or(patterns) => {
            let inner: Vec<proc_macro2::TokenStream> = patterns
                .iter()
                .map(pattern_to_rust_tokens)
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! { #(#inner)|* })
        }
        HirPattern::Wildcard => Ok(quote! { _ }),
    }
}

/// Convert binary expression to Rust tokens with special operator handling
///
fn binary_expr_to_rust_tokens(
    op: &BinOp,
    left: &HirExpr,
    right: &HirExpr,
) -> Result<proc_macro2::TokenStream> {
    let left_tokens = expr_to_rust_tokens(left)?;
    let right_tokens = expr_to_rust_tokens(right)?;

    // Special handling for specific operators
    match op {
        BinOp::Sub if is_len_call(left) => {
            // Use saturating_sub to prevent underflow when subtracting from array length
            Ok(quote! { #left_tokens.saturating_sub(#right_tokens) })
        }
        BinOp::FloorDiv => {
            // Python floor division semantics
            // For now, assume numeric types and use the integer floor division formula
            Ok(quote! {
                {
                    let a = #left_tokens;
                    let b = #right_tokens;
                    let q = a / b;
                    let r = a % b;
                    let r_negative = r < 0;
                    let b_negative = b < 0;
                    let r_nonzero = r != 0;
                    let signs_differ = r_negative != b_negative;
                    let needs_adjustment = r_nonzero && signs_differ;
                    if needs_adjustment { q - 1 } else { q }
                }
            })
        }
        _ => {
            let op_tokens = binop_to_rust_tokens(op);
            Ok(quote! { (#left_tokens #op_tokens #right_tokens) })
        }
    }
}

/// Convert function call expression to Rust tokens
fn call_expr_to_rust_tokens(func: &str, args: &[HirExpr]) -> Result<proc_macro2::TokenStream> {
    let func_ident = syn::Ident::new(func, proc_macro2::Span::call_site());
    let arg_tokens: Vec<_> = args
        .iter()
        .map(expr_to_rust_tokens)
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! { #func_ident(#(#arg_tokens),*) })
}

/// Convert list literal to Rust vec! macro
fn list_literal_to_rust_tokens(items: &[HirExpr]) -> Result<proc_macro2::TokenStream> {
    let item_tokens: Vec<_> = items
        .iter()
        .map(expr_to_rust_tokens)
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! { vec![#(#item_tokens),*] })
}

/// Convert dict literal to Rust HashMap
fn dict_literal_to_rust_tokens(items: &[(HirExpr, HirExpr)]) -> Result<proc_macro2::TokenStream> {
    let mut entries = Vec::new();
    for (key, value) in items {
        let key_tokens = expr_to_rust_tokens(key)?;
        let value_tokens = expr_to_rust_tokens(value)?;
        entries.push(quote! { (#key_tokens, #value_tokens) });
    }
    Ok(quote! {
        {
            let mut map = HashMap::new();
            #(map.insert #entries;)*
            map
        }
    })
}

/// Convert tuple literal to Rust tuple
fn tuple_literal_to_rust_tokens(items: &[HirExpr]) -> Result<proc_macro2::TokenStream> {
    let item_tokens: Vec<_> = items
        .iter()
        .map(expr_to_rust_tokens)
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! { (#(#item_tokens),*) })
}

/// Convert borrow expression to Rust reference
fn borrow_expr_to_rust_tokens(expr: &HirExpr, mutable: bool) -> Result<proc_macro2::TokenStream> {
    let expr_tokens = expr_to_rust_tokens(expr)?;
    if mutable {
        Ok(quote! { &mut #expr_tokens })
    } else {
        Ok(quote! { &#expr_tokens })
    }
}

/// Convert method call expression to Rust method call
fn method_call_to_rust_tokens(
    object: &HirExpr,
    method: &str,
    args: &[HirExpr],
) -> Result<proc_macro2::TokenStream> {
    let obj_tokens = expr_to_rust_tokens(object)?;
    let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
    let arg_tokens: Vec<_> = args
        .iter()
        .map(expr_to_rust_tokens)
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! { #obj_tokens.#method_ident(#(#arg_tokens),*) })
}

/// Convert slice expression to Rust slice notation
fn slice_expr_to_rust_tokens(
    base: &HirExpr,
    start: &Option<Box<HirExpr>>,
    stop: &Option<Box<HirExpr>>,
    step: &Option<Box<HirExpr>>,
) -> Result<proc_macro2::TokenStream> {
    let base_tokens = expr_to_rust_tokens(base)?;
    // Simple codegen - just use slice notation where possible
    match (start, stop, step) {
        (None, None, None) => Ok(quote! { #base_tokens.clone() }),
        (Some(start), Some(stop), None) => {
            let start_tokens = expr_to_rust_tokens(start)?;
            let stop_tokens = expr_to_rust_tokens(stop)?;
            Ok(quote! { #base_tokens[#start_tokens..#stop_tokens].to_vec() })
        }
        (Some(start), None, None) => {
            let start_tokens = expr_to_rust_tokens(start)?;
            Ok(quote! { #base_tokens[#start_tokens..].to_vec() })
        }
        (None, Some(stop), None) => {
            let stop_tokens = expr_to_rust_tokens(stop)?;
            Ok(quote! { #base_tokens[..#stop_tokens].to_vec() })
        }
        _ => {
            // For complex cases with step, fall back to method call
            Ok(quote! { slice_complex(#base_tokens) })
        }
    }
}

/// Convert list comprehension to Rust iterator chain
fn list_comp_to_rust_tokens(
    element: &HirExpr,
    target: &str,
    iter: &HirExpr,
    condition: &Option<Box<HirExpr>>,
) -> Result<proc_macro2::TokenStream> {
    let target_ident = syn::Ident::new(target, proc_macro2::Span::call_site());
    let iter_tokens = expr_to_rust_tokens(iter)?;
    let element_tokens = expr_to_rust_tokens(element)?;

    if let Some(cond) = condition {
        // With condition: iter().filter().map().collect()
        let cond_tokens = expr_to_rust_tokens(cond)?;
        Ok(quote! {
            #iter_tokens
                .into_iter()
                .filter(|#target_ident| #cond_tokens)
                .map(|#target_ident| #element_tokens)
                .collect::<Vec<_>>()
        })
    } else {
        // Without condition: iter().map().collect()
        Ok(quote! {
            #iter_tokens
                .into_iter()
                .map(|#target_ident| #element_tokens)
                .collect::<Vec<_>>()
        })
    }
}

/// Convert flattened list comprehension to Rust flat_map chain
fn flattened_list_comp_to_rust_tokens(
    element: &HirExpr,
    generators: &[HirComprehension],
) -> Result<proc_macro2::TokenStream> {
    if generators.len() < 2 {
        bail!("FlattenedListComp requires at least 2 generators");
    }

    let outer_gen = &generators[0];
    let outer_target = syn::Ident::new(&outer_gen.target, proc_macro2::Span::call_site());
    let outer_iter = expr_to_rust_tokens(&outer_gen.iter)?;

    let inner_gen = &generators[1];
    let inner_target = syn::Ident::new(&inner_gen.target, proc_macro2::Span::call_site());
    let inner_iter = expr_to_rust_tokens(&inner_gen.iter)?;

    let element_tokens = expr_to_rust_tokens(element)?;

    // Check if element is just the inner target variable
    let is_identity = matches!(element, HirExpr::Var(v) if v == &inner_gen.target);

    if is_identity {
        Ok(quote! {
            #outer_iter
                .into_iter()
                .flat_map(|#outer_target| #inner_iter.into_iter())
                .collect::<Vec<_>>()
        })
    } else {
        Ok(quote! {
            #outer_iter
                .into_iter()
                .flat_map(|#outer_target| #inner_iter.into_iter().map(|#inner_target| #element_tokens))
                .collect::<Vec<_>>()
        })
    }
}

/// Convert lambda expression to Rust closure
fn lambda_to_rust_tokens(params: &[String], body: &HirExpr) -> Result<proc_macro2::TokenStream> {
    // Convert parameters to identifiers
    let param_idents: Vec<proc_macro2::Ident> = params
        .iter()
        .map(|p| quote::format_ident!("{}", p))
        .collect();

    // Convert body
    let body_tokens = expr_to_rust_tokens(body)?;

    // Generate closure
    if params.is_empty() {
        Ok(quote! { || #body_tokens })
    } else {
        Ok(quote! { |#(#param_idents),*| #body_tokens })
    }
}

/// Convert set literal to Rust HashSet
fn set_literal_to_rust_tokens(items: &[HirExpr]) -> Result<proc_macro2::TokenStream> {
    let item_tokens: Vec<_> = items
        .iter()
        .map(expr_to_rust_tokens)
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! {
        {
            let mut set = HashSet::new();
            #(set.insert(#item_tokens);)*
            set
        }
    })
}

/// Convert frozenset literal to Rust Arc<HashSet>
fn frozen_set_to_rust_tokens(items: &[HirExpr]) -> Result<proc_macro2::TokenStream> {
    let item_tokens: Vec<_> = items
        .iter()
        .map(expr_to_rust_tokens)
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! {
        {
            let mut set = HashSet::new();
            #(set.insert(#item_tokens);)*
            std::sync::Arc::new(set)
        }
    })
}

/// Convert set comprehension to Rust iterator chain
fn set_comp_to_rust_tokens(
    element: &HirExpr,
    target: &str,
    iter: &HirExpr,
    condition: &Option<Box<HirExpr>>,
) -> Result<proc_macro2::TokenStream> {
    let target_ident = syn::Ident::new(target, proc_macro2::Span::call_site());
    let iter_tokens = expr_to_rust_tokens(iter)?;
    let element_tokens = expr_to_rust_tokens(element)?;

    if let Some(cond) = condition {
        // With condition: iter().filter().map().collect()
        let cond_tokens = expr_to_rust_tokens(cond)?;
        Ok(quote! {
            #iter_tokens
                .into_iter()
                .filter(|#target_ident| #cond_tokens)
                .map(|#target_ident| #element_tokens)
                .collect::<HashSet<_>>()
        })
    } else {
        // Without condition: iter().map().collect()
        Ok(quote! {
            #iter_tokens
                .into_iter()
                .map(|#target_ident| #element_tokens)
                .collect::<HashSet<_>>()
        })
    }
}

/// Convert dict comprehension to Rust iterator chain
fn dict_comp_to_rust_tokens(
    key: &HirExpr,
    value: &HirExpr,
    target: &str,
    iter: &HirExpr,
    condition: &Option<Box<HirExpr>>,
) -> Result<proc_macro2::TokenStream> {
    let target_ident = syn::Ident::new(target, proc_macro2::Span::call_site());
    let iter_tokens = expr_to_rust_tokens(iter)?;
    let key_tokens = expr_to_rust_tokens(key)?;
    let value_tokens = expr_to_rust_tokens(value)?;

    if let Some(cond) = condition {
        // With condition: iter().filter().map().collect()
        let cond_tokens = expr_to_rust_tokens(cond)?;
        Ok(quote! {
            #iter_tokens
                .into_iter()
                .filter(|#target_ident| #cond_tokens)
                .map(|#target_ident| (#key_tokens, #value_tokens))
                .collect::<HashMap<_, _>>()
        })
    } else {
        // Without condition: iter().map().collect()
        Ok(quote! {
            #iter_tokens
                .into_iter()
                .map(|#target_ident| (#key_tokens, #value_tokens))
                .collect::<HashMap<_, _>>()
        })
    }
}

fn expr_to_rust_tokens(expr: &HirExpr) -> Result<proc_macro2::TokenStream> {
    match expr {
        HirExpr::Literal(lit) => literal_to_rust_tokens(lit),
        HirExpr::Var(name) => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            Ok(quote! { #ident })
        }
        HirExpr::Binary { op, left, right } => binary_expr_to_rust_tokens(op, left, right),
        HirExpr::Unary { op, operand } => {
            let operand_tokens = expr_to_rust_tokens(operand)?;
            let op_tokens = unaryop_to_rust_tokens(op);
            Ok(quote! { (#op_tokens #operand_tokens) })
        }
        HirExpr::Call { func, args, .. } => call_expr_to_rust_tokens(func, args),
        HirExpr::Index { base, index } => {
            let base_tokens = expr_to_rust_tokens(base)?;
            let index_tokens = expr_to_rust_tokens(index)?;
            Ok(quote! { #base_tokens[#index_tokens] })
        }
        HirExpr::List(items) => list_literal_to_rust_tokens(items),
        HirExpr::Dict(items) => dict_literal_to_rust_tokens(items),
        HirExpr::Tuple(items) => tuple_literal_to_rust_tokens(items),
        HirExpr::Attribute { value, attr } => {
            let value_tokens = expr_to_rust_tokens(value)?;
            let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
            Ok(quote! { #value_tokens.#attr_ident })
        }
        HirExpr::Borrow { expr, mutable } => borrow_expr_to_rust_tokens(expr, *mutable),
        HirExpr::MethodCall {
            object,
            method,
            args,
            ..
        } => method_call_to_rust_tokens(object, method, args),
        HirExpr::Uninitialized => {
            bail!("Uninitialized expression cannot be converted to Rust tokens")
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => slice_expr_to_rust_tokens(base, start, stop, step),
        HirExpr::ListComp {
            element,
            target,
            iter,
            condition,
        } => list_comp_to_rust_tokens(element, target, iter, condition),
        HirExpr::FlattenedListComp {
            element,
            generators,
        } => flattened_list_comp_to_rust_tokens(element, generators),
        HirExpr::Lambda { params, body } => lambda_to_rust_tokens(params, body),
        HirExpr::Set(items) => set_literal_to_rust_tokens(items),
        HirExpr::FrozenSet(items) => frozen_set_to_rust_tokens(items),
        HirExpr::SetComp {
            element,
            target,
            iter,
            condition,
        } => set_comp_to_rust_tokens(element, target, iter, condition),
        HirExpr::DictComp {
            key,
            value,
            target,
            iter,
            condition,
        } => dict_comp_to_rust_tokens(key, value, target, iter, condition),
        HirExpr::Await { value } => {
            let value_tokens = expr_to_rust_tokens(value)?;
            Ok(quote! { #value_tokens.await })
        }
        HirExpr::Yield { value } => {
            if let Some(v) = value {
                let value_tokens = expr_to_rust_tokens(v)?;
                Ok(quote! { yield #value_tokens })
            } else {
                Ok(quote! { yield })
            }
        }
        HirExpr::FString { .. } => {
            anyhow::bail!("FString not yet implemented in codegen")
        }
        HirExpr::IfExpr { test, body, orelse } => {
            let test_tokens = expr_to_rust_tokens(test)?;
            let body_tokens = expr_to_rust_tokens(body)?;
            let orelse_tokens = expr_to_rust_tokens(orelse)?;
            Ok(quote! { if #test_tokens { #body_tokens } else { #orelse_tokens } })
        }
        HirExpr::SortByKey {
            iterable,
            key_params,
            key_body,
            reverse,
        } => {
            let iter_tokens = expr_to_rust_tokens(iterable)?;
            let body_tokens = expr_to_rust_tokens(key_body)?;

            if key_params.len() != 1 {
                bail!("sorted() key lambda must have exactly one parameter");
            }

            let param = syn::Ident::new(&key_params[0], proc_macro2::Span::call_site());

            if *reverse {
                // When reverse=True, sort and then reverse
                Ok(quote! {
                    {
                        let mut __sorted_result = #iter_tokens.clone();
                        __sorted_result.sort_by_key(|#param| #body_tokens);
                        __sorted_result.reverse();
                        __sorted_result
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let mut __sorted_result = #iter_tokens.clone();
                        __sorted_result.sort_by_key(|#param| #body_tokens);
                        __sorted_result
                    }
                })
            }
        }
        HirExpr::GeneratorExp { .. } => {
            // Note: Generator expressions are fully implemented in rust_gen.rs (v3.13.0, 20/20 tests).
            // This codegen.rs path is legacy HIR-to-Rust conversion, not used in main transpiler pipeline.
            // The primary implementation is in crates/depyler-core/src/rust_gen.rs::convert_generator_expression()
            bail!(
                "Generator expressions require rust_gen.rs (use DepylerPipeline instead of direct codegen)"
            )
        }
        HirExpr::NamedExpr { target, value } => {
            // Walrus operator: (x := expr) → { let x = expr; x }
            let value_tokens = expr_to_rust_tokens(value)?;
            let ident = syn::Ident::new(target, proc_macro2::Span::call_site());
            Ok(quote! {
                {
                    let #ident = #value_tokens;
                    #ident
                }
            })
        }
    }
}

fn literal_to_rust_tokens(lit: &Literal) -> Result<proc_macro2::TokenStream> {
    match lit {
        Literal::Int(i) => Ok(quote! { #i }),
        Literal::Float(f) => Ok(quote! { #f }),
        Literal::String(s) => Ok(quote! { #s.to_string() }),
        Literal::Bytes(b) => {
            // Generate byte string literal
            let byte_lit = proc_macro2::Literal::byte_string(b);
            Ok(quote! { #byte_lit })
        }
        Literal::Bool(b) => Ok(quote! { #b }),
        Literal::None => Ok(quote! { None }),
        Literal::Ellipsis => Ok(quote! { () }),
        Literal::Complex(real, imag) => Ok(quote! { Complex::new(#real, #imag) }),
    }
}

fn binop_to_rust_tokens(op: &BinOp) -> proc_macro2::TokenStream {
    match op {
        BinOp::Add => quote! { + },
        BinOp::Sub => quote! { - },
        BinOp::Mul => quote! { * },
        BinOp::Div => quote! { / },
        BinOp::FloorDiv => quote! { / }, // Note: not exact equivalent
        BinOp::Mod => quote! { % },
        BinOp::Pow => quote! { .pow },      // Special handling needed
        BinOp::MatMul => quote! { matmul }, // Special handling needed - no direct operator
        BinOp::Eq => quote! { == },
        BinOp::NotEq => quote! { != },
        BinOp::Lt => quote! { < },
        BinOp::LtEq => quote! { <= },
        BinOp::Gt => quote! { > },
        BinOp::GtEq => quote! { >= },
        BinOp::And => quote! { && },
        BinOp::Or => quote! { || },
        BinOp::BitAnd => quote! { & },
        BinOp::BitOr => quote! { | },
        BinOp::BitXor => quote! { ^ },
        BinOp::LShift => quote! { << },
        BinOp::RShift => quote! { >> },
        BinOp::In => quote! { .contains }, // Special handling needed
        BinOp::NotIn => quote! { .not_contains }, // Special handling needed
        BinOp::Is => quote! { == },
        BinOp::IsNot => quote! { != },
    }
}

fn unaryop_to_rust_tokens(op: &UnaryOp) -> proc_macro2::TokenStream {
    match op {
        UnaryOp::Not => quote! { ! },
        UnaryOp::Neg => quote! { - },
        UnaryOp::Pos => quote! { + },
        UnaryOp::BitNot => quote! { ! },
    }
}

fn prettify_rust_code(code: String) -> String {
    // Very basic formatting - in production, use rustfmt
    code.replace(" ; ", ";\n    ")
        .replace(" { ", " {\n    ")
        .replace(" } ", "\n}\n")
        .replace("} ;", "};")
        .replace(
            "use std :: collections :: HashMap ;",
            "use std::collections::HashMap;",
        )
        // Fix method call spacing
        .replace(" . ", ".")
        // Fix operators with spaces BEFORE paren fixes
        // The syn pretty-printer sometimes generates ` ! = ` instead of ` != `
        .replace(" ! = ", " != ")
        .replace(" ! = (", " != (")
        .replace(") ! = ", ") != ")
        .replace(") ! = (", ") != (")
        .replace(" ! =", " !=")
        .replace("! = ", "!= ")
        .replace(" = = ", " == ")
        .replace(" = =", " ==")
        .replace("= = ", "== ")
        .replace(" < =", " <=")
        .replace(" > =", " >=")
        // Now fix spacing around parentheses
        .replace(" (", "(")
        .replace(" )", ")")
        // Fix != operator patterns created by paren removal (AFTER paren fixes)
        .replace(") ! =(", ") !=(")
        .replace(")! = (", ")!=(")
        .replace(") ! = (", ") !=(")
        .replace("&&((", "&&(")
        .replace(")!=(", ") !=(")
        // Now fix the !( and != spacing
        .replace("!=(", "!= (")
        // Add spaces around comparison operators (after paren fixes)
        .replace("(r<0", "(r < 0")
        .replace("(b<0", "(b < 0")
        .replace("r<0)", "r < 0)")
        .replace("b<0)", "b < 0)")
        .replace("r<0;", "r < 0;")
        .replace("b<0;", "b < 0;")
        .replace(" = r<0", " = r < 0")
        .replace(" = b<0", " = b < 0")
        .replace(" = n>0", " = n > 0")
        .replace(" = n<0", " = n < 0")
        // Generic comparison operator spacing - multiple passes to catch all patterns
        .replace(">0;", " > 0;")
        .replace("<0;", " < 0;")
        .replace(">=0;", " >= 0;")
        .replace("<=0;", " <= 0;")
        // Fix comparison operators in assignments/conditions (second pass after parens removed)
        .replace("n>0", "n > 0")
        .replace("n<0", "n < 0")
        .replace("r>0", "r > 0")
        .replace("b>0", "b > 0")
        .replace("a>0", "a > 0")
        .replace("x>0", "x > 0")
        .replace("y>0", "y > 0")
        .replace("<0)", " < 0)")
        .replace("<0))", " < 0))")
        // Fix specific common patterns
        .replace(".len ()", ".len()")
        .replace(".push (", ".push(")
        .replace(".insert (", ".insert(")
        .replace(".get (", ".get(")
        .replace(".contains_key (", ".contains_key(")
        .replace(".to_string ()", ".to_string()")
        // Fix control flow keywords (AFTER paren fixes to ensure proper spacing)
        .replace("if(", "if ")
        .replace("while(", "while ")
        .replace("for(", "for ")
        .replace("match(", "match ")
        .replace("} else", "}\nelse")
        // FINAL PASS: Catch any remaining != operator spacing issues
        .replace(" ! = ", " != ")
        .replace(") ! = ", ") != ")
        .replace(" ! =(", " !=(")
        .replace(") ! =(", ") !=(")
        .replace(" ! =", " !=")
        .replace("! = ", "!= ")
        // Fix spacing around operators in some contexts
        .replace(" ::", "::")
        // Fix attribute spacing
        .replace("# [", "#[")
        // Fix type annotations
        .replace(" : ", ": ")
        .replace(";\n    }", "\n}")
}

/// Check if an expression is a len() call
fn is_len_call(expr: &HirExpr) -> bool {
    matches!(expr, HirExpr::Call { func, args , ..} if func == "len" && args.len() == 1)
}
