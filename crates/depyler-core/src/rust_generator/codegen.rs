use super::context::RustCodeGen as _;
use super::{context, type_gen};
use crate::analysis::borrowing_context::BorrowingStrategy;
use crate::hir::*;
use crate::optimizations::string_optimization::StringOptimizer;
use crate::types::type_mapper::AnnotationAwareTypeMapper;
use anyhow::{Result, bail};
use quote::{ToTokens, quote};
use std::collections::{HashMap, HashSet};
use syn::{self, parse_quote}; // bring trait into scope for .to_rust_tokens() / .to_rust_expr()
// Re-import type/infer helpers from type_gen so moved functions need no source changes.
use type_gen::{
    infer_constant_hir_type, infer_dict_kv_types, infer_list_element_type, infer_set_element_type,
    infer_single_expr_type, infer_tuple_type, infer_unary_type, is_const_safe_list_element,
    is_copy_rust_type, is_field_copy_type, is_heap_allocated_rust_type, tuple_element_needs_heap,
};

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
        AssignTarget::Starred(_) => {
            bail!("Starred expression can only appear inside tuple unpacking")
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
            AssignTarget::Starred(_) => {
                anyhow::bail!(
                    "Starred expression in tuple unpacking not yet supported in old codegen"
                )
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
            // This is handled by the main rust_generator module
            // This codegen.rs module is a legacy simplified codegen path
            // For now, just return empty - nested functions use the main rust_generator path
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
            // Python floor division: rounds towards negative infinity
            if matches!(left, HirExpr::Var(_) | HirExpr::Literal(_))
                && matches!(right, HirExpr::Var(_) | HirExpr::Literal(_))
            {
                Ok(quote! {
                    {
                        let d = #left_tokens / #right_tokens;
                        let r = #left_tokens % #right_tokens;
                        if r != 0 && (#left_tokens ^ #right_tokens) < 0 { d - 1 } else { d }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let a = #left_tokens;
                        let b = #right_tokens;
                        let d = a / b;
                        let r = a % b;
                        if r != 0 && (a ^ b) < 0 { d - 1 } else { d }
                    }
                })
            }
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
            // Note: Generator expressions are fully implemented in rust_generator.rs.
            // This codegen.rs path is legacy HIR-to-Rust conversion, not used in main transpiler pipeline.
            bail!(
                "Generator expressions require rust_generator.rs (use QuantSimPipeline instead of direct codegen)"
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

// ============================================================================
// Pipeline orchestration helpers (moved from rust_generator.rs §31)
// ============================================================================

/// Performs string optimization analysis on all functions.
fn analyze_string_optimization(ctx: &mut super::CodeGenContext, functions: &[HirFunction]) {
    for func in functions {
        ctx.string_optimizer.analyze_function(func);
    }
}

/// Pre-populate function_param_borrows for all functions.
///
/// Two-pass interprocedural analysis: first determines direct mutations,
/// then propagates mutability requirements via fixpoint iteration.
fn populate_function_param_borrows(
    functions: &[HirFunction],
    ctx: &mut super::CodeGenContext,
) -> Result<()> {
    use crate::analysis::lifetime_analysis::LifetimeInference;

    // Phase 1: Initial analysis - determine direct mutations
    for func in functions {
        let mut lifetime_inference = LifetimeInference::new();
        let lifetime_result = lifetime_inference
            .apply_elision_rules_with_interprocedural(
                func,
                ctx.type_mapper,
                ctx.interprocedural_analysis,
                &ctx.enum_names,
                &ctx.copy_structs,
            )
            .unwrap_or_else(|| {
                lifetime_inference.analyze_function_with_interprocedural(
                    func,
                    ctx.type_mapper,
                    ctx.interprocedural_analysis,
                    &ctx.enum_names,
                    &ctx.copy_structs,
                )
            });

        let mut param_borrows = Vec::new();
        let mut param_strategies = Vec::new();

        for param in &func.params {
            let inferred = lifetime_result
                .param_lifetimes
                .get(&param.name)
                .map(|inf| (inf.should_borrow, inf.needs_mut))
                .unwrap_or((false, false));

            let strategy = lifetime_result
                .borrowing_strategies
                .get(&param.name)
                .cloned()
                .unwrap_or(BorrowingStrategy::TakeOwnership);

            param_borrows.push(context::ParamBorrowInfo {
                should_borrow: inferred.0,
                needs_mut: inferred.1,
                takes_ownership: matches!(strategy, BorrowingStrategy::TakeOwnership),
            });
            param_strategies.push(strategy);
        }

        for (idx, param) in func.params.iter().enumerate() {
            if param_borrows[idx].should_borrow
                && !param_borrows[idx].needs_mut
                && parameter_has_field_mutations(func, &param.name)
            {
                param_borrows[idx].needs_mut = true;
            }
        }

        ctx.function_param_borrows
            .insert(func.name.clone(), param_borrows);
        ctx.function_param_strategies
            .insert(func.name.clone(), param_strategies);
    }

    // Phase 2: Interprocedural fixpoint propagation of mutability requirements
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 10;

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        for func in functions {
            let calls = find_function_calls(&func.body);

            for (called_func_name, arg_exprs) in calls {
                let callee_borrows = ctx.function_param_borrows.get(&called_func_name).cloned();

                if let Some(callee_borrows) = callee_borrows {
                    for (arg_idx, arg_expr) in arg_exprs.iter().enumerate() {
                        if let HirExpr::Var(var_name) = arg_expr {
                            if let Some(param_idx) =
                                func.params.iter().position(|p| &p.name == var_name)
                            {
                                let current_caller_info = ctx
                                    .function_param_borrows
                                    .get(&func.name)
                                    .and_then(|borrows| borrows.get(param_idx))
                                    .cloned();

                                if let Some(callee_info) = callee_borrows.get(arg_idx) {
                                    let currently_takes_ownership = current_caller_info
                                        .as_ref()
                                        .map(|info| info.takes_ownership)
                                        .unwrap_or(false);

                                    let param_has_mutations =
                                        parameter_has_field_mutations(func, var_name);

                                    let should_upgrade_to_borrow = currently_takes_ownership;
                                    let should_be_mut =
                                        param_has_mutations || callee_info.needs_mut;

                                    if should_upgrade_to_borrow {
                                        let param_type = &func.params[param_idx].ty;
                                        let rust_type = ctx.type_mapper.map_type(param_type);
                                        if is_copy_rust_type(
                                            &rust_type,
                                            &ctx.enum_names,
                                            &ctx.copy_structs,
                                        ) {
                                            // Copy type: leave as TakeOwnership (pass by value)
                                        } else if let Some(caller_borrows) =
                                            ctx.function_param_borrows.get_mut(&func.name)
                                        {
                                            if let Some(caller_info) =
                                                caller_borrows.get_mut(param_idx)
                                            {
                                                if caller_info.takes_ownership {
                                                    caller_info.should_borrow = true;
                                                    caller_info.takes_ownership = false;
                                                    caller_info.needs_mut = should_be_mut;
                                                    changed = true;
                                                }
                                            }
                                        }
                                    } else if callee_info.needs_mut {
                                        let param_type = &func.params[param_idx].ty;
                                        let rust_type = ctx.type_mapper.map_type(param_type);
                                        if !is_copy_rust_type(
                                            &rust_type,
                                            &ctx.enum_names,
                                            &ctx.copy_structs,
                                        ) {
                                            if let Some(caller_borrows) =
                                                ctx.function_param_borrows.get_mut(&func.name)
                                            {
                                                if let Some(caller_info) =
                                                    caller_borrows.get_mut(param_idx)
                                                {
                                                    if !caller_info.needs_mut {
                                                        caller_info.needs_mut = true;
                                                        changed = true;
                                                    }
                                                }
                                            }
                                        }
                                    } else if callee_info.should_borrow
                                        && !callee_info.takes_ownership
                                    {
                                        let param_type = &func.params[param_idx].ty;
                                        let rust_type = ctx.type_mapper.map_type(param_type);
                                        if !is_copy_rust_type(
                                            &rust_type,
                                            &ctx.enum_names,
                                            &ctx.copy_structs,
                                        ) {
                                            if let Some(caller_borrows) =
                                                ctx.function_param_borrows.get_mut(&func.name)
                                            {
                                                if let Some(caller_info) =
                                                    caller_borrows.get_mut(param_idx)
                                                {
                                                    if !caller_info.should_borrow {
                                                        caller_info.should_borrow = true;
                                                        caller_info.takes_ownership = false;
                                                        changed = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Detect functions that should NOT use the reference-return optimisation.
fn compute_functions_suppress_ref_return(
    functions: &[HirFunction],
    ctx: &mut super::CodeGenContext,
) {
    use crate::rust_generator::func_gen::collect_indexed_field_vars;

    let mut funcs_with_indexed_params: HashMap<String, HashSet<String>> = HashMap::new();
    for func in functions {
        let borrows = match ctx.function_param_borrows.get(&func.name) {
            Some(b) => b,
            None => continue,
        };
        let borrowed_params: HashSet<String> = func
            .params
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                borrows
                    .get(*i)
                    .map(|b| b.should_borrow && !b.needs_mut)
                    .unwrap_or(false)
            })
            .map(|(_, p)| p.name.clone())
            .collect();
        if borrowed_params.is_empty() {
            continue;
        }
        let mut ref_var_sources: HashMap<String, String> = HashMap::new();
        collect_indexed_field_vars(&func.body, &borrowed_params, &mut ref_var_sources);
        if !ref_var_sources.is_empty() {
            let source_params: HashSet<String> = ref_var_sources.values().cloned().collect();
            funcs_with_indexed_params.insert(func.name.clone(), source_params);
        }
    }

    for caller in functions {
        let caller_borrows = match ctx.function_param_borrows.get(&caller.name) {
            Some(b) => b.clone(),
            None => continue,
        };
        let mut_params: HashSet<String> = caller
            .params
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                caller_borrows
                    .get(*i)
                    .map(|b| b.should_borrow && b.needs_mut)
                    .unwrap_or(false)
            })
            .map(|(_, p)| p.name.clone())
            .collect();
        if mut_params.is_empty() {
            continue;
        }

        let calls = find_function_calls(&caller.body);
        for (callee_name, args) in &calls {
            let indexed_params = match funcs_with_indexed_params.get(callee_name) {
                Some(p) => p,
                None => continue,
            };
            let callee_func = match functions.iter().find(|f| &f.name == callee_name) {
                Some(f) => f,
                None => continue,
            };
            for (arg_idx, arg_expr) in args.iter().enumerate() {
                if let HirExpr::Var(var_name) = arg_expr {
                    if !mut_params.contains(var_name) {
                        continue;
                    }
                    let callee_param_name = match callee_func.params.get(arg_idx) {
                        Some(p) => &p.name,
                        None => continue,
                    };
                    if indexed_params.contains(callee_param_name) {
                        ctx.functions_suppress_ref_return
                            .insert(callee_name.clone());
                    }
                }
            }
        }
    }
}

// ---- Mutation / alias analysis helpers ------------------------------------

fn parameter_has_field_mutations(func: &HirFunction, param_name: &str) -> bool {
    let mut aliases: HashSet<String> = HashSet::new();
    aliases.insert(param_name.to_string());
    collect_param_aliases(&func.body, param_name, &mut aliases);
    check_stmts_for_alias_mutation(&func.body, &aliases)
}

fn collect_param_aliases(stmts: &[HirStmt], param_name: &str, aliases: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                value,
                ..
            } => {
                if is_derived_from_param(value, param_name) {
                    aliases.insert(name.clone());
                }
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_param_aliases(then_body, param_name, aliases);
                if let Some(else_body) = else_body {
                    collect_param_aliases(else_body, param_name, aliases);
                }
            }
            HirStmt::While { body, .. } => {
                collect_param_aliases(body, param_name, aliases);
            }
            HirStmt::For {
                target, iter, body, ..
            } => {
                if let AssignTarget::Symbol(name) = target {
                    if is_derived_from_param(iter, param_name) {
                        aliases.insert(name.clone());
                    }
                }
                collect_param_aliases(body, param_name, aliases);
            }
            _ => {}
        }
    }
}

fn is_derived_from_param(expr: &HirExpr, param_name: &str) -> bool {
    match expr {
        HirExpr::Attribute { value, .. } | HirExpr::Index { base: value, .. } => {
            matches!(extract_root_var_from_expr(value), Some(name) if name == param_name)
                || is_derived_from_param(value, param_name)
        }
        HirExpr::Slice { base, .. } => is_derived_from_param(base, param_name),
        HirExpr::IfExpr { body, orelse, .. } => {
            is_derived_from_param(body, param_name) && is_derived_from_param(orelse, param_name)
        }
        HirExpr::Call { func, args, .. } => {
            matches!(
                func.as_str(),
                "enumerate" | "reversed" | "sorted" | "iter" | "list"
            ) && args
                .first()
                .map_or(false, |a| is_derived_from_param(a, param_name))
        }
        HirExpr::Var(name) => name == param_name,
        _ => false,
    }
}

fn extract_root_var_from_expr(expr: &HirExpr) -> Option<String> {
    match expr {
        HirExpr::Var(name) => Some(name.clone()),
        HirExpr::Attribute { value, .. } | HirExpr::Index { base: value, .. } => {
            extract_root_var_from_expr(value)
        }
        _ => None,
    }
}

fn check_stmts_for_alias_mutation(stmts: &[HirStmt], aliases: &HashSet<String>) -> bool {
    stmts
        .iter()
        .any(|stmt| check_stmt_for_alias_mutation(stmt, aliases))
}

fn check_stmt_for_alias_mutation(stmt: &HirStmt, aliases: &HashSet<String>) -> bool {
    match stmt {
        HirStmt::Assign { target, .. } => match target {
            AssignTarget::Attribute { value, .. }
            | AssignTarget::Index { base: value, .. }
            | AssignTarget::Slice { base: value, .. } => {
                matches!(extract_root_var_from_expr(value), Some(name) if aliases.contains(&name))
            }
            _ => false,
        },
        HirStmt::Expr(expr) => check_expr_for_alias_mutation(expr, aliases),
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            check_stmts_for_alias_mutation(then_body, aliases)
                || else_body
                    .as_ref()
                    .map(|body| check_stmts_for_alias_mutation(body, aliases))
                    .unwrap_or(false)
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            check_stmts_for_alias_mutation(body, aliases)
        }
        _ => false,
    }
}

fn check_expr_for_alias_mutation(expr: &HirExpr, aliases: &HashSet<String>) -> bool {
    match expr {
        HirExpr::MethodCall { object, method, .. } => {
            if is_known_mutating_method(method) {
                if let Some(root) = extract_root_var_from_expr(object) {
                    return aliases.contains(&root);
                }
            }
            false
        }
        HirExpr::Binary { left, right, .. } => {
            check_expr_for_alias_mutation(left, aliases)
                || check_expr_for_alias_mutation(right, aliases)
        }
        HirExpr::Unary { operand, .. } => check_expr_for_alias_mutation(operand, aliases),
        HirExpr::Call { args, .. } => args
            .iter()
            .any(|arg| check_expr_for_alias_mutation(arg, aliases)),
        _ => false,
    }
}

fn is_known_mutating_method(method: &str) -> bool {
    matches!(
        method,
        "append"
            | "extend"
            | "insert"
            | "remove"
            | "pop"
            | "clear"
            | "sort"
            | "reverse"
            | "update"
            | "add"
            | "discard"
            | "setdefault"
            | "popitem"
            | "push"
            | "push_front"
            | "pop_front"
            | "push_back"
            | "pop_back"
    )
}

// ---- Call-graph traversal -------------------------------------------------

fn find_function_calls(stmts: &[HirStmt]) -> Vec<(String, Vec<HirExpr>)> {
    let mut calls = Vec::new();

    fn scan_expr(expr: &HirExpr, calls: &mut Vec<(String, Vec<HirExpr>)>) {
        match expr {
            HirExpr::Call { func, args, .. } => {
                calls.push((func.clone(), args.clone()));
                for arg in args {
                    scan_expr(arg, calls);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                scan_expr(left, calls);
                scan_expr(right, calls);
            }
            HirExpr::Unary { operand, .. } => scan_expr(operand, calls),
            HirExpr::MethodCall { object, args, .. } => {
                scan_expr(object, calls);
                for arg in args {
                    scan_expr(arg, calls);
                }
            }
            HirExpr::Attribute { value, .. } => scan_expr(value, calls),
            HirExpr::Index { base, index } => {
                scan_expr(base, calls);
                scan_expr(index, calls);
            }
            HirExpr::IfExpr { test, body, orelse } => {
                scan_expr(test, calls);
                scan_expr(body, calls);
                scan_expr(orelse, calls);
            }
            HirExpr::List(exprs) | HirExpr::Tuple(exprs) | HirExpr::Set(exprs) => {
                for e in exprs {
                    scan_expr(e, calls);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    scan_expr(k, calls);
                    scan_expr(v, calls);
                }
            }
            HirExpr::Lambda { body, .. } => scan_expr(body, calls),
            HirExpr::ListComp { element, .. } | HirExpr::SetComp { element, .. } => {
                scan_expr(element, calls);
            }
            HirExpr::DictComp { key, value, .. } => {
                scan_expr(key, calls);
                scan_expr(value, calls);
            }
            _ => {}
        }
    }

    fn scan_stmt(stmt: &HirStmt, calls: &mut Vec<(String, Vec<HirExpr>)>) {
        match stmt {
            HirStmt::Expr(expr) => scan_expr(expr, calls),
            HirStmt::Assign { value, .. } => scan_expr(value, calls),
            HirStmt::Return(Some(expr)) => scan_expr(expr, calls),
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                scan_expr(condition, calls);
                for s in then_body {
                    scan_stmt(s, calls);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        scan_stmt(s, calls);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                scan_expr(condition, calls);
                for s in body {
                    scan_stmt(s, calls);
                }
            }
            HirStmt::For { iter, body, .. } => {
                scan_expr(iter, calls);
                for s in body {
                    scan_stmt(s, calls);
                }
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                for s in body {
                    scan_stmt(s, calls);
                }
                for handler in handlers {
                    for s in &handler.body {
                        scan_stmt(s, calls);
                    }
                }
                if let Some(else_stmts) = orelse {
                    for s in else_stmts {
                        scan_stmt(s, calls);
                    }
                }
                if let Some(final_stmts) = finalbody {
                    for s in final_stmts {
                        scan_stmt(s, calls);
                    }
                }
            }
            _ => {}
        }
    }

    for stmt in stmts {
        scan_stmt(stmt, &mut calls);
    }
    calls
}

// ---- Validator / argparse analysis ----------------------------------------

fn analyze_validators(
    ctx: &mut super::CodeGenContext,
    functions: &[HirFunction],
    constants: &[HirConstant],
) {
    for func in functions {
        scan_stmts_for_validators(&func.body, ctx);
    }
    for constant in constants {
        scan_expr_for_validators(&constant.value, ctx);
    }
}

fn scan_stmts_for_validators(stmts: &[HirStmt], ctx: &mut super::CodeGenContext) {
    for stmt in stmts {
        match stmt {
            HirStmt::Expr(expr) => scan_expr_for_validators(expr, ctx),
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                scan_stmts_for_validators(then_body, ctx);
                if let Some(else_stmts) = else_body {
                    scan_stmts_for_validators(else_stmts, ctx);
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                scan_stmts_for_validators(body, ctx);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                scan_stmts_for_validators(body, ctx);
                for handler in handlers {
                    scan_stmts_for_validators(&handler.body, ctx);
                }
                if let Some(else_stmts) = orelse {
                    scan_stmts_for_validators(else_stmts, ctx);
                }
                if let Some(final_stmts) = finalbody {
                    scan_stmts_for_validators(final_stmts, ctx);
                }
            }
            _ => {}
        }
    }
}

fn scan_expr_for_validators(expr: &HirExpr, ctx: &mut super::CodeGenContext) {
    match expr {
        HirExpr::MethodCall { method, kwargs, .. } if method == "add_argument" => {
            for (kw_name, kw_value) in kwargs {
                if kw_name == "type" {
                    if let HirExpr::Var(type_name) = kw_value {
                        if !matches!(type_name.as_str(), "str" | "int" | "float" | "Path") {
                            ctx.validator_functions.insert(type_name.clone());
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

// ---- Mutable-variable analysis --------------------------------------------

pub(super) fn analyze_mutable_vars(
    stmts: &[HirStmt],
    ctx: &mut super::CodeGenContext,
    params: &[HirParam],
) {
    let mut declared = HashSet::new();
    let mut loop_var_origins: HashMap<String, String> = HashMap::new();
    let mut field_source_origins: HashMap<String, String> = HashMap::new();

    for param in params {
        declared.insert(param.name.clone());
    }

    fn extract_param_from_expr(expr: &HirExpr, declared: &HashSet<String>) -> Option<String> {
        match expr {
            HirExpr::Var(name) => {
                if declared.contains(name) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            HirExpr::Attribute { value, .. } => extract_param_from_expr(value, declared),
            HirExpr::Index { base, .. } => extract_param_from_expr(base, declared),
            _ => None,
        }
    }

    fn extract_root_var(expr: &HirExpr) -> Option<String> {
        match expr {
            HirExpr::Var(name) => Some(name.clone()),
            HirExpr::Attribute { value, .. } => extract_root_var(value),
            HirExpr::Index { base, .. } => extract_root_var(base),
            _ => None,
        }
    }

    fn extract_param_from_attribute_source(
        expr: &HirExpr,
        params: &HashSet<String>,
    ) -> Option<String> {
        match expr {
            HirExpr::Attribute { value, .. } => {
                if let Some(root) = extract_root_var(value) {
                    if params.contains(&root) {
                        return Some(root);
                    }
                }
                None
            }
            HirExpr::Index { base, .. } => extract_param_from_attribute_source(base, params),
            HirExpr::IfExpr { body, orelse, .. } => {
                let body_param = extract_param_from_attribute_source(body, params)?;
                let orelse_param = extract_param_from_attribute_source(orelse, params)?;
                if body_param == orelse_param {
                    Some(body_param)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn analyze_expr_for_mutations(
        expr: &HirExpr,
        mutable: &mut HashSet<String>,
        var_types: &HashMap<String, String>,
        mutating_methods: &HashMap<String, HashSet<String>>,
    ) {
        match expr {
            HirExpr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                let is_mut = if is_mutating_method(method) {
                    true
                } else if let HirExpr::Var(var_name) = &**object {
                    if let Some(class_name) = var_types.get(var_name) {
                        mutating_methods
                            .get(class_name)
                            .map(|m| m.contains(method))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                };
                if is_mut {
                    if let HirExpr::Var(var_name) = &**object {
                        mutable.insert(var_name.clone());
                    }
                }
                analyze_expr_for_mutations(object, mutable, var_types, mutating_methods);
                for arg in args {
                    analyze_expr_for_mutations(arg, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                analyze_expr_for_mutations(left, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(right, mutable, var_types, mutating_methods);
            }
            HirExpr::Unary { operand, .. } => {
                analyze_expr_for_mutations(operand, mutable, var_types, mutating_methods);
            }
            HirExpr::Call { args, .. } => {
                for arg in args {
                    analyze_expr_for_mutations(arg, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::IfExpr { test, body, orelse } => {
                analyze_expr_for_mutations(test, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(body, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(orelse, mutable, var_types, mutating_methods);
            }
            HirExpr::List(items)
            | HirExpr::Tuple(items)
            | HirExpr::Set(items)
            | HirExpr::FrozenSet(items) => {
                for item in items {
                    analyze_expr_for_mutations(item, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::Dict(pairs) => {
                for (key, value) in pairs {
                    analyze_expr_for_mutations(key, mutable, var_types, mutating_methods);
                    analyze_expr_for_mutations(value, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::Index { base, index } => {
                analyze_expr_for_mutations(base, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(index, mutable, var_types, mutating_methods);
            }
            HirExpr::Attribute { value, .. } => {
                analyze_expr_for_mutations(value, mutable, var_types, mutating_methods);
            }
            _ => {}
        }
    }

    fn is_mutating_method(method: &str) -> bool {
        matches!(
            method,
            "append"
                | "extend"
                | "insert"
                | "remove"
                | "pop"
                | "clear"
                | "reverse"
                | "sort"
                | "update"
                | "setdefault"
                | "popitem"
                | "add"
                | "discard"
                | "difference_update"
                | "intersection_update"
        )
    }

    fn analyze_stmt(
        stmt: &HirStmt,
        declared: &mut HashSet<String>,
        mutable: &mut HashSet<String>,
        var_types: &mut HashMap<String, String>,
        mutating_methods: &HashMap<String, HashSet<String>>,
        loop_var_origins: &mut HashMap<String, String>,
        field_source_origins: &mut HashMap<String, String>,
    ) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                analyze_expr_for_mutations(value, mutable, var_types, mutating_methods);
                match target {
                    AssignTarget::Symbol(name) => {
                        if let HirExpr::Call { func, .. } = value {
                            var_types.insert(name.clone(), func.clone());
                        }
                        if let Some(param_name) =
                            extract_param_from_attribute_source(value, declared)
                        {
                            field_source_origins.insert(name.clone(), param_name);
                        }
                        if declared.contains(name) {
                            mutable.insert(name.clone());
                        } else {
                            declared.insert(name.clone());
                        }
                    }
                    AssignTarget::Tuple(targets) => {
                        for t in targets {
                            if let AssignTarget::Symbol(name) = t {
                                if declared.contains(name) {
                                    mutable.insert(name.clone());
                                } else {
                                    declared.insert(name.clone());
                                }
                            }
                        }
                    }
                    AssignTarget::Attribute { value: obj, .. } => {
                        if let Some(var_name) = extract_root_var(obj) {
                            if let Some(param_name) = loop_var_origins.get(&var_name) {
                                mutable.insert(param_name.clone());
                            } else {
                                mutable.insert(var_name.clone());
                            }
                            if let Some(source_param) = field_source_origins.get(&var_name) {
                                mutable.insert(source_param.clone());
                            }
                        }
                    }
                    AssignTarget::Index { base, .. } => {
                        if let Some(var_name) = extract_root_var(base) {
                            if let Some(param_name) = loop_var_origins.get(&var_name) {
                                mutable.insert(param_name.clone());
                            } else {
                                mutable.insert(var_name.clone());
                            }
                            if let Some(source_param) = field_source_origins.get(&var_name) {
                                mutable.insert(source_param.clone());
                            }
                        }
                    }
                    AssignTarget::Slice { base, .. } => {
                        if let Some(var_name) = extract_root_var(base) {
                            if let Some(param_name) = loop_var_origins.get(&var_name) {
                                mutable.insert(param_name.clone());
                            } else {
                                mutable.insert(var_name.clone());
                            }
                            if let Some(source_param) = field_source_origins.get(&var_name) {
                                mutable.insert(source_param.clone());
                            }
                        }
                    }
                    AssignTarget::Starred(_) => {}
                }
            }
            HirStmt::Expr(expr) => {
                analyze_expr_for_mutations(expr, mutable, var_types, mutating_methods);
            }
            HirStmt::Return(Some(expr)) => {
                analyze_expr_for_mutations(expr, mutable, var_types, mutating_methods);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                analyze_expr_for_mutations(condition, mutable, var_types, mutating_methods);
                for stmt in then_body {
                    analyze_stmt(
                        stmt,
                        declared,
                        mutable,
                        var_types,
                        mutating_methods,
                        loop_var_origins,
                        field_source_origins,
                    );
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        analyze_stmt(
                            stmt,
                            declared,
                            mutable,
                            var_types,
                            mutating_methods,
                            loop_var_origins,
                            field_source_origins,
                        );
                    }
                }
            }
            HirStmt::While {
                condition, body, ..
            } => {
                analyze_expr_for_mutations(condition, mutable, var_types, mutating_methods);
                for stmt in body {
                    analyze_stmt(
                        stmt,
                        declared,
                        mutable,
                        var_types,
                        mutating_methods,
                        loop_var_origins,
                        field_source_origins,
                    );
                }
            }
            HirStmt::For { target, iter, body } => {
                if let AssignTarget::Symbol(loop_var) = target {
                    if !matches!(iter, HirExpr::Var(_)) {
                        if let Some(param_name) = extract_param_from_expr(iter, declared) {
                            loop_var_origins.insert(loop_var.clone(), param_name);
                        }
                    }
                }
                for stmt in body {
                    analyze_stmt(
                        stmt,
                        declared,
                        mutable,
                        var_types,
                        mutating_methods,
                        loop_var_origins,
                        field_source_origins,
                    );
                }
            }
            _ => {}
        }
    }

    let mut var_types = HashMap::new();
    let mutating_methods = &ctx.mutating_methods.clone();
    for stmt in stmts {
        analyze_stmt(
            stmt,
            &mut declared,
            &mut ctx.mutable_vars,
            &mut var_types,
            mutating_methods,
            &mut loop_var_origins,
            &mut field_source_origins,
        );
    }

    mark_mut_ref_call_args(stmts, &mut ctx.mutable_vars, &ctx.function_param_borrows);
    analyze_mut_ref_index_vars(stmts, ctx);
}

fn analyze_mut_ref_index_vars(stmts: &[HirStmt], ctx: &mut super::CodeGenContext) {
    let mut index_source_vars = HashSet::new();
    let mut field_mutated_vars = HashSet::new();
    let mut reassigned_vars = HashSet::new();
    let mut declared = HashSet::new();

    fn extract_root_var(expr: &HirExpr) -> Option<String> {
        match expr {
            HirExpr::Var(name) => Some(name.clone()),
            HirExpr::Attribute { value, .. } => extract_root_var(value),
            HirExpr::Index { base, .. } => extract_root_var(base),
            _ => None,
        }
    }

    fn scan_stmts(
        stmts: &[HirStmt],
        index_source_vars: &mut HashSet<String>,
        field_mutated_vars: &mut HashSet<String>,
        reassigned_vars: &mut HashSet<String>,
        declared: &mut HashSet<String>,
    ) {
        for stmt in stmts {
            scan_stmt(
                stmt,
                index_source_vars,
                field_mutated_vars,
                reassigned_vars,
                declared,
            );
        }
    }

    fn scan_stmt(
        stmt: &HirStmt,
        index_source_vars: &mut HashSet<String>,
        field_mutated_vars: &mut HashSet<String>,
        reassigned_vars: &mut HashSet<String>,
        declared: &mut HashSet<String>,
    ) {
        match stmt {
            HirStmt::Assign { target, value, .. } => match target {
                AssignTarget::Symbol(name) => {
                    if matches!(value, HirExpr::Index { .. }) {
                        index_source_vars.insert(name.clone());
                    }
                    if declared.contains(name) {
                        reassigned_vars.insert(name.clone());
                    } else {
                        declared.insert(name.clone());
                    }
                }
                AssignTarget::Attribute { value: obj, .. } => {
                    if let Some(root) = extract_root_var(obj) {
                        field_mutated_vars.insert(root);
                    }
                }
                AssignTarget::Index { base, .. } => {
                    if let Some(root) = extract_root_var(base) {
                        field_mutated_vars.insert(root);
                    }
                }
                _ => {}
            },
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                scan_stmts(
                    then_body,
                    index_source_vars,
                    field_mutated_vars,
                    reassigned_vars,
                    declared,
                );
                if let Some(else_stmts) = else_body {
                    scan_stmts(
                        else_stmts,
                        index_source_vars,
                        field_mutated_vars,
                        reassigned_vars,
                        declared,
                    );
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                scan_stmts(
                    body,
                    index_source_vars,
                    field_mutated_vars,
                    reassigned_vars,
                    declared,
                );
            }
            _ => {}
        }
    }

    scan_stmts(
        &stmts,
        &mut index_source_vars,
        &mut field_mutated_vars,
        &mut reassigned_vars,
        &mut declared,
    );
    ctx.mut_ref_index_vars = index_source_vars
        .into_iter()
        .filter(|v| field_mutated_vars.contains(v) && !reassigned_vars.contains(v))
        .collect();
}

fn mark_mut_ref_call_args(
    stmts: &[HirStmt],
    mutable: &mut HashSet<String>,
    function_param_borrows: &HashMap<String, Vec<context::ParamBorrowInfo>>,
) {
    for stmt in stmts {
        mark_mut_ref_call_args_in_stmt(stmt, mutable, function_param_borrows);
    }
}

fn mark_mut_ref_call_args_in_expr(
    expr: &HirExpr,
    mutable: &mut HashSet<String>,
    function_param_borrows: &HashMap<String, Vec<context::ParamBorrowInfo>>,
) {
    match expr {
        HirExpr::Call { func, args, .. } => {
            if let Some(borrows) = function_param_borrows.get(func.as_str()) {
                for (idx, arg) in args.iter().enumerate() {
                    if let HirExpr::Var(var_name) = arg {
                        if let Some(info) = borrows.get(idx) {
                            if info.should_borrow && info.needs_mut {
                                mutable.insert(var_name.clone());
                            }
                        }
                    }
                }
            }
            for arg in args {
                mark_mut_ref_call_args_in_expr(arg, mutable, function_param_borrows);
            }
        }
        HirExpr::MethodCall { object, args, .. } => {
            mark_mut_ref_call_args_in_expr(object, mutable, function_param_borrows);
            for arg in args {
                mark_mut_ref_call_args_in_expr(arg, mutable, function_param_borrows);
            }
        }
        HirExpr::Binary { left, right, .. } => {
            mark_mut_ref_call_args_in_expr(left, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(right, mutable, function_param_borrows);
        }
        HirExpr::Unary { operand, .. } => {
            mark_mut_ref_call_args_in_expr(operand, mutable, function_param_borrows);
        }
        HirExpr::IfExpr { test, body, orelse } => {
            mark_mut_ref_call_args_in_expr(test, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(body, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(orelse, mutable, function_param_borrows);
        }
        HirExpr::List(items)
        | HirExpr::Tuple(items)
        | HirExpr::Set(items)
        | HirExpr::FrozenSet(items) => {
            for item in items {
                mark_mut_ref_call_args_in_expr(item, mutable, function_param_borrows);
            }
        }
        HirExpr::Dict(pairs) => {
            for (key, value) in pairs {
                mark_mut_ref_call_args_in_expr(key, mutable, function_param_borrows);
                mark_mut_ref_call_args_in_expr(value, mutable, function_param_borrows);
            }
        }
        HirExpr::Index { base, index } => {
            mark_mut_ref_call_args_in_expr(base, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(index, mutable, function_param_borrows);
        }
        HirExpr::Attribute { value, .. } => {
            mark_mut_ref_call_args_in_expr(value, mutable, function_param_borrows);
        }
        HirExpr::ListComp {
            element,
            iter,
            condition,
            ..
        } => {
            mark_mut_ref_call_args_in_expr(element, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(iter, mutable, function_param_borrows);
            if let Some(cond) = condition {
                mark_mut_ref_call_args_in_expr(cond, mutable, function_param_borrows);
            }
        }
        HirExpr::FlattenedListComp {
            element,
            generators,
        } => {
            mark_mut_ref_call_args_in_expr(element, mutable, function_param_borrows);
            for generator in generators {
                mark_mut_ref_call_args_in_expr(&generator.iter, mutable, function_param_borrows);
                for cond in &generator.conditions {
                    mark_mut_ref_call_args_in_expr(cond, mutable, function_param_borrows);
                }
            }
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            mark_mut_ref_call_args_in_expr(base, mutable, function_param_borrows);
            if let Some(s) = start {
                mark_mut_ref_call_args_in_expr(s, mutable, function_param_borrows);
            }
            if let Some(s) = stop {
                mark_mut_ref_call_args_in_expr(s, mutable, function_param_borrows);
            }
            if let Some(s) = step {
                mark_mut_ref_call_args_in_expr(s, mutable, function_param_borrows);
            }
        }
        HirExpr::Lambda { body, .. } => {
            mark_mut_ref_call_args_in_expr(body, mutable, function_param_borrows);
        }
        _ => {}
    }
}

fn mark_mut_ref_call_args_in_stmt(
    stmt: &HirStmt,
    mutable: &mut HashSet<String>,
    function_param_borrows: &HashMap<String, Vec<context::ParamBorrowInfo>>,
) {
    match stmt {
        HirStmt::Assign { value, .. } => {
            mark_mut_ref_call_args_in_expr(value, mutable, function_param_borrows);
        }
        HirStmt::Expr(expr) => {
            mark_mut_ref_call_args_in_expr(expr, mutable, function_param_borrows);
        }
        HirStmt::Return(Some(expr)) => {
            mark_mut_ref_call_args_in_expr(expr, mutable, function_param_borrows);
        }
        HirStmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            mark_mut_ref_call_args_in_expr(condition, mutable, function_param_borrows);
            mark_mut_ref_call_args(then_body, mutable, function_param_borrows);
            if let Some(else_stmts) = else_body {
                mark_mut_ref_call_args(else_stmts, mutable, function_param_borrows);
            }
        }
        HirStmt::While {
            condition, body, ..
        } => {
            mark_mut_ref_call_args_in_expr(condition, mutable, function_param_borrows);
            mark_mut_ref_call_args(body, mutable, function_param_borrows);
        }
        HirStmt::For { body, .. } => {
            mark_mut_ref_call_args(body, mutable, function_param_borrows);
        }
        _ => {}
    }
}

// ---- Class/function code generation helpers --------------------------------

fn convert_classes_to_rust(
    classes: &[HirClass],
    type_mapper: &crate::types::type_mapper::TypeMapper,
    enum_names: &HashSet<String>,
    copy_structs: &HashSet<String>,
) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut abc_classes = HashMap::new();
    for class in classes {
        if class.is_abc {
            abc_classes.insert(class.name.clone(), class);
        }
    }

    let mut class_items = Vec::new();
    for class in classes {
        if class.is_abc {
            let trait_item =
                crate::rust_generator::direct_rules::convert_abc_to_trait(class, type_mapper)?;
            class_items.push(trait_item.to_token_stream());
        } else if class.is_intflag {
            let items = crate::rust_generator::direct_rules::convert_class_to_intflag(class)?;
            for item in items {
                class_items.push(item.to_token_stream());
            }
        } else if class.is_enum {
            let items = crate::rust_generator::direct_rules::convert_class_to_enum(class)?;
            for item in items {
                class_items.push(item.to_token_stream());
            }
        } else {
            let items = crate::rust_generator::direct_rules::convert_class_to_struct(
                class,
                type_mapper,
                &abc_classes,
                enum_names,
                copy_structs,
            )?;
            for item in items {
                class_items.push(item.to_token_stream());
            }
        }
    }
    Ok(class_items)
}

fn convert_functions_to_rust(
    functions: &[HirFunction],
    ctx: &mut super::CodeGenContext,
) -> Result<Vec<proc_macro2::TokenStream>> {
    functions
        .iter()
        .map(|f| f.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()
}

// ---- Import preamble emission (moved from rust_generator.rs §31) ----------

fn deduplicate_use_statements(
    items: Vec<proc_macro2::TokenStream>,
) -> Vec<proc_macro2::TokenStream> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for item in items {
        let item_str = item.to_string();
        if item_str.starts_with("use ") {
            if seen.insert(item_str) {
                deduped.push(item);
            }
        } else {
            deduped.push(item);
        }
    }
    deduped
}

fn generate_conditional_imports(ctx: &super::CodeGenContext) -> Vec<proc_macro2::TokenStream> {
    use super::context::Import;

    let mut imports = Vec::new();

    let conditional_imports: &[(Import, proc_macro2::TokenStream)] = &[
        (Import::HashMap, quote! { use std::collections::HashMap; }),
        (Import::HashSet, quote! { use std::collections::HashSet; }),
        (Import::VecDeque, quote! { use std::collections::VecDeque; }),
        (Import::FnvHashMap, quote! { use fnv::FnvHashMap; }),
        (Import::AHashMap, quote! { use ahash::AHashMap; }),
        (Import::Arc, quote! { use std::sync::Arc; }),
        (Import::Rc, quote! { use std::rc::Rc; }),
        (Import::Cow, quote! { use std::borrow::Cow; }),
        (Import::SerdeJson, quote! { use serde_json; }),
    ];

    for (import, import_tokens) in conditional_imports {
        if ctx.requires(*import) {
            imports.push(import_tokens.clone());
        }
    }

    if ctx.requires(Import::SmallRng) {
        imports.push(quote! { use rand::Rng; });
        imports.push(quote! { use rand::SeedableRng; });
        imports.push(quote! { use rand::rngs::SmallRng; });
        imports.push(quote! {
            thread_local! {
                static DEPYLER_RNG: std::cell::RefCell<SmallRng> =
                    std::cell::RefCell::new(SmallRng::from_os_rng());
            }
        });
    }
    if ctx.requires(Import::SliceRandom) {
        imports.push(quote! { use rand::seq::SliceRandom; });
    }
    if ctx.requires(Import::IndexedRandom) {
        imports.push(quote! { use rand::seq::IndexedRandom; });
    }
    imports
}

fn generate_import_tokens(
    imports: &[Import],
    module_mapper: &crate::mappings::module_mapper::ModuleMapper,
) -> Vec<proc_macro2::TokenStream> {
    let mut items = Vec::new();
    let mut external_imports = Vec::new();
    let mut std_imports = Vec::new();

    for import in imports {
        let rust_imports = module_mapper.map_import(import);
        for rust_import in rust_imports {
            if rust_import.path.starts_with("//") {
                let comment = &rust_import.path;
                items.push(quote! { #[doc = #comment] });
            } else if rust_import.is_external {
                external_imports.push(rust_import);
            } else {
                std_imports.push(rust_import);
            }
        }
    }

    let mut seen_paths = HashSet::new();

    for import in external_imports {
        let key = format!("{}:{:?}", import.path, import.alias);
        if !seen_paths.insert(key) {
            continue;
        }
        let path: syn::Path =
            syn::parse_str(&import.path).unwrap_or_else(|_| parse_quote! { unknown });
        if let Some(alias) = import.alias {
            let alias_ident = syn::Ident::new(&alias, proc_macro2::Span::call_site());
            items.push(quote! { use #path as #alias_ident; });
        } else {
            items.push(quote! { use #path; });
        }
    }

    for import in std_imports {
        if import.path.starts_with("::") || import.path.is_empty() {
            continue;
        }
        let key = format!("{}:{:?}", import.path, import.alias);
        if !seen_paths.insert(key) {
            continue;
        }
        let path: syn::Path = syn::parse_str(&import.path).unwrap_or_else(|_| parse_quote! { std });
        if let Some(alias) = import.alias {
            let alias_ident = syn::Ident::new(&alias, proc_macro2::Span::call_site());
            items.push(quote! { use #path as #alias_ident; });
        } else {
            items.push(quote! { use #path; });
        }
    }
    items
}

fn generate_interned_string_tokens(_optimizer: &StringOptimizer) -> Vec<proc_macro2::TokenStream> {
    vec![]
}

// ---- Constant token generation --------------------------------------------

fn generate_constant_tokens(
    constants: &[HirConstant],
    ctx: &mut super::CodeGenContext,
) -> Result<Vec<proc_macro2::TokenStream>> {
    use super::context::ToRustExpr;

    let mut items = Vec::new();

    for constant in constants {
        let name_ident = syn::Ident::new(&constant.name, proc_macro2::Span::call_site());
        let mut value_expr = constant.value.to_rust_expr(ctx)?;

        let (type_annotation, needs_lazy) = if let Some(ref ty) = constant.type_annotation {
            if let (crate::hir::Type::List(_), HirExpr::List(elts)) = (ty, &constant.value) {
                let elem_type = infer_list_element_type(elts);
                let elem_type_str = elem_type.to_string();
                if !elts.is_empty()
                    && !elem_type_str.contains("serde_json")
                    && elts.iter().all(is_const_safe_list_element)
                {
                    let len_lit =
                        syn::LitInt::new(&elts.len().to_string(), proc_macro2::Span::call_site());
                    let elem_exprs: Vec<syn::Expr> = elts
                        .iter()
                        .map(|e| e.to_rust_expr(ctx))
                        .collect::<Result<Vec<_>>>()?;
                    value_expr = syn::parse_quote! { [#(#elem_exprs),*] };
                    (quote! { : [#elem_type; #len_lit] }, false)
                } else {
                    let rust_type = ctx.type_mapper.map_type(ty);
                    let syn_type = type_gen::rust_type_to_syn(&rust_type)?;
                    (quote! { : #syn_type }, true)
                }
            } else {
                let rust_type = ctx.type_mapper.map_type(ty);
                let needs_lazy = is_heap_allocated_rust_type(&rust_type);
                let syn_type = type_gen::rust_type_to_syn(&rust_type)?;
                (quote! { : #syn_type }, needs_lazy)
            }
        } else {
            match &constant.value {
                HirExpr::Literal(Literal::Int(_)) => (quote! { : i32 }, false),
                HirExpr::Literal(Literal::Float(_)) => (quote! { : f64 }, false),
                HirExpr::Literal(Literal::String(_)) => (quote! { : &str }, false),
                HirExpr::Literal(Literal::Bool(_)) => (quote! { : bool }, false),
                HirExpr::MethodCall { object, method, .. }
                    if matches!(object.as_ref(), HirExpr::Literal(Literal::String(_)))
                        && matches!(
                            method.as_str(),
                            "upper"
                                | "lower"
                                | "strip"
                                | "lstrip"
                                | "rstrip"
                                | "replace"
                                | "title"
                                | "capitalize"
                                | "swapcase"
                                | "center"
                                | "ljust"
                                | "rjust"
                                | "zfill"
                                | "expandtabs"
                                | "join"
                        ) =>
                {
                    (quote! { : String }, true)
                }
                HirExpr::MethodCall { object, method, .. }
                    if matches!(object.as_ref(), HirExpr::Literal(Literal::String(_)))
                        && matches!(
                            method.as_str(),
                            "find" | "rfind" | "index" | "rindex" | "count"
                        ) =>
                {
                    (quote! { : i32 }, false)
                }
                HirExpr::MethodCall { object, method, .. }
                    if matches!(object.as_ref(), HirExpr::Literal(Literal::String(_)))
                        && matches!(
                            method.as_str(),
                            "startswith"
                                | "endswith"
                                | "isdigit"
                                | "isalpha"
                                | "isalnum"
                                | "isspace"
                                | "islower"
                                | "isupper"
                                | "istitle"
                                | "isascii"
                                | "isprintable"
                        ) =>
                {
                    (quote! { : bool }, false)
                }
                HirExpr::Unary { op, operand } => {
                    let ty = infer_unary_type(op, operand);
                    (quote! { : #ty }, false)
                }
                HirExpr::Dict(pairs) => {
                    let (kt, vt) = infer_dict_kv_types(pairs);
                    let kt_s = kt.to_string();
                    let vt_s = vt.to_string();
                    if kt_s.contains("serde_json") || vt_s.contains("serde_json") {
                        ctx.require(super::context::Import::SerdeJson);
                        (quote! { : serde_json::Value }, true)
                    } else {
                        ctx.require(super::context::Import::HashMap);
                        (quote! { : HashMap<#kt, #vt> }, true)
                    }
                }
                HirExpr::Set(elts) | HirExpr::FrozenSet(elts) => {
                    let elem_type = infer_set_element_type(elts);
                    let elem_s = elem_type.to_string();
                    if elem_s.contains("serde_json") {
                        ctx.require(super::context::Import::SerdeJson);
                        (quote! { : serde_json::Value }, true)
                    } else {
                        ctx.require(super::context::Import::HashSet);
                        (quote! { : HashSet<#elem_type> }, true)
                    }
                }
                HirExpr::List(elts) => {
                    let elem_type = infer_list_element_type(elts);
                    let elem_type_str = elem_type.to_string();
                    if !elts.is_empty()
                        && !elem_type_str.contains("serde_json")
                        && elts.iter().all(is_const_safe_list_element)
                    {
                        let len_lit = syn::LitInt::new(
                            &elts.len().to_string(),
                            proc_macro2::Span::call_site(),
                        );
                        let elem_exprs: Vec<syn::Expr> = elts
                            .iter()
                            .map(|e| e.to_rust_expr(ctx))
                            .collect::<Result<Vec<_>>>()?;
                        value_expr = syn::parse_quote! { [#(#elem_exprs),*] };
                        (quote! { : [#elem_type; #len_lit] }, false)
                    } else {
                        (quote! { : Vec<#elem_type> }, true)
                    }
                }
                HirExpr::Tuple(elems) => {
                    let tuple_type = infer_tuple_type(elems);
                    let needs_lazy = elems.iter().any(tuple_element_needs_heap);
                    (quote! { : #tuple_type }, needs_lazy)
                }
                _ => {
                    ctx.require(super::context::Import::SerdeJson);
                    (quote! { : serde_json::Value }, true)
                }
            }
        };

        let use_static = matches!(&constant.value, HirExpr::List(_)) && !needs_lazy;

        if needs_lazy {
            ctx.require(super::context::Import::LazyStatic);
            ctx.lazy_static_constants.insert(constant.name.clone());
            items.push(quote! {
                lazy_static::lazy_static! {
                    pub static ref #name_ident #type_annotation = #value_expr;
                }
            });
        } else if use_static {
            ctx.static_array_constants.insert(constant.name.clone());
            items.push(quote! {
                pub static #name_ident #type_annotation = #value_expr;
            });
        } else {
            items.push(quote! {
                pub const #name_ident #type_annotation = #value_expr;
            });
        }
    }

    Ok(items)
}

// ---- Main orchestrator (moved from rust_generator.rs §31) ------------------

/// Generate a complete Rust file from an HIR module.
///
/// This is the sole public entry point for code generation; it is re-exported
/// from the `rust_generator` module root.
pub fn generate_rust_file(
    module: &HirModule,
    type_mapper: &crate::types::type_mapper::TypeMapper,
) -> Result<(String, Vec<super::cargo_toml_gen::Dependency>)> {
    use super::error_gen::generate_error_type_definitions;
    use super::format::format_rust_code;
    use super::import_gen::process_module_imports;

    let module_mapper = crate::mappings::module_mapper::ModuleMapper::new();

    let mut interprocedural_analyzer = crate::interprocedural::InterproceduralAnalyzer::new(module);
    let interprocedural_analysis = interprocedural_analyzer.analyze();

    let (imported_modules, imported_items) =
        process_module_imports(&module.imports, &module_mapper);

    let class_names: HashSet<String> = module
        .classes
        .iter()
        .map(|class| class.name.clone())
        .collect();

    let mut mutating_methods: HashMap<String, HashSet<String>> = HashMap::new();
    for class in &module.classes {
        let mut mut_methods = HashSet::new();
        for method in &class.methods {
            if crate::rust_generator::direct_rules::method_mutates_self(method) {
                mut_methods.insert(method.name.clone());
            }
        }
        mutating_methods.insert(class.name.clone(), mut_methods);
    }

    let mut ctx = super::CodeGenContext {
        type_mapper,
        annotation_aware_mapper: AnnotationAwareTypeMapper::with_base_mapper(type_mapper.clone()),
        string_optimizer: crate::optimizations::string_optimization::StringOptimizer::new(),
        union_enum_generator: super::union_enum_gen::UnionEnumGenerator::new(),
        generated_enums: Vec::new(),
        required_imports: std::collections::BTreeSet::new(),
        declared_vars: vec![HashSet::new()],
        current_function_can_fail: false,
        current_function_name: None,
        current_return_type: None,
        effective_return_type: None,
        module_mapper,
        imported_modules,
        imported_items,
        mutable_vars: HashSet::new(),
        in_generator: false,
        is_classmethod: false,
        generator_state_vars: HashSet::new(),
        var_types: HashMap::new(),
        class_names,
        mutating_methods,
        function_return_types: HashMap::new(),
        function_param_borrows: HashMap::new(),
        function_param_strategies: HashMap::new(),
        current_function_param_ownership: HashMap::new(),
        param_clone_requirements: HashSet::new(),
        tuple_iter_vars: HashSet::new(),
        is_final_statement: false,
        result_bool_functions: HashSet::new(),
        result_returning_functions: HashSet::new(),
        current_error_type: None,
        exception_scopes: Vec::new(),
        argparser_tracker: super::argparse_transform::ArgParserTracker::new(),
        generated_args_struct: None,
        generated_commands_enum: None,
        current_subcommand_fields: None,
        validator_functions: HashSet::new(),
        stdlib_mappings: crate::mappings::stdlib_mappings::StdlibMappings::new(),
        interprocedural_analysis: Some(&interprocedural_analysis),
        classes_needing_dynamic_access: HashSet::new(),
        current_func_mut_ref_params: HashSet::new(),
        current_func_ref_params: HashSet::new(),
        shadowed_ref_params: HashSet::new(),
        function_param_names: HashMap::new(),
        function_param_types: HashMap::new(),
        var_usage_counts: HashMap::new(),
        var_usage_current: HashMap::new(),
        optional_vars: HashSet::new(),
        lazy_static_constants: HashSet::new(),
        static_array_constants: HashSet::new(),
        is_assignment_target: false,
        prevent_clone: false,
        returns_reference: false,
        return_reference_positions: Vec::new(),
        returns_mutable_reference: false,
        borrowable_vars: HashSet::new(),
        mut_borrowable_vars: HashSet::new(),
        generate_borrow: false,
        generate_mut_borrow: false,
        clone_already_applied: false,
        in_primitive_cast: false,
        vars_needing_clone_at_assign: HashSet::new(),
        vars_needing_clone_for_move: HashSet::new(),
        move_consume_current: HashMap::new(),
        move_consume_totals: HashMap::new(),
        enum_names: HashSet::new(),
        copy_structs: HashSet::new(),
        class_field_types: HashMap::new(),
        function_param_muts: HashMap::new(),
        functions_with_mutated_return: HashSet::new(),
        functions_returning_refs: HashSet::new(),
        functions_suppress_ref_return: HashSet::new(),
        filter_deref_vars: HashSet::new(),
        mut_ref_index_vars: HashSet::new(),
    };

    analyze_string_optimization(&mut ctx, &module.functions);
    analyze_validators(&mut ctx, &module.functions, &module.constants);

    for class in &module.classes {
        if class.is_enum || class.is_intflag {
            ctx.enum_names.insert(class.name.clone());
        }
    }

    for class in &module.classes {
        if !class.is_enum && !class.is_intflag {
            let mut field_map = HashMap::new();
            for field in &class.fields {
                field_map.insert(field.name.clone(), field.field_type.clone());
            }
            ctx.class_field_types.insert(class.name.clone(), field_map);
        }
    }

    // First pass: identify structs whose fields are Copy
    for class in &module.classes {
        if !class.is_enum && !class.is_intflag {
            let has_drop_impl = class
                .methods
                .iter()
                .any(|m| m.name == "__del__" || m.name == "close");
            let all_fields_copyable = class
                .fields
                .iter()
                .filter(|f| !f.is_class_var)
                .all(|f| is_field_copy_type(&f.field_type, &ctx.enum_names, &ctx.copy_structs));
            if all_fields_copyable && !has_drop_impl {
                ctx.copy_structs.insert(class.name.clone());
            }
        }
    }
    // Second pass: re-check with first-pass results available
    for class in &module.classes {
        if !class.is_enum && !class.is_intflag && !ctx.copy_structs.contains(&class.name) {
            let has_drop_impl = class
                .methods
                .iter()
                .any(|m| m.name == "__del__" || m.name == "close");
            let all_fields_copyable = class
                .fields
                .iter()
                .filter(|f| !f.is_class_var)
                .all(|f| is_field_copy_type(&f.field_type, &ctx.enum_names, &ctx.copy_structs));
            if all_fields_copyable && !has_drop_impl {
                ctx.copy_structs.insert(class.name.clone());
            }
        }
    }

    for constant in &module.constants {
        let const_type = if let Some(ref ty) = constant.type_annotation {
            ty.clone()
        } else {
            infer_constant_hir_type(&constant.value)
        };
        if !matches!(const_type, crate::hir::Type::Unknown) {
            ctx.var_types.insert(constant.name.clone(), const_type);
        }
    }

    for constant in &module.constants {
        if constant
            .name
            .chars()
            .next()
            .map_or(false, |c| c.is_uppercase())
        {
            let is_list_value = matches!(&constant.value, HirExpr::List(_));
            let is_list_annotation = constant
                .type_annotation
                .as_ref()
                .is_some_and(|ty| matches!(ty, crate::hir::Type::List(_)));
            if is_list_value || is_list_annotation {
                if let HirExpr::List(elts) = &constant.value {
                    let elem_type = infer_list_element_type(elts);
                    let elem_type_str = elem_type.to_string();
                    if !elts.is_empty()
                        && !elem_type_str.contains("serde_json")
                        && elts.iter().all(is_const_safe_list_element)
                    {
                        ctx.static_array_constants.insert(constant.name.clone());
                        continue;
                    }
                }
            }
            ctx.lazy_static_constants.insert(constant.name.clone());
        }
    }

    populate_function_param_borrows(&module.functions, &mut ctx)?;
    compute_functions_suppress_ref_return(&module.functions, &mut ctx);

    for func in &module.functions {
        ctx.function_return_types
            .insert(func.name.clone(), func.ret_type.clone());
        ctx.function_param_types.insert(
            func.name.clone(),
            func.params.iter().map(|p| p.ty.clone()).collect(),
        );
        ctx.function_param_names.insert(
            func.name.clone(),
            func.params.iter().map(|p| p.name.clone()).collect(),
        );
    }

    for class in &module.classes {
        for method in &class.methods {
            let params: Vec<String> = method
                .params
                .iter()
                .filter(|p| p.name != "self")
                .map(|p| p.name.clone())
                .collect();
            ctx.function_param_names
                .insert(format!("{}.{}", class.name, method.name), params.clone());
            ctx.function_param_names.insert(method.name.clone(), params);
        }
    }

    let classes = convert_classes_to_rust(
        &module.classes,
        ctx.type_mapper,
        &ctx.enum_names,
        &ctx.copy_structs,
    )?;

    let functions = convert_functions_to_rust(&module.functions, &mut ctx)?;

    let mut items = Vec::new();

    let import_mapper = crate::mappings::module_mapper::ModuleMapper::new();
    items.extend(generate_import_tokens(&module.imports, &import_mapper));
    items.extend(generate_interned_string_tokens(&ctx.string_optimizer));

    let constant_tokens = generate_constant_tokens(&module.constants, &mut ctx)?;

    items.extend(generate_conditional_imports(&ctx));
    items.extend(constant_tokens);
    items = deduplicate_use_statements(items);

    items.extend(generate_error_type_definitions(&ctx));
    items.extend(ctx.generated_enums.clone());
    items.extend(classes);

    if let Some(ref commands_enum) = ctx.generated_commands_enum {
        items.push(commands_enum.clone());
    }
    if let Some(ref args_struct) = ctx.generated_args_struct {
        items.push(args_struct.clone());
    }

    items.extend(functions);

    let file = quote! { #(#items)* };

    let mut dependencies = super::cargo_toml_gen::extract_dependencies(&ctx);
    let mut formatted_code = format_rust_code(file.to_string());

    if formatted_code.contains("serde_json::") && !ctx.requires(super::context::Import::SerdeJson) {
        formatted_code = format!("use serde_json;\n{}", formatted_code);
        dependencies.push(super::cargo_toml_gen::Dependency::new("serde_json", "1.0"));
        dependencies.push(
            super::cargo_toml_gen::Dependency::new("serde", "1.0")
                .with_features(vec!["derive".to_string()]),
        );
        formatted_code = format_rust_code(formatted_code);
    }

    Ok((formatted_code, dependencies))
}

/// Generate a main() function from module-level statements.
pub fn generate_main_from_statements(
    statements: &[HirStmt],
    ctx: &mut super::CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use super::context::RustCodeGen;

    let mut body_tokens = Vec::new();
    for stmt in statements {
        let stmt_tokens = stmt.to_rust_tokens(ctx)?;
        body_tokens.push(stmt_tokens);
    }

    Ok(quote! {
        fn main() {
            #(#body_tokens)*
        }
    })
}
