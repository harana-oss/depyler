//! Rust code formatting utilities
//!
//! This module provides post-processing formatting for generated Rust code.
//! The primary function `format_rust_code` applies various string replacements
//! to clean up spacing and formatting issues in the generated token streams.

/// Format Rust code using rustfmt for idiomatic formatting
pub fn format_rust_code(code: String) -> String {
    // Extract lazy_static blocks before string replacements to protect their content
    let (code_without_lazy, lazy_blocks) = extract_lazy_static_blocks(&code);

    // Apply string replacements to non-macro code
    let code = apply_string_replacements(code_without_lazy);

    // Reinsert lazy_static blocks (formatted separately)
    let code = reinsert_lazy_static_blocks(code, lazy_blocks);

    // Run rustfmt on the combined result
    match run_rustfmt(&code) {
        Ok(formatted) => formatted,
        Err(_) => code,
    }
}

/// Extract lazy_static blocks from code, replacing them with placeholders.
/// Returns the code with placeholders and the extracted blocks (pre-formatted).
fn extract_lazy_static_blocks(code: &str) -> (String, Vec<String>) {
    if !code.contains("lazy_static :: lazy_static !") {
        return (code.to_string(), Vec::new());
    }

    let mut result = String::with_capacity(code.len());
    let mut blocks = Vec::new();
    let mut remaining = code;

    while let Some(start) = remaining.find("lazy_static :: lazy_static !") {
        // Add everything before this block
        result.push_str(&remaining[..start]);

        let after = &remaining[start..];
        // Find the opening brace
        if let Some(brace_offset) = after.find('{') {
            let mut depth = 1;
            let mut pos = brace_offset + 1;
            let bytes = after.as_bytes();
            while pos < after.len() && depth > 0 {
                match bytes[pos] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
            // Extract the raw macro block
            let raw_block = &after[..pos];
            // Format it properly
            let formatted = format_lazy_static_block(raw_block);
            let placeholder = format!("/* __LAZY_STATIC_{}__ */", blocks.len());
            blocks.push(formatted);
            result.push_str(&placeholder);
            remaining = &after[pos..];
        } else {
            result.push_str(after);
            remaining = "";
        }
    }
    result.push_str(remaining);
    (result, blocks)
}

/// Format a raw lazy_static block's content properly.
fn format_lazy_static_block(raw: &str) -> String {
    // The raw content from prettyplease looks like:
    // lazy_static :: lazy_static ! { pub static ref NAME : TYPE = VALUE ; }
    // We need to extract the inner declaration and format it.

    // Find the content between { and }
    let brace_start = raw.find('{').unwrap_or(0);
    let brace_end = raw.rfind('}').unwrap_or(raw.len());
    let inner = raw[brace_start + 1..brace_end].trim().trim_end_matches(';');

    // Build a dummy const so rustfmt can format the entire declaration:
    // const NAME: TYPE = VALUE;
    if let Some(eq_pos) = find_declaration_equals(inner) {
        let type_and_name = inner[..eq_pos].trim();
        let value = inner[eq_pos + 1..].trim();

        // Strip "pub static ref" prefix to get "NAME : TYPE"
        let name_type = type_and_name
            .strip_prefix("pub static ref")
            .unwrap_or(type_and_name)
            .trim();

        // Format the entire declaration using rustfmt
        let dummy_code = format!("const {name_type} = {value};");
        match run_rustfmt(&dummy_code) {
            Ok(formatted) => {
                // Extract the formatted "NAME: TYPE = VALUE" from "const NAME: TYPE = VALUE;"
                let formatted = formatted.trim().trim_end_matches(';').trim();
                if let Some(stripped) = formatted.strip_prefix("const ") {
                    // Re-indent multi-line values: rustfmt indents from column 0,
                    // but inside lazy_static! the content needs 4 extra spaces
                    let lines: Vec<&str> = stripped.trim().lines().collect();
                    if lines.len() <= 1 {
                        format!(
                            "lazy_static::lazy_static! {{\n    pub static ref {};\n}}",
                            stripped.trim()
                        )
                    } else {
                        let mut result = String::from("lazy_static::lazy_static! {\n    pub static ref ");
                        result.push_str(lines[0]);
                        result.push('\n');
                        let last_idx = lines.len() - 1;
                        for (i, line) in lines[1..].iter().enumerate() {
                            if line.is_empty() {
                                result.push('\n');
                            } else {
                                result.push_str("    ");
                                result.push_str(line);
                                // Add semicolon after the closing brace of the value block
                                if i == last_idx - 1 {
                                    result.push(';');
                                }
                                result.push('\n');
                            }
                        }
                        result.push_str("}");
                        result
                    }
                } else {
                    fallback_format_lazy_static(inner)
                }
            }
            Err(_) => fallback_format_lazy_static(inner),
        }
    } else {
        fallback_format_lazy_static(inner)
    }
}

/// Find the `=` that separates the declaration from the value.
/// Handles generic types like `Vec<i32>` by tracking angle bracket depth.
fn find_declaration_equals(s: &str) -> Option<usize> {
    let mut angle_depth = 0;
    let mut paren_depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' => angle_depth += 1,
            '>' => {
                if angle_depth > 0 {
                    angle_depth -= 1;
                }
            }
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            '=' if angle_depth == 0 && paren_depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Fallback: basic cleanup without rustfmt.
fn fallback_format_lazy_static(inner: &str) -> String {
    let cleaned = inner
        .replace("lazy_static :: lazy_static !", "lazy_static::lazy_static!")
        .replace(" :: ", "::")
        .replace(":: ", "::")
        .replace(" ::", "::")
        .replace(" < ", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace("> ", ">")
        .replace(" !", "!")
        .replace("vec! [", "vec![");
    format!(
        "lazy_static::lazy_static! {{\n    pub static ref {};\n}}",
        cleaned.trim().trim_end_matches(';')
    )
}

/// Reinsert formatted lazy_static blocks in place of their placeholders.
fn reinsert_lazy_static_blocks(code: String, blocks: Vec<String>) -> String {
    let mut result = code;
    for (i, block) in blocks.iter().enumerate() {
        let placeholder = format!("/* __LAZY_STATIC_{i}__ */");
        result = result.replace(&placeholder, block);
    }
    result
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
