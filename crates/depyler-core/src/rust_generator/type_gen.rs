//! Type conversion utilities
//!
//! This module provides conversion functions from internal RustType representation
//! to syn::Type tokens used for code generation.

use crate::hir::BinOp;
use crate::rust_generator::CodeGenContext;
use anyhow::{Result, bail};
use syn::{self, parse_quote};

/// Convert logical and bitwise binary operators to syn::BinOp
///
/// # Complexity
/// 8 (match with 7 arms)
#[inline]
fn convert_logical_bitwise_binop(op: BinOp) -> Option<Result<syn::BinOp>> {
    use BinOp::*;

    Some(Ok(match op {
        // Logical operators
        And => parse_quote! { && },
        Or => parse_quote! { || },

        // Bitwise operators
        BitAnd => parse_quote! { & },
        BitOr => parse_quote! { | },
        BitXor => parse_quote! { ^ },
        LShift => parse_quote! { << },
        RShift => parse_quote! { >> },

        _ => return None,
    }))
}

/// Convert binary operator to syn::BinOp
///
/// Maps Python binary operators to their Rust equivalents.
/// Special operators like FloorDiv and Pow are handled separately
/// in convert_binary with Python semantics.
///
/// # Arguments
/// * `op` - The HIR binary operator
///
/// # Returns
/// The corresponding syn::BinOp token
///
/// # Errors
/// Returns error for special operators (FloorDiv, Pow, In, NotIn) that
/// require custom handling in convert_binary
///
/// # Complexity
/// 10 (main match + logical/bitwise helper)
pub fn convert_binop(op: BinOp) -> Result<syn::BinOp> {
    use BinOp::*;

    // Try logical/bitwise operators first
    if let Some(result) = convert_logical_bitwise_binop(op) {
        return result;
    }

    match op {
        // Arithmetic operators
        Add => Ok(parse_quote! { + }),
        Sub => Ok(parse_quote! { - }),
        Mul => Ok(parse_quote! { * }),
        Div => Ok(parse_quote! { / }),
        Mod => Ok(parse_quote! { % }),

        // Special arithmetic cases handled by convert_binary
        FloorDiv => {
            bail!("Floor division handled by convert_binary with Python semantics")
        }
        Pow => bail!("Power operator handled by convert_binary with type-specific logic"),
        MatMul => bail!("Matrix multiplication handled by convert_binary as matmul() call"),

        // Comparison operators (include identity checks)
        Eq => Ok(parse_quote! { == }),
        NotEq => Ok(parse_quote! { != }),
        Is => Ok(parse_quote! { == }),
        IsNot => Ok(parse_quote! { != }),
        Lt => Ok(parse_quote! { < }),
        LtEq => Ok(parse_quote! { <= }),
        Gt => Ok(parse_quote! { > }),
        GtEq => Ok(parse_quote! { >= }),

        // Special membership operators handled in convert_binary
        In | NotIn => bail!("in/not in operators should be handled by convert_binary"),

        // Logical/bitwise handled above
        And | Or | BitAnd | BitOr | BitXor | LShift | RShift => {
            unreachable!("Logical/bitwise operators handled by convert_logical_bitwise_binop")
        }
    }
}

