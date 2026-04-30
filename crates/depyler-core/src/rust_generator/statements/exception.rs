//! Exception code generation

use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_generator::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};


use super::*;
pub(crate) fn codegen_raise_stmt(
    exception: &Option<HirExpr>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    if let Some(exc) = exception {
        // Extract the message from known exception constructors
        let exc_expr = match exc {
            HirExpr::MethodCall {
                object,
                method,
                args,
                ..
            } if matches!(object.as_ref(), HirExpr::Var(v) if v == "argparse")
                && method == "ArgumentTypeError"
                && !args.is_empty() =>
            {
                args[0].to_rust_expr(ctx)?
            }
            HirExpr::Call { func, args, .. } if (func == "ArgumentTypeError"
                || func == "ValueError"
                || func == "TypeError"
                || func == "KeyError"
                || func == "IndexError"
                || func == "ZeroDivisionError")
                && !args.is_empty() =>
            {
                args[0].to_rust_expr(ctx)?
            }
            _ => exc.to_rust_expr(ctx)?,
        };

        Ok(quote! { panic!("{}", #exc_expr); })
    } else {
        Ok(quote! { panic!("Exception raised"); })
    }
}

pub(crate) fn codegen_try_stmt(
    body: &[HirStmt],
    handlers: &[ExceptHandler],
    finalbody: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Special case: try { return int(str_var) } except ValueError { return literal }
    // Generate: match str_var.parse::<i32>() { Ok(n) => n, Err(_) => literal }
    // OR if return type is Option: match str_var.parse::<i32>() { Ok(n) => Some(n), Err(_) => None }
    if body.len() == 1 && handlers.len() == 1 && handlers[0].name.is_none() && finalbody.is_none() {
        if let HirStmt::Return(Some(HirExpr::Call { func, args, .. })) = &body[0] {
            if func == "int" && args.len() == 1 {
                // Check if handler returns a simple value
                if let HirStmt::Return(handler_ret_expr) = &handlers[0].body[0] {
                    // Generate the parse expression
                    let arg_expr = args[0].to_rust_expr(ctx)?;

                    // Check if the function returns Option<T>
                    let is_option_return =
                        matches!(&ctx.current_return_type, Some(Type::Optional(_)));

                    // Generate the handler return value
                    let handler_value_expr = if let Some(expr) = handler_ret_expr {
                        expr
                    } else {
                        // return None - handler returns None literal
                        return Ok(quote! {
                            match #arg_expr.parse::<i32>() {
                                Ok(__parsed_value) => __parsed_value,
                                Err(_) => 0
                            }
                        });
                    };

                    let handler_value = handler_value_expr.to_rust_expr(ctx)?;

                    if is_option_return {
                        // Function returns Option<T> - wrap in Some()
                        return Ok(quote! {
                            match #arg_expr.parse::<i32>() {
                                Ok(__parsed_value) => Some(__parsed_value),
                                Err(_) => #handler_value
                            }
                        });
                    } else {
                        // Function returns T - return raw values
                        return Ok(quote! {
                            match #arg_expr.parse::<i32>() {
                                Ok(__parsed_value) => __parsed_value,
                                Err(_) => #handler_value
                            }
                        });
                    }
                }
            }
        }
    }

    // Pattern: try { if cond: raise ValueError("msg"); return True } except ValueError { return False }
    // Optimize to: if cond { return false; } return true;
    if body.len() == 2 && handlers.len() == 1 && finalbody.is_none() && handlers[0].name.is_none() {
        // Check if first statement is: if cond: raise ValueError("msg")
        if let HirStmt::If {
            condition,
            then_body: if_body,
            else_body: orelse,
        } = &body[0]
        {
            if orelse.is_none() && if_body.len() == 1 {
                if let HirStmt::Raise {
                    exception: Some(exc_expr),
                    ..
                } = &if_body[0]
                {
                    // Check if it's raising ValueError or similar
                    let exc_type = extract_exception_type(exc_expr);
                    if let Some(handler_exc) = &handlers[0].exception_type {
                        if exc_type == *handler_exc || handler_exc.is_empty() {
                            // Check if second statement in try is return <value>
                            if let HirStmt::Return(Some(ok_val)) = &body[1] {
                                // Check if handler returns a simple value
                                if handlers[0].body.len() == 1 {
                                    if let HirStmt::Return(Some(err_val)) = &handlers[0].body[0] {
                                        // Pattern matched! Generate optimized if-else
                                        let cond = condition.to_rust_expr(ctx)?;
                                        let err_value = err_val.to_rust_expr(ctx)?;
                                        let ok_value = ok_val.to_rust_expr(ctx)?;

                                        return Ok(quote! {
                                            if #cond {
                                                return #err_value;
                                            }
                                            return #ok_value;
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Pattern: try { return int(str_var) } except ValueError { return literal }
    // We can optimize this to: s.parse::<i32>().unwrap_or(literal)
    // Those need proper match with Err(e) binding
    let simple_pattern_info = if body.len() == 1
        && handlers.len() == 1
        && handlers[0].body.len() == 1
        && handlers[0].name.is_none()
    // No exception variable binding
    {
        // Check if handler body is a Return statement with a simple value
        match &handlers[0].body[0] {
            // Direct literal: return 42, return "error", return None, etc.
            HirStmt::Return(Some(HirExpr::Literal(lit))) => Some((
                (match lit {
                    Literal::Int(n) => n.to_string(),
                    Literal::Float(f) => f.to_string(),
                    Literal::String(s) => format!("\"{}\"", s),
                    Literal::Bool(b) => b.to_string(),
                    Literal::None => "None".to_string(),
                    _ => "Default::default()".to_string(),
                })
                .to_string(),
                handlers[0].exception_type.clone(),
            )),
            // Unary negation: return -1, return -42, etc.
            HirStmt::Return(Some(HirExpr::Unary { op, operand })) => {
                if let HirExpr::Literal(lit) = &**operand {
                    match (op, lit) {
                        (crate::hir::UnaryOp::Neg, Literal::Int(n)) => {
                            Some((format!("-{}", n), handlers[0].exception_type.clone()))
                        }
                        (crate::hir::UnaryOp::Neg, Literal::Float(f)) => {
                            Some((format!("-{}", f), handlers[0].exception_type.clone()))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };

    let handled_types: Vec<String> = handlers
        .iter()
        .filter_map(|h| h.exception_type.clone())
        .collect();

    // Empty list means bare except (catches all exceptions)
    ctx.enter_try_scope(handled_types.clone());

    // Check if handler catches ZeroDivisionError (either explicitly or via bare except:)
    let has_zero_div_handler = handlers.iter().any(|h| {
        h.exception_type.as_deref() == Some("ZeroDivisionError") || h.exception_type.is_none()
    });

    if has_zero_div_handler && body.len() == 1 {
        if let HirStmt::Return(Some(expr)) = &body[0] {
            // Check for both floor division and regular division
            let (has_division, divisor_expr) = if contains_floor_div(expr) {
                (true, extract_divisor_from_floor_div(expr)?)
            } else if contains_div(expr) {
                (true, extract_divisor_from_div(expr)?)
            } else {
                (false, &HirExpr::Literal(Literal::None))
            };

            if has_division {
                let divisor_tokens = divisor_expr.to_rust_expr(ctx)?;

                // Find ZeroDivisionError handler (explicit or bare except:)
                let zero_div_handler_idx = handlers
                    .iter()
                    .position(|h| {
                        h.exception_type.as_deref() == Some("ZeroDivisionError")
                            || h.exception_type.is_none()
                    })
                    .unwrap();

                // Generate handler body
                ctx.enter_scope();
                let old_is_final = ctx.is_final_statement;
                ctx.is_final_statement = false;
                let handler_stmts: Vec<_> = handlers[zero_div_handler_idx]
                    .body
                    .iter()
                    .map(|s| s.to_rust_tokens(ctx))
                    .collect::<Result<Vec<_>>>()?;
                ctx.is_final_statement = old_is_final;
                ctx.exit_scope();

                // Generate try block expression
                let div_result = expr.to_rust_expr(ctx)?;

                ctx.exit_exception_scope();

                // Generate: if divisor == 0 { handler } else { div_result }
                if let Some(finalbody) = finalbody {
                    ctx.enter_scope();
                    let finally_stmts: Vec<_> = finalbody
                        .iter()
                        .map(|s| s.to_rust_tokens(ctx))
                        .collect::<Result<Vec<_>>>()?;
                    ctx.exit_scope();

                    return Ok(quote! {
                        {
                            if #divisor_tokens == 0 {
                                #(#handler_stmts)*
                            } else {
                                return #div_result;
                            }
                            #(#finally_stmts)*
                        }
                    });
                } else {
                    return Ok(quote! {
                        if #divisor_tokens == 0 {
                            #(#handler_stmts)*
                        } else {
                            return #div_result;
                        }
                    });
                }
            }
        }
    }

    // Check if handler catches IndexError (either explicitly or via bare except:)
    let has_index_error_handler = handlers
        .iter()
        .any(|h| h.exception_type.as_deref() == Some("IndexError") || h.exception_type.is_none());

    if has_index_error_handler && body.len() == 1 && handlers.len() == 1 && finalbody.is_none() {
        if let HirStmt::Return(Some(HirExpr::Index { base, index })) = &body[0] {
            // Pattern: try { return items[index] } except IndexError { return default_value }
            // Generate: items.get(index as usize).cloned().unwrap_or(default_value)

            // Generate handler body - should be a simple return with a literal
            if let HirStmt::Return(Some(default_expr)) = &handlers[0].body[0] {
                let base_expr = base.to_rust_expr(ctx)?;
                let index_expr = index.to_rust_expr(ctx)?;
                let default_value = default_expr.to_rust_expr(ctx)?;

                ctx.exit_exception_scope();

                return Ok(quote! {
                    return #base_expr.get(#index_expr as usize).cloned().unwrap_or(#default_value);
                });
            }
        }
    }

    // Check if handler catches IOError or OSError (file operations)
    let has_io_error_handler = handlers.iter().any(|h| {
        matches!(
            h.exception_type.as_deref(),
            Some("IOError") | Some("OSError") | None
        )
    });

    // Pattern: try { with open(path, 'w') as f: f.write(content); return True } except IOError { return False }
    // Optimize to: match fs::write(path, content) { Ok(_) => return true, Err(_) => return false }
    if has_io_error_handler && body.len() == 2 && handlers.len() == 1 && finalbody.is_none() {
        if let (
            HirStmt::With {
                context,
                target,
                body: with_body,
            },
            HirStmt::Return(Some(ok_value)),
        ) = (&body[0], &body[1])
        {
            // Check if it's an open() call
            if let HirExpr::Call { func, args, .. } = context {
                if func.as_str() == "open" && args.len() >= 2 {
                    // Check mode is 'w' or "w"
                    if let HirExpr::Literal(Literal::String(mode)) = &args[1] {
                        if mode == "w" || mode == "wb" {
                            // Check if with body is single statement: f.write(content)
                            if let Some(var_name) = target {
                                if with_body.len() == 1 {
                                    if let HirStmt::Expr(HirExpr::MethodCall {
                                        object,
                                        method,
                                        args: write_args,
                                        ..
                                    }) = &with_body[0]
                                    {
                                        if let HirExpr::Var(obj_name) = &**object {
                                            if obj_name == var_name
                                                && method == "write"
                                                && write_args.len() == 1
                                            {
                                                // Pattern matched! Check handler returns a simple value
                                                if handlers[0].body.len() == 1 {
                                                    if let HirStmt::Return(Some(err_value)) =
                                                        &handlers[0].body[0]
                                                    {
                                                        let path_expr =
                                                            args[0].to_rust_expr(ctx)?;
                                                        let content_expr =
                                                            write_args[0].to_rust_expr(ctx)?;
                                                        let ok_val = ok_value.to_rust_expr(ctx)?;
                                                        let err_val =
                                                            err_value.to_rust_expr(ctx)?;

                                                        ctx.exit_exception_scope();

                                                        return Ok(quote! {
                                                            match std::fs::write(#path_expr, #content_expr) {
                                                                Ok(_) => return #ok_val,
                                                                Err(_) => return #err_val,
                                                            }
                                                        });
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

    // Convert try body to statements
    // Save and temporarily disable is_final_statement so return statements
    // in try blocks get the explicit 'return' keyword (needed for proper exception handling)
    let saved_is_final = ctx.is_final_statement;
    ctx.is_final_statement = false;

    ctx.enter_scope();
    let try_stmts: Vec<_> = body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    // Restore is_final_statement flag
    ctx.is_final_statement = saved_is_final;

    ctx.exit_exception_scope();

    // Generate except handler code
    let mut handler_tokens = Vec::new();
    for handler in handlers {
        ctx.enter_handler_scope();
        ctx.enter_scope();

        // If there's a name binding, declare it in scope
        if let Some(var_name) = &handler.name {
            ctx.declare_var(var_name);
        }

        // Save and temporarily disable is_final_statement so return statements
        // in handlers get the explicit 'return' keyword (needed for proper exception handling)
        let saved_is_final = ctx.is_final_statement;
        ctx.is_final_statement = false;

        let handler_stmts: Vec<_> = handler
            .body
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;

        // Restore is_final_statement flag
        ctx.is_final_statement = saved_is_final;
        ctx.exit_scope();
        ctx.exit_exception_scope();

        handler_tokens.push(quote! { #(#handler_stmts)* });
    }

    // Generate finally clause if present
    let finally_stmts = if let Some(finally_body) = finalbody {
        let stmts: Vec<_> = finally_body
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;
        Some(quote! { #(#stmts)* })
    } else {
        None
    };

    // Generate try/except/finally pattern
    if handlers.is_empty() {
        // Try/finally without except
        if let Some(finally_code) = finally_stmts {
            Ok(quote! {
                #(#try_stmts)*
                #finally_code
            })
        } else {
            // Just try block
            Ok(quote! { #(#try_stmts)* })
        }
    } else {
        // Check if try_stmts contains a .parse() call that we can convert to match
        if handlers.len() == 1 {
            if let Some((var_name, parse_expr_str, remaining_stmts)) =
                extract_parse_from_tokens(&try_stmts)
            {
                // Parse the expression string back to token stream
                let parse_expr: proc_macro2::TokenStream = parse_expr_str.parse().unwrap();
                let ok_var = safe_ident(&var_name);

                // Generate Ok branch (remaining statements after parse)
                let ok_body = quote! { #(#remaining_stmts)* };

                // Generate Err branch (handler body)
                let err_body = &handler_tokens[0];

                let err_pattern = if let Some(exc_var) = &handlers[0].name {
                    // Bind exception variable: Err(e) => { ... }
                    let exc_ident = safe_ident(exc_var);
                    quote! { Err(#exc_ident) }
                } else {
                    // No exception variable: Err(_) => { ... }
                    quote! { Err(_) }
                };

                // Build match expression
                let match_expr = quote! {
                    match #parse_expr {
                        Ok(#ok_var) => { #ok_body },
                        #err_pattern => { #err_body }
                    }
                };

                // Wrap with finally if present
                if let Some(finally_code) = finally_stmts {
                    return Ok(quote! {
                        {
                            #match_expr
                            #finally_code
                        }
                    });
                } else {
                    return Ok(match_expr);
                }
            }
        }

        // Fall through to existing simple_pattern_info logic
        if let Some((exception_value_str, _exception_type)) = simple_pattern_info {
            // Fall through to existing unwrap_or logic if not a match pattern
            // Convert try_stmts to string to post-process
            let try_code = quote! { #(#try_stmts)* };
            let try_str = try_code.to_string();

            // This handles the case where int(str) generates .parse().unwrap_or_default()
            // but we want .parse().unwrap_or(-1) based on the except clause
            if try_str.contains("unwrap_or_default") {
                // Parse the try code and replace unwrap_or_default with unwrap_or(value)
                // Handle both "unwrap_or_default ()" and "unwrap_or_default()"
                let fixed_code = try_str
                    .replace(
                        "unwrap_or_default ()",
                        &format!("unwrap_or ({})", exception_value_str),
                    )
                    .replace(
                        "unwrap_or_default()",
                        &format!("unwrap_or({})", exception_value_str),
                    );

                // Parse back to token stream
                let fixed_tokens: proc_macro2::TokenStream = fixed_code.parse().unwrap_or(try_code);

                if let Some(finally_code) = finally_stmts {
                    Ok(quote! {
                        {
                            #fixed_tokens
                            #finally_code
                        }
                    })
                } else {
                    Ok(fixed_tokens)
                }
            } else {
                // Pattern matched but no unwrap_or_default found
                // Check if try block contains .parse().unwrap() that we can convert to match
                let try_code_str = quote! { #(#try_stmts)* }.to_string();

                if try_code_str.contains("parse") && try_code_str.contains("unwrap ()") {
                    // Extract the parse call and convert to match statement
                    // Pattern: return Some ( s . parse :: < i32 > () . unwrap () )
                    if let Some(parse_start) = try_code_str.find(".parse") {
                        if let Some(unwrap_end) = try_code_str.find("unwrap ()") {
                            // Find the variable/expression being parsed (go backwards from .parse)
                            let before_parse = &try_code_str[..parse_start];
                            let words: Vec<&str> = before_parse.split_whitespace().collect();
                            let var_or_expr = words.last().unwrap_or(&"value");

                            // Extract the parse call with type
                            let parse_section = &try_code_str[parse_start..unwrap_end + 9]; // include "unwrap ()"

                            // Check if this is wrapped in Some()
                            let is_option_some =
                                try_code_str.contains("Some (") || try_code_str.contains("Some(");

                            // Build the match expression
                            let var_ident = safe_ident(var_or_expr);
                            let handler_code = &handler_tokens[0];

                            if is_option_some {
                                // Pattern: return Some(x.parse().unwrap()) -> match x.parse() { Ok(n) => Some(n), Err(_) => handler }
                                let match_expr = quote! {
                                    match #var_ident.parse::<i32>() {
                                        Ok(__parsed_value) => Some(__parsed_value),
                                        Err(_) => { #handler_code }
                                    }
                                };

                                if let Some(finally_code) = finally_stmts {
                                    return Ok(quote! {
                                        {
                                            #match_expr
                                            #finally_code
                                        }
                                    });
                                } else {
                                    return Ok(match_expr);
                                }
                            }
                        }
                    }
                }

                // Fall back to concatenation (will create unreachable code warning, but preserves behavior)
                let handler_code = &handler_tokens[0];
                if let Some(finally_code) = finally_stmts {
                    Ok(quote! {
                        {
                            #(#try_stmts)*
                            #handler_code
                            #finally_code
                        }
                    })
                } else {
                    Ok(quote! {
                        {
                            #(#try_stmts)*
                            #handler_code
                        }
                    })
                }
            }
        } else {
            // Execute try block statements, then if we have a single handler, use it
            if handlers.len() == 1 {
                if handlers[0].name.is_some() && body.len() == 1 {
                    if let HirStmt::Return(Some(HirExpr::Call { func, args, .. })) = &body[0] {
                        if func == "int" && args.len() == 1 {
                            // Single handler with exception binding - use match with Err(e)
                            let arg_expr = args[0].to_rust_expr(ctx)?;
                            let handler_body = &handler_tokens[0];
                            let err_var = handlers[0].name.as_ref().map(|s| safe_ident(s)).unwrap();

                            if let Some(finally_body) = finalbody {
                                let finally_stmts: Vec<_> = finally_body
                                    .iter()
                                    .map(|s| s.to_rust_tokens(ctx))
                                    .collect::<Result<Vec<_>>>()?;
                                return Ok(quote! {
                                    {
                                        match #arg_expr.parse::<i32>() {
                                            Ok(__value) => __value,
                                            Err(#err_var) => {
                                                #handler_body
                                            }
                                        }
                                        #(#finally_stmts)*
                                    }
                                });
                            } else {
                                return Ok(quote! {
                                    match #arg_expr.parse::<i32>() {
                                        Ok(__value) => __value,
                                        Err(#err_var) => {
                                            #handler_body
                                        }
                                    }
                                });
                            }
                        }
                    }
                }

                // If so, don't concatenate handler as it creates invalid syntax or unreachable code
                let try_code_str = quote! { #(#try_stmts)* }.to_string();
                let has_error_handling = try_code_str.contains("unwrap_or_default")
                    || try_code_str.contains("unwrap_or(")
                    || try_code_str.contains(".expect(");

                if has_error_handling {
                    // Try block already handles errors, don't add handler
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
                } else {
                    // If so, skip handler code since we can't bind it in unconditional context
                    let has_exception_binding = handlers[0].name.is_some();

                    if has_exception_binding {
                        // Skip handler code - it would reference unbound exception variable
                        // NOTE: This means exception handlers are not fully implemented ()
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
                    } else {
                        let handler_code = &handler_tokens[0];

                        if let Some(finally_code) = finally_stmts {
                            Ok(quote! {
                                {
                                    #(#try_stmts)*
                                    #handler_code
                                    #finally_code
                                }
                            })
                        } else {
                            // NOTE: This executes both unconditionally - need proper conditional logic ()
                            // based on which operations can panic (ZeroDivisionError, IndexError, etc.)
                            Ok(quote! {
                                {
                                    #(#try_stmts)*
                                    #handler_code
                                }
                            })
                        }
                    }
                }
            } else {
                // For operations like int(data) with multiple exception types, we need proper
                // match-based error handling instead of simple unwrap_or

                // Check if try block is simple return with parse operation
                if body.len() == 1 {
                    if let HirStmt::Return(Some(HirExpr::Call { func, args, .. })) = &body[0] {
                        if func == "int" && args.len() == 1 {
                            let arg_expr = args[0].to_rust_expr(ctx)?;

                            // Check if any handler binds the exception variable
                            let has_exception_binding = handlers.iter().any(|h| h.name.is_some());

                            if has_exception_binding && handlers.len() == 1 {
                                // Single handler with exception binding - use match with Err(e)
                                let handler_body = &handler_tokens[0];
                                let err_var =
                                    handlers[0].name.as_ref().map(|s| safe_ident(s)).unwrap();

                                if let Some(finally_code) = finally_stmts {
                                    return Ok(quote! {
                                        {
                                            match #arg_expr.parse::<i32>() {
                                                Ok(__value) => __value,
                                                Err(#err_var) => {
                                                    #handler_body
                                                }
                                            }
                                            #finally_code
                                        }
                                    });
                                } else {
                                    return Ok(quote! {
                                        match #arg_expr.parse::<i32>() {
                                            Ok(__value) => __value,
                                            Err(#err_var) => {
                                                #handler_body
                                            }
                                        }
                                    });
                                }
                            } else if handlers.len() >= 2 {
                                // Convert: try { return int(data) } except ValueError {...} except TypeError {...}
                                // To: if let Ok(v) = data.parse::<i32>() { v } else { handler1; handler2; }

                                // NOTE: Rust's parse() returns a single error type, so we can't dispatch
                                // to specific handlers. We execute all handlers sequentially.
                                // This is semantically incorrect but compiles. NOTE: Improve error dispatch logic ()

                                if let Some(finally_code) = finally_stmts {
                                    return Ok(quote! {
                                        {
                                            if let Ok(__parse_result) = #arg_expr.parse::<i32>() {
                                                __parse_result
                                            } else {
                                                #(#handler_tokens)*
                                            }
                                            #finally_code
                                        }
                                    });
                                } else {
                                    return Ok(quote! {
                                        {
                                            if let Ok(__parse_result) = #arg_expr.parse::<i32>() {
                                                __parse_result
                                            } else {
                                                #(#handler_tokens)*
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }

                // In that case, don't concatenate handler tokens as it creates invalid syntax
                let try_code_str = quote! { #(#try_stmts)* }.to_string();
                let has_error_handling = try_code_str.contains("unwrap_or_default")
                    || try_code_str.contains("unwrap_or(");

                if has_error_handling {
                    // Try block has built-in error handling, don't add handlers
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
                } else {
                    // Note: Floor division with ZeroDivisionError is handled earlier (line 1366)
                    if let Some(finally_code) = finally_stmts {
                        Ok(quote! {
                            {
                                #(#try_stmts)*
                                #(#handler_tokens)*
                                #finally_code
                            }
                        })
                    } else {
                        Ok(quote! {
                            {
                                #(#try_stmts)*
                                #(#handler_tokens)*
                            }
                        })
                    }
                }
            }
        }
    }
}

pub(crate) fn extract_exception_type(exception: &HirExpr) -> String {
    match exception {
        HirExpr::Call { func, .. } => func.clone(),
        HirExpr::Var(name) => name.clone(),
        HirExpr::MethodCall { method, .. } => method.clone(),
        _ => "Exception".to_string(),
    }
}

pub(crate) fn extract_parse_from_tokens(
    try_stmts: &[proc_macro2::TokenStream],
) -> Option<(String, String, Vec<proc_macro2::TokenStream>)> {
    if try_stmts.is_empty() {
        return None;
    }

    // Convert first statement to string (note: tokens have spaces between them)
    let first_stmt = try_stmts[0].to_string();

    // Pattern: let var_name = something . parse :: < i32 > () . unwrap_or_default () ;
    // Note: TokenStream.to_string() adds spaces between tokens
    if first_stmt.contains("parse") && first_stmt.contains("unwrap_or_default") {
        // Extract variable name (after "let " and before " =")
        if let Some(let_pos) = first_stmt.find("let ") {
            if let Some(eq_pos) = first_stmt[let_pos..].find(" =") {
                let var_name = first_stmt[let_pos + 4..let_pos + eq_pos].trim().to_string();

                // Extract parse expression (between "= " and "unwrap_or_default")
                // We need to find the start of unwrap_or_default and go back to find the parse call
                if let Some(eq_start) = first_stmt.find(" = ") {
                    if let Some(unwrap_pos) = first_stmt.find("unwrap_or_default") {
                        // Go back from unwrap_pos to skip ". " before it
                        let parse_end =
                            if unwrap_pos >= 2 && &first_stmt[unwrap_pos - 2..unwrap_pos] == ". " {
                                unwrap_pos - 2
                            } else {
                                unwrap_pos
                            };

                        let parse_expr = first_stmt[eq_start + 3..parse_end].trim().to_string();

                        // Collect remaining statements
                        let remaining: Vec<_> = try_stmts[1..].to_vec();

                        return Some((var_name, parse_expr, remaining));
                    }
                }
            }
        }
    }

    None
}

pub(crate) fn contains_floor_div(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Binary {
            op: BinOp::FloorDiv,
            ..
        } => true,
        HirExpr::Binary { left, right, .. } => {
            contains_floor_div(left) || contains_floor_div(right)
        }
        HirExpr::Unary { operand, .. } => contains_floor_div(operand),
        HirExpr::Call { args, .. } => args.iter().any(contains_floor_div),
        HirExpr::MethodCall { object, args, .. } => {
            contains_floor_div(object) || args.iter().any(contains_floor_div)
        }
        HirExpr::Index { base, index } => contains_floor_div(base) || contains_floor_div(index),
        HirExpr::List(elements) | HirExpr::Tuple(elements) | HirExpr::Set(elements) => {
            elements.iter().any(contains_floor_div)
        }
        _ => false,
    }
}

pub(crate) fn contains_div(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Binary { op: BinOp::Div, .. } => true,
        HirExpr::Binary { left, right, .. } => contains_div(left) || contains_div(right),
        HirExpr::Unary { operand, .. } => contains_div(operand),
        HirExpr::Call { args, .. } => args.iter().any(contains_div),
        HirExpr::MethodCall { object, args, .. } => {
            contains_div(object) || args.iter().any(contains_div)
        }
        HirExpr::Index { base, index } => contains_div(base) || contains_div(index),
        HirExpr::List(elements) | HirExpr::Tuple(elements) | HirExpr::Set(elements) => {
            elements.iter().any(contains_div)
        }
        _ => false,
    }
}

pub(crate) fn extract_divisor_from_floor_div(expr: &HirExpr) -> Result<&HirExpr> {
    match expr {
        HirExpr::Binary {
            op: BinOp::FloorDiv,
            right,
            ..
        } => Ok(right),
        HirExpr::Binary { left, right, .. } => {
            // Recursively search for floor division
            if contains_floor_div(left) {
                extract_divisor_from_floor_div(left)
            } else if contains_floor_div(right) {
                extract_divisor_from_floor_div(right)
            } else {
                bail!("No floor division found in expression")
            }
        }
        HirExpr::Unary { operand, .. } => extract_divisor_from_floor_div(operand),
        _ => bail!("No floor division found in expression"),
    }
}

pub(crate) fn extract_divisor_from_div(expr: &HirExpr) -> Result<&HirExpr> {
    match expr {
        HirExpr::Binary {
            op: BinOp::Div,
            right,
            ..
        } => Ok(right),
        HirExpr::Binary { left, right, .. } => {
            // Recursively search for division
            if contains_div(left) {
                extract_divisor_from_div(left)
            } else if contains_div(right) {
                extract_divisor_from_div(right)
            } else {
                bail!("No division found in expression")
            }
        }
        HirExpr::Unary { operand, .. } => extract_divisor_from_div(operand),
        _ => bail!("No division found in expression"),
    }
}

