//! Argparse code generation

use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_generator::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};


use super::*;
pub(crate) fn try_generate_subcommand_match(
    condition: &HirExpr,
    then_body: &[HirStmt],
    else_body: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<Option<proc_macro2::TokenStream>> {
    use quote::{format_ident, quote};

    // Check if condition matches: args.command == "string"
    let command_name = match is_subcommand_check(condition) {
        Some(name) => name,
        None => return Ok(None),
    };

    // Collect all branches (if + elif chain)
    let mut branches = vec![(command_name, then_body)];

    // Check if else is another if statement (elif pattern)
    let mut current_else = else_body;
    while let Some(else_stmts) = current_else {
        // Check if else body is a single If statement
        if else_stmts.len() == 1 {
            if let HirStmt::If {
                condition: elif_cond,
                then_body: elif_then,
                else_body: elif_else,
            } = &else_stmts[0]
            {
                if let Some(elif_name) = is_subcommand_check(elif_cond) {
                    branches.push((elif_name, elif_then.as_slice()));
                    current_else = elif_else;
                    continue;
                }
            }
        }
        // Not an elif pattern, stop collecting
        break;
    }

    // Generate match arms
    let arms: Vec<proc_macro2::TokenStream> = branches
        .iter()
        .map(|(cmd_name, body)| {
            // Convert command name to PascalCase variant
            let variant_name = format_ident!("{}", to_pascal_case_subcommand(cmd_name));

            // Get subcommand info to extract fields
            let subcommand_info = ctx
                .argparser_tracker
                .subcommands
                .values()
                .find(|sc| sc.name == *cmd_name);

            // Generate field bindings
            let field_bindings: Vec<_> = if let Some(sc) = subcommand_info {
                sc.arguments
                    .iter()
                    .map(|arg| format_ident!("{}", arg.rust_field_name()))
                    .collect()
            } else {
                vec![]
            };

            // Generate body statements
            ctx.enter_scope();
            let body_stmts: Vec<_> = body
                .iter()
                .map(|s| s.to_rust_tokens(ctx))
                .collect::<Result<Vec<_>>>()
                .unwrap_or_default();
            ctx.exit_scope();

            if field_bindings.is_empty() {
                quote! {
                    Commands::#variant_name => {
                        #(#body_stmts)*
                    }
                }
            } else {
                quote! {
                    Commands::#variant_name { #(#field_bindings),* } => {
                        #(#body_stmts)*
                    }
                }
            }
        })
        .collect();

    Ok(Some(quote! {
        match args.command {
            #(#arms)*
        }
    }))
}

pub(crate) fn is_subcommand_check(expr: &HirExpr) -> Option<String> {
    match expr {
        HirExpr::Binary {
            op: BinOp::Eq,
            left,
            right,
        } => {
            // Check if left side is args.command
            let is_command_attr = matches!(
                left.as_ref(),
                HirExpr::Attribute { attr, .. } if attr == "command"
            );

            // Check if right side is a string literal
            if is_command_attr {
                if let HirExpr::Literal(Literal::String(cmd_name)) = right.as_ref() {
                    return Some(cmd_name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn to_pascal_case_subcommand(s: &str) -> String {
    s.split(&['-', '_'][..])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

pub(crate) fn extract_string_literal(expr: &HirExpr) -> String {
    match expr {
        HirExpr::Literal(Literal::String(s)) => s.clone(),
        _ => String::new(),
    }
}

pub(crate) fn extract_kwarg_string(kwargs: &[(String, HirExpr)], key: &str) -> Option<String> {
    kwargs
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            HirExpr::Literal(Literal::String(s)) => Some(s.clone()),
            _ => None,
        })
}

pub(crate) fn extract_kwarg_bool(kwargs: &[(String, HirExpr)], key: &str) -> Option<bool> {
    kwargs
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            HirExpr::Var(s) if s == "True" => Some(true),
            HirExpr::Var(s) if s == "False" => Some(false),
            _ => None,
        })
}