/// Convert Str type with optional lifetime to syn::Type
///
/// Handles both `&str` and `&'a str` variants.
///
/// # Complexity
/// 2 (single if/else branch)
fn str_type_to_syn(lifetime: &Option<String>) -> syn::Type {
    if let Some(lt) = lifetime {
        let lt_ident = syn::Lifetime::new(lt, proc_macro2::Span::call_site());
        parse_quote! { &#lt_ident str }
    } else {
        parse_quote! { &str }
    }
}

/// Convert Reference type with mutable and lifetime to syn::Type
///
/// Handles all 4 combinations of mutable × lifetime:
/// - `&T`, `&mut T`, `&'a T`, `&'a mut T`
///
/// # Complexity
/// 5 (nested if/else for mutable and lifetime)
fn reference_type_to_syn(
    lifetime: &Option<String>,
    mutable: bool,
    inner: &crate::types::type_mapper::RustType,
) -> Result<syn::Type> {
    let inner_ty = rust_type_to_syn(inner)?;

    Ok(if mutable {
        if let Some(lt) = lifetime {
            let lt_ident = syn::Lifetime::new(lt, proc_macro2::Span::call_site());
            parse_quote! { &#lt_ident mut #inner_ty }
        } else {
            parse_quote! { &mut #inner_ty }
        }
    } else if let Some(lt) = lifetime {
        let lt_ident = syn::Lifetime::new(lt, proc_macro2::Span::call_site());
        parse_quote! { &#lt_ident #inner_ty }
    } else {
        parse_quote! { &#inner_ty }
    })
}

/// Convert Array type with const generic size to syn::Type
///
/// Handles 3 const generic size variants:
/// - Literal: `[T; 10]`
/// - Parameter: `[T; N]`
/// - Expression: `[T; SIZE * 2]`
///
/// # Complexity
/// 4 (match with 3 arms)
fn array_type_to_syn(
    element_type: &crate::types::type_mapper::RustType,
    size: &crate::types::type_mapper::RustConstGeneric,
) -> Result<syn::Type> {
    let element = rust_type_to_syn(element_type)?;

    Ok(match size {
        crate::types::type_mapper::RustConstGeneric::Literal(n) => {
            let size_lit = syn::LitInt::new(&n.to_string(), proc_macro2::Span::call_site());
            parse_quote! { [#element; #size_lit] }
        }
        crate::types::type_mapper::RustConstGeneric::Parameter(name) => {
            let param_ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            parse_quote! { [#element; #param_ident] }
        }
        crate::types::type_mapper::RustConstGeneric::Expression(expr) => {
            let expr_tokens: proc_macro2::TokenStream = expr.parse().unwrap_or_else(|_| {
                quote::quote! { /* invalid const expression */ }
            });
            parse_quote! { [#element; #expr_tokens] }
        }
    })
}

/// Convert collection types (Vec, HashMap, HashSet, Option, Result, Tuple)
///
/// # Complexity
/// 7 (match with 6 arms + recursive call)
#[inline]
fn collection_type_to_syn(
    rust_type: &crate::types::type_mapper::RustType,
) -> Result<Option<syn::Type>> {
    use crate::types::type_mapper::RustType;

    Ok(Some(match rust_type {
        RustType::Vec(inner) => {
            let inner_ty = rust_type_to_syn(inner)?;
            parse_quote! { Vec<#inner_ty> }
        }
        RustType::HashMap(k, v) => {
            let key_ty = rust_type_to_syn(k)?;
            let val_ty = rust_type_to_syn(v)?;
            parse_quote! { HashMap<#key_ty, #val_ty> }
        }
        RustType::HashSet(inner) => {
            let inner_ty = rust_type_to_syn(inner)?;
            parse_quote! { HashSet<#inner_ty> }
        }
        RustType::Option(inner) => {
            let inner_ty = rust_type_to_syn(inner)?;
            parse_quote! { Option<#inner_ty> }
        }
        RustType::Result(ok, err) => {
            let ok_ty = rust_type_to_syn(ok)?;
            let err_ty = rust_type_to_syn(err)?;
            parse_quote! { Result<#ok_ty, #err_ty> }
        }
        RustType::Tuple(types) => {
            let tys: Vec<_> = types
                .iter()
                .map(rust_type_to_syn)
                .collect::<Result<Vec<_>>>()?;
            parse_quote! { (#(#tys),*) }
        }
        _ => return Ok(None),
    }))
}

/// Convert RustType to syn::Type
///
/// Main type conversion function that recursively converts internal RustType
/// representation to syn::Type tokens for code generation.
///
/// # Arguments
/// * `rust_type` - The internal RustType to convert
///
/// # Returns
/// The corresponding syn::Type token
///
/// # Errors
/// Returns error for unsupported types or invalid custom type strings
///
/// # Complexity
/// 10 (main match + collection helper)
pub fn rust_type_to_syn(rust_type: &crate::types::type_mapper::RustType) -> Result<syn::Type> {
    use crate::types::type_mapper::RustType;

    // Try collection types first
    if let Some(ty) = collection_type_to_syn(rust_type)? {
        return Ok(ty);
    }

    Ok(match rust_type {
        RustType::Primitive(p) => {
            let ident = syn::Ident::new(p.to_rust_string(), proc_macro2::Span::call_site());
            parse_quote! { #ident }
        }
        RustType::String => parse_quote! { String },
        RustType::Str { lifetime } => str_type_to_syn(lifetime),
        RustType::Cow { lifetime } => {
            let lt_ident = syn::Lifetime::new(lifetime, proc_macro2::Span::call_site());
            parse_quote! { Cow<#lt_ident, str> }
        }
        RustType::Reference {
            lifetime,
            mutable,
            inner,
        } => reference_type_to_syn(lifetime, *mutable, inner)?,
        RustType::Unit => parse_quote! { () },
        RustType::Custom(name) => {
            let ty: syn::Type = syn::parse_str(name)?;
            ty
        }
        RustType::Unsupported(reason) => bail!("Unsupported Rust type: {}", reason),
        RustType::TypeParam(name) => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            parse_quote! { #ident }
        }
        RustType::Generic { base, params } => {
            let base_ident = syn::Ident::new(base, proc_macro2::Span::call_site());
            let param_types: Vec<_> = params
                .iter()
                .map(rust_type_to_syn)
                .collect::<Result<Vec<_>>>()?;
            parse_quote! { #base_ident<#(#param_types),*> }
        }
        RustType::Enum { name, .. } => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            parse_quote! { #ident }
        }
        RustType::Array { element_type, size } => array_type_to_syn(element_type, size)?,
        // Collection types handled above
        RustType::Vec(_)
        | RustType::HashMap(_, _)
        | RustType::HashSet(_)
        | RustType::Option(_)
        | RustType::Result(_, _)
        | RustType::Tuple(_) => {
            unreachable!("Collection types handled by collection_type_to_syn")
        }
    })
}

/// Updates import needs for custom type names
///
/// Detects specific custom types (FnvHashMap, AHashMap, Arc, Rc, HashMap)
/// and sets appropriate import flags.
///
/// # Complexity
/// 6 (if-else chain for 5 patterns)
#[inline]
fn update_custom_type_imports(ctx: &mut CodeGenContext, name: &str) {
    if name.contains("FnvHashMap") {
        ctx.require(crate::rust_generator::context::Import::FnvHashMap);
    } else if name.contains("AHashMap") {
        ctx.require(crate::rust_generator::context::Import::AHashMap);
    } else if name.contains("Arc<") {
        ctx.require(crate::rust_generator::context::Import::Arc);
    } else if name.contains("Rc<") {
        ctx.require(crate::rust_generator::context::Import::Rc);
    } else if name.contains("HashMap<")
        && !name.contains("FnvHashMap")
        && !name.contains("AHashMap")
    {
        ctx.require(crate::rust_generator::context::Import::HashMap);
    }
}

// ============================================================================
// Copy-type analysis (moved from rust_generator.rs §31)
// ============================================================================

/// Check if a field type is Copy (for determining struct Copy derivation).
///
/// Delegates to `Type::is_copy()` for intrinsic Copy types, then checks the
/// caller-supplied sets for user-defined enums and all-Copy-field structs.
pub(crate) fn is_field_copy_type(
    ty: &crate::hir::Type,
    enum_names: &std::collections::HashSet<String>,
    copy_structs: &std::collections::HashSet<String>,
) -> bool {
    match ty {
        _ if ty.is_copy() => true,
        crate::hir::Type::Custom(name) => enum_names.contains(name) || copy_structs.contains(name),
        _ => false,
    }
}

/// Check if a `RustType` is Copy (primitives + unit + enums + Copy tuples).
///
/// Copy types are cheap to pass by value — they should not be borrowed.
pub(crate) fn is_copy_rust_type(
    rust_type: &crate::types::type_mapper::RustType,
    enum_names: &std::collections::HashSet<String>,
    copy_structs: &std::collections::HashSet<String>,
) -> bool {
    use crate::types::type_mapper::RustType;
    match rust_type {
        RustType::Primitive(_) | RustType::Unit => true,
        RustType::Tuple(types) => types
            .iter()
            .all(|t| is_copy_rust_type(t, enum_names, copy_structs)),
        RustType::Custom(name) => enum_names.contains(name),
        _ => false,
    }
}

// ============================================================================
// Constant type-inference helpers (moved from rust_generator.rs §31)
// Used by `generate_constant_tokens` in codegen.rs.
// ============================================================================

/// Infer the Rust type token for a single HIR expression (used for constant codegen).
pub(crate) fn infer_single_expr_type(expr: &crate::hir::HirExpr) -> proc_macro2::TokenStream {
    use crate::hir::{HirExpr, Literal};
    use quote::quote;
    match expr {
        HirExpr::Literal(Literal::Int(_)) => quote! { i32 },
        HirExpr::Literal(Literal::Float(_)) => quote! { f64 },
        HirExpr::Literal(Literal::String(_)) => quote! { String },
        HirExpr::Literal(Literal::Bool(_)) => quote! { bool },
        HirExpr::Unary { op, operand } => infer_unary_type(op, operand),
        HirExpr::List(inner) => {
            let inner_type = infer_list_element_type(inner);
            quote! { Vec<#inner_type> }
        }
        HirExpr::Tuple(elems) => infer_tuple_type(elems),
        _ => quote! { serde_json::Value },
    }
}

/// Infer the Rust element type for a list constant from its elements.
pub(crate) fn infer_list_element_type(elts: &[crate::hir::HirExpr]) -> proc_macro2::TokenStream {
    use quote::quote;
    let first = match elts.first() {
        Some(expr) => infer_single_expr_type(expr),
        None => return quote! { serde_json::Value },
    };
    let first_str = first.to_string();
    for expr in &elts[1..] {
        if infer_single_expr_type(expr).to_string() != first_str {
            return quote! { serde_json::Value };
        }
    }
    first
}

/// Infer the Rust key/value types for a dict constant.
pub(crate) fn infer_dict_kv_types(
    pairs: &[(crate::hir::HirExpr, crate::hir::HirExpr)],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    use quote::quote;
    let fallback = || (quote! { serde_json::Value }, quote! { serde_json::Value });
    let (first_k, first_v) = match pairs.first() {
        Some((k, v)) => (infer_single_expr_type(k), infer_single_expr_type(v)),
        None => return fallback(),
    };
    let k_str = first_k.to_string();
    let v_str = first_v.to_string();
    for (k, v) in &pairs[1..] {
        if infer_single_expr_type(k).to_string() != k_str
            || infer_single_expr_type(v).to_string() != v_str
        {
            return fallback();
        }
    }
    (first_k, first_v)
}

/// Infer the Rust element type for a set constant.
pub(crate) fn infer_set_element_type(elts: &[crate::hir::HirExpr]) -> proc_macro2::TokenStream {
    infer_list_element_type(elts)
}

/// Infer a Rust tuple type token from its elements.
pub(crate) fn infer_tuple_type(elems: &[crate::hir::HirExpr]) -> proc_macro2::TokenStream {
    use quote::quote;
    let types: Vec<proc_macro2::TokenStream> = elems.iter().map(infer_single_expr_type).collect();
    quote! { (#(#types),*) }
}

/// Infer the Rust type for a unary expression.
pub(crate) fn infer_unary_type(
    op: &crate::hir::UnaryOp,
    operand: &crate::hir::HirExpr,
) -> proc_macro2::TokenStream {
    use crate::hir::{HirExpr, Literal, UnaryOp};
    use quote::quote;
    match (op, operand) {
        (UnaryOp::Neg | UnaryOp::Pos, HirExpr::Literal(Literal::Int(_))) => quote! { i32 },
        (UnaryOp::Neg | UnaryOp::Pos, HirExpr::Literal(Literal::Float(_))) => quote! { f64 },
        (UnaryOp::Not, HirExpr::Literal(Literal::Bool(_))) => quote! { bool },
        _ => quote! { serde_json::Value },
    }
}

/// Check if a tuple element needs heap allocation.
pub(crate) fn tuple_element_needs_heap(expr: &crate::hir::HirExpr) -> bool {
    use crate::hir::{HirExpr, Literal};
    matches!(
        expr,
        HirExpr::Literal(Literal::String(_)) | HirExpr::List(_) | HirExpr::Dict(_)
    )
}

/// Check if a list element is const-safe (can live in a static array).
pub(crate) fn is_const_safe_list_element(expr: &crate::hir::HirExpr) -> bool {
    use crate::hir::{HirExpr, Literal};
    match expr {
        HirExpr::Literal(Literal::Int(_) | Literal::Float(_) | Literal::Bool(_)) => true,
        HirExpr::Unary { operand, .. } => is_const_safe_list_element(operand),
        HirExpr::Tuple(elems) => elems.iter().all(is_const_safe_list_element),
        _ => false,
    }
}

/// Check if a `RustType` requires heap allocation (cannot be `const`).
pub(crate) fn is_heap_allocated_rust_type(ty: &crate::types::type_mapper::RustType) -> bool {
    use crate::types::type_mapper::RustType;
    matches!(
        ty,
        RustType::String
            | RustType::Vec(_)
            | RustType::HashMap(_, _)
            | RustType::HashSet(_)
            | RustType::Custom(_)
    )
}

/// Infer the HIR `Type` for a module-level constant from its value expression.
pub(crate) fn infer_constant_hir_type(expr: &crate::hir::HirExpr) -> crate::hir::Type {
    use crate::hir::{HirExpr, Literal, Type};
    match expr {
        HirExpr::Literal(Literal::Int(_)) => Type::Int,
        HirExpr::Literal(Literal::Float(_)) => Type::Float,
        HirExpr::Literal(Literal::String(_)) => Type::String,
        HirExpr::Literal(Literal::Bool(_)) => Type::Bool,
        HirExpr::Unary { operand, .. } => infer_constant_hir_type(operand),
        HirExpr::Tuple(elems) => {
            let elem_types: Vec<Type> = elems.iter().map(infer_constant_hir_type).collect();
            if elem_types.iter().any(|t| matches!(t, Type::Unknown)) {
                Type::Unknown
            } else {
                Type::Tuple(elem_types)
            }
        }
        HirExpr::List(elems) => {
            if elems.is_empty() {
                return Type::Unknown;
            }
            let elem_type = infer_constant_hir_type(&elems[0]);
            if matches!(elem_type, Type::Unknown) {
                Type::Unknown
            } else {
                Type::List(Box::new(elem_type))
            }
        }
        _ => Type::Unknown,
    }
}

// ============================================================================

/// Updates the import needs based on the rust type being used
///
/// Recursively walks through type structure to determine which imports
/// are needed (HashMap, Cow, Arc, Rc, etc.)
///
/// # Arguments
/// * `ctx` - Code generation context to update import flags
/// * `rust_type` - The type to analyze for import needs
///
/// # Complexity
/// 9 (match with 8 arms + helper function)
pub fn update_import_needs(
    ctx: &mut CodeGenContext,
    rust_type: &crate::types::type_mapper::RustType,
) {
    match rust_type {
        crate::types::type_mapper::RustType::HashMap(_, _) => ctx.require(crate::rust_generator::context::Import::HashMap),
        crate::types::type_mapper::RustType::HashSet(inner) => {
            ctx.require(crate::rust_generator::context::Import::HashSet);
            update_import_needs(ctx, inner);
        }
        crate::types::type_mapper::RustType::Cow { .. } => ctx.require(crate::rust_generator::context::Import::Cow),
        crate::types::type_mapper::RustType::Custom(name) => update_custom_type_imports(ctx, name),
        crate::types::type_mapper::RustType::Reference { inner, .. } => {
            update_import_needs(ctx, inner);
        }
        crate::types::type_mapper::RustType::Vec(inner) => {
            update_import_needs(ctx, inner);
        }
        crate::types::type_mapper::RustType::Option(inner) => {
            update_import_needs(ctx, inner);
        }
        crate::types::type_mapper::RustType::Result(ok, err) => {
            update_import_needs(ctx, ok);
            update_import_needs(ctx, err);
        }
        crate::types::type_mapper::RustType::Tuple(types) => {
            for t in types {
                update_import_needs(ctx, t);
            }
        }
        _ => {}
    }
}
