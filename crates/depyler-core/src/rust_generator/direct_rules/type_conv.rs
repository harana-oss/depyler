use crate::types::type_mapper::RustType;
use anyhow::Result;
use syn::parse_quote;

fn convert_simple_type(rust_type: &RustType) -> Result<syn::Type> {
    use RustType::*;
    Ok(match rust_type {
        Unit => parse_quote! { () },
        String => parse_quote! { String },
        Custom(name) => {
            if name == "&Self" {
                parse_quote! { &Self }
            } else if name.contains("::") {
                let path: syn::Path = syn::parse_str(name)
                    .unwrap_or_else(|_| panic!("Failed to parse type path: {}", name));
                parse_quote! { #path }
            } else {
                let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                parse_quote! { #ident }
            }
        }
        TypeParam(name) => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            parse_quote! { #ident }
        }
        Enum { name, .. } => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            parse_quote! { #ident }
        }
        _ => unreachable!("convert_simple_type called with non-simple type"),
    })
}

#[inline]
fn convert_primitive_type(prim_type: &crate::types::type_mapper::PrimitiveType) -> Result<syn::Type> {
    use crate::types::type_mapper::PrimitiveType;
    Ok(match prim_type {
        PrimitiveType::Bool => parse_quote! { bool },
        PrimitiveType::I8 => parse_quote! { i8 },
        PrimitiveType::I16 => parse_quote! { i16 },
        PrimitiveType::I32 => parse_quote! { i32 },
        PrimitiveType::I64 => parse_quote! { i64 },
        PrimitiveType::I128 => parse_quote! { i128 },
        PrimitiveType::ISize => parse_quote! { isize },
        PrimitiveType::U8 => parse_quote! { u8 },
        PrimitiveType::U16 => parse_quote! { u16 },
        PrimitiveType::U32 => parse_quote! { u32 },
        PrimitiveType::U64 => parse_quote! { u64 },
        PrimitiveType::U128 => parse_quote! { u128 },
        PrimitiveType::USize => parse_quote! { usize },
        PrimitiveType::F32 => parse_quote! { f32 },
        PrimitiveType::F64 => parse_quote! { f64 },
    })
}

#[inline]
fn convert_lifetime_type(rust_type: &RustType) -> Result<syn::Type> {
    use RustType::*;
    Ok(match rust_type {
        Str { lifetime } => {
            if let Some(lt) = lifetime {
                let lifetime_token =
                    syn::Lifetime::new(&format!("'{}", lt), proc_macro2::Span::call_site());
                parse_quote! { &#lifetime_token str }
            } else {
                parse_quote! { &str }
            }
        }
        Cow { lifetime } => {
            let lifetime_token =
                syn::Lifetime::new(&format!("'{}", lifetime), proc_macro2::Span::call_site());
            parse_quote! { std::borrow::Cow<#lifetime_token, str> }
        }
        _ => unreachable!("convert_lifetime_type called with non-lifetime type"),
    })
}

#[inline]
fn convert_unsupported_type(name: &str) -> Result<syn::Type> {
    let ident = syn::Ident::new(
        &format!("UnsupportedType_{}", name.replace(" ", "_")),
        proc_macro2::Span::call_site(),
    );
    Ok(parse_quote! { #ident })
}

#[inline]
fn convert_container_type(rust_type: &RustType) -> Result<syn::Type> {
    use RustType::*;
    Ok(match rust_type {
        Vec(inner) => {
            let inner_type = rust_type_to_syn_type(inner)?;
            parse_quote! { Vec<#inner_type> }
        }
        HashMap(key, value) => {
            let key_type = rust_type_to_syn_type(key)?;
            let value_type = rust_type_to_syn_type(value)?;
            parse_quote! { HashMap<#key_type, #value_type> }
        }
        Option(inner) => {
            let inner_type = rust_type_to_syn_type(inner)?;
            parse_quote! { Option<#inner_type> }
        }
        Result(ok, err) => {
            let ok_type = rust_type_to_syn_type(ok)?;
            let err_type = rust_type_to_syn_type(err)?;
            parse_quote! { Result<#ok_type, #err_type> }
        }
        HashSet(inner) => {
            let inner_type = rust_type_to_syn_type(inner)?;
            parse_quote! { HashSet<#inner_type> }
        }
        _ => unreachable!("convert_container_type called with non-container type"),
    })
}

#[inline]
fn convert_complex_type(rust_type: &RustType) -> Result<syn::Type> {
    use RustType::*;
    Ok(match rust_type {
        Tuple(types) => {
            let type_tokens: anyhow::Result<std::vec::Vec<_>> =
                types.iter().map(rust_type_to_syn_type).collect();
            let type_tokens = type_tokens?;
            parse_quote! { (#(#type_tokens),*) }
        }
        Generic { base, params } => {
            let base_ident = syn::Ident::new(base, proc_macro2::Span::call_site());
            let param_types: anyhow::Result<std::vec::Vec<_>> =
                params.iter().map(rust_type_to_syn_type).collect();
            let param_types = param_types?;
            parse_quote! { #base_ident<#(#param_types),*> }
        }
        Reference { inner, mutable, .. } => {
            let inner_type = rust_type_to_syn_type(inner)?;
            if *mutable {
                parse_quote! { &mut #inner_type }
            } else {
                parse_quote! { &#inner_type }
            }
        }
        _ => unreachable!("convert_complex_type called with non-complex type"),
    })
}

#[inline]
fn convert_array_type(rust_type: &RustType) -> Result<syn::Type> {
    use RustType::*;
    if let Array { element_type, size } = rust_type {
        let element = rust_type_to_syn_type(element_type)?;
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
                let expr_tokens: proc_macro2::TokenStream = expr
                    .parse()
                    .unwrap_or_else(|_| "/* invalid const expression */".parse().unwrap());
                parse_quote! { [#element; #expr_tokens] }
            }
        })
    } else {
        unreachable!("convert_array_type called with non-array type")
    }
}

pub(crate) fn rust_type_to_syn_type(rust_type: &RustType) -> Result<syn::Type> {
    use RustType::*;
    Ok(match rust_type {
        Unit | String | Custom(_) | TypeParam(_) | Enum { .. } => convert_simple_type(rust_type)?,
        Primitive(prim_type) => convert_primitive_type(prim_type)?,
        Str { .. } | Cow { .. } => convert_lifetime_type(rust_type)?,
        Unsupported(name) => convert_unsupported_type(name)?,
        Vec(_) | HashMap(_, _) | Option(_) | Result(_, _) | HashSet(_) => {
            convert_container_type(rust_type)?
        }
        Tuple(_) | Generic { .. } | Reference { .. } => convert_complex_type(rust_type)?,
        Array { .. } => convert_array_type(rust_type)?,
    })
}

