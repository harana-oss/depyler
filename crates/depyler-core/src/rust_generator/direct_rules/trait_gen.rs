use crate::hir::*;
use crate::types::type_mapper::TypeMapper;
use anyhow::Result;
use syn::parse_quote;

pub(crate) fn convert_abc_to_trait(
    class: &HirClass,
    type_mapper: &TypeMapper,
) -> Result<syn::Item> {
    let trait_name = syn::Ident::new(&class.name, proc_macro2::Span::call_site());

    // Convert ABC methods to trait methods (excluding __init__ and other special methods)
    let mut trait_items = Vec::new();
    for method in &class.methods {
        // Skip __init__ and __del__ as they're not part of the trait interface
        if method.name == "__init__" || method.name == "__del__" {
            continue;
        }

        let method_item = convert_abc_method_to_trait_method(method, type_mapper)?;
        trait_items.push(method_item);
    }

    Ok(syn::Item::Trait(syn::ItemTrait {
        attrs: vec![],
        vis: parse_quote! { pub },
        unsafety: None,
        auto_token: None,
        restriction: None,
        trait_token: syn::Token![trait](proc_macro2::Span::call_site()),
        ident: trait_name,
        generics: syn::Generics::default(),
        colon_token: None,
        supertraits: syn::punctuated::Punctuated::new(),
        brace_token: syn::token::Brace::default(),
        items: trait_items,
    }))
}

fn convert_abc_method_to_trait_method(
    method: &HirMethod,
    type_mapper: &TypeMapper,
) -> Result<syn::TraitItem> {
    let method_name = if super::is_rust_keyword(&method.name) {
        syn::Ident::new_raw(&method.name, proc_macro2::Span::call_site())
    } else {
        syn::Ident::new(&method.name, proc_macro2::Span::call_site())
    };

    // Convert parameters (skip 'self')
    let mut params: Vec<syn::FnArg> = Vec::new();

    // Add self parameter if needed
    if !method.is_static {
        params.push(parse_quote! { &self });
    }

    // Add other parameters
    for param in &method.params {
        let param_name = syn::Ident::new(&param.name, proc_macro2::Span::call_site());
        let rust_type = type_mapper.map_type(&param.ty);
        let param_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;
        params.push(parse_quote! { #param_name: #param_type });
    }

    // Convert return type
    let return_type = if method.ret_type == Type::None {
        parse_quote! { -> () }
    } else {
        let rust_return_type = type_mapper.map_type(&method.ret_type);
        let syn_return_type = super::type_conv::rust_type_to_syn_type(&rust_return_type)?;
        parse_quote! { -> #syn_return_type }
    };

    // Create trait method signature (no body for abstract methods)
    Ok(syn::TraitItem::Fn(syn::TraitItemFn {
        attrs: vec![],
        sig: syn::Signature {
            constness: None,
            asyncness: None,
            unsafety: None,
            abi: None,
            fn_token: syn::Token![fn](proc_macro2::Span::call_site()),
            ident: method_name,
            generics: syn::Generics::default(),
            paren_token: syn::token::Paren::default(),
            inputs: params.into_iter().collect(),
            variadic: None,
            output: return_type,
        },
        default: None,
        semi_token: Some(syn::Token![;](proc_macro2::Span::call_site())),
    }))
}
