//! Functions code generation

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};


use super::*;
pub(crate) fn codegen_nested_function_def(
    name: &str,
    params: &[HirParam],
    ret_type: &Type,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use quote::quote;

    // Generate function name
    let fn_name = syn::Ident::new(name, proc_macro2::Span::call_site());

    // Generate parameters
    let param_tokens: Vec<proc_macro2::TokenStream> = params
        .iter()
        .map(|p| {
            let param_name = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            let param_type = hir_type_to_tokens(&p.ty, ctx);

            quote! { #param_name: #param_type }
        })
        .collect();

    // Generate return type
    let return_type = hir_type_to_tokens(ret_type, ctx);

    // Generate body
    let body_tokens: Vec<proc_macro2::TokenStream> = body
        .iter()
        .map(|stmt| stmt.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    // Generate inner function
    Ok(quote! {
        fn #fn_name(#(#param_tokens),*) -> #return_type {
            #(#body_tokens)*
        }
    })
}

pub(crate) fn codegen_async_nested_function_def(
    name: &str,
    params: &[HirParam],
    ret_type: &Type,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use quote::quote;

    let fn_name = syn::Ident::new(name, proc_macro2::Span::call_site());

    let param_tokens: Vec<proc_macro2::TokenStream> = params
        .iter()
        .map(|p| {
            let param_name = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            let param_type = hir_type_to_tokens(&p.ty, ctx);
            quote! { #param_name: #param_type }
        })
        .collect();

    let return_type = hir_type_to_tokens(ret_type, ctx);

    let body_tokens: Vec<proc_macro2::TokenStream> = body
        .iter()
        .map(|stmt| stmt.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        async fn #fn_name(#(#param_tokens),*) -> #return_type {
            #(#body_tokens)*
        }
    })
}

pub(crate) fn codegen_global_stmt(_names: &[String]) -> Result<proc_macro2::TokenStream> {
    // Global statement is a declaration marker in Python, not executable code.
    // The actual semantics depend on how the variable is used later.
    // For now, emit a comment noting the global declaration.
    Ok(quote! {})
}

pub(crate) fn codegen_nonlocal_stmt(_names: &[String]) -> Result<proc_macro2::TokenStream> {
    // Nonlocal statement is a declaration marker in Python, not executable code.
    // The actual semantics require tracking captured variables in closures.
    // For now, emit nothing as the closure capture handles this.
    Ok(quote! {})
}

