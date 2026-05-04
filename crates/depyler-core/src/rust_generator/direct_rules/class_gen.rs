use crate::hir::*;
use crate::types::type_mapper::TypeMapper;
use anyhow::Result;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::parse_quote;

/// Check if a type can implement Copy trait.
fn is_copy_type(ty: &Type, enum_names: &HashSet<String>, copy_structs: &HashSet<String>) -> bool {
    match ty {
        Type::Int | Type::Float | Type::Bool | Type::None => true,
        Type::Optional(inner) | Type::Final(inner) => is_copy_type(inner, enum_names, copy_structs),
        Type::Tuple(elements) => elements
            .iter()
            .all(|t| is_copy_type(t, enum_names, copy_structs)),
        Type::Array { element_type, .. } => is_copy_type(element_type, enum_names, copy_structs),
        Type::Custom(name) => enum_names.contains(name) || copy_structs.contains(name),
        // These types are not Copy in Rust
        Type::String
        | Type::List(_)
        | Type::Dict(_, _)
        | Type::Set(_)
        | Type::Function { .. }
        | Type::TypeVar(_)
        | Type::Generic { .. }
        | Type::Union(_)
        | Type::Unknown => false,
    }
}

/// Build derive attributes for a struct, combining default derives with additional derives from annotations.
fn build_derive_attributes(
    class: &HirClass,
    enum_names: &HashSet<String>,
    copy_structs: &HashSet<String>,
) -> Vec<syn::Attribute> {
    // Check if the struct has no instance fields (only class constants)
    let has_instance_fields = class.fields.iter().any(|f| !f.is_class_var);

    // Check if this class will have a Drop implementation
    // Drop and Copy are mutually exclusive in Rust
    let has_drop_impl = class
        .methods
        .iter()
        .any(|m| m.name == "__del__" || m.name == "close");

    // Check if all instance fields are Copy-able
    let all_fields_copyable = class
        .fields
        .iter()
        .filter(|f| !f.is_class_var)
        .all(|f| is_copy_type(&f.field_type, enum_names, copy_structs));

    // Start with base derives depending on whether it's a dataclass
    let mut derives: Vec<String> = if class.is_dataclass {
        let mut d = vec![
            "Debug".to_string(),
            "Clone".to_string(),
            "PartialEq".to_string(),
            "Default".to_string(),
        ];
        // Only add Copy if fields are copyable AND there's no Drop implementation
        if all_fields_copyable && !has_drop_impl {
            d.insert(1, "Copy".to_string()); // Insert after Debug, before Clone
        }
        d
    } else if !has_instance_fields {
        // Empty struct (only class constants like IntEnum) can implement Copy
        vec!["Debug".to_string(), "Copy".to_string(), "Clone".to_string()]
    } else if all_fields_copyable && !has_drop_impl {
        // Only add Copy if fields are copyable AND there's no Drop implementation
        vec!["Debug".to_string(), "Copy".to_string(), "Clone".to_string()]
    } else {
        vec!["Debug".to_string(), "Clone".to_string()]
    };

    // Add additional derives from annotations (avoiding duplicates)
    for derive in &class.annotations.additional_derives {
        if !derives.iter().any(|d| d == derive) {
            derives.push(derive.clone());
        }
    }

    // Build the derive attribute
    let derive_idents: Vec<syn::Ident> = derives
        .iter()
        .map(|d| syn::Ident::new(d, proc_macro2::Span::call_site()))
        .collect();

    vec![parse_quote! { #[derive(#(#derive_idents),*)] }]
}

/// Convert a HIR class to Rust struct and impl blocks
///
/// This function transforms a Python-like class in HIR representation
/// into a Rust struct with associated impl blocks for methods.
pub fn convert_class_to_struct(
    class: &HirClass,
    type_mapper: &TypeMapper,
    abc_classes: &HashMap<String, &HirClass>,
    enum_names: &HashSet<String>,
    copy_structs: &HashSet<String>,
) -> Result<Vec<syn::Item>> {
    let mut items = Vec::new();
    let struct_name = syn::Ident::new(&class.name, proc_macro2::Span::call_site());

    // Filter out KW_ONLY sentinel (field named _ with type KW_ONLY)
    let is_kw_only_sentinel =
        |f: &&HirField| f.name == "_" && matches!(&f.field_type, Type::Custom(t) if t == "KW_ONLY");

    // Separate instance fields from class fields (constants/statics), excluding KW_ONLY
    let (instance_fields, class_fields): (Vec<_>, Vec<_>) = class
        .fields
        .iter()
        .filter(|f| !is_kw_only_sentinel(f))
        .partition(|f| !f.is_class_var);

    // Generate struct fields (only instance fields)
    let mut fields = Vec::new();
    for field in instance_fields {
        let field_name = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
        let rust_type = type_mapper.map_type(&field.field_type);
        let field_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;

        fields.push(syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Public(syn::Token![pub](proc_macro2::Span::call_site())),
            mutability: syn::FieldMutability::None,
            ident: Some(field_name),
            colon_token: Some(syn::Token![:](proc_macro2::Span::call_site())),
            ty: field_type,
        });
    }

    // Build derive attributes including any additional derives from annotations
    let derive_attrs = build_derive_attributes(class, enum_names, copy_structs);

    // Create the struct
    let struct_item = syn::Item::Struct(syn::ItemStruct {
        attrs: derive_attrs,
        vis: syn::Visibility::Public(syn::Token![pub](proc_macro2::Span::call_site())),
        struct_token: syn::Token![struct](proc_macro2::Span::call_site()),
        ident: struct_name.clone(),
        generics: syn::Generics::default(),
        fields: syn::Fields::Named(syn::FieldsNamed {
            brace_token: syn::token::Brace::default(),
            named: fields.into_iter().collect(),
        }),
        semi_token: None,
    });
    items.push(struct_item);

    // Generate impl block with methods
    let mut impl_items = Vec::new();

    // Add class constants first
    for class_field in &class_fields {
        if let Some(default_value) = &class_field.default_value {
            let const_name = syn::Ident::new(&class_field.name, proc_macro2::Span::call_site());
            let rust_type = type_mapper.map_type(&class_field.field_type);
            let const_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;
            let value_expr = super::convert_expr(default_value, type_mapper)?;

            // Generate: pub const NAME: Type = value;
            impl_items.push(parse_quote! {
                pub const #const_name: #const_type = #value_expr;
            });
        }
    }

    // Build field_types map for use in method conversion
    let field_types: HashMap<String, Type> = class
        .fields
        .iter()
        .filter(|f| !f.is_class_var)
        .map(|f| (f.name.clone(), f.field_type.clone()))
        .collect();

    // Build set of trait method names to avoid duplication
    let mut trait_method_names = std::collections::HashSet::new();
    for base_class_name in &class.base_classes {
        if let Some(&abc_class) = abc_classes.get(base_class_name) {
            for abc_method in &abc_class.methods {
                if abc_method.name != "__init__" {
                    trait_method_names.insert(abc_method.name.clone());
                }
            }
        }
    }

    // Check if class has explicit __init__
    let has_init = class.methods.iter().any(|m| m.name == "__init__");

    // Convert __init__ to new() if present, or generate default new() for dataclasses
    if has_init {
        for method in &class.methods {
            if method.name == "__init__" {
                let new_method = convert_init_to_new(method, class, &struct_name, type_mapper)?;
                impl_items.push(syn::ImplItem::Fn(new_method));
            } else if method.name == "__del__" {
                // Skip __del__ - it will be converted to Drop trait
                continue;
            } else if trait_method_names.contains(&method.name) {
                // Skip trait methods - they'll be in impl Trait for Struct
                continue;
            } else {
                let rust_method = convert_method_to_impl_item(method, type_mapper, &field_types)?;
                impl_items.push(syn::ImplItem::Fn(rust_method));
            }
        }
    } else {
        // Generate default new() for dataclasses or classes with default field values
        if class.is_dataclass
            || class
                .fields
                .iter()
                .all(|f| f.default_value.is_some() || f.field_type == Type::Int)
        {
            let new_method = generate_dataclass_new(class, &struct_name, type_mapper)?;
            impl_items.push(syn::ImplItem::Fn(new_method));
        }

        // Add other methods
        for method in &class.methods {
            if method.name == "__del__" {
                // Skip __del__ - it will be converted to Drop trait
                continue;
            } else if trait_method_names.contains(&method.name) {
                // Skip trait methods - they'll be in impl Trait for Struct
                continue;
            }
            let rust_method = convert_method_to_impl_item(method, type_mapper, &field_types)?;
            impl_items.push(syn::ImplItem::Fn(rust_method));
        }
    }

    // Generate _get_field method for dynamic attribute access
    // Only generate if the class defines __getattr__ or __setattr__ methods
    if class.needs_dynamic_field_access {
        if let Some(get_field_method) = generate_get_field_method(class, type_mapper)? {
            impl_items.push(syn::ImplItem::Fn(get_field_method));
        }

        // Generate _set_field method for dynamic attribute mutation
        if let Some(set_field_method) = generate_set_field_method(class, type_mapper)? {
            impl_items.push(syn::ImplItem::Fn(set_field_method));
        }
    }

    // Only generate impl block if there are methods
    if !impl_items.is_empty() {
        let impl_block = syn::Item::Impl(syn::ItemImpl {
            attrs: vec![],
            defaultness: None,
            unsafety: None,
            impl_token: syn::Token![impl](proc_macro2::Span::call_site()),
            generics: syn::Generics::default(),
            trait_: None,
            self_ty: Box::new(parse_quote! { #struct_name }),
            brace_token: syn::token::Brace::default(),
            items: impl_items,
        });
        items.push(impl_block);
    }

    // Generate Drop trait implementation if __del__ is present
    if let Some(drop_impl) = generate_drop_impl(class, &struct_name, type_mapper)? {
        items.push(drop_impl);
    }

    // Generate trait implementations for ABC base classes
    for base_class_name in &class.base_classes {
        if let Some(&abc_class) = abc_classes.get(base_class_name) {
            // Generate impl Trait for Struct
            let trait_name = syn::Ident::new(base_class_name, proc_macro2::Span::call_site());
            let mut trait_impl_items = Vec::new();

            // Find methods in the current class that implement trait methods
            for abc_method in &abc_class.methods {
                if abc_method.name == "__init__" {
                    continue;
                }

                // Find corresponding method in current class
                if let Some(class_method) = class.methods.iter().find(|m| m.name == abc_method.name)
                {
                    let method_name = if super::is_rust_keyword(&class_method.name) {
                        syn::Ident::new_raw(&class_method.name, proc_macro2::Span::call_site())
                    } else {
                        syn::Ident::new(&class_method.name, proc_macro2::Span::call_site())
                    };

                    // Build parameters (skip self)
                    let mut params: Vec<syn::FnArg> = Vec::new();
                    if !class_method.is_static && !class_method.is_classmethod {
                        params.push(parse_quote! { &self });
                    }

                    for param in &class_method.params {
                        let param_name =
                            syn::Ident::new(&param.name, proc_macro2::Span::call_site());
                        let rust_type = type_mapper.map_type(&param.ty);
                        let param_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;
                        params.push(parse_quote! { #param_name: #param_type });
                    }

                    // Return type
                    let return_type = if class_method.ret_type == Type::None {
                        parse_quote! { () }
                    } else {
                        let rust_ret_type = type_mapper.map_type(&class_method.ret_type);
                        super::type_conv::rust_type_to_syn_type(&rust_ret_type)?
                    };

                    // Generate method body
                    let empty_field_types = HashMap::new();
                    let body = super::convert_block_for_trait_impl(
                        &class_method.body,
                        type_mapper,
                        class_method.is_classmethod,
                        &empty_field_types,
                        base_class_name,
                    )?;

                    trait_impl_items.push(syn::ImplItem::Fn(syn::ImplItemFn {
                        attrs: vec![],
                        vis: syn::Visibility::Inherited,
                        defaultness: None,
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
                            output: parse_quote! { -> #return_type },
                        },
                        block: body,
                    }));
                }
            }

            // Only generate trait impl if there are methods
            if !trait_impl_items.is_empty() {
                let trait_impl = syn::Item::Impl(syn::ItemImpl {
                    attrs: vec![],
                    defaultness: None,
                    unsafety: None,
                    impl_token: syn::Token![impl](proc_macro2::Span::call_site()),
                    generics: syn::Generics::default(),
                    trait_: Some((
                        None,
                        parse_quote! { #trait_name },
                        syn::Token![for](proc_macro2::Span::call_site()),
                    )),
                    self_ty: Box::new(parse_quote! { #struct_name }),
                    brace_token: syn::token::Brace::default(),
                    items: trait_impl_items,
                });
                items.push(trait_impl);
            }
        }
    }

    Ok(items)
}

/// Convert a Python Enum/IntEnum class to a Rust enum with integer discriminants.

fn generate_dataclass_new(
    class: &HirClass,
    _struct_name: &syn::Ident,
    type_mapper: &TypeMapper,
) -> Result<syn::ImplItemFn> {
    // Filter out KW_ONLY sentinel (field named _ with type KW_ONLY)
    let is_kw_only_sentinel =
        |f: &&HirField| f.name == "_" && matches!(&f.field_type, Type::Custom(t) if t == "KW_ONLY");

    // Generate parameters from fields (skip fields with defaults, class variables, and KW_ONLY sentinel)
    let mut inputs = syn::punctuated::Punctuated::new();
    let fields_without_defaults: Vec<_> = class
        .fields
        .iter()
        .filter(|f| !f.is_class_var && f.default_value.is_none() && !is_kw_only_sentinel(f))
        .collect();

    for field in &fields_without_defaults {
        let param_ident = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
        let rust_type = type_mapper.map_type(&field.field_type);
        let param_syn_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;

        inputs.push(syn::FnArg::Typed(syn::PatType {
            attrs: vec![],
            pat: Box::new(syn::Pat::Ident(syn::PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: None,
                ident: param_ident.clone(),
                subpat: None,
            })),
            colon_token: syn::Token![:](proc_macro2::Span::call_site()),
            ty: Box::new(param_syn_type),
        }));
    }

    // Generate body that initializes struct fields (skip class variables and KW_ONLY sentinel)
    let field_inits = class
        .fields
        .iter()
        .filter(|f| !f.is_class_var && !is_kw_only_sentinel(f))
        .map(|field| {
            let field_ident = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
            if let Some(default_value) = &field.default_value {
                // Check if this is a field() call with default_factory
                if let HirExpr::Call { func, kwargs, .. } = default_value {
                    // Check if it's a call to field()
                    if func == "field" {
                        // Look for default_factory keyword argument
                        if let Some((_, factory_expr)) =
                            kwargs.iter().find(|(k, _)| k == "default_factory")
                        {
                            // Generate appropriate initialization based on factory
                            if let HirExpr::Var(factory_name) = factory_expr {
                                match factory_name.as_str() {
                                    "set" => return quote! { #field_ident: HashSet::new() },
                                    "list" => return quote! { #field_ident: Vec::new() },
                                    "dict" => return quote! { #field_ident: HashMap::new() },
                                    _ => {}
                                }
                            }
                        } else if let Some((_, default_val)) =
                            kwargs.iter().find(|(k, _)| k == "default")
                        {
                            // Handle field(default=value)
                            if let Ok(expr) = super::convert_expr(default_val, type_mapper) {
                                return quote! { #field_ident: #expr };
                            }
                        }
                        // No default_factory or default, use Default::default()
                        return quote! { #field_ident: Default::default() };
                    }
                }

                // Not a field() call or other default value expression
                if field.field_type == Type::Int {
                    quote! { #field_ident: 0 }
                } else {
                    quote! { #field_ident: Default::default() }
                }
            } else {
                // Use parameter
                quote! { #field_ident }
            }
        })
        .collect::<Vec<_>>();

    let body = parse_quote! {
        {
            Self {
                #(#field_inits),*
            }
        }
    };

    Ok(syn::ImplItemFn {
        attrs: vec![],
        vis: syn::Visibility::Public(syn::Token![pub](proc_macro2::Span::call_site())),
        defaultness: None,
        sig: syn::Signature {
            constness: None,
            asyncness: None,
            unsafety: None,
            abi: None,
            fn_token: syn::Token![fn](proc_macro2::Span::call_site()),
            ident: syn::Ident::new("new", proc_macro2::Span::call_site()),
            generics: syn::Generics::default(),
            paren_token: syn::token::Paren::default(),
            inputs,
            variadic: None,
            output: syn::ReturnType::Type(
                syn::Token![->](proc_macro2::Span::call_site()),
                Box::new(parse_quote! { Self }),
            ),
        },
        block: body,
    })
}

/// Generate Drop trait implementation if __del__ method is present
fn generate_drop_impl(
    class: &HirClass,
    struct_name: &syn::Ident,
    type_mapper: &TypeMapper,
) -> Result<Option<syn::Item>> {
    // Find __del__ method
    let del_method = class.methods.iter().find(|m| m.name == "__del__");

    // If there's a __del__ method, use its body
    let body = if let Some(del_method) = del_method {
        let empty_field_types = HashMap::new();
        super::convert_block_with_context(&del_method.body, type_mapper, false, &empty_field_types)?
    } else {
        // Check if there's a close() method
        let close_method = class.methods.iter().find(|m| m.name == "close");

        if let Some(_) = close_method {
            // Generate a simple Drop that calls self.close()
            parse_quote! {{
                self.close();
            }}
        } else {
            // No __del__ or close() method
            return Ok(None);
        }
    };

    // Create Drop trait implementation
    let drop_impl = syn::Item::Impl(syn::ItemImpl {
        attrs: vec![],
        defaultness: None,
        unsafety: None,
        impl_token: syn::Token![impl](proc_macro2::Span::call_site()),
        generics: syn::Generics::default(),
        trait_: Some((
            None,
            parse_quote! { Drop },
            syn::Token![for](proc_macro2::Span::call_site()),
        )),
        self_ty: Box::new(parse_quote! { #struct_name }),
        brace_token: syn::token::Brace::default(),
        items: vec![syn::ImplItem::Fn(syn::ImplItemFn {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            defaultness: None,
            sig: syn::Signature {
                constness: None,
                asyncness: None,
                unsafety: None,
                abi: None,
                fn_token: syn::Token![fn](proc_macro2::Span::call_site()),
                ident: syn::Ident::new("drop", proc_macro2::Span::call_site()),
                generics: syn::Generics::default(),
                paren_token: syn::token::Paren::default(),
                inputs: {
                    let mut inputs = syn::punctuated::Punctuated::new();
                    inputs.push(parse_quote! { &mut self });
                    inputs
                },
                variadic: None,
                output: syn::ReturnType::Default,
            },
            block: body,
        })],
    });

    Ok(Some(drop_impl))
}

/// Generate _get_field method for dynamic attribute access via getattr()
fn generate_get_field_method(
    class: &HirClass,
    type_mapper: &TypeMapper,
) -> Result<Option<syn::ImplItemFn>> {
    // Filter out KW_ONLY sentinel (field named _ with type KW_ONLY)
    let is_kw_only_sentinel =
        |f: &&HirField| f.name == "_" && matches!(&f.field_type, Type::Custom(t) if t == "KW_ONLY");

    let instance_fields: Vec<_> = class
        .fields
        .iter()
        .filter(|f| !f.is_class_var && !is_kw_only_sentinel(f))
        .collect();
    if instance_fields.is_empty() {
        return Ok(None);
    }

    // Build match arms for each field
    let mut match_arms = Vec::new();
    for field in &instance_fields {
        let field_name = &field.name;
        let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
        let rust_type = type_mapper.map_type(&field.field_type);

        // Always clone - works for both Copy and non-Copy types
        let _ = rust_type; // Suppress unused warning
        let clone_expr = quote! { self.#field_ident.clone() };

        match_arms.push(quote! {
            #field_name => Some(Box::new(#clone_expr) as Box<dyn std::any::Any>)
        });
    }

    // Add fallback arm
    match_arms.push(quote! {
        _ => None
    });

    let method: syn::ImplItemFn = parse_quote! {
        pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
            match name {
                #(#match_arms),*
            }
        }
    };

    Ok(Some(method))
}

/// Generate _set_field method for dynamic attribute mutation via setattr()
fn generate_set_field_method(
    class: &HirClass,
    type_mapper: &TypeMapper,
) -> Result<Option<syn::ImplItemFn>> {
    // Filter out KW_ONLY sentinel (field named _ with type KW_ONLY)
    let is_kw_only_sentinel =
        |f: &&HirField| f.name == "_" && matches!(&f.field_type, Type::Custom(t) if t == "KW_ONLY");

    let instance_fields: Vec<_> = class
        .fields
        .iter()
        .filter(|f| !f.is_class_var && !is_kw_only_sentinel(f))
        .collect();
    if instance_fields.is_empty() {
        return Ok(None);
    }

    // Build match arms for each field
    let mut match_arms = Vec::new();
    for field in &instance_fields {
        let field_name = &field.name;
        let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
        let rust_type = type_mapper.map_type(&field.field_type);
        let syn_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;

        match_arms.push(quote! {
            #field_name => {
                if let Some(v) = value.downcast_ref::<#syn_type>() {
                    self.#field_ident = v.clone();
                    true
                } else {
                    false
                }
            }
        });
    }

    // Add fallback arm
    match_arms.push(quote! {
        _ => false
    });

    let method: syn::ImplItemFn = parse_quote! {
        pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
            match name {
                #(#match_arms),*
            }
        }
    };

    Ok(Some(method))
}

/// Extract field initializations from __init__ body
/// Scans for assignments like self.field = value and stores them
fn extract_field_initializations(
    stmts: &[HirStmt],
    field_values: &mut std::collections::HashMap<String, HirExpr>,
) {
    use crate::hir::{AssignTarget, HirExpr, HirStmt};

    for stmt in stmts {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                // Check if this is a self.field assignment
                if let AssignTarget::Attribute {
                    value: base,
                    attr: field_name,
                } = target
                {
                    // Check if base is self
                    if let HirExpr::Var(var_name) = base.as_ref() {
                        if var_name == "self" {
                            // Store the field initialization value
                            field_values.insert(field_name.clone(), value.clone());
                        }
                    }
                }
            }
            // Recursively search in control flow structures
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                extract_field_initializations(then_body, field_values);
                if let Some(else_stmts) = else_body {
                    extract_field_initializations(else_stmts, field_values);
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                extract_field_initializations(body, field_values);
            }
            _ => {}
        }
    }
}

fn convert_init_to_new(
    init_method: &HirMethod,
    class: &HirClass,
    _struct_name: &syn::Ident,
    type_mapper: &TypeMapper,
) -> Result<syn::ImplItemFn> {
    // Convert parameters
    let mut inputs = syn::punctuated::Punctuated::new();

    for param in &init_method.params {
        // Rename 'self' to 'self_param' since 'self' is a Rust keyword
        let param_name = if param.name == "self" {
            "self_param"
        } else {
            &param.name
        };
        let param_ident = syn::Ident::new(param_name, proc_macro2::Span::call_site());
        let rust_type = type_mapper.map_type(&param.ty);
        let param_syn_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;

        inputs.push(syn::FnArg::Typed(syn::PatType {
            attrs: vec![],
            pat: Box::new(syn::Pat::Ident(syn::PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: None,
                ident: param_ident,
                subpat: None,
            })),
            colon_token: syn::Token![:](proc_macro2::Span::call_site()),
            ty: Box::new(param_syn_type),
        }));
    }

    // Extract field initializations from __init__ body
    // Look for assignments like self.field = value
    let mut field_init_values = std::collections::HashMap::new();
    extract_field_initializations(&init_method.body, &mut field_init_values);

    // Generate field initializers based on class fields and parameters
    // Skip class variables (constants) - only initialize instance fields
    let mut field_inits = Vec::new();

    for field in &class.fields {
        // Skip class variables (constants/statics)
        if field.is_class_var {
            continue;
        }

        let field_ident = syn::Ident::new(&field.name, proc_macro2::Span::call_site());

        // Check if this field matches a parameter name
        if init_method
            .params
            .iter()
            .any(|param| param.name == field.name)
        {
            // Initialize from parameter
            field_inits.push(quote! { #field_ident });
        } else if let Some(init_value) = field_init_values.get(&field.name) {
            // Initialize from value assigned in __init__ body
            // Special case: If field is Option<T> and init_value is None, use None instead of ()
            let init_expr = if matches!(&field.field_type, Type::Optional(_))
                && matches!(init_value, HirExpr::Literal(Literal::None))
            {
                parse_quote! { None }
            } else {
                super::convert_expr(init_value, type_mapper)?
            };
            field_inits.push(quote! { #field_ident: #init_expr });
        } else {
            // Initialize with default value based on type
            let default_value = match &field.field_type {
                Type::Int => quote! { 0 },
                Type::Float => quote! { 0.0 },
                Type::String => quote! { String::new() },
                Type::Bool => quote! { false },
                Type::List(_) => quote! { Vec::new() },
                Type::Dict(_, _) => quote! { std::collections::HashMap::new() },
                Type::Set(_) => quote! { std::collections::HashSet::new() },
                Type::Optional(_) => quote! { None },
                _ => quote! { Default::default() },
            };
            field_inits.push(quote! { #field_ident: #default_value });
        }
    }

    let body = parse_quote! {
        {
            Self {
                #(#field_inits),*
            }
        }
    };

    Ok(syn::ImplItemFn {
        attrs: vec![],
        vis: syn::Visibility::Public(syn::Token![pub](proc_macro2::Span::call_site())),
        defaultness: None,
        sig: syn::Signature {
            constness: None,
            asyncness: None,
            unsafety: None,
            abi: None,
            fn_token: syn::Token![fn](proc_macro2::Span::call_site()),
            ident: syn::Ident::new("new", proc_macro2::Span::call_site()),
            generics: syn::Generics::default(),
            paren_token: syn::token::Paren::default(),
            inputs,
            variadic: None,
            output: syn::ReturnType::Type(
                syn::Token![->](proc_macro2::Span::call_site()),
                Box::new(parse_quote! { Self }),
            ),
        },
        block: body,
    })
}

/// Check if a method mutates self (requires &mut self)
/// Scans the method body for assignments to self attributes
pub fn method_mutates_self(method: &HirMethod) -> bool {
    for stmt in &method.body {
        if stmt_mutates_self(stmt) {
            return true;
        }
    }
    false
}

/// Check if a statement mutates self
fn stmt_mutates_self(stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Assign { target, .. } => {
            // Check if target is self.field assignment
            matches!(target, AssignTarget::Attribute { value, .. }
                if matches!(value.as_ref(), HirExpr::Var(sym) if sym.as_str() == "self"))
        }
        HirStmt::Expr(expr) => {
            // Check if the expression calls a mutating method on self or self.field
            expr_mutates_self(expr)
        }
        HirStmt::Return(Some(expr)) => {
            // Check if the return expression mutates self
            expr_mutates_self(expr)
        }
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(stmt_mutates_self)
                || else_body
                    .as_ref()
                    .is_some_and(|body| body.iter().any(stmt_mutates_self))
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            body.iter().any(stmt_mutates_self)
        }
        _ => false,
    }
}

/// Check if an expression mutates self
fn expr_mutates_self(expr: &HirExpr) -> bool {
    match expr {
        // Any method call on self or self.field is conservatively treated as
        // mutating.  This avoids the false-negative that the old name-based
        // heuristic produced when a user-defined mutating method was not in
        // the hard-coded allow-list.
        HirExpr::MethodCall { object, .. } => is_self_or_self_field(object),
        // Any other expression patterns that might mutate self
        _ => false,
    }
}

/// Check if an expression is `self` or `self.field`
fn is_self_or_self_field(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Var(sym) if sym.as_str() == "self" => true,
        HirExpr::Attribute { value, .. } => {
            // Check if it's self.something
            matches!(value.as_ref(), HirExpr::Var(sym) if sym.as_str() == "self")
        }
        _ => false,
    }
}

/// Infer the return type of a method by delegating to the shared
/// `func_gen::infer_return_type_from_body` which uses the full
/// dataflow-aware expression type inferencer.
fn infer_method_return_type(method: &HirMethod) -> Option<Type> {
    // Use an empty class-field map; at this point we do not have cross-class
    // information, but the method body is sufficient for common literal / binary
    // expression returns.
    let empty_class_fields = std::collections::HashMap::new();
    crate::rust_generator::func_gen::infer_return_type_from_body(
        &method.body,
        &method.params,
        &empty_class_fields,
    )
}

fn convert_method_to_impl_item(
    method: &HirMethod,
    type_mapper: &TypeMapper,
    field_types: &HashMap<String, Type>,
) -> Result<syn::ImplItemFn> {
    let method_name = if super::is_rust_keyword(&method.name) {
        syn::Ident::new_raw(&method.name, proc_macro2::Span::call_site())
    } else {
        syn::Ident::new(&method.name, proc_macro2::Span::call_site())
    };

    // Convert parameters
    let mut inputs = syn::punctuated::Punctuated::new();

    // Add self parameter based on method type
    if method.is_static {
        // Static methods have no self parameter
    } else if method.is_classmethod {
        // Note: Class methods would ideally take a type parameter (e.g., &Self),
        // but currently classmethods are transpiled without self parameter.
        // Proper classmethod support with type parameter is a known limitation.
    } else if method.is_property {
        // Properties typically use &self
        inputs.push(parse_quote! { &self });
    } else {
        // Regular instance methods: use &mut self if method mutates self, otherwise &self
        if method_mutates_self(method) {
            inputs.push(parse_quote! { &mut self });
        } else {
            inputs.push(parse_quote! { &self });
        }
    }

    // Add other parameters
    for param in &method.params {
        // Rename 'self' to 'self_param' since 'self' is a Rust keyword
        let param_name = if param.name == "self" {
            "self_param"
        } else {
            &param.name
        };
        let param_ident = syn::Ident::new(param_name, proc_macro2::Span::call_site());
        let rust_type = type_mapper.map_type(&param.ty);
        let param_syn_type = super::type_conv::rust_type_to_syn_type(&rust_type)?;

        inputs.push(syn::FnArg::Typed(syn::PatType {
            attrs: vec![],
            pat: Box::new(syn::Pat::Ident(syn::PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: None,
                ident: param_ident,
                subpat: None,
            })),
            colon_token: syn::Token![:](proc_macro2::Span::call_site()),
            ty: Box::new(param_syn_type),
        }));
    }

    // Convert return type - infer if Unknown or None
    // Five-Whys Root Cause:
    // 1. Why: expected `()`, found `bool` in __exit__ method
    // 2. Why: Method has no return type annotation, so ret_type is Unknown/None
    // 3. Why: convert_method_to_impl_item uses type_mapper.map_type directly
    // 4. Why: No return type inference is applied to class methods
    // 5. ROOT CAUSE: direct_rules.rs doesn't infer return type for methods
    let effective_ret_type = if matches!(method.ret_type, Type::Unknown | Type::None) {
        // Try to infer from body - if we find a typed return, use it
        infer_method_return_type(method).unwrap_or_else(|| method.ret_type.clone())
    } else {
        method.ret_type.clone()
    };
    let rust_ret_type = type_mapper.map_type(&effective_ret_type);
    let ret_type = super::type_conv::rust_type_to_syn_type(&rust_ret_type)?;

    // Convert method body
    let body = if method.body.is_empty() {
        // Empty body - just return default
        parse_quote! { {} }
    } else {
        // Convert the method body statements with classmethod context
        super::convert_block_with_context(
            &method.body,
            type_mapper,
            method.is_classmethod,
            field_types,
        )?
    };

    Ok(syn::ImplItemFn {
        attrs: vec![],
        vis: syn::Visibility::Public(syn::Token![pub](proc_macro2::Span::call_site())),
        defaultness: None,
        sig: syn::Signature {
            constness: None,
            asyncness: if method.is_async {
                Some(syn::Token![async](proc_macro2::Span::call_site()))
            } else {
                None
            },
            unsafety: None,
            abi: None,
            fn_token: syn::Token![fn](proc_macro2::Span::call_site()),
            ident: method_name,
            generics: syn::Generics::default(),
            paren_token: syn::token::Paren::default(),
            inputs,
            variadic: None,
            output: if matches!(effective_ret_type, Type::None) {
                syn::ReturnType::Default
            } else {
                syn::ReturnType::Type(
                    syn::Token![->](proc_macro2::Span::call_site()),
                    Box::new(ret_type),
                )
            },
        },
        block: body,
    })
}
