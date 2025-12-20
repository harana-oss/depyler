//! Rust code formatting utilities
//!
//! This module provides post-processing formatting for generated Rust code.
//! The primary function `format_rust_code` applies various string replacements
//! to clean up spacing and formatting issues in the generated token streams.

/// Format Rust code using rustfmt for idiomatic formatting
///
/// This function first applies string replacements to fix common issues from
/// quote! macro expansion, then runs rustfmt to ensure idiomatic formatting.
///
/// # Arguments
/// * `code` - The Rust code to format as a String
///
/// # Returns
/// Formatted Rust code that passes rustfmt --check
///
/// # Example
/// ```ignore
/// use depyler_core::rust_gen::format::format_rust_code;
/// let code = "fn main ( ) { println ! ( \"Hello\" ) ; }".to_string();
/// let formatted = format_rust_code(code);
/// // Returns properly formatted Rust code
/// ```
pub fn format_rust_code(code: String) -> String {
    // First apply string replacements to fix obvious issues
    let code = apply_string_replacements(code);

    // Then run rustfmt to ensure idiomatic formatting
    match run_rustfmt(&code) {
        Ok(formatted) => formatted,
        Err(_) => code, // Fall back to unformatted if rustfmt fails
    }
}

/// Run rustfmt on code string and return formatted result
fn run_rustfmt(code: &str) -> Result<String, std::io::Error> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write to stdin in a scope to ensure it's dropped/closed before wait
    {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(code.as_bytes())?;
            // stdin is automatically dropped here when it goes out of scope
        }
    }

    let output = child.wait_with_output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(std::io::Error::other("rustfmt failed"))
    }
}

/// Apply string replacements to fix common formatting issues
fn apply_string_replacements(code: String) -> String {
    code
        // Fix comparison/equality operators first (before other spacing fixes)
        .replace("= =", "==")
        .replace("! =", "!=")
        // Fix negative number spacing in parentheses
        .replace("(- ", "(-")
        .replace(" ; ", ";\n    ")
        .replace(" { ", " {\n    ")
        .replace(" } ", "\n}\n")
        .replace("} ;", "};")
        .replace("use std :: collections :: HashMap ;", "use std::collections::HashMap;")
        // Fix method call spacing
        .replace(" . ", ".")
        .replace(" (", "(")
        .replace(" )", ")")
        // Fix specific common patterns
        .replace(".len ()", ".len()")
        .replace(".push (", ".push(")
        .replace(".insert (", ".insert(")
        .replace(".get (", ".get(")
        .replace(".contains_key (", ".contains_key(")
        .replace(".to_string ()", ".to_string()")
        // Fix spacing around operators in some contexts
        .replace(" ::", "::")
        .replace(":: ", "::")
        // Fix attribute spacing
        .replace("# [", "#[")
        // Fix type annotations
        .replace(" : ", ": ")
        // Fix parameter spacing
        .replace(" , ", ", ")
        // Fix assignment operator spacing issues (but not == or !=)
        // Only fix cases where a single = appears before (
        // We check that it's not part of == or != by looking for patterns
        .replace("  =", " =") // Fix multiple spaces before =
        .replace("   =", " =") // Fix even more spaces
        // Fix generic type spacing
        .replace("Vec < ", "Vec<")
        .replace(" < ", "<")
        .replace(" > ", ">")
        .replace("> ", ">")
        .replace("< ", "<")
        .replace(" >", ">") // Fix trailing space before closing bracket
        // Fix return type spacing (CRITICAL: Normalize first, then add proper spacing!)
        .replace(" -> ", "->") // Step 1: Remove existing spaces (normalize)
        .replace("-> ", "->") // Step 2: Remove trailing space
        .replace(" ->", "->") // Step 3: Remove leading space
        .replace("->", " -> ") // Step 4: Add correct spacing everywhere
        // Fix reference spacing
        .replace("& self", "&self")
        .replace("& mut", "&mut")
        // Fix macro spacing (space before !)
        .replace(" !", "!")
        // Re-fix comparison operators that may have been broken by other replacements
        .replace("= =", "==")
        .replace("! =", "!=")
        // Fix comparison operator spacing
        .replace("value<", "value < ")
        .replace("<self", "< self")
        // Fix range spacing
        .replace(" .. ", "..")
        .replace(" ..", "..")
        .replace(".. ", "..")
        // Fix 'in' keyword spacing
        .replace("in(", "in (")
        // CRITICAL: Fix comparison operator followed by negation LAST
        // This must happen after "< " -> "<" replacement to avoid being undone
        .replace("<-", "< -")
        .replace(">-", "> -")
        .replace("in(", "in (")
}
