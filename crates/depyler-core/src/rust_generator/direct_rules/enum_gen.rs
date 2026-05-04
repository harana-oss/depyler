use crate::hir::*;
use anyhow::Result;
use quote;
use syn::parse_quote;

pub fn convert_class_to_enum(class: &HirClass) -> Result<Vec<syn::Item>> {
    let enum_name = syn::Ident::new(&class.name, proc_macro2::Span::call_site());

    let variant_info: Vec<(syn::Ident, i64)> = class
        .fields
        .iter()
        .filter(|f| f.is_class_var && f.default_value.is_some())
        .filter_map(|field| {
            // Preserve original casing from Python
            let variant_ident = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
            if let Some(HirExpr::Literal(Literal::Int(value))) = &field.default_value {
                Some((variant_ident, *value))
            } else {
                None
            }
        })
        .collect();

    let variants: Vec<syn::Variant> = variant_info
        .iter()
        .enumerate()
        .map(|(idx, (ident, value))| {
            let lit = syn::LitInt::new(&value.to_string(), proc_macro2::Span::call_site());
            let attrs = if idx == 0 {
                vec![parse_quote! { #[default] }]
            } else {
                vec![]
            };
            syn::Variant {
                attrs,
                ident: ident.clone(),
                fields: syn::Fields::Unit,
                discriminant: Some((
                    syn::Token![=](proc_macro2::Span::call_site()),
                    syn::Expr::Lit(syn::ExprLit {
                        attrs: vec![],
                        lit: syn::Lit::Int(lit),
                    }),
                )),
            }
        })
        .collect();

    let enum_item = syn::Item::Enum(syn::ItemEnum {
        attrs: vec![
            parse_quote! { #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)] },
        ],
        vis: syn::Visibility::Public(syn::Token![pub](proc_macro2::Span::call_site())),
        enum_token: syn::Token![enum](proc_macro2::Span::call_site()),
        ident: enum_name.clone(),
        generics: syn::Generics::default(),
        brace_token: syn::token::Brace::default(),
        variants: variants.into_iter().collect(),
    });

    // Generate match arms for from_i32
    let match_arms: Vec<syn::Arm> = variant_info
        .iter()
        .map(|(ident, value)| {
            let lit = syn::LitInt::new(&value.to_string(), proc_macro2::Span::call_site());
            parse_quote! { #lit => Some(Self::#ident), }
        })
        .collect();

    let first_variant = variant_info.first().map(|(ident, _)| ident.clone());
    let default_variant =
        first_variant.unwrap_or_else(|| syn::Ident::new("Unknown", proc_macro2::Span::call_site()));

    let impl_item: syn::Item = parse_quote! {
        impl #enum_name {
            pub fn from_i32(value: i32) -> Option<Self> {
                match value {
                    #(#match_arms)*
                    _ => None,
                }
            }

            pub fn from_i32_or_default(value: i32) -> Self {
                Self::from_i32(value).unwrap_or(Self::#default_variant)
            }
        }
    };

    Ok(vec![enum_item, impl_item])
}

/// Convert a Python IntFlag class to a unit struct with i32 associated constants.
pub fn convert_class_to_intflag(class: &HirClass) -> Result<Vec<syn::Item>> {
    let struct_name = syn::Ident::new(&class.name, proc_macro2::Span::call_site());

    // Collect constant info: (name, value)
    let const_info: Vec<(syn::Ident, i64)> = class
        .fields
        .iter()
        .filter(|f| f.is_class_var && f.default_value.is_some())
        .filter_map(|field| {
            let const_ident = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
            // Handle both literal ints and binary shift expressions
            if let Some(value) = evaluate_intflag_value(&field.default_value) {
                Some((const_ident, value))
            } else {
                None
            }
        })
        .collect();

    // Generate associated constants as i32
    let const_decls: Vec<proc_macro2::TokenStream> = const_info
        .iter()
        .map(|(ident, value)| {
            let lit = syn::LitInt::new(&value.to_string(), proc_macro2::Span::call_site());
            quote::quote! {
                pub const #ident: i32 = #lit;
            }
        })
        .collect();

    // Create a unit struct to hold the constants as associated items
    let struct_item: syn::Item = parse_quote! {
        pub struct #struct_name;
    };

    let impl_consts: syn::Item = parse_quote! {
        impl #struct_name {
            #(#const_decls)*
        }
    };

    Ok(vec![struct_item, impl_consts])
}

/// Evaluate an IntFlag constant value, handling both literals and shift expressions.
fn evaluate_intflag_value(expr: &Option<HirExpr>) -> Option<i64> {
    match expr {
        Some(HirExpr::Literal(Literal::Int(value))) => Some(*value),
        Some(HirExpr::Binary { op, left, right }) => {
            if matches!(op, BinOp::LShift) {
                let left_val = evaluate_intflag_value(&Some(*left.clone()))?;
                let right_val = evaluate_intflag_value(&Some(*right.clone()))?;
                Some(left_val << right_val)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.flat_map(|c| c.to_lowercase()))
                    .collect(),
            }
        })
        .collect()
}
