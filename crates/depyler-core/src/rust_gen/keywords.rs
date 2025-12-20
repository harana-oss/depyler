//! Rust keyword escaping utilities
//!
//! Centralized module for handling Rust keywords when generating identifiers.
//! Ensures Python variable names that are Rust keywords get properly escaped.

use proc_macro2::Span;
use syn::Ident;

/// Check if a name is a Rust keyword that needs escaping
pub fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

/// Create a safe Rust identifier, using raw identifier syntax (r#) if needed
///
/// # Examples
/// ```ignore
/// use depyler_core::rust_gen::keywords::safe_ident;
///
/// let ident = safe_ident("match");  // Creates r#match
/// let ident = safe_ident("value");  // Creates value
/// ```
pub fn safe_ident(name: &str) -> Ident {
    if is_rust_keyword(name) {
        // Use raw identifier syntax: r#match
        Ident::new_raw(name, Span::call_site())
    } else {
        Ident::new(name, Span::call_site())
    }
}
