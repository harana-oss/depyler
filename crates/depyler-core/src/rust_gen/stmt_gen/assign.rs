//! Assign code generation

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::func_gen::infer_expr_type_with_env;
use crate::rust_gen::keywords::safe_ident;
use crate::rust_gen::type_gen::rust_type_to_syn;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};

use super::*;

pub(crate) fn codegen_assign_stmt(
    target: &AssignTarget,
    value: &HirExpr,
    type_annotation: &Option<Type>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // When we have subcommands, assignments like `_cse_temp_0 = args.command == "clone"`
    // would try to compare Commands enum to string (won't compile).
    // Transform into a match expression that returns bool:
    // let _cse_temp_0 = matches!(args.command, Commands::Clone { .. });
    if ctx.argparser_tracker.has_subcommands() {
        if let Some(cmd_name) = is_subcommand_check(value) {
            if let AssignTarget::Symbol(cse_var) = target {
                use quote::{format_ident, quote};
                let variant_name = format_ident!("{}", to_pascal_case_subcommand(&cmd_name));
                let var_ident = safe_ident(cse_var);

                return Ok(quote! {
                    let #var_ident = matches!(args.command, Commands::#variant_name { .. });
                });
            }
        }
    }

    // Pattern 1: parser = argparse.ArgumentParser(...) [MethodCall with object=argparse]
    // Pattern 2: args = parser.parse_args() [MethodCall with object=parser]
    if let AssignTarget::Symbol(var_name) = target {
        if let HirExpr::MethodCall {
            method,
            object,
            args,
            kwargs,
            ..
        } = value
        {
            // Pattern 1: ArgumentParser constructor
            if method == "ArgumentParser" {
                if let HirExpr::Var(module_name) = object.as_ref() {
                    if module_name == "argparse" {
                        // Register this as an ArgumentParser instance
                        let mut info = crate::rust_gen::argparse_transform::ArgParserInfo::new(
                            var_name.clone(),
                        );

                        // Extract description and epilog from kwargs
                        for (key, value_expr) in kwargs {
                            if key == "description" {
                                if let HirExpr::Literal(crate::hir::Literal::String(s)) = value_expr
                                {
                                    info.description = Some(s.clone());
                                }
                            } else if key == "epilog" {
                                if let HirExpr::Literal(crate::hir::Literal::String(s)) = value_expr
                                {
                                    info.epilog = Some(s.clone());
                                }
                            }
                        }

                        ctx.argparser_tracker
                            .register_parser(var_name.clone(), info);

                        // Skip generating this statement - it will be replaced by Args struct
                        return Ok(quote! {});
                    }
                }
            }

            // Pattern 2: args = parser.parse_args()
            if method == "parse_args" {
                if let HirExpr::Var(parser_var) = object.as_ref() {
                    // Check if this parser is tracked
                    if let Some(parser_info) = ctx.argparser_tracker.get_parser_mut(parser_var) {
                        // Set the args variable name
                        parser_info.set_args_var(var_name.clone());

                        // Generate Args::parse() instead
                        let var_ident = syn::Ident::new(var_name, proc_macro2::Span::call_site());
                        return Ok(quote! {
                            let #var_ident = Args::parse();
                        });
                    }
                }
            }

            // Pattern: group = parser.add_argument_group(...)
            //      OR: nested_group = group.add_mutually_exclusive_group(...)
            // These methods aren't needed with clap derive - skip the assignment
            if matches!(
                method.as_str(),
                "add_argument_group" | "add_mutually_exclusive_group" | "set_defaults"
            ) {
                if let HirExpr::Var(parent_var) = object.as_ref() {
                    // Check if parent_var is a parser OR a group
                    let is_parser_or_group = ctx.argparser_tracker.get_parser(parent_var).is_some()
                        || ctx
                            .argparser_tracker
                            .get_parser_for_group(parent_var)
                            .is_some();

                    if is_parser_or_group {
                        // add_argument() calls on it later (e.g., input_group.add_argument())
                        // This handles both:
                        //   - group = parser.add_argument_group() → register group → parser
                        //   - nested = group.add_mutually_exclusive_group() → register nested → group
                        // Recursive resolution will handle nested → group → parser chain
                        if let AssignTarget::Symbol(group_var) = target {
                            ctx.argparser_tracker
                                .register_group(group_var.clone(), parent_var.clone());
                        }
                        // Skip this assignment - not needed with clap
                        return Ok(quote! {});
                    }
                }
            }

            if method == "add_subparsers" {
                if let HirExpr::Var(parser_var) = object.as_ref() {
                    if ctx.argparser_tracker.get_parser(parser_var).is_some() {
                        // Extract dest and required from kwargs
                        let dest_field = extract_kwarg_string(kwargs, "dest")
                            .unwrap_or_else(|| "command".to_string());
                        let required = extract_kwarg_bool(kwargs, "required").unwrap_or(false);
                        let help = extract_kwarg_string(kwargs, "help");

                        if let AssignTarget::Symbol(subparsers_var) = target {
                            use crate::rust_gen::argparse_transform::SubparserInfo;
                            ctx.argparser_tracker.register_subparsers(
                                subparsers_var.clone(),
                                SubparserInfo {
                                    parser_var: parser_var.clone(),
                                    dest_field,
                                    required,
                                    help,
                                },
                            );
                        }
                        // Skip this assignment - not needed with clap
                        return Ok(quote! {});
                    }
                }
            }

            if method == "add_parser" {
                if let HirExpr::Var(subparsers_var) = object.as_ref() {
                    if ctx
                        .argparser_tracker
                        .get_subparsers(subparsers_var)
                        .is_some()
                    {
                        // Extract command name from first positional arg
                        if !args.is_empty() {
                            let command_name = extract_string_literal(&args[0]);
                            let help = extract_kwarg_string(kwargs, "help");

                            if let AssignTarget::Symbol(subcommand_var) = target {
                                use crate::rust_gen::argparse_transform::SubcommandInfo;
                                ctx.argparser_tracker.register_subcommand(
                                    subcommand_var.clone(),
                                    SubcommandInfo {
                                        name: command_name,
                                        help,
                                        arguments: vec![],
                                        subparsers_var: subparsers_var.clone(),
                                    },
                                );
                            }
                        }
                        // Skip this assignment - not needed with clap
                        return Ok(quote! {});
                    }
                }
            }
        }
    }

    // If we have dict[key] += value, avoid borrow-after-move by evaluating old value first
    if is_dict_augassign_pattern(target, value, ctx) {
        if let AssignTarget::Index { base, index } = target {
            if let HirExpr::Binary { op, left: _, right } = value {
                // Generate: let old_val = dict.get(&key).cloned().unwrap_or_default();
                //           dict.insert(key, old_val + right_value);
                let base_expr = build_expr_no_clone(base);
                let mut index_expr = index.to_rust_expr(ctx)?;
                let right_expr = right.to_rust_expr(ctx)?;

                // Convert string literal keys to String for HashMap operations
                if matches!(index.as_ref(), HirExpr::Literal(Literal::String(_))) {
                    index_expr = parse_quote! { #index_expr.to_string() };
                }

                let op_token = match op {
                    BinOp::Add => quote! { + },
                    BinOp::Sub => quote! { - },
                    BinOp::Mul => quote! { * },
                    BinOp::Div => quote! { / },
                    BinOp::Mod => quote! { % },
                    _ => bail!("Unsupported augmented assignment operator for dict"),
                };

                return Ok(quote! {
                    {
                        let _key = #index_expr;
                        let _old_val = #base_expr.get(&_key).cloned().unwrap_or_default();
                        #base_expr.insert(_key, _old_val #op_token #right_expr);
                    }
                });
            }
        }
    }

    // Handle augmented assignment on Optional field: obj.field += value
    // Generate: obj.field = Some(obj.field.unwrap() + value)
    if is_optional_attr_augassign_pattern(target, value, ctx) {
        if let AssignTarget::Attribute {
            value: base_value,
            attr,
        } = target
        {
            if let HirExpr::Binary { op, left: _, right } = value {
                let base_expr = base_value.to_rust_expr(ctx)?;
                let attr_ident = format_ident!("{}", attr);
                let right_expr = right.to_rust_expr(ctx)?;
                let op_token = match op {
                    BinOp::Add => quote! { + },
                    BinOp::Sub => quote! { - },
                    BinOp::Mul => quote! { * },
                    BinOp::Div => quote! { / },
                    BinOp::FloorDiv => quote! { / }, // Integer division in Rust is /
                    BinOp::Mod => quote! { % },
                    BinOp::BitAnd => quote! { & },
                    BinOp::BitOr => quote! { | },
                    BinOp::BitXor => quote! { ^ },
                    BinOp::LShift => quote! { << },
                    BinOp::RShift => quote! { >> },
                    BinOp::Pow => {
                        // Power operation needs special handling
                        return Ok(quote! {
                            #base_expr.#attr_ident = Some(#base_expr.#attr_ident.unwrap().pow(#right_expr as u32));
                        });
                    }
                    _ => bail!("Unsupported augmented assignment operator for Optional field"),
                };

                return Ok(quote! {
                    #base_expr.#attr_ident = Some(#base_expr.#attr_ident.unwrap() #op_token #right_expr);
                });
            }
        }
    }

    // Handle augmented assignment on Optional variable: x += value where x: Optional[int]
    // Generate: x = Some(x.unwrap() + value)
    if is_optional_var_augassign_pattern(target, value, ctx) {
        if let AssignTarget::Symbol(var_name) = target {
            if let HirExpr::Binary { op, left: _, right } = value {
                let var_ident = safe_ident(var_name);
                let right_expr = right.to_rust_expr(ctx)?;
                let op_token = match op {
                    BinOp::Add => quote! { + },
                    BinOp::Sub => quote! { - },
                    BinOp::Mul => quote! { * },
                    BinOp::Div => quote! { / },
                    BinOp::FloorDiv => quote! { / },
                    BinOp::Mod => quote! { % },
                    BinOp::BitAnd => quote! { & },
                    BinOp::BitOr => quote! { | },
                    BinOp::BitXor => quote! { ^ },
                    BinOp::LShift => quote! { << },
                    BinOp::RShift => quote! { >> },
                    BinOp::Pow => {
                        return Ok(quote! {
                            #var_ident = Some(#var_ident.unwrap().pow(#right_expr as u32));
                        });
                    }
                    _ => bail!("Unsupported augmented assignment operator for Optional variable"),
                };

                return Ok(quote! {
                    #var_ident = Some(#var_ident.unwrap() #op_token #right_expr);
                });
            }
        }
    }

    // Handle general augmented assignment: target op= value
    // This converts x = x + 1 to x += 1, obj.field = obj.field + 1 to obj.field += 1, etc.
    // Only applies to simple cases where target and left side of binary expr are the same
    // Skip compound assignment for generator state variables - use expanded form instead
    let skip_compound_for_generator = if let AssignTarget::Symbol(var_name) = target {
        ctx.in_generator && ctx.generator_state_vars.contains(var_name)
    } else {
        false
    };

    if !skip_compound_for_generator {
        if let Some(op) = is_augassign_pattern(target, value) {
            if let HirExpr::Binary { right, .. } = value {
                // Set assignment target flag to prevent .clone() on struct field access
                let was_assignment_target = ctx.is_assignment_target;
                ctx.is_assignment_target = true;

                let target_expr = match target {
                    AssignTarget::Symbol(var_name) => {
                        let ident = safe_ident(var_name);
                        parse_quote! { #ident }
                    }
                    AssignTarget::Attribute { value, attr } => {
                        let base_expr = value.to_rust_expr(ctx)?;
                        let attr_ident = format_ident!("{}", attr);
                        parse_quote! { #base_expr.#attr_ident }
                    }
                    AssignTarget::Index { base, index } => {
                        let base_expr = base.to_rust_expr(ctx)?;
                        let index_expr = index.to_rust_expr(ctx)?;
                        parse_quote! { #base_expr[#index_expr as usize] }
                    }
                    _ => {
                        // For other target types, fall through to normal assignment
                        // (tuple unpacking, slicing, etc. don't support augmented assignment)
                        syn::Expr::Verbatim(quote! {})
                    }
                };

                ctx.is_assignment_target = was_assignment_target;

                // Only proceed if we successfully generated a target expression
                if !matches!(target_expr, syn::Expr::Verbatim(_)) {
                    // List concatenation: x = x + [elem] => x.extend(vec![elem])
                    // Vec doesn't implement AddAssign, so use .extend() instead
                    if matches!(op, BinOp::Add)
                        && is_list_concat_augassign(target, right, ctx)
                    {
                        let right_expr = right.to_rust_expr(ctx)?;
                        return Ok(quote! {
                            #target_expr.extend(#right_expr);
                        });
                    }

                    let right_expr = right.to_rust_expr(ctx)?;
                    let op_token = match op {
                        BinOp::Add => quote! { += },
                        BinOp::Sub => quote! { -= },
                        BinOp::Mul => quote! { *= },
                        BinOp::Div => quote! { /= },
                        BinOp::FloorDiv => quote! { /= }, // Floor division maps to /= in Rust
                        BinOp::Mod => quote! { %= },
                        BinOp::BitAnd => quote! { &= },
                        BinOp::BitOr => quote! { |= },
                        BinOp::BitXor => quote! { ^= },
                        BinOp::LShift => quote! { <<= },
                        BinOp::RShift => quote! { >>= },
                        BinOp::Pow => {
                            // Power doesn't have a compound assignment in Rust
                            // Fall through to normal assignment
                            return Ok(quote! {}); // Will be handled by normal assignment path
                        }
                        _ => {
                            // For unsupported operators, fall through to normal assignment
                            return Ok(quote! {});
                        }
                    };

                    return Ok(quote! {
                        #target_expr #op_token #right_expr;
                    });
                }
            }
        }
    }

    // This allows proper method dispatch for user-defined classes
    if let AssignTarget::Symbol(var_name) = target {
        // This enables correct {:?} vs {} selection in println! for collections
        // Example: result = merge(&a, &b) where merge returns Vec<i32>
        // Also track Optional types for proper Some() wrapping in reassignments
        if let Some(annot_type) = type_annotation {
            match annot_type {
                Type::List(_) | Type::Dict(_, _) | Type::Set(_) | Type::Optional(_)
                | Type::Tuple(_) => {
                    ctx.var_types.insert(var_name.clone(), annot_type.clone());
                    // Track variables declared as Option<T> for proper unwrapping in field access
                    if matches!(annot_type, Type::Optional(_)) {
                        ctx.optional_vars.insert(var_name.clone());
                    }
                }
                _ => {}
            }
        }

        match value {
            HirExpr::Call { func, args, .. } => {
                // Check if this is a user-defined class constructor
                if ctx.class_names.contains(func) {
                    ctx.var_types
                        .insert(var_name.clone(), Type::Custom(func.clone()));
                }
                // This enables correct HashSet.contains() vs HashMap.contains_key() selection
                else if func == "set" {
                    // Infer element type from type annotation or default to Int
                    let elem_type = if let Some(Type::Set(elem)) = type_annotation {
                        elem.as_ref().clone()
                    } else {
                        Type::Int // Default for untyped sets
                    };
                    ctx.var_types
                        .insert(var_name.clone(), Type::Set(Box::new(elem_type)));
                }
                // Lookup function return type and track it for type inference
                // Enables: result = merge(&a, &b) where merge returns list[int]
                // Also tracks primitive types (Int, Float, Bool, String) for arithmetic/concat detection
                // And Custom types (structs/dataclasses) for clone analysis
                else if let Some(ret_type) = ctx.function_return_types.get(func) {
                    if matches!(
                        ret_type,
                        Type::List(_)
                            | Type::Dict(_, _)
                            | Type::Set(_)
                            | Type::Tuple(_)
                            | Type::Int
                            | Type::Float
                            | Type::Bool
                            | Type::String
                            | Type::Custom(_)
                    ) {
                        ctx.var_types.insert(var_name.clone(), ret_type.clone());
                    }
                    // Track if function returns Optional type
                    if matches!(ret_type, Type::Optional(_)) {
                        ctx.var_types.insert(var_name.clone(), ret_type.clone());
                        ctx.optional_vars.insert(var_name.clone());
                    }
                }
                // These all return Option<Match> in Rust
                else if matches!(func.as_str(), "search" | "match" | "find") {
                    // Only track if this looks like a regex call (needs more context to be sure)
                    // For now, track any call to search/match/find as Optional
                    // This is a heuristic - could be improved with module tracking
                    ctx.var_types
                        .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                    ctx.optional_vars.insert(var_name.clone());
                }
                // Track built-in functions that return int
                else if matches!(func.as_str(), "len" | "int" | "ord" | "round") {
                    ctx.var_types.insert(var_name.clone(), Type::Int);
                }
                // Track built-in functions that return float
                else if func == "float" {
                    ctx.var_types.insert(var_name.clone(), Type::Float);
                }
                // Track abs() - returns the same type as its argument
                else if func == "abs" {
                    if !args.is_empty() {
                        let arg_type = infer_expr_type_with_env(&args[0], &ctx.var_types);
                        if matches!(arg_type, Type::Int | Type::Float) {
                            ctx.var_types.insert(var_name.clone(), arg_type);
                        } else {
                            // Default to Int for abs() if we can't infer
                            ctx.var_types.insert(var_name.clone(), Type::Int);
                        }
                    }
                }
                // Track min() and max() - return Float if any argument is Float, else Int
                else if matches!(func.as_str(), "min" | "max") {
                    if !args.is_empty() {
                        let has_float = args.iter().any(|arg| {
                            matches!(infer_expr_type_with_env(arg, &ctx.var_types), Type::Float)
                        });
                        if has_float {
                            ctx.var_types.insert(var_name.clone(), Type::Float);
                        } else {
                            ctx.var_types.insert(var_name.clone(), Type::Int);
                        }
                    }
                }
                // Track next() builtin: with default (2 args) returns Option<T>, without default (1 arg) returns T
                else if func == "next" {
                    if args.len() == 2 {
                        // next(iter, default) returns Option<T> which unwraps to default if None
                        ctx.var_types
                            .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                        ctx.optional_vars.insert(var_name.clone());
                    }
                    // next(iter) without default uses .expect() and returns T directly (not Optional)
                }
                // Track divmod() as returning a tuple of (int, int)
                else if func == "divmod" {
                    ctx.var_types.insert(
                        var_name.clone(),
                        Type::Tuple(vec![Type::Int, Type::Int]),
                    );
                }
            }
            HirExpr::Tuple(elements) => {
                let elem_types: Vec<Type> = elements
                    .iter()
                    .map(|e| infer_expr_type_with_env(e, &ctx.var_types))
                    .collect();
                ctx.var_types
                    .insert(var_name.clone(), Type::Tuple(elem_types));
            }
            HirExpr::List(elements) => {
                // When v = [1, 2], mark v as List(Int) so it gets borrowed when calling f(&v)
                // When v = [(1, 2), (3, 4)], mark v as List(Tuple(Int, Int)) for proper tuple indexing
                let elem_type = if let Some(Type::List(elem)) = type_annotation {
                    elem.as_ref().clone()
                } else if !elements.is_empty() {
                    // Infer from first element (handles tuples, nested lists, etc.)
                    infer_expr_type_with_env(&elements[0], &ctx.var_types)
                } else {
                    Type::Unknown
                };
                ctx.var_types
                    .insert(var_name.clone(), Type::List(Box::new(elem_type)));
            }
            HirExpr::Dict(items) => {
                // When info = {"a": 1}, mark info as Dict(String, Int) so it gets borrowed
                let (key_type, val_type) = if let Some(Type::Dict(k, v)) = type_annotation {
                    (k.as_ref().clone(), v.as_ref().clone())
                } else if !items.is_empty() {
                    // Infer from first item (assume homogeneous dict)
                    // For string literal keys and int values
                    (Type::String, Type::Int)
                } else {
                    (Type::Unknown, Type::Unknown)
                };
                ctx.var_types.insert(
                    var_name.clone(),
                    Type::Dict(Box::new(key_type), Box::new(val_type)),
                );
            }
            HirExpr::Set(elements) | HirExpr::FrozenSet(elements) => {
                // Track set type from literal for proper method dispatch
                // Use type annotation if available, otherwise infer from elements
                let elem_type = if let Some(Type::Set(elem)) = type_annotation {
                    elem.as_ref().clone()
                } else if !elements.is_empty() {
                    // Infer from first element (assume homogeneous set)
                    // For int literals, use Int type
                    Type::Int
                } else {
                    Type::Unknown
                };
                ctx.var_types
                    .insert(var_name.clone(), Type::Set(Box::new(elem_type)));
            }
            HirExpr::Slice { base, .. } => {
                // When rest = numbers[1:], mark rest as List(Int) so it gets borrowed on call
                // Infer element type from base variable if available
                let elem_type = if let HirExpr::Var(base_var) = base.as_ref() {
                    if let Some(Type::List(elem)) = ctx.var_types.get(base_var) {
                        elem.as_ref().clone()
                    } else {
                        Type::Int // Default to Int for untyped slices
                    }
                } else {
                    Type::Int // Default to Int
                };
                ctx.var_types
                    .insert(var_name.clone(), Type::List(Box::new(elem_type)));
            }
            // E.g., value_str = data.get(...) where data: Vec<String> → value_str: String
            HirExpr::MethodCall { object, method, args, .. } => {
                // Track .get() return type: Option for 1-arg, unwrapped for 2-arg
                if method == "get" {
                    if let HirExpr::Var(obj_var) = object.as_ref() {
                        if let Some(Type::List(elem_type)) = ctx.var_types.get(obj_var) {
                            if args.len() == 1 {
                                // 1-arg .get() returns Option<&T> → Option<T> after .cloned()
                                ctx.var_types.insert(
                                    var_name.clone(),
                                    Type::Optional(Box::new(elem_type.as_ref().clone())),
                                );
                                ctx.optional_vars.insert(var_name.clone());
                            } else {
                                // 2-arg .get() uses unwrap_or → returns T directly
                                ctx.var_types
                                    .insert(var_name.clone(), elem_type.as_ref().clone());
                            }
                        } else if let Some(Type::Dict(_, val_type)) = ctx.var_types.get(obj_var) {
                            if args.len() == 1 {
                                // dict.get(key) returns Option<&V> → Option<V> after .cloned()
                                ctx.var_types.insert(
                                    var_name.clone(),
                                    Type::Optional(Box::new(val_type.as_ref().clone())),
                                );
                                ctx.optional_vars.insert(var_name.clone());
                            } else {
                                // dict.get(key, default) returns V directly
                                ctx.var_types
                                    .insert(var_name.clone(), val_type.as_ref().clone());
                            }
                        }
                    }
                }
                // Track .split() and .split_whitespace() as List(String) for truthiness conversion
                else if matches!(method.as_str(), "split" | "split_whitespace" | "splitlines") {
                    ctx.var_types
                        .insert(var_name.clone(), Type::List(Box::new(Type::String)));
                }
                // String methods that return String
                else if matches!(
                    method.as_str(),
                    "upper"
                        | "lower"
                        | "strip"
                        | "lstrip"
                        | "rstrip"
                        | "title"
                        | "replace"
                        | "format"
                ) {
                    ctx.var_types.insert(var_name.clone(), Type::String);
                }
                // str.find() returns int; regex .find()/.search()/.match() returns Optional
                else if matches!(method.as_str(), "find" | "search" | "match") {
                    let obj_type = infer_expr_type_with_env(object, &ctx.var_types);
                    if matches!(obj_type, Type::String) && method == "find" {
                        ctx.var_types.insert(var_name.clone(), Type::Int);
                    } else {
                        ctx.var_types
                            .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                        ctx.optional_vars.insert(var_name.clone());
                    }
                }
                // Track .next() as Optional since it returns Option<T>
                else if method == "next" {
                    ctx.var_types
                        .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                    ctx.optional_vars.insert(var_name.clone());
                }
            }
            // When message = "hello", track message as String so it gets borrowed when calling f(&str)
            // But preserve Optional wrapper if the variable was declared as Optional[str]
            HirExpr::Literal(Literal::String(_)) => {
                // Only update if not already tracked, or if tracked but not Optional
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::String);
                }
            }
            // When match = 5, track match as Int so `if match:` becomes `if match != 0`
            // But preserve Optional wrapper if the variable was declared as Optional[int]
            HirExpr::Literal(Literal::Int(_)) => {
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::Int);
                }
            }
            HirExpr::Literal(Literal::Float(_)) => {
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::Float);
                }
            }
            HirExpr::Literal(Literal::Bool(_)) => {
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::Bool);
                }
            }
            // Track binary arithmetic expressions (e.g., c = a - b * 4)
            HirExpr::Binary { op, left, right } => {
                if !ctx.var_types.contains_key(var_name) {
                    let inferred_type = infer_binary_expr_type(ctx, op, left, right);
                    ctx.var_types.insert(var_name.clone(), inferred_type);
                }
            }
            // Track list comprehensions: exps = [math.exp(x) for x in logits]
            HirExpr::ListComp { element, .. } => {
                let elem_type = infer_expr_type_with_env(element, &ctx.var_types);
                ctx.var_types
                    .insert(var_name.clone(), Type::List(Box::new(elem_type)));
            }
            // Track set comprehensions: unique = {x.lower() for x in words}
            HirExpr::SetComp { element, .. } => {
                let elem_type = infer_expr_type_with_env(element, &ctx.var_types);
                ctx.var_types
                    .insert(var_name.clone(), Type::Set(Box::new(elem_type)));
            }
            // Track dict comprehensions: counts = {k: v * 2 for k, v in items}
            HirExpr::DictComp {
                key,
                value: val_expr,
                ..
            } => {
                let key_type = infer_expr_type_with_env(key, &ctx.var_types);
                let val_type = infer_expr_type_with_env(val_expr, &ctx.var_types);
                ctx.var_types.insert(
                    var_name.clone(),
                    Type::Dict(Box::new(key_type), Box::new(val_type)),
                );
            }
            // Track ternary expressions: win_factor = 500 if team_won else 0
            HirExpr::IfExpr { body, orelse, .. } => {
                if !ctx.var_types.contains_key(var_name) {
                    let body_type = infer_expr_type_with_env(body, &ctx.var_types);
                    let orelse_type = infer_expr_type_with_env(orelse, &ctx.var_types);
                    // If both branches have the same type, use that type
                    // Otherwise, if one is Float and one is Int, prefer Float (promotion)
                    let inferred_type = if body_type == orelse_type {
                        body_type
                    } else if matches!(
                        (&body_type, &orelse_type),
                        (Type::Float, Type::Int) | (Type::Int, Type::Float)
                    ) {
                        Type::Float
                    } else {
                        // Default to the body type if we can't unify
                        body_type
                    };
                    ctx.var_types.insert(var_name.clone(), inferred_type);
                }
            }
            // Propagate types from one variable to another: abs_margin = _cse_temp_0
            HirExpr::Var(source_var) => {
                if !ctx.var_types.contains_key(var_name) {
                    if let Some(source_type) = ctx.var_types.get(source_var) {
                        ctx.var_types.insert(var_name.clone(), source_type.clone());
                    }
                }
            }
            // Track attribute access types: x = obj.field where field might be Optional
            HirExpr::Attribute { value, attr } => {
                if !ctx.var_types.contains_key(var_name) {
                    if let Some(field_type) = ctx.get_attribute_field_type(value, attr) {
                        ctx.var_types.insert(var_name.clone(), field_type);
                    }
                }
            }
            // Track index access types: x = arr[i] where arr is List<T> means x is T
            HirExpr::Index { base, .. } => {
                if !ctx.var_types.contains_key(var_name) {
                    // Use get_expr_type first to resolve Attribute bases through class field
                    // types (e.g., state.players[idx] resolves state.players via class fields)
                    let base_type = ctx.get_expr_type(base)
                        .unwrap_or_else(|| infer_expr_type_with_env(base, &ctx.var_types));
                    let elem_type = match base_type {
                        Type::List(elem) => Some(*elem),
                        Type::Tuple(elems) => elems.first().cloned(),
                        Type::Dict(_, val) => Some(*val),
                        _ => None,
                    };
                    if let Some(t) = elem_type {
                        ctx.var_types.insert(var_name.clone(), t);
                    }
                }
            }
            _ => {}
        }
    }

    // Handle uninitialized annotated declarations (Python: `x: T`)
    let is_uninitialized = matches!(value, HirExpr::Uninitialized);

    // Track type for uninitialized annotated declarations
    // This ensures Optional types are tracked for truthiness conversion in conditionals
    if is_uninitialized {
        if let AssignTarget::Symbol(var_name) = target {
            if let Some(annot_type) = type_annotation {
                ctx.var_types.insert(var_name.clone(), annot_type.clone());
            }
        }
    }

    // Check if this is a field access assignment that can use borrowing
    // Pattern: `let players = state.all_players` where players is only used for iteration
    // Also handles: `let team_stats = (state.home_stats if cond else state.away_stats)`
    // Don't borrow Copy types (primitives like i32, f64, bool) - they should be copied directly
    // Don't borrow enum variants - they are Copy types
    let (should_borrow, should_mut_borrow) = if let AssignTarget::Symbol(var_name) = target {
        let is_attribute_sourced = is_attribute_sourced_expr(value);
        let is_copy_type = if let HirExpr::Attribute { value: base, attr } = value {
            ctx.is_attribute_copy_type(base, attr)
        } else if let HirExpr::IfExpr { body, orelse, .. } = value {
            is_copy_type_branch(body, ctx) && is_copy_type_branch(orelse, ctx)
        } else {
            false
        };
        // Check if the value (or both branches of an IfExpr) are enum variants
        let is_enum_variant = is_enum_variant_expr(value, ctx);

        // Check if this is an empty collection initialization for a variable that will be
        // later assigned from an attribute source. In this case, we need to declare it
        // with a reference type even though the current value isn't attribute-sourced.
        let is_empty_init_for_borrowable = is_empty_collection_init_expr(value)
            && (ctx.should_borrow_var(var_name) || ctx.should_mut_borrow_var(var_name));

        if is_attribute_sourced && !is_copy_type && !is_enum_variant {
            if ctx.should_mut_borrow_var(var_name) {
                (false, true)
            } else if ctx.should_borrow_var(var_name) {
                (true, false)
            } else {
                (false, false)
            }
        } else {
            (false, false)
        }
    } else {
        (false, false)
    };

    // For type annotations, use the same borrow flags as for values
    let (type_should_borrow, type_should_mut_borrow) = (should_borrow, should_mut_borrow);

    // Check if this variable should be a mutable reference from subscript access
    // Pattern: `player = state.players[idx]` where player's fields are later mutated
    let is_mut_ref_index = if let AssignTarget::Symbol(var_name) = target {
        ctx.mut_ref_index_vars.contains(var_name) && matches!(value, HirExpr::Index { .. })
    } else {
        false
    };

    // Convert the value expression unless it's an Uninitialized marker
    let mut value_expr = if is_uninitialized {
        // Placeholder; won't be used when is_uninitialized is true
        parse_quote! { () }
    } else if is_mut_ref_index {
        // Generate get_mut().unwrap() instead of get().cloned().unwrap()
        let was_assignment_target = ctx.is_assignment_target;
        ctx.is_assignment_target = true;
        let expr = value.to_rust_expr(ctx)?;
        ctx.is_assignment_target = was_assignment_target;
        expr
    } else if should_mut_borrow {
        // Generate a mutable borrow for field access from &mut T source
        ctx.set_generate_borrow(true);
        ctx.set_generate_mut_borrow(true);
        let expr = value.to_rust_expr(ctx)?;
        ctx.set_generate_mut_borrow(false);
        ctx.set_generate_borrow(false);
        // For conditional expressions (IfExpr), the &mut is added inside each branch
        // For direct attribute access, wrap in &mut
        if matches!(value, HirExpr::IfExpr { .. }) {
            expr
        } else {
            parse_quote! { &mut #expr }
        }
    } else if should_borrow {
        // Generate an immutable borrow instead of clone for field access
        ctx.set_generate_borrow(true);
        let expr = value.to_rust_expr(ctx)?;
        ctx.set_generate_borrow(false);
        // For conditional expressions (IfExpr), the & is added inside each branch
        // For direct attribute access, wrap in &
        if matches!(value, HirExpr::IfExpr { .. }) {
            expr
        } else {
            parse_quote! { &#expr }
        }
    } else {
        value.to_rust_expr(ctx)?
    };

    // Static array constants need .to_vec() when assigned to local variables,
    // since the local expects Vec<T> but the constant is [T; N].
    if let HirExpr::Var(rhs_name) = value {
        if ctx.static_array_constants.contains(rhs_name) {
            value_expr = parse_quote! { #value_expr.to_vec() };
        }
    }

    // BORROW CONFLICT RESOLUTION:
    // If this variable is in vars_needing_clone_at_assign, it holds a reference
    // that would conflict with a later mutable borrow. Clone/to_vec to release
    // the borrow immediately.
    // Pattern detected: var1 = f(&state) returns &T, var2 = g(&mut state), use(var1)
    if let AssignTarget::Symbol(var_name) = target {
        if ctx.vars_needing_clone_at_assign.contains(var_name) {
            // Check if the value is a function call that likely returns &Vec<T>
            if let HirExpr::Call { .. } = value {
                // Use .to_vec() for Vec references, .clone() for others
                // Heuristic: if function name contains "get_players" or similar list getters
                // Actually, safer to use .clone() which works for both Vec and other types
                value_expr = parse_quote! { #value_expr.clone() };
            }
        }
    }

    // When assigning from a function that returns Result<T, E> in a non-Result context,
    // we need to unwrap it.

    // If there's a type annotation, handle type conversions
    let (type_annotation_tokens, is_final) = if let Some(target_type) = type_annotation {
        // Check if this is a Final type annotation
        let (actual_type, is_const) = match target_type {
            Type::Final(inner) => (inner.as_ref(), true),
            _ => (target_type, false),
        };

        let target_rust_type = ctx.type_mapper.map_type(actual_type);
        let target_syn_type = rust_type_to_syn(&target_rust_type)?;

        // When borrowing, wrap the type in a reference
        // Use type_should_borrow which accounts for empty collection inits
        let final_syn_type: syn::Type = if type_should_borrow {
            parse_quote! { &#target_syn_type }
        } else if type_should_mut_borrow {
            parse_quote! { &mut #target_syn_type }
        } else {
            target_syn_type
        };

        // Auto-unwrap Optional values when assigning to non-Optional annotated variables
        let value_is_optional = expr_is_optional(value, ctx);
        let target_is_optional = matches!(actual_type, Type::Optional(_));
        if value_is_optional && !target_is_optional {
            value_expr = parse_quote! { #value_expr.unwrap() };
        }

        // Pass the value expression to determine if cast is actually needed
        // NOTE: This handles string literals → String conversion via apply_type_conversion
        if needs_type_conversion(actual_type, value) {
            value_expr = apply_type_conversion(value_expr, actual_type);
        }

        (Some(quote! { : #final_syn_type }), is_const)
    } else {
        // No explicit type annotation, but we may still need conversions
        // When assigning string literals without type annotation,
        // they should become owned Strings (Python semantics)
        if matches!(value, HirExpr::Literal(Literal::String(_))) {
            value_expr = parse_quote! { #value_expr.to_string() };
        }
        // NOTE: Struct field cloning is handled in convert_attribute via field_needs_clone
        // which properly checks if the field type is Copy or not
        (None, false)
    };

    // When assigning to an Option<T> variable, wrap non-None values in Some()
    if let AssignTarget::Symbol(symbol) = target {
        // Check if the variable has an Optional type (either from annotation or previous declaration)
        let is_optional_type = if let Some(target_type) = type_annotation {
            matches!(target_type, Type::Optional(_))
        } else if ctx.is_declared(symbol) {
            // Check if variable was previously declared with Optional type
            ctx.var_types
                .get(symbol)
                .map_or(false, |ty| matches!(ty, Type::Optional(_)))
        } else {
            false
        };

        // Check if the value being assigned is already Optional (to avoid double-wrapping)
        let value_is_optional = expr_is_optional(value, ctx);

        // Wrap non-None values in Some() when assigning to Option<T>, unless the value is already Optional
        if is_optional_type
            && !value_is_optional
            && !matches!(value, HirExpr::Literal(Literal::None))
        {
            value_expr = parse_quote! { Some(#value_expr) };
        }
    }

    // When assigning to an Optional attribute (struct field), wrap non-None values in Some()
    // This handles augmented assignments like `c.value += 1` where `c.value: Optional[int]`
    // which get transformed by CSE into `let _cse_temp = c.value.unwrap() + 1; c.value = _cse_temp;`
    if let AssignTarget::Attribute {
        value: target_base,
        attr,
    } = target
    {
        // Build a temporary HirExpr::Attribute to check if the target field is Optional
        let target_attr_expr = HirExpr::Attribute {
            value: target_base.clone(),
            attr: attr.clone(),
        };
        let target_is_optional = expr_is_optional(&target_attr_expr, ctx);

        // Check if the value being assigned is already Optional (to avoid double-wrapping)
        let value_is_optional = expr_is_optional(value, ctx);

        // Wrap non-None values in Some() when assigning to Optional field
        if target_is_optional
            && !value_is_optional
            && !matches!(value, HirExpr::Literal(Literal::None))
        {
            value_expr = parse_quote! { Some(#value_expr) };
        }

        // Unwrap Optional values when assigning to non-Optional field
        // This handles cases like: state.field = optional_var.clone()
        // where state.field: T but optional_var: Option<T>
        if !target_is_optional && value_is_optional {
            value_expr = parse_quote! { #value_expr.unwrap() };
        }

        // Clone reference parameters when assigning to struct fields.
        // When assigning `&T` to a field expecting `T`, we need to clone.
        // Example: player.sin_bin_status = sin_bin_status where sin_bin_status: &SinBinStatus
        if let HirExpr::Var(var_name) = value {
            if ctx.current_func_ref_params.contains(var_name)
                && !ctx.shadowed_ref_params.contains(var_name)
            {
                value_expr = parse_quote! { #value_expr.clone() };
            }
        }
    }

    // If this is an annotated declaration without a value, emit a declaration
    // without initializer. Only supported for simple symbol targets.
    if is_uninitialized {
        if let AssignTarget::Symbol(symbol) = target {
            // Declare the variable in the current scope
            ctx.declare_var(symbol);
            let target_ident = safe_ident(symbol);

            // If mutable, include mut
            let mutability = if ctx.mutable_vars.contains(symbol) {
                quote! { mut }
            } else {
                quote! {}
            };

            if let Some(type_ann) = type_annotation_tokens {
                // let mut x: Type;
                return Ok(quote! { let #mutability #target_ident #type_ann; });
            } else {
                // No type annotation - emit simple declaration without initializer
                // This is not valid Rust, so fall back to declaring with a unit value
                // to avoid generating invalid code. Prefer explicit type annotations
                // in source to avoid this path.
                if ctx.mutable_vars.contains(symbol) {
                    return Ok(quote! { let mut #target_ident = (); });
                } else {
                    return Ok(quote! { let #target_ident = (); });
                }
            }
        } else {
            // Annotated declarations without values for complex targets are unsupported
            bail!("Annotated assignment without value for non-symbol target not supported")
        }
    }

    // Clone variables consumed in multiple move positions to prevent use-after-move.
    // e.g., player.total_statistics = total_statistics; list.push(total_statistics);
    if let HirExpr::Var(var_name) = value {
        if ctx.should_clone_for_move(var_name) {
            value_expr = parse_quote! { #value_expr.clone() };
        }
    }

    // Track optional_vars for tuple element variables when the RHS function returns
    // a Tuple containing Optional elements (e.g., `player, idx = resolve(...)` where
    // resolve returns Tuple[Optional[Player], int])
    if let AssignTarget::Tuple(targets) = target {
        let tuple_types = match value {
            HirExpr::Call { func, .. } => ctx
                .function_return_types
                .get(func)
                .and_then(|rt| match rt {
                    Type::Tuple(types) => Some(types.clone()),
                    _ => None,
                }),
            _ => None,
        };
        if let Some(types) = tuple_types {
            for (tgt, ty) in targets.iter().zip(types.iter()) {
                if let AssignTarget::Symbol(var_name) = tgt {
                    if matches!(ty, Type::Optional(_)) {
                        ctx.optional_vars.insert(var_name.clone());
                        ctx.var_types.insert(var_name.clone(), ty.clone());
                    }
                }
            }
        }
    }

    match target {
        AssignTarget::Symbol(symbol) => {
            codegen_assign_symbol(symbol, value_expr, type_annotation_tokens, is_final, ctx)
        }
        AssignTarget::Index { base, index } => codegen_assign_index(base, index, value_expr, ctx),
        AssignTarget::Slice { base, .. } => codegen_assign_slice(base, value_expr, ctx),
        AssignTarget::Attribute { value, attr } => {
            codegen_assign_attribute(value, attr, value_expr, ctx)
        }
        AssignTarget::Tuple(targets) => {
            codegen_assign_tuple(targets, value_expr, type_annotation_tokens, ctx)
        }
        AssignTarget::Starred(_) => {
            bail!("Starred expression can only appear inside tuple unpacking")
        }
    }
}

pub(crate) fn codegen_assign_symbol(
    symbol: &str,
    value_expr: syn::Expr,
    type_annotation_tokens: Option<proc_macro2::TokenStream>,
    is_final: bool,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let target_ident = safe_ident(symbol);

    // Inside generators, check if variable is a state variable
    if ctx.in_generator && ctx.generator_state_vars.contains(symbol) {
        // State variable assignment: self.field = value
        Ok(quote! { self.#target_ident = #value_expr; })
    } else if is_final {
        // Final type annotation - generate const instead of let
        if let Some(type_ann) = type_annotation_tokens {
            Ok(quote! { const #target_ident #type_ann = #value_expr; })
        } else {
            // Final without explicit type annotation - shouldn't happen, but handle gracefully
            Ok(quote! { const #target_ident = #value_expr; })
        }
    } else if ctx.is_declared(symbol) {
        // Variable already exists, just assign
        Ok(quote! { #target_ident = #value_expr; })
    } else {
        // First declaration - check if variable needs mut
        ctx.declare_var(symbol);
        if ctx.mutable_vars.contains(symbol) {
            if let Some(type_ann) = type_annotation_tokens {
                Ok(quote! { let mut #target_ident #type_ann = #value_expr; })
            } else {
                Ok(quote! { let mut #target_ident = #value_expr; })
            }
        } else if let Some(type_ann) = type_annotation_tokens {
            Ok(quote! { let #target_ident #type_ann = #value_expr; })
        } else {
            Ok(quote! { let #target_ident = #value_expr; })
        }
    }
}

pub(crate) fn codegen_assign_index(
    base: &HirExpr,
    index: &HirExpr,
    value_expr: syn::Expr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let mut final_index = index.to_rust_expr(ctx)?;

    // Check base variable type to determine if this is Vec or HashMap
    // Vec.insert() requires usize index, HashMap.insert() takes key of any type
    let is_numeric_index = if let HirExpr::Var(base_name) = base {
        // Check if we have type information for this variable
        if let Some(base_type) = ctx.var_types.get(base_name) {
            // Type-based detection (most reliable)
            match base_type {
                Type::List(_) => true,     // List/Vec → numeric index
                Type::Dict(_, _) => false, // Dict/HashMap → key (not numeric)
                _ => {
                    // Fall back to index heuristic for other types
                    match index {
                        HirExpr::Var(name) => {
                            let name_str = name.as_str();
                            // String-like variable names → NOT numeric
                            if name_str == "key"
                                || name_str == "k"
                                || name_str == "name"
                                || name_str == "id"
                                || name_str == "word"
                                || name_str == "text"
                                || name_str == "char"
                                || name_str == "character"
                                || name_str == "c"
                                || name_str.ends_with("_key")
                                || name_str.ends_with("_name")
                            {
                                false
                            } else {
                                // Default: assume numeric for other variables
                                true
                            }
                        }
                        HirExpr::Binary { .. } | HirExpr::Literal(crate::hir::Literal::Int(_)) => {
                            true
                        }
                        _ => false,
                    }
                }
            }
        } else {
            // No type info - use heuristic
            match index {
                HirExpr::Var(name) => {
                    let name_str = name.as_str();
                    // String-like variable names → NOT numeric
                    if name_str == "key"
                        || name_str == "k"
                        || name_str == "name"
                        || name_str == "id"
                        || name_str == "word"
                        || name_str == "text"
                        || name_str == "char"
                        || name_str == "character"
                        || name_str == "c"
                        || name_str.ends_with("_key")
                        || name_str.ends_with("_name")
                    {
                        false
                    } else {
                        // Default: assume numeric for other variables
                        true
                    }
                }
                HirExpr::Binary { .. } | HirExpr::Literal(crate::hir::Literal::Int(_)) => true,
                _ => false,
            }
        }
    } else {
        // Base is not a simple variable - use heuristic
        match index {
            HirExpr::Var(name) => {
                let name_str = name.as_str();
                // String-like variable names → NOT numeric
                if name_str == "key"
                    || name_str == "k"
                    || name_str == "name"
                    || name_str == "id"
                    || name_str == "word"
                    || name_str == "text"
                    || name_str == "char"
                    || name_str == "character"
                    || name_str == "c"
                    || name_str.ends_with("_key")
                    || name_str.ends_with("_name")
                {
                    false
                } else {
                    // Default: assume numeric for other variables
                    true
                }
            }
            HirExpr::Binary { .. } | HirExpr::Literal(crate::hir::Literal::Int(_)) => true,
            _ => false,
        }
    };

    // Convert string literal keys to String for HashMap operations
    // String literals (&str) need to be converted to String for HashMap<String, V>
    if !is_numeric_index && matches!(index, HirExpr::Literal(Literal::String(_))) {
        final_index = parse_quote! { #final_index.to_string() };
    }

    // Extract the base and all intermediate indices
    // For mutation operations on field accesses (both dict and list), we need to avoid cloning
    // Check if the base (or its root for nested Index) is a field access
    let base_is_field_access = match base {
        HirExpr::Attribute { .. } => true,
        HirExpr::Index {
            base: inner_base, ..
        } => {
            // For nested subscripts like matrix[0][1], check if the root is an attribute
            fn has_attribute_root(expr: &HirExpr) -> bool {
                match expr {
                    HirExpr::Attribute { .. } => true,
                    HirExpr::Index { base, .. } => has_attribute_root(base),
                    _ => false,
                }
            }
            has_attribute_root(inner_base)
        }
        _ => false,
    };

    let (base_expr, indices) = if base_is_field_access {
        // Generate field access without clone for mutation operations
        extract_nested_indices_tokens_no_clone(base, ctx)?
    } else {
        extract_nested_indices_tokens(base, ctx)?
    };

    // Check if value_expr is a string literal and the dict value type is String
    let value_expr = if !is_numeric_index {
        // Get the base variable name to look up its type
        let base_name = match base {
            HirExpr::Var(name) => Some(name.as_str()),
            HirExpr::Index {
                base: inner_base, ..
            } => {
                // For nested subscripts, get the root variable
                fn get_root_var(expr: &HirExpr) -> Option<&str> {
                    match expr {
                        HirExpr::Var(name) => Some(name.as_str()),
                        HirExpr::Index { base, .. } => get_root_var(base),
                        _ => None,
                    }
                }
                get_root_var(inner_base)
            }
            _ => None,
        };

        // Check if we need to convert string literal to String
        let needs_string_conversion = if let Some(name) = base_name {
            if let Some(base_type) = ctx.var_types.get(name) {
                // Navigate through nested Dict types to find the innermost value type
                let depth = indices.len() + 1; // +1 for the final index
                let mut current_type = base_type.clone();
                for _ in 0..depth {
                    if let Type::Dict(_, val_type) = current_type {
                        current_type = (*val_type).clone();
                    } else {
                        break;
                    }
                }
                // Check if innermost value type is String
                matches!(current_type, Type::String)
            } else {
                false
            }
        } else {
            false
        };

        // Check if value_expr is a string literal
        let is_string_literal =
            matches!(&value_expr, syn::Expr::Lit(lit) if matches!(&lit.lit, syn::Lit::Str(_)));

        if needs_string_conversion && is_string_literal {
            parse_quote! { #value_expr.to_string() }
        } else {
            value_expr
        }
    } else {
        value_expr
    };

    // Check variable type from context first, then fall back to name heuristic
    let needs_as_object_mut = if let HirExpr::Var(base_name) = base {
        if !is_numeric_index {
            // First check actual type from context
            if let Some(var_type) = ctx.var_types.get(base_name) {
                // If we know the type is Dict/HashMap, don't use as_object_mut
                if matches!(var_type, Type::Dict(_, _)) {
                    false
                } else {
                    // For Unknown types, use name heuristic
                    let name_str = base_name.as_str();
                    // Variables commonly used with serde_json::Value
                    name_str == "config"
                        || name_str == "value"
                        || name_str == "current"
                        || name_str == "obj"
                        || name_str == "json"
                }
            } else {
                // No type info available, use name heuristic
                // But exclude "data" as it's commonly used for HashMap
                let name_str = base_name.as_str();
                name_str == "config"
                    || name_str == "value"
                    || name_str == "current"
                    || name_str == "obj"
                    || name_str == "json"
            }
        } else {
            false
        }
    } else {
        false
    };

    if indices.is_empty() {
        // Simple assignment: d[k] = v OR list[i] = x
        if is_numeric_index {
            // For Vec/List: use direct indexing to replace the element
            // Note: Vec::insert() INSERTS a new element, we want to REPLACE
            Ok(quote! { #base_expr[#final_index as usize] = #value_expr; })
        } else if needs_as_object_mut {
            Ok(quote! { #base_expr.as_object_mut().unwrap().insert(#final_index, #value_expr); })
        } else {
            // HashMap.insert(key, value)
            Ok(quote! { #base_expr.insert(#final_index, #value_expr); })
        }
    } else {
        // Nested assignment: build chain of get_mut calls
        let mut chain = quote! { #base_expr };
        for idx in &indices {
            // Check if the intermediate index is numeric or dict key
            // For list types (Vec), indices should be cast to usize without & reference
            // For now, use the outer is_numeric_index as a heuristic - if the final
            // index is numeric, intermediate indices are likely also numeric (nested lists)
            if is_numeric_index {
                chain = quote! {
                    #chain.get_mut(#idx as usize).unwrap()
                };
            } else {
                chain = quote! {
                    #chain.get_mut(&#idx).unwrap()
                };
            }
        }

        if is_numeric_index {
            // For Vec/List: use direct indexing to replace the element
            // Note: Vec::insert() INSERTS a new element, we want to REPLACE
            Ok(quote! { #chain[#final_index as usize] = #value_expr; })
        } else if needs_as_object_mut {
            Ok(quote! { #chain.as_object_mut().unwrap().insert(#final_index, #value_expr); })
        } else {
            // HashMap.insert(key, value)
            Ok(quote! { #chain.insert(#final_index, #value_expr); })
        }
    }
}

pub(crate) fn codegen_assign_slice(
    base: &HirExpr,
    value_expr: syn::Expr,
    _ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Use build_expr_no_clone to avoid adding .clone() to the base
    // This is a mutation operation, so we need direct access to the field
    let base_expr = build_expr_no_clone(base);
    // For full slice assignment x[:] = value, clear and extend
    Ok(quote! {
        #base_expr.clear();
        #base_expr.extend(#value_expr);
    })
}

pub(crate) fn codegen_assign_attribute(
    base: &HirExpr,
    attr: &str,
    value_expr: syn::Expr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // For assignment targets, we need mutable access - don't use .get().cloned() patterns
    // Instead, use direct indexing which gives us a mutable reference

    // Set flag to indicate we're generating an assignment target (LHS)
    // This prevents adding .clone() to the base expression
    let was_assignment_target = ctx.is_assignment_target;
    ctx.is_assignment_target = true;

    let mut base_expr = if let HirExpr::Index {
        base: inner_base,
        index,
    } = base
    {
        // Generate direct index access for mutation: items[0] instead of items.get(0).cloned().unwrap()
        let inner_base_expr = inner_base.to_rust_expr(ctx)?;
        let index_expr = index.to_rust_expr(ctx)?;
        // Check if index is a literal integer
        if let HirExpr::Literal(crate::hir::Literal::Int(n)) = &**index {
            let idx = *n as usize;
            parse_quote! { #inner_base_expr[#idx] }
        } else {
            parse_quote! { #inner_base_expr[#index_expr as usize] }
        }
    } else {
        base.to_rust_expr(ctx)?
    };

    // Restore flag
    ctx.is_assignment_target = was_assignment_target;

    // Handle Optional variable unwrapping for assignment targets.
    // When the base is an Optional<T> variable (e.g., player: Option<Player>),
    // we need to unwrap it to access the inner type's fields.
    // Example: player.sin_bin_status = x → player.as_mut().unwrap().sin_bin_status = x
    if let HirExpr::Var(var_name) = base {
        if let Some(Type::Optional(_)) = ctx.var_types.get(var_name) {
            base_expr = parse_quote! { #base_expr.as_mut().unwrap() };
        }
    }

    // Check if target field is Option<T> and wrap value in Some() if needed
    let final_value_expr = if let HirExpr::Var(var_name) = base {
        // Check if this is a self.field assignment in a class
        if var_name == "self" {
            // Look up the field type in current class
            if let Some(field_type) = ctx.get_self_field_type(attr) {
                if matches!(field_type, Type::Optional(_)) {
                    // Wrap value in Some()
                    parse_quote! { Some(#value_expr) }
                } else {
                    value_expr
                }
            } else {
                value_expr
            }
        } else {
            value_expr
        }
    } else {
        value_expr
    };

    let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
    Ok(quote! { #base_expr.#attr_ident = #final_value_expr; })
}

pub(crate) fn codegen_assign_tuple(
    targets: &[AssignTarget],
    value_expr: syn::Expr,
    _type_annotation_tokens: Option<proc_macro2::TokenStream>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Check if there's a starred expression in the targets
    let has_starred = targets
        .iter()
        .any(|t| matches!(t, AssignTarget::Starred(_)));

    if has_starred {
        // Handle starred unpacking: a, *rest, b = data
        return codegen_starred_unpack(targets, value_expr, ctx);
    }

    // Check if all targets are simple symbols or nested simple tuples
    fn all_simple_symbols_or_tuples(targets: &[AssignTarget]) -> bool {
        targets.iter().all(|t| match t {
            AssignTarget::Symbol(_) => true,
            AssignTarget::Tuple(nested) => all_simple_symbols_or_tuples(nested),
            _ => false,
        })
    }

    // Check if targets contain only symbols (no nested tuples)
    let all_symbols: Option<Vec<&str>> = targets
        .iter()
        .map(|t| match t {
            AssignTarget::Symbol(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();

    match all_symbols {
        Some(symbols) => {
            // Simple case: all targets are symbols (no nested tuples)
            let all_declared = symbols.iter().all(|s| ctx.is_declared(s));

            if all_declared {
                // All variables exist, do reassignment
                let idents: Vec<_> = symbols.iter().map(|s| safe_ident(s)).collect();
                Ok(quote! { (#(#idents),*) = #value_expr; })
            } else {
                // First declaration - mark each variable individually
                symbols.iter().for_each(|s| ctx.declare_var(s));
                let idents_with_mut: Vec<_> = symbols
                    .iter()
                    .map(|s| {
                        let ident = safe_ident(s);
                        if ctx.mutable_vars.contains(*s) {
                            quote! { mut #ident }
                        } else {
                            quote! { #ident }
                        }
                    })
                    .collect();
                Ok(quote! { let (#(#idents_with_mut),*) = #value_expr; })
            }
        }
        None => {
            // Complex case: contains nested tuples, index targets, or attributes
            if all_simple_symbols_or_tuples(targets) {
                // All targets are symbols or nested tuples (no index/attribute access)
                // We can use a clean let pattern without temporaries
                let mut temp_counter = 0;
                let (pattern, _) = build_unpack_pattern(targets, ctx, &mut temp_counter)?;
                Ok(quote! { let (#pattern) = #value_expr; })
            } else {
                // Contains index/attribute targets - need temporaries for safe unpacking
                codegen_complex_tuple_unpack(targets, value_expr, ctx)
            }
        }
    }
}

pub(crate) fn codegen_complex_tuple_unpack(
    targets: &[AssignTarget],
    value_expr: syn::Expr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let mut temp_counter = 0;
    let (pattern, target_temps) = build_unpack_pattern(targets, ctx, &mut temp_counter)?;

    // Generate the let binding with the pattern
    let capture_stmt = quote! { let (#pattern) = #value_expr; };

    // Generate individual assignments from temporaries to complex targets
    let mut assignments = Vec::new();
    for (target, temp_name) in target_temps {
        let assign = match target {
            AssignTarget::Index { base, index } => {
                let base_expr = base.to_rust_expr(ctx)?;
                let index_expr = index.to_rust_expr(ctx)?;
                quote! { #base_expr[#index_expr as usize] = #temp_name; }
            }
            AssignTarget::Attribute { value: base, attr } => {
                let base_expr = base.to_rust_expr(ctx)?;
                let attr_ident = syn::Ident::new(&attr, proc_macro2::Span::call_site());
                quote! { #base_expr.#attr_ident = #temp_name; }
            }
            AssignTarget::Slice { .. } => {
                bail!("Slice target in tuple unpacking not supported")
            }
            _ => {
                // Should not happen since we only add non-symbol/non-tuple targets
                bail!("Unexpected target type in complex unpacking")
            }
        };
        assignments.push(assign);
    }

    if assignments.is_empty() {
        // No complex assignments, just the let binding
        Ok(quote! { #capture_stmt })
    } else {
        Ok(quote! {
            {
                #capture_stmt
                #(#assignments)*
            }
        })
    }
}

pub(crate) fn codegen_starred_unpack(
    targets: &[AssignTarget],
    value_expr: syn::Expr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Find the position of the starred expression
    let star_pos = targets
        .iter()
        .position(|t| matches!(t, AssignTarget::Starred(_)))
        .ok_or_else(|| anyhow::anyhow!("No starred expression found"))?;

    // Count non-starred targets before and after the starred expression
    let before_count = star_pos;
    let after_count = targets.len() - star_pos - 1;

    // Store data in temporary variable for slicing
    let data_var = syn::Ident::new("_data", proc_macro2::Span::call_site());

    let mut stmts = Vec::new();

    // First, store the data
    stmts.push(quote! { let #data_var = #value_expr; });

    // Generate code for assignments before the starred expression
    for (i, target) in targets.iter().take(before_count).enumerate() {
        if let AssignTarget::Symbol(name) = target {
            let ident = safe_ident(name);
            let index = syn::Index::from(i);
            ctx.declare_var(name);
            if ctx.mutable_vars.contains(name.as_str()) {
                stmts.push(quote! { let mut #ident = #data_var[#index]; });
            } else {
                stmts.push(quote! { let #ident = #data_var[#index]; });
            }
        } else {
            bail!("Complex targets in starred unpacking not yet supported");
        }
    }

    // Generate code for the starred expression
    if let AssignTarget::Starred(star_name) = &targets[star_pos] {
        let star_ident = safe_ident(star_name);
        ctx.declare_var(star_name);

        if after_count == 0 {
            // *rest at end: data[before_count..]
            stmts.push(quote! {
                let #star_ident: Vec<_> = #data_var[#before_count..].to_vec();
            });
        } else if before_count == 0 {
            // *rest at beginning: data[..data.len() - after_count]
            stmts.push(quote! {
                let #star_ident: Vec<_> = #data_var[..#data_var.len() - #after_count].to_vec();
            });
        } else {
            // *rest in middle: data[before_count..data.len() - after_count]
            stmts.push(quote! {
                let #star_ident: Vec<_> = #data_var[#before_count..#data_var.len() - #after_count].to_vec();
            });
        }
    }

    // Generate code for assignments after the starred expression
    for (i, target) in targets.iter().skip(star_pos + 1).enumerate() {
        if let AssignTarget::Symbol(name) = target {
            let ident = safe_ident(name);
            ctx.declare_var(name);

            // Index from end: data[data.len() - after_count + i]
            let offset = after_count - i - 1;
            if ctx.mutable_vars.contains(name.as_str()) {
                stmts.push(quote! {
                    let mut #ident = #data_var[#data_var.len() - #offset - 1];
                });
            } else {
                stmts.push(quote! {
                    let #ident = #data_var[#data_var.len() - #offset - 1];
                });
            }
        } else {
            bail!("Complex targets in starred unpacking not yet supported");
        }
    }

    // Generate all statements sequentially (no block needed)
    Ok(quote! { #(#stmts)* })
}

pub(crate) fn build_unpack_pattern(
    targets: &[AssignTarget],
    ctx: &mut CodeGenContext,
    temp_counter: &mut usize,
) -> Result<(proc_macro2::TokenStream, Vec<(AssignTarget, syn::Ident)>)> {
    let mut pattern_parts = Vec::new();
    let mut target_temps = Vec::new();

    for target in targets {
        match target {
            AssignTarget::Symbol(symbol) => {
                let ident = safe_ident(symbol);
                // Mark variable as declared
                ctx.declare_var(symbol);
                // Check if mutable
                if ctx.mutable_vars.contains(symbol.as_str()) {
                    pattern_parts.push(quote! { mut #ident });
                } else {
                    pattern_parts.push(quote! { #ident });
                }
            }
            AssignTarget::Tuple(nested_targets) => {
                // Recursively build pattern for nested tuple
                let (nested_pattern, nested_temps) =
                    build_unpack_pattern(nested_targets, ctx, temp_counter)?;
                pattern_parts.push(quote! { (#nested_pattern) });
                target_temps.extend(nested_temps);
            }
            _ => {
                // For complex targets (index, attribute, slice), use temp variable
                let temp_name = syn::Ident::new(
                    &format!("_unpack_tmp{}", temp_counter),
                    proc_macro2::Span::call_site(),
                );
                *temp_counter += 1;
                pattern_parts.push(quote! { #temp_name });
                target_temps.push((target.clone(), temp_name));
            }
        }
    }

    Ok((quote! { #(#pattern_parts),* }, target_temps))
}

pub(crate) fn is_dict_augassign_pattern(target: &AssignTarget, value: &HirExpr, ctx: &CodeGenContext) -> bool {
    if let AssignTarget::Index {
        base: target_base,
        index: target_index,
    } = target
    {
        // First check if this is actually a Dict, not a List
        // For lists/vectors, we want to use direct augmented assignment (arr[i] += x)
        // For dicts/hashmaps, we need the get/insert pattern to avoid borrow checker issues
        let is_dict = match target_base.as_ref() {
            HirExpr::Var(var_name) => {
                // Check type information
                matches!(ctx.var_types.get(var_name), Some(Type::Dict(_, _)))
            }
            HirExpr::Attribute { value, attr } => {
                // For self.field, check the field type in current class
                if let HirExpr::Var(base_var) = value.as_ref() {
                    if base_var == "self" {
                        // Check if this field is a Dict
                        matches!(ctx.get_self_field_type(attr), Some(Type::Dict(_, _)))
                    } else {
                        // Can't determine, assume not dict to prefer augmented assignment
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };

        // Only proceed if this is actually a dict
        if !is_dict {
            return false;
        }

        if let HirExpr::Binary { left, .. } = value {
            if let HirExpr::Index {
                base: value_base,
                index: value_index,
            } = left.as_ref()
            {
                // Check if both indices refer to the same dict[key] location
                // Compare base and index expressions
                let bases_match = exprs_are_equivalent(target_base.as_ref(), value_base.as_ref());
                let indices_match =
                    exprs_are_equivalent(target_index.as_ref(), value_index.as_ref());
                return bases_match && indices_match;
            }
        }
    }
    false
}

pub(crate) fn is_optional_attr_augassign_pattern(
    target: &AssignTarget,
    value: &HirExpr,
    ctx: &CodeGenContext,
) -> bool {
    if let AssignTarget::Attribute {
        value: target_base,
        attr: target_attr,
    } = target
    {
        if let HirExpr::Binary { left, .. } = value {
            if let HirExpr::Attribute {
                value: left_base,
                attr: left_attr,
            } = left.as_ref()
            {
                // Check if target and left refer to the same attribute
                if target_attr == left_attr {
                    // Check if the bases refer to the same variable
                    if let (HirExpr::Var(t_var), HirExpr::Var(l_var)) =
                        (target_base.as_ref(), left_base.as_ref())
                    {
                        if t_var == l_var {
                            // Now check if this attribute is Optional
                            return expr_is_optional(left.as_ref(), ctx);
                        }
                    }
                }
            }
        }
    }
    false
}

pub(crate) fn is_optional_var_augassign_pattern(
    target: &AssignTarget,
    value: &HirExpr,
    ctx: &CodeGenContext,
) -> bool {
    if let AssignTarget::Symbol(target_var) = target {
        if let HirExpr::Binary { left, .. } = value {
            if let HirExpr::Var(left_var) = left.as_ref() {
                // Check if target and left refer to the same variable
                if target_var == left_var {
                    // Check if this variable is Optional
                    return matches!(ctx.var_types.get(target_var), Some(Type::Optional(_)));
                }
            }
        }
    }
    false
}

/// Detect if an augmented assignment involves list concatenation.
/// Vec doesn't implement AddAssign, so we need .extend() instead of +=.
fn is_list_concat_augassign(
    target: &AssignTarget,
    right: &HirExpr,
    ctx: &CodeGenContext,
) -> bool {
    if matches!(right, HirExpr::List(_)) {
        return true;
    }
    if let HirExpr::Var(name) = right {
        if matches!(ctx.var_types.get(name), Some(Type::List(_))) {
            return true;
        }
    }
    match target {
        AssignTarget::Symbol(var_name) => {
            matches!(ctx.var_types.get(var_name), Some(Type::List(_)))
        }
        AssignTarget::Attribute { value, attr } => {
            if let HirExpr::Var(base_var) = value.as_ref() {
                if base_var == "self" {
                    matches!(ctx.get_self_field_type(attr), Some(Type::List(_)))
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

pub(crate) fn is_augassign_pattern(target: &AssignTarget, value: &HirExpr) -> Option<BinOp> {
    if let HirExpr::Binary { op, left, .. } = value {
        // Check if the target and left side of the binary operation refer to the same location
        match (target, left.as_ref()) {
            // Simple variable: x += 1  becomes  x = x + 1
            (AssignTarget::Symbol(target_var), HirExpr::Var(left_var))
                if target_var == left_var =>
            {
                Some(*op)
            }
            // Attribute: obj.field += 1  becomes  obj.field = obj.field + 1
            (
                AssignTarget::Attribute {
                    value: target_base,
                    attr: target_attr,
                },
                HirExpr::Attribute {
                    value: left_base,
                    attr: left_attr,
                },
            ) if target_attr == left_attr => {
                // Check if bases are the same (simple heuristic: both are Var with same name)
                match (target_base.as_ref(), left_base.as_ref()) {
                    (HirExpr::Var(t_var), HirExpr::Var(l_var)) if t_var == l_var => Some(*op),
                    _ => None,
                }
            }
            // Index: arr[i] += 1  becomes  arr[i] = arr[i] + 1
            (
                AssignTarget::Index {
                    base: target_base,
                    index: target_index,
                },
                HirExpr::Index {
                    base: left_base,
                    index: left_index,
                },
            ) => {
                // Check if both base and index are the same
                // Support multiple patterns:
                // 1. Simple variables: arr[i] where both arr and i are vars
                // 2. Attributes: self.values[index] where base is attribute access
                // 3. Literal indices: arr[0] where index is a literal

                let bases_match = match (target_base.as_ref(), left_base.as_ref()) {
                    // Both simple vars with same name
                    (HirExpr::Var(t_base), HirExpr::Var(l_base)) => t_base == l_base,
                    // Both attribute access with same var and attr
                    (
                        HirExpr::Attribute {
                            value: t_val,
                            attr: t_attr,
                        },
                        HirExpr::Attribute {
                            value: l_val,
                            attr: l_attr,
                        },
                    ) if t_attr == l_attr => {
                        matches!((t_val.as_ref(), l_val.as_ref()),
                            (HirExpr::Var(t_var), HirExpr::Var(l_var)) if t_var == l_var)
                    }
                    _ => false,
                };

                let indices_match = match (target_index.as_ref(), left_index.as_ref()) {
                    // Both vars with same name
                    (HirExpr::Var(t_idx), HirExpr::Var(l_idx)) => t_idx == l_idx,
                    // Both literal ints with same value
                    (
                        HirExpr::Literal(crate::hir::Literal::Int(t_val)),
                        HirExpr::Literal(crate::hir::Literal::Int(l_val)),
                    ) => t_val == l_val,
                    _ => false,
                };

                if bases_match && indices_match {
                    Some(*op)
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    }
}

