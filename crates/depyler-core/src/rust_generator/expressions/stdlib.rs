//! Expression code generation - stdlib

#![allow(unused_imports)]

use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, ToRustExpr};
use crate::rust_generator::direct_rules::to_pascal_case;
use crate::rust_generator::return_type_expects_float;
use crate::rust_generator::type_gen::convert_binop;
use crate::optimizations::string_optimization::{StringContext, StringOptimizer};
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::{self, parse_quote};
use super::ExpressionConverter;

impl<'a, 'b> ExpressionConverter<'a, 'b> {
    /// Try to convert json module method calls
    #[inline]
    pub(super) fn try_convert_json_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need serde_json crate
        self.ctx.require(crate::rust_generator::context::Import::SerdeJson);

        let result = match method {
            // String serialization/deserialization
            "dumps" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("json.dumps() requires 1 or 2 arguments");
                }
                let obj = &arg_exprs[0];

                // json.dumps(result, indent=2) has 2 arguments after HIR conversion
                // (keyword args become positional args in HIR)
                if arg_exprs.len() >= 2 {
                    // json.dumps(obj, indent=n) → serde_json::to_string_pretty(&obj).unwrap()
                    parse_quote! { serde_json::to_string_pretty(&#obj).unwrap() }
                } else {
                    // json.dumps(obj) → serde_json::to_string(&obj).unwrap()
                    parse_quote! { serde_json::to_string(&#obj).unwrap() }
                }
            }

            "loads" => {
                if arg_exprs.len() != 1 {
                    bail!("json.loads() requires exactly 1 argument");
                }
                let s = &arg_exprs[0];
                // json.loads(s) → serde_json::from_str(&s).unwrap()
                // Returns serde_json::Value (dynamic JSON value)
                parse_quote! { serde_json::from_str::<serde_json::Value>(&#s).unwrap() }
            }

            // File-based serialization/deserialization
            "dump" => {
                if arg_exprs.len() != 2 {
                    bail!("json.dump() requires exactly 2 arguments (obj, file)");
                }
                let obj = &arg_exprs[0];
                let file = &arg_exprs[1];
                // json.dump(obj, file) → serde_json::to_writer(file, &obj).unwrap()
                parse_quote! { serde_json::to_writer(#file, &#obj).unwrap() }
            }

            "load" => {
                if arg_exprs.len() != 1 {
                    bail!("json.load() requires exactly 1 argument (file)");
                }
                let file = &arg_exprs[0];
                // json.load(file) → serde_json::from_reader(file).unwrap()
                parse_quote! { serde_json::from_reader::<_, serde_json::Value>(#file).unwrap() }
            }

            _ => {
                bail!("json.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert unicodedata module method calls
    ///
    /// Maps Python unicodedata functions to Rust unicode-normalization crate:
    /// - unicodedata.normalize("NFC", s) → s.nfc().collect::<String>()
    /// - unicodedata.normalize("NFD", s) → s.nfd().collect::<String>()
    /// - unicodedata.normalize("NFKC", s) → s.nfkc().collect::<String>()
    /// - unicodedata.normalize("NFKD", s) → s.nfkd().collect::<String>()
    #[inline]
    pub(super) fn try_convert_unicodedata_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need unicode-normalization crate
        self.ctx.require(crate::rust_generator::context::Import::UnicodeNormalization);

        let result = match method {
            "normalize" => {
                if arg_exprs.len() != 2 {
                    bail!("unicodedata.normalize() requires exactly 2 arguments (form, string)");
                }

                // Extract the normalization form (should be a string literal like "NFC", "NFD", etc.)
                let form_arg = &args[0];
                let string_arg = &arg_exprs[1];

                // Try to extract the string literal value for the form
                let form = match form_arg {
                    HirExpr::Literal(Literal::String(s)) => s.as_str(),
                    _ => bail!("unicodedata.normalize() first argument must be a string literal"),
                };

                // Map to the appropriate unicode-normalization method
                match form {
                    "NFC" => {
                        // unicodedata.normalize("NFC", s) → s.nfc().collect::<String>()
                        // The UnicodeNormalization trait is imported at the top of the file
                        parse_quote! { (#string_arg).nfc().collect::<String>() }
                    }
                    "NFD" => {
                        parse_quote! { (#string_arg).nfd().collect::<String>() }
                    }
                    "NFKC" => {
                        parse_quote! { (#string_arg).nfkc().collect::<String>() }
                    }
                    "NFKD" => {
                        parse_quote! { (#string_arg).nfkd().collect::<String>() }
                    }
                    _ => bail!(
                        "Unsupported normalization form: {}. Supported forms are NFC, NFD, NFKC, NFKD",
                        form
                    ),
                }
            }

            "category" | "name" | "lookup" => {
                // These functions don't have direct equivalents in unicode-normalization crate
                // They would need a different crate like unicode-width or a custom implementation
                bail!(
                    "unicodedata.{} is not yet supported - no direct Rust equivalent in unicode-normalization crate",
                    method
                )
            }

            _ => {
                bail!("unicodedata.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert re (regular expressions) module method calls
    ///
    ///
    /// Maps Python re module functions to Rust regex crate:
    /// - re.search() → Regex::new().find()
    /// - re.match() → Regex::new().is_match() with ^ anchor
    /// - re.findall() → Regex::new().find_iter()
    /// - re.sub() → Regex::new().replace_all()
    /// - re.split() → Regex::new().split()
    /// - re.compile() → Regex::new()
    /// - re.escape() → regex::escape()
    ///
    /// 10 (match with 10 branches)
    #[inline]
    pub(super) fn try_convert_re_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need regex crate
        self.ctx.require(crate::rust_generator::context::Import::Regex);

        let result = match method {
            // Pattern matching functions
            "search" => {
                if arg_exprs.len() < 2 {
                    bail!("re.search() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // Handle optional flags (simplified - just check for IGNORECASE)
                if arg_exprs.len() >= 3 {
                    // With flags: use RegexBuilder
                    parse_quote! {
                        regex::RegexBuilder::new(#pattern)
                            .case_insensitive(true)
                            .build()
                            .unwrap()
                            .find(#text)
                    }
                } else {
                    // No flags: direct Regex::new()
                    // re.search(pattern, text) → Regex::new(pattern).unwrap().find(text)
                    parse_quote! { regex::Regex::new(#pattern).unwrap().find(#text) }
                }
            }

            "match" => {
                if arg_exprs.len() < 2 {
                    bail!("re.match() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // Returns Option<Match> to support .group() calls
                // NOTE: Add start-of-string constraint in future (check match.start() == 0 or prepend ^) ()
                // For now, using .find() like search() - compatible with Match object usage
                parse_quote! { regex::Regex::new(#pattern).unwrap().find(#text) }
            }

            "fullmatch" => {
                if arg_exprs.len() < 2 {
                    bail!("re.fullmatch() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // re.fullmatch(pattern, text) → matches entire string
                // In Rust regex, we use is_match with anchors or capture the whole string
                parse_quote! {
                    regex::Regex::new(#pattern).unwrap().find(#text).filter(|m| m.start() == 0 && m.end() == #text.len())
                }
            }

            "findall" => {
                if arg_exprs.len() < 2 {
                    bail!("re.findall() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // re.findall(pattern, text) → Regex::new(pattern).unwrap().find_iter(text).map(|m| m.as_str()).collect::<Vec<_>>()
                parse_quote! {
                    regex::Regex::new(#pattern)
                        .unwrap()
                        .find_iter(#text)
                        .map(|m| m.as_str().to_string())
                        .collect::<Vec<_>>()
                }
            }

            "finditer" => {
                if arg_exprs.len() < 2 {
                    bail!("re.finditer() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // re.finditer(pattern, text) → Regex::new(pattern).unwrap().find_iter(text)
                parse_quote! {
                    regex::Regex::new(#pattern)
                        .unwrap()
                        .find_iter(#text)
                        .map(|m| m.as_str().to_string())
                        .collect::<Vec<_>>()
                }
            }

            // String substitution
            "sub" => {
                if arg_exprs.len() < 3 {
                    bail!("re.sub() requires at least 3 arguments (pattern, repl, string)");
                }
                let pattern = &arg_exprs[0];
                let repl = &arg_exprs[1];
                let text = &arg_exprs[2];

                // re.sub(pattern, repl, text) → Regex::new(pattern).unwrap().replace_all(text, repl)
                parse_quote! {
                    regex::Regex::new(#pattern)
                        .unwrap()
                        .replace_all(#text, #repl)
                        .to_string()
                }
            }

            "subn" => {
                if arg_exprs.len() < 3 {
                    bail!("re.subn() requires at least 3 arguments (pattern, repl, string)");
                }
                let pattern = &arg_exprs[0];
                let repl = &arg_exprs[1];
                let text = &arg_exprs[2];

                // re.subn(pattern, repl, text) → returns (result, count)
                parse_quote! {
                    {
                        let re = regex::Regex::new(#pattern).unwrap();
                        let count = re.find_iter(#text).count();
                        let result = re.replace_all(#text, #repl).to_string();
                        (result, count)
                    }
                }
            }

            // Pattern compilation
            "compile" => {
                if arg_exprs.is_empty() {
                    bail!("re.compile() requires at least 1 argument (pattern)");
                }
                let pattern = &arg_exprs[0];

                // Check for flags
                if arg_exprs.len() >= 2 {
                    // With flags: use RegexBuilder
                    // For now, simplified handling of common flags
                    parse_quote! {
                        regex::RegexBuilder::new(#pattern)
                            .case_insensitive(true)
                            .build()
                            .unwrap()
                    }
                } else {
                    // No flags: direct Regex::new()
                    // re.compile(pattern) → Regex::new(pattern).unwrap()
                    parse_quote! { regex::Regex::new(#pattern).unwrap() }
                }
            }

            // String splitting
            "split" => {
                if arg_exprs.len() < 2 {
                    bail!("re.split() requires at least 2 arguments (pattern, string)");
                }
                let pattern = &arg_exprs[0];
                let text = &arg_exprs[1];

                // Check for maxsplit argument
                if arg_exprs.len() >= 3 {
                    let maxsplit = &arg_exprs[2];
                    // re.split(pattern, text, maxsplit) → Regex::new(pattern).unwrap().splitn(maxsplit + 1, text)
                    parse_quote! {
                        regex::Regex::new(#pattern)
                            .unwrap()
                            .splitn(#text, #maxsplit + 1)
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    }
                } else {
                    // re.split(pattern, text) → Regex::new(pattern).unwrap().split(text)
                    parse_quote! {
                        regex::Regex::new(#pattern)
                            .unwrap()
                            .split(#text)
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    }
                }
            }

            // Escaping special characters
            "escape" => {
                if arg_exprs.len() != 1 {
                    bail!("re.escape() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // re.escape(text) → regex::escape(text)
                parse_quote! { regex::escape(#text).to_string() }
            }

            _ => {
                bail!("re.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert string module method calls
    ///
    ///
    /// Maps Python string module functions to Rust equivalents:
    /// - string.capwords() → split/capitalize/join
    /// - string.Template → String formatting
    ///
    /// 2 (match with 2 branches)
    #[inline]
    pub(super) fn try_convert_string_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // String utilities
            "capwords" => {
                if arg_exprs.is_empty() {
                    bail!("string.capwords() requires at least 1 argument (text)");
                }
                let text = &arg_exprs[0];

                // string.capwords(text) → text.split_whitespace().map(|w| {
                //     let mut c = w.chars();
                //     match c.next() {
                //         None => String::new(),
                //         Some(f) => f.to_uppercase().collect::<String>() + c.as_str()
                //     }
                // }).collect::<Vec<_>>().join(" ")
                parse_quote! {
                    #text.split_whitespace()
                        .map(|w| {
                            let mut chars = w.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    let mut result = first.to_uppercase().collect::<String>();
                                    result.push_str(&chars.as_str().to_lowercase());
                                    result
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            }

            _ => {
                bail!("string.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert time module method calls
    ///
    ///
    /// Maps Python time module functions to Rust equivalents:
    /// - time.time() → SystemTime::now()
    /// - time.sleep() → thread::sleep()
    /// - time.monotonic() → Instant::now()
    ///
    /// 7 (match with 7+ branches)
    #[inline]
    pub(super) fn try_convert_time_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Basic time measurement
            "time" => {
                // time.time() → SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64()
                parse_quote! {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                }
            }

            "monotonic" | "perf_counter" => {
                // time.monotonic() → Instant::now() (returns Instant, need elapsed)
                // For now, simplified: just generate the call
                // In real usage, user would call .elapsed() later
                parse_quote! { std::time::Instant::now() }
            }

            "process_time" => {
                // time.process_time() → CPU time (requires platform-specific code)
                // Simplified: use Instant as approximation
                parse_quote! { std::time::Instant::now() }
            }

            "thread_time" => {
                // time.thread_time() → thread-specific time
                // Simplified: use Instant
                parse_quote! { std::time::Instant::now() }
            }

            // Sleep function
            "sleep" => {
                if arg_exprs.len() != 1 {
                    bail!("time.sleep() requires exactly 1 argument (seconds)");
                }
                let seconds = &arg_exprs[0];

                // time.sleep(seconds) → thread::sleep(Duration::from_secs_f64(seconds))
                parse_quote! {
                    std::thread::sleep(std::time::Duration::from_secs_f64(#seconds))
                }
            }

            // Time formatting (requires chrono for full support)
            "ctime" => {
                self.ctx.require(crate::rust_generator::context::Import::Chrono);
                if arg_exprs.len() != 1 {
                    bail!("time.ctime() requires exactly 1 argument (timestamp)");
                }
                let timestamp = &arg_exprs[0];

                // time.ctime(timestamp) → chrono formatting
                // Simplified: convert timestamp to DateTime
                parse_quote! {
                    {
                        let secs = #timestamp as i64;
                        let nanos = ((#timestamp - secs as f64) * 1_000_000_000.0) as u32;
                        chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nanos)
                            .unwrap()
                            .to_string()
                    }
                }
            }

            "strftime" => {
                self.ctx.require(crate::rust_generator::context::Import::Chrono);
                if arg_exprs.len() < 2 {
                    bail!("time.strftime() requires at least 2 arguments (format, time_tuple)");
                }
                let format = &arg_exprs[0];
                let _time_tuple = &arg_exprs[1];

                // time.strftime(format, time_tuple) → chrono formatting
                // Simplified: assume current time for now
                parse_quote! {
                    chrono::Local::now().format(#format).to_string()
                }
            }

            "strptime" => {
                self.ctx.require(crate::rust_generator::context::Import::Chrono);
                if arg_exprs.len() < 2 {
                    bail!("time.strptime() requires at least 2 arguments (string, format)");
                }
                let time_str = &arg_exprs[0];
                let format = &arg_exprs[1];

                // time.strptime(string, format) → chrono parsing
                parse_quote! {
                    chrono::NaiveDateTime::parse_from_str(#time_str, #format).unwrap()
                }
            }

            // Time conversion
            "gmtime" => {
                self.ctx.require(crate::rust_generator::context::Import::Chrono);
                let timestamp = if arg_exprs.is_empty() {
                    parse_quote! { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64() }
                } else {
                    arg_exprs[0].clone()
                };

                // time.gmtime(timestamp) → chrono UTC conversion
                parse_quote! {
                    {
                        let secs = #timestamp as i64;
                        let nanos = ((#timestamp - secs as f64) * 1_000_000_000.0) as u32;
                        chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nanos).unwrap()
                    }
                }
            }

            "localtime" => {
                self.ctx.require(crate::rust_generator::context::Import::Chrono);
                let timestamp = if arg_exprs.is_empty() {
                    parse_quote! { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64() }
                } else {
                    arg_exprs[0].clone()
                };

                // time.localtime(timestamp) → chrono Local conversion
                parse_quote! {
                    {
                        let secs = #timestamp as i64;
                        let nanos = ((#timestamp - secs as f64) * 1_000_000_000.0) as u32;
                        chrono::DateTime::<chrono::Local>::from_timestamp(secs, nanos).unwrap()
                    }
                }
            }

            "mktime" => {
                self.ctx.require(crate::rust_generator::context::Import::Chrono);
                if arg_exprs.len() != 1 {
                    bail!("time.mktime() requires exactly 1 argument (time_tuple)");
                }
                let time_tuple = &arg_exprs[0];

                // time.mktime(time_tuple) → timestamp conversion
                // Simplified: assume time_tuple is a chrono DateTime
                parse_quote! { #time_tuple.timestamp() as f64 }
            }

            "asctime" => {
                self.ctx.require(crate::rust_generator::context::Import::Chrono);
                if arg_exprs.len() != 1 {
                    bail!("time.asctime() requires exactly 1 argument (time_tuple)");
                }
                let time_tuple = &arg_exprs[0];

                // time.asctime(time_tuple) → ASCII time string
                parse_quote! { #time_tuple.to_string() }
            }

            _ => {
                bail!("time.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert csv module method calls
    ///
    ///
    /// Maps Python csv module to Rust csv crate:
    /// - csv.reader() → csv::Reader::from_reader()
    /// - csv.writer() → csv::Writer::from_writer()
    /// - csv.DictReader → csv with headers
    /// - csv.DictWriter → csv with headers
    ///
    /// 4 (match with 4 branches - simplified for core operations)
    #[inline]
    pub(super) fn try_convert_csv_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need csv crate
        self.ctx.require(crate::rust_generator::context::Import::Csv);

        let result = match method {
            // CSV Reader
            "reader" => {
                if arg_exprs.is_empty() {
                    bail!("csv.reader() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // csv.reader(file) → csv::Reader::from_reader(file)
                // Note: Real implementation needs more context for delimiter, etc.
                parse_quote! { csv::Reader::from_reader(#file) }
            }

            // CSV Writer
            "writer" => {
                if arg_exprs.is_empty() {
                    bail!("csv.writer() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // csv.writer(file) → csv::Writer::from_writer(file)
                parse_quote! { csv::Writer::from_writer(#file) }
            }

            // DictReader (simplified - actual implementation more complex)
            "DictReader" => {
                if arg_exprs.is_empty() {
                    bail!("csv.DictReader() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // csv.DictReader(file) → csv::ReaderBuilder::new().has_headers(true).from_reader(file)
                parse_quote! {
                    csv::ReaderBuilder::new()
                        .has_headers(true)
                        .from_reader(#file)
                }
            }

            // DictWriter (simplified)
            // csv.DictWriter(file, fieldnames=[...]) or csv.DictWriter(file, fieldnames=...)
            "DictWriter" => {
                // Get file argument (first positional arg required)
                if arg_exprs.is_empty() {
                    bail!("csv.DictWriter() requires at least 1 argument (file)");
                }
                let file = &arg_exprs[0];

                // Get fieldnames from either positional arg or kwargs
                let _fieldnames = if arg_exprs.len() >= 2 {
                    // Positional: csv.DictWriter(file, ['col1', 'col2'])
                    Some(&arg_exprs[1])
                } else {
                    // Keyword: csv.DictWriter(file, fieldnames=['col1', 'col2'])
                    kwargs
                        .iter()
                        .find(|(key, _)| key == "fieldnames")
                        .map(|(_, value)| value.to_rust_expr(self.ctx))
                        .transpose()?
                        .as_ref()
                        .map(|_| &arg_exprs[0]) // Placeholder, we don't use fieldnames yet
                };

                if _fieldnames.is_none() {
                    bail!("csv.DictWriter() requires fieldnames argument (positional or keyword)");
                }

                // csv.DictWriter(file, fieldnames) → csv::Writer::from_writer(file)
                // Note: fieldnames handling requires more context
                parse_quote! { csv::Writer::from_writer(#file) }
            }

            _ => {
                bail!("csv.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert os module method calls
    ///
    /// Maps Python os module to Rust std::env:
    /// - os.getenv(key) → std::env::var(key)?
    /// - os.getenv(key, default) → std::env::var(key).unwrap_or_else(|_| default.to_string())
    #[inline]
    pub(super) fn try_convert_os_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "getenv" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("os.getenv() requires 1 or 2 arguments");
                }

                if arg_exprs.len() == 1 {
                    // os.getenv("KEY") → std::env::var("KEY")?
                    let key = &arg_exprs[0];
                    parse_quote! { std::env::var(#key)? }
                } else {
                    // os.getenv("KEY", "default") → std::env::var("KEY").unwrap_or_else(|_| "default".to_string())
                    let key = &arg_exprs[0];
                    let default = &arg_exprs[1];

                    // Python's os.getenv() always returns str, so the default must be converted to String.
                    // The unwrap_or_else closure must return String, so we always add .to_string()
                    // to ensure the default value is owned.
                    parse_quote! {
                        std::env::var(#key).unwrap_or_else(|_| #default.to_string())
                    }
                }
            }
            _ => {
                return Ok(None);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert os.environ method calls
    ///
    /// Maps Python os.environ methods to Rust std::env:
    /// - os.environ.get(key) → std::env::var(key).ok()
    /// - os.environ.get(key, default) → std::env::var(key).unwrap_or_else(|_| default.to_string())
    #[inline]
    pub(super) fn try_convert_os_environ_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "get" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("os.environ.get() requires 1 or 2 arguments");
                }

                if arg_exprs.len() == 1 {
                    // os.environ.get("KEY") → std::env::var("KEY").ok()
                    // Returns Option<String>: Some(value) if exists, None otherwise
                    let key = &arg_exprs[0];
                    parse_quote! { std::env::var(#key).ok() }
                } else {
                    // os.environ.get("KEY", "default") → std::env::var("KEY").unwrap_or_else(|_| "default".to_string())
                    // Returns String: value if exists, default otherwise
                    let key = &arg_exprs[0];
                    let default = &arg_exprs[1];
                    parse_quote! {
                        std::env::var(#key).unwrap_or_else(|_| #default.to_string())
                    }
                }
            }
            _ => {
                return Ok(None);
            }
        };

        Ok(Some(result))
    }

    /// Convert subprocess.run() to std::process::Command
    ///
    /// Maps Python subprocess.run() to Rust std::process::Command:
    /// - subprocess.run(cmd) → Command::new(cmd[0]).args(&cmd[1..]).status()
    /// - capture_output=True → .output() instead of .status()
    /// - cwd=path → .current_dir(path)
    /// - check=True → verify exit status (NOTE: add error handling )
    ///
    /// Returns anonymous struct with: returncode, stdout, stderr
    #[inline]
    pub(super) fn convert_subprocess_run(
        &mut self,
        args: &[HirExpr],
        kwargs: &[(Symbol, HirExpr)],
    ) -> Result<syn::Expr> {
        if args.is_empty() {
            bail!("subprocess.run() requires at least 1 argument (command list)");
        }

        // First argument is the command list
        let cmd_expr = args[0].to_rust_expr(self.ctx)?;

        // Parse keyword arguments
        let mut capture_output = false;
        let mut _text = false;
        let mut cwd_expr: Option<syn::Expr> = None;
        let mut _check = false;

        for (key, value) in kwargs {
            match key.as_str() {
                "capture_output" => {
                    if let HirExpr::Literal(Literal::Bool(b)) = value {
                        capture_output = *b;
                    }
                }
                "text" => {
                    if let HirExpr::Literal(Literal::Bool(b)) = value {
                        _text = *b;
                    }
                }
                "cwd" => {
                    cwd_expr = Some(value.to_rust_expr(self.ctx)?);
                }
                "check" => {
                    if let HirExpr::Literal(Literal::Bool(b)) = value {
                        _check = *b;
                    }
                }
                _ => {} // Ignore unknown kwargs for now
            }
        }

        // Build the Command construction
        // Python: subprocess.run(["echo", "hello"], capture_output=True, cwd="/tmp")
        // Rust: {
        //   let mut cmd = std::process::Command::new(&cmd_list[0]);
        //   cmd.args(&cmd_list[1..]);
        //   if cwd { cmd.current_dir(cwd); }
        //   let output = cmd.output()?;
        //   // Create result struct
        //   SubprocessResult {
        //     returncode: output.status.code().unwrap_or(-1),
        //     stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        //     stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        //   }
        // }

        let result = if capture_output {
            // Use .output() to capture stdout/stderr
            if let Some(cwd) = cwd_expr {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        cmd.current_dir(#cwd);
                        let output = cmd.output().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: output.status.code().unwrap_or(-1),
                            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        }
                    }
                }
            } else {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        let output = cmd.output().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: output.status.code().unwrap_or(-1),
                            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        }
                    }
                }
            }
        } else {
            // Use .status() for exit code only (no capture)
            if let Some(cwd) = cwd_expr {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        cmd.current_dir(#cwd);
                        let status = cmd.status().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: status.code().unwrap_or(-1),
                            stdout: String::new(),
                            stderr: String::new(),
                        }
                    }
                }
            } else {
                parse_quote! {
                    {
                        let cmd_list = #cmd_expr;
                        let mut cmd = std::process::Command::new(&cmd_list[0]);
                        cmd.args(&cmd_list[1..]);
                        let status = cmd.status().expect("subprocess.run() failed");
                        struct SubprocessResult {
                            returncode: i32,
                            stdout: String,
                            stderr: String,
                        }
                        SubprocessResult {
                            returncode: status.code().unwrap_or(-1),
                            stdout: String::new(),
                            stderr: String::new(),
                        }
                    }
                }
            }
        };

        Ok(result)
    }

    /// Try to convert os.path module method calls
    ///
    ///
    /// Maps Python os.path module to Rust std::path + std::fs:
    /// - os.path.join() → PathBuf::new().join()
    /// - os.path.basename() → Path::file_name()
    /// - os.path.exists() → Path::exists()
    ///
    /// 10 (match with 10 primary branches - split into helper methods as needed)
    #[inline]
    pub(super) fn try_convert_os_path_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Path construction
            "join" => {
                if arg_exprs.is_empty() {
                    bail!("os.path.join() requires at least 1 argument");
                }

                // os.path.join(a, b, c, ...) → PathBuf::from(a).join(b).join(c)...
                let first = &arg_exprs[0];
                if arg_exprs.len() == 1 {
                    parse_quote! { std::path::PathBuf::from(#first) }
                } else {
                    let mut result: syn::Expr = parse_quote! { std::path::PathBuf::from(#first) };
                    for part in &arg_exprs[1..] {
                        result = parse_quote! { #result.join(#part) };
                    }
                    parse_quote! { #result.to_string_lossy().to_string() }
                }
            }

            // Path decomposition
            "basename" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.basename() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.basename(path) → Path::new(path).file_name()
                parse_quote! {
                    std::path::Path::new(#path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                }
            }

            "dirname" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.dirname() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.dirname(path) → Path::new(path).parent()
                parse_quote! {
                    std::path::Path::new(#path)
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                        .to_string()
                }
            }

            "split" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.split() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.split(path) → (dirname, basename) tuple
                parse_quote! {
                    {
                        let p = std::path::Path::new(#path);
                        let dirname = p.parent().and_then(|p| p.to_str()).unwrap_or("").to_string();
                        let basename = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                        (dirname, basename)
                    }
                }
            }

            "splitext" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.splitext() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.splitext(path) → (stem, extension) tuple
                parse_quote! {
                    {
                        let p = std::path::Path::new(#path);
                        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                        let ext = p.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
                        (stem, ext)
                    }
                }
            }

            // Path predicates
            "exists" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.exists() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.exists(path) → Path::new(path).exists()
                parse_quote! { std::path::Path::new(#path).exists() }
            }

            "isfile" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.isfile() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.isfile(path) → Path::new(path).is_file()
                parse_quote! { std::path::Path::new(#path).is_file() }
            }

            "isdir" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.isdir() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.isdir(path) → Path::new(path).is_dir()
                parse_quote! { std::path::Path::new(#path).is_dir() }
            }

            "isabs" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.isabs() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.isabs(path) → Path::new(path).is_absolute()
                parse_quote! { std::path::Path::new(#path).is_absolute() }
            }

            // Path normalization
            "abspath" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.abspath() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.abspath(path) → std::fs::canonicalize() or manual absolute path
                // Using canonicalize (resolves symlinks too, like realpath)
                parse_quote! {
                    std::fs::canonicalize(#path)
                        .unwrap_or_else(|_| std::path::PathBuf::from(#path))
                        .to_string_lossy()
                        .to_string()
                }
            }

            "normpath" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.normpath() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.normpath(path) → normalize path components
                // Rust Path doesn't have direct normpath, but we can use PathBuf operations
                parse_quote! {
                    {
                        let p = std::path::Path::new(#path);
                        let mut components = Vec::new();
                        for component in p.components() {
                            match component {
                                std::path::Component::CurDir => {},
                                std::path::Component::ParentDir => {
                                    components.pop();
                                }
                                _ => components.push(component),
                            }
                        }
                        components.iter()
                            .map(|c| c.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(std::path::MAIN_SEPARATOR_STR)
                    }
                }
            }

            "realpath" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.realpath() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.realpath(path) → std::fs::canonicalize()
                parse_quote! {
                    std::fs::canonicalize(#path)
                        .unwrap_or_else(|_| std::path::PathBuf::from(#path))
                        .to_string_lossy()
                        .to_string()
                }
            }

            // Path properties
            "getsize" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.getsize() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.getsize(path) → std::fs::metadata().len()
                parse_quote! {
                    std::fs::metadata(#path).unwrap().len() as i64
                }
            }

            "getmtime" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.getmtime() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.getmtime(path) → std::fs::metadata().modified()
                parse_quote! {
                    std::fs::metadata(#path)
                        .unwrap()
                        .modified()
                        .unwrap()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                }
            }

            "getctime" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.getctime() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.getctime(path) → std::fs::metadata().created()
                // Note: On Unix, this is ctime (change time), but Rust only has created()
                parse_quote! {
                    std::fs::metadata(#path)
                        .unwrap()
                        .created()
                        .unwrap()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                }
            }

            // Path expansion
            "expanduser" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.expanduser() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.expanduser(path) → expand ~ to home directory
                parse_quote! {
                    {
                        let p = #path;
                        if p.starts_with("~") {
                            if let Some(home) = std::env::var_os("HOME") {
                                format!("{}{}", home.to_string_lossy(), &p[1..])
                            } else {
                                p.to_string()
                            }
                        } else {
                            p.to_string()
                        }
                    }
                }
            }

            "expandvars" => {
                if arg_exprs.len() != 1 {
                    bail!("os.path.expandvars() requires exactly 1 argument");
                }
                let path = &arg_exprs[0];

                // os.path.expandvars(path) → expand environment variables
                // Simplified: just return path as-is for now (full implementation complex)
                parse_quote! { #path.to_string() }
            }

            //
            "relpath" => {
                if arg_exprs.len() != 2 {
                    bail!("os.path.relpath() requires exactly 2 arguments");
                }
                let path = &arg_exprs[0];
                let start = &arg_exprs[1];

                // os.path.relpath(path, start) → compute relative path from start to path
                parse_quote! {
                    {
                        let path_obj = std::path::Path::new(#path);
                        let start_obj = std::path::Path::new(#start);
                        path_obj
                            .strip_prefix(start_obj)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| #path.to_string())
                    }
                }
            }

            _ => {
                // For functions not yet implemented, return None to allow fallback
                return Ok(None);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert base64 module method calls
    ///
    ///
    /// Maps Python base64 module to Rust base64 crate:
    /// - base64.b64encode() → base64::encode()
    /// - base64.b64decode() → base64::decode()
    /// - base64.urlsafe_b64encode() → URL-safe encoding
    ///
    /// 10 (match with 10 branches for different encodings)
    #[inline]
    pub(super) fn try_convert_base64_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need base64 crate
        self.ctx.require(crate::rust_generator::context::Import::Base64);

        let result = match method {
            // Standard Base64
            "b64encode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b64encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b64encode(data) → base64::engine::general_purpose::STANDARD.encode(data)
                parse_quote! {
                    base64::engine::general_purpose::STANDARD.encode(#data)
                }
            }

            "b64decode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b64decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b64decode(data) → base64::engine::general_purpose::STANDARD.decode(data).unwrap()
                parse_quote! {
                    base64::engine::general_purpose::STANDARD.decode(#data).unwrap()
                }
            }

            // URL-safe Base64
            "urlsafe_b64encode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.urlsafe_b64encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.urlsafe_b64encode(data) → base64::engine::general_purpose::URL_SAFE.encode(data)
                parse_quote! {
                    base64::engine::general_purpose::URL_SAFE.encode(#data)
                }
            }

            "urlsafe_b64decode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.urlsafe_b64decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.urlsafe_b64decode(data) → base64::engine::general_purpose::URL_SAFE.decode(data).unwrap()
                parse_quote! {
                    base64::engine::general_purpose::URL_SAFE.decode(#data).unwrap()
                }
            }

            // Base32 using data-encoding crate
            "b32encode" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 1 {
                    bail!("base64.b32encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b32encode(data) → data_encoding::BASE32.encode(data).into_bytes()
                parse_quote! {
                    data_encoding::BASE32.encode(#data).into_bytes()
                }
            }

            "b32decode" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 1 {
                    bail!("base64.b32decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b32decode(data) → data_encoding::BASE32.decode(data).unwrap()
                parse_quote! {
                    data_encoding::BASE32.decode(#data).unwrap()
                }
            }

            // Base16 (Hex)
            "b16encode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b16encode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b16encode(data) → hex::encode_upper(data)
                parse_quote! {
                    hex::encode_upper(#data)
                }
            }

            "b16decode" => {
                if arg_exprs.len() != 1 {
                    bail!("base64.b16decode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // base64.b16decode(data) → hex::decode(data).unwrap()
                parse_quote! {
                    hex::decode(#data).unwrap()
                }
            }

            // Base85 (also needs additional crate)
            "b85encode" | "b85decode" => {
                // Simplified: note that full implementation needs additional crate
                bail!(
                    "base64.{} requires base85 encoding crate (not yet integrated)",
                    method
                );
            }

            _ => {
                bail!("base64.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert secrets module method calls
    ///
    ///
    /// Maps Python secrets module to Rust rand crate (cryptographic RNG):
    /// - secrets.randbelow() → rand::thread_rng().gen_range()
    /// - secrets.token_bytes() → Cryptographically secure random bytes
    ///
    /// 5 (match with 5 branches)
    #[inline]
    pub(super) fn try_convert_secrets_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need rand crate (ThreadRng is cryptographically secure)
        self.ctx.require(crate::rust_generator::context::Import::Rand);
        self.ctx.require(crate::rust_generator::context::Import::Base64); // For token_urlsafe

        let result = match method {
            // Random number generation
            "randbelow" => {
                if arg_exprs.len() != 1 {
                    bail!("secrets.randbelow() requires exactly 1 argument");
                }
                let n = &arg_exprs[0];

                // secrets.randbelow(n) → rand::thread_rng().gen_range(0..n)
                parse_quote! { rand::thread_rng().gen_range(0..#n) }
            }

            "choice" => {
                if arg_exprs.len() != 1 {
                    bail!("secrets.choice() requires exactly 1 argument");
                }
                let seq = &arg_exprs[0];
                self.ctx.require(crate::rust_generator::context::Import::IndexedRandom);

                // secrets.choice(seq) → seq.choose(&mut rand::thread_rng()).cloned().unwrap()
                parse_quote! { #seq.choose(&mut rand::thread_rng()).cloned().unwrap() }
            }

            // Token generation
            "token_bytes" => {
                let nbytes = if arg_exprs.is_empty() {
                    parse_quote! { 32 } // Default 32 bytes
                } else {
                    arg_exprs[0].clone()
                };

                // secrets.token_bytes(n) → generate n random bytes
                parse_quote! {
                    {
                        let mut bytes = vec![0u8; #nbytes];
                        rand::thread_rng().fill(&mut bytes[..]);
                        bytes
                    }
                }
            }

            "token_hex" => {
                let nbytes = if arg_exprs.is_empty() {
                    parse_quote! { 32 }
                } else {
                    arg_exprs[0].clone()
                };

                // secrets.token_hex(n) → generate n random bytes and encode as hex
                parse_quote! {
                    {
                        let mut bytes = vec![0u8; #nbytes];
                        rand::thread_rng().fill(&mut bytes[..]);
                        hex::encode(&bytes)
                    }
                }
            }

            "token_urlsafe" => {
                let nbytes = if arg_exprs.is_empty() {
                    parse_quote! { 32 }
                } else {
                    arg_exprs[0].clone()
                };

                // secrets.token_urlsafe(n) → generate n random bytes and encode as URL-safe base64
                parse_quote! {
                    {
                        let mut bytes = vec![0u8; #nbytes];
                        rand::thread_rng().fill(&mut bytes[..]);
                        base64::engine::general_purpose::URL_SAFE.encode(&bytes)
                    }
                }
            }

            _ => {
                bail!("secrets.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert hashlib module method calls
    ///
    ///
    /// Supports: md5, sha1, sha224, sha256, sha384, sha512, sha3_256, blake2b, blake2s
    /// Returns hex digest directly (one-shot hashing pattern)
    #[inline]
    pub(super) fn try_convert_hashlib_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // All hash functions need hex encoding
        self.ctx.require(crate::rust_generator::context::Import::Hex);

        let result = match method {
            // MD5 hash
            "md5" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.md5() accepts 0 or 1 arguments");
                }
                self.ctx.require(crate::rust_generator::context::Import::Md5);

                // hashlib.md5(data) → hex::encode(md5::compute(data))
                // If no arguments, use empty bytes (Python default: data=b'')
                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use md5::Digest;
                            let mut hasher = md5::Md5::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use md5::Digest;
                            let mut hasher = md5::Md5::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // SHA-1 hash
            "sha1" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.sha1() requires exactly 1 argument");
                }
                self.ctx.require(crate::rust_generator::context::Import::Sha2);
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use sha1::Digest;
                        let mut hasher = sha1::Sha1::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // SHA-224 hash
            "sha224" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.sha224() requires exactly 1 argument");
                }
                self.ctx.require(crate::rust_generator::context::Import::Sha2);
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use sha2::Digest;
                        let mut hasher = sha2::Sha224::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // SHA-256 hash
            "sha256" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.sha256() accepts 0 or 1 arguments");
                }
                self.ctx.require(crate::rust_generator::context::Import::Sha2);

                // If no arguments, use empty bytes (Python default: data=b'')
                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha256::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha256::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // SHA-384 hash
            "sha384" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.sha384() accepts 0 or 1 arguments");
                }
                self.ctx.require(crate::rust_generator::context::Import::Sha2);

                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha384::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha384::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // SHA-512 hash
            "sha512" => {
                if arg_exprs.len() > 1 {
                    bail!("hashlib.sha512() accepts 0 or 1 arguments");
                }
                self.ctx.require(crate::rust_generator::context::Import::Sha2);

                if arg_exprs.is_empty() {
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha512::new();
                            hex::encode(hasher.finalize())
                        }
                    }
                } else {
                    let data = &arg_exprs[0];
                    parse_quote! {
                        {
                            use sha2::Digest;
                            let mut hasher = sha2::Sha512::new();
                            hasher.update(#data);
                            hex::encode(hasher.finalize())
                        }
                    }
                }
            }

            // BLAKE2b hash
            "blake2b" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.blake2b() requires exactly 1 argument");
                }
                self.ctx.require(crate::rust_generator::context::Import::Blake2);
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use blake2::Digest;
                        let mut hasher = blake2::Blake2b512::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // BLAKE2s hash
            "blake2s" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.blake2s() requires exactly 1 argument");
                }
                self.ctx.require(crate::rust_generator::context::Import::Blake2);
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use blake2::Digest;
                        let mut hasher = blake2::Blake2s256::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            // SHA3-256 hash
            "sha3_256" => {
                if arg_exprs.len() != 1 {
                    bail!("hashlib.sha3_256() requires exactly 1 argument");
                }
                self.ctx.require(crate::rust_generator::context::Import::Sha3);
                let data = &arg_exprs[0];

                parse_quote! {
                    {
                        use sha3::Digest;
                        let mut hasher = sha3::Sha3_256::new();
                        hasher.update(#data);
                        hex::encode(hasher.finalize())
                    }
                }
            }

            _ => {
                bail!(
                    "hashlib.{} not implemented yet (try: md5, sha1, sha224, sha256, sha384, sha512, sha3_256, blake2b, blake2s)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert uuid module method calls
    ///
    ///
    /// Supports: uuid1 (time-based), uuid3 (MD5), uuid4 (random), uuid5 (SHA1)
    /// Returns string representation of UUID
    #[inline]
    pub(super) fn try_convert_uuid_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need uuid crate
        self.ctx.require(crate::rust_generator::context::Import::Uuid);

        let result = match method {
            // UUID v1 - time-based
            "uuid1" => {
                if !arg_exprs.is_empty() {
                    bail!("uuid.uuid1() takes no arguments (node/clock_seq not yet supported)");
                }

                // uuid.uuid1() → Uuid::new_v1(...).to_string()
                // Note: Requires context (timestamp + node ID)
                parse_quote! {
                    {
                        use uuid::Uuid;
                        // Generate time-based UUID v1
                        // Note: Using placeholder implementation (actual v1 needs timestamp context)
                        Uuid::new_v4().to_string()  // NOTE: Implement proper UUID v1 with timestamp ()
                    }
                }
            }

            // UUID v3 - MD5 hash-based
            "uuid3" => {
                if arg_exprs.len() != 2 {
                    bail!("uuid.uuid3() requires exactly 2 arguments (namespace, name)");
                }
                let namespace = &arg_exprs[0];
                let name = &arg_exprs[1];

                // uuid.uuid3(namespace, name) → Uuid::new_v3(&namespace, name.as_bytes()).to_string()
                parse_quote! {
                    {
                        use uuid::Uuid;
                        Uuid::new_v3(&#namespace, #name.as_bytes()).to_string()
                    }
                }
            }

            // UUID v4 - random (most common)
            "uuid4" => {
                if !arg_exprs.is_empty() {
                    bail!("uuid.uuid4() takes no arguments");
                }

                // uuid.uuid4() → Uuid::new_v4().to_string()
                parse_quote! {
                    {
                        use uuid::Uuid;
                        Uuid::new_v4().to_string()
                    }
                }
            }

            // UUID v5 - SHA1 hash-based
            "uuid5" => {
                if arg_exprs.len() != 2 {
                    bail!("uuid.uuid5() requires exactly 2 arguments (namespace, name)");
                }
                let namespace = &arg_exprs[0];
                let name = &arg_exprs[1];

                // uuid.uuid5(namespace, name) → Uuid::new_v5(&namespace, name.as_bytes()).to_string()
                parse_quote! {
                    {
                        use uuid::Uuid;
                        Uuid::new_v5(&#namespace, #name.as_bytes()).to_string()
                    }
                }
            }

            _ => {
                bail!(
                    "uuid.{} not implemented yet (try: uuid1, uuid3, uuid4, uuid5)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert hmac module method calls
    ///
    ///
    /// Supports: new() with SHA256, compare_digest()
    /// Returns hex digest for one-shot HMAC
    #[inline]
    pub(super) fn try_convert_hmac_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need hmac and related crates
        self.ctx.require(crate::rust_generator::context::Import::Hmac);
        self.ctx.require(crate::rust_generator::context::Import::Sha2); // For SHA256
        self.ctx.require(crate::rust_generator::context::Import::Hex);

        let result = match method {
            // HMAC creation - simplified to SHA256
            "new" => {
                if arg_exprs.is_empty() {
                    bail!("hmac.new() requires at least 1 argument (key)");
                }
                let key = &arg_exprs[0];

                // Check if we have a message (2nd positional arg) or just key + digestmod
                // hmac.new(key, msg, digestmod) or hmac.new(key, digestmod=hashlib.sha256)
                if arg_exprs.len() >= 2 {
                    let msg = &arg_exprs[1];
                    // hmac.new(key, msg, hashlib.sha256) → HMAC-SHA256 hex digest
                    parse_quote! {
                        {
                            use hmac::{Hmac, Mac};
                            use sha2::Sha256;

                            type HmacSha256 = Hmac<Sha256>;
                            let mut mac = HmacSha256::new_from_slice(#key).expect("HMAC key error");
                            mac.update(#msg);
                            hex::encode(mac.finalize().into_bytes())
                        }
                    }
                } else {
                    // hmac.new(key, digestmod=...) - returns HMAC object for incremental updates
                    // We'll return a tuple of (mac_type, key) that can be used with .update()
                    parse_quote! {
                        {
                            use hmac::{Hmac, Mac};
                            use sha2::Sha256;

                            type HmacSha256 = Hmac<Sha256>;
                            HmacSha256::new_from_slice(#key).expect("HMAC key error")
                        }
                    }
                }
            }

            // Timing-safe comparison
            "compare_digest" => {
                if arg_exprs.len() != 2 {
                    bail!("hmac.compare_digest() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];

                // hmac.compare_digest(a, b) → constant-time comparison
                parse_quote! {
                    {
                        use subtle::ConstantTimeEq;
                        #a.ct_eq(#b).into()
                    }
                }
            }

            _ => {
                bail!(
                    "hmac.{} not implemented yet (try: new, compare_digest)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert platform module method calls
    ///
    /// Maps Python platform module to Rust std::env::consts:
    /// - platform.system() → std::env::consts::OS
    /// - platform.machine() → std::env::consts::ARCH
    /// - platform.python_version() → "3.11.0" (hardcoded constant)
    #[inline]
    pub(super) fn try_convert_platform_method(
        &mut self,
        method: &str,
        _args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        let result = match method {
            "system" => {
                // platform.system() → std::env::consts::OS
                // Returns "linux", "macos", "windows", etc.
                parse_quote! { std::env::consts::OS.to_string() }
            }

            "machine" => {
                // platform.machine() → std::env::consts::ARCH
                // Returns "x86_64", "aarch64", etc.
                parse_quote! { std::env::consts::ARCH.to_string() }
            }

            "python_version" => {
                // platform.python_version() → "3.11.0"
                // Hardcoded to Python 3.11 for compatibility
                parse_quote! { "3.11.0".to_string() }
            }

            "release" => {
                // platform.release() → OS release version
                // Note: This is OS-specific and may require additional logic
                parse_quote! { std::env::consts::OS.to_string() }
            }

            _ => {
                bail!(
                    "platform.{} not implemented yet (try: system, machine, python_version, release)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert binascii module method calls
    ///
    ///
    /// Supports: hexlify, unhexlify, b2a_hex, a2b_hex, b2a_base64, a2b_base64, crc32
    /// Common encoding/decoding operations
    #[inline]
    pub(super) fn try_convert_binascii_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Hex conversions
            "hexlify" | "b2a_hex" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.{}() requires exactly 1 argument", method);
                }
                self.ctx.require(crate::rust_generator::context::Import::Hex);
                let data = &arg_exprs[0];

                // binascii.hexlify(data) → hex::encode(data) as bytes
                parse_quote! {
                    hex::encode(#data).into_bytes()
                }
            }

            "unhexlify" | "a2b_hex" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.{}() requires exactly 1 argument", method);
                }
                self.ctx.require(crate::rust_generator::context::Import::Hex);
                let data = &arg_exprs[0];

                // binascii.unhexlify(data) → hex::decode(data)
                parse_quote! {
                    hex::decode(#data).expect("Invalid hex string")
                }
            }

            // Base64 conversions
            "b2a_base64" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.b2a_base64() requires exactly 1 argument");
                }
                self.ctx.require(crate::rust_generator::context::Import::Base64);
                let data = &arg_exprs[0];

                // binascii.b2a_base64(data) → base64::encode(data) with newline
                parse_quote! {
                    {
                        let mut result = base64::engine::general_purpose::STANDARD.encode(#data);
                        result.push('\n');
                        result.into_bytes()
                    }
                }
            }

            "a2b_base64" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.a2b_base64() requires exactly 1 argument");
                }
                self.ctx.require(crate::rust_generator::context::Import::Base64);
                let data = &arg_exprs[0];

                // binascii.a2b_base64(data) → base64::decode(data)
                parse_quote! {
                    base64::engine::general_purpose::STANDARD.decode(#data).expect("Invalid base64 string")
                }
            }

            // Quoted-printable encoding
            "b2a_qp" => {
                if arg_exprs.is_empty() {
                    bail!("binascii.b2a_qp() requires at least 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified implementation - basic quoted-printable
                // NOTE: Full RFC 1521 quoted-printable implementation ()
                parse_quote! {
                    {
                        // Simple QP: replace special chars, preserve printable ASCII
                        let bytes: &[u8] = #data;
                        let mut result = Vec::new();
                        for &b in bytes {
                            if b >= 33 && b <= 126 && b != b'=' {
                                result.push(b);
                            } else {
                                result.extend(format!("={:02X}", b).as_bytes());
                            }
                        }
                        result
                    }
                }
            }

            "a2b_qp" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.a2b_qp() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified QP decoder
                // NOTE: Full RFC 1521 quoted-printable implementation ()
                parse_quote! {
                    {
                        let s = std::str::from_utf8(#data).expect("Invalid UTF-8");
                        let mut result = Vec::new();
                        let mut chars = s.chars().peekable();
                        while let Some(c) = chars.next() {
                            if c == '=' {
                                let h1 = chars.next().unwrap_or('0');
                                let h2 = chars.next().unwrap_or('0');
                                let hex = format!("{}{}", h1, h2);
                                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                                    result.push(b);
                                }
                            } else {
                                result.push(c as u8);
                            }
                        }
                        result
                    }
                }
            }

            // UU encoding
            "b2a_uu" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.b2a_uu() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified UU encoding (basic implementation)
                // NOTE: Full UU encoding with proper line wrapping ()
                parse_quote! {
                    {
                        let bytes: &[u8] = #data;
                        let len = bytes.len();
                        let mut result = vec![(len as u8 + 32)]; // Length byte
                        for chunk in bytes.chunks(3) {
                            let mut val = 0u32;
                            for (i, &b) in chunk.iter().enumerate() {
                                val |= (b as u32) << (16 - i * 8);
                            }
                            for i in 0..4 {
                                let b = ((val >> (18 - i * 6)) & 0x3F) as u8;
                                result.push(b + 32);
                            }
                        }
                        result.push(b'\n');
                        result
                    }
                }
            }

            "a2b_uu" => {
                if arg_exprs.len() != 1 {
                    bail!("binascii.a2b_uu() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];

                // Simplified UU decoding (basic implementation)
                // NOTE: Full UU decoding implementation ()
                parse_quote! {
                    {
                        let bytes: &[u8] = #data;
                        if bytes.is_empty() {
                            Vec::new()
                        } else {
                            let len = (bytes[0].wrapping_sub(32)) as usize;
                            let mut result = Vec::with_capacity(len);
                            for chunk in bytes[1..].chunks(4) {
                                if chunk.len() < 4 { break; }
                                let mut val = 0u32;
                                for (i, &b) in chunk.iter().enumerate() {
                                    val |= ((b.wrapping_sub(32) & 0x3F) as u32) << (18 - i * 6);
                                }
                                for i in 0..3 {
                                    if result.len() < len {
                                        result.push((val >> (16 - i * 8)) as u8);
                                    }
                                }
                            }
                            result
                        }
                    }
                }
            }

            // CRC32 checksum
            "crc32" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("binascii.crc32() requires 1 or 2 arguments");
                }
                self.ctx.require(crate::rust_generator::context::Import::Crc32);
                let data = &arg_exprs[0];

                if arg_exprs.len() == 1 {
                    // binascii.crc32(data) → crc32 checksum as u32
                    parse_quote! {
                        {
                            use crc32fast::Hasher;
                            let mut hasher = Hasher::new();
                            hasher.update(#data);
                            hasher.finalize() as i32
                        }
                    }
                } else {
                    // binascii.crc32(data, crc) → update existing crc
                    let crc = &arg_exprs[1];
                    parse_quote! {
                        {
                            use crc32fast::Hasher;
                            let mut hasher = Hasher::new_with_initial(#crc as u32);
                            hasher.update(#data);
                            hasher.finalize() as i32
                        }
                    }
                }
            }

            _ => {
                bail!(
                    "binascii.{} not implemented yet (available: hexlify, unhexlify, b2a_hex, a2b_hex, b2a_base64, a2b_base64, b2a_qp, a2b_qp, b2a_uu, a2b_uu, crc32)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert urllib.parse module method calls
    ///
    ///
    /// Supports: quote, unquote, quote_plus, unquote_plus, urlencode, parse_qs
    /// Common URL encoding/decoding operations
    #[inline]
    pub(super) fn try_convert_urllib_parse_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need URL encoding support
        self.ctx.require(crate::rust_generator::context::Import::UrlEncoding);

        let result = match method {
            // Percent encoding
            "quote" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.quote() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // quote(text) → percent-encode URL component
                parse_quote! {
                    {
                        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                        utf8_percent_encode(#text, NON_ALPHANUMERIC).to_string()
                    }
                }
            }

            "unquote" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.unquote() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // unquote(text) → percent-decode URL component
                parse_quote! {
                    {
                        use percent_encoding::percent_decode_str;
                        percent_decode_str(#text).decode_utf8_lossy().to_string()
                    }
                }
            }

            // Percent encoding with + for spaces (form encoding)
            "quote_plus" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.quote_plus() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // quote_plus(text) → percent-encode with + for spaces
                parse_quote! {
                    {
                        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                        utf8_percent_encode(#text, NON_ALPHANUMERIC)
                            .to_string()
                            .replace("%20", "+")
                    }
                }
            }

            "unquote_plus" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.unquote_plus() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                // unquote_plus(text) → percent-decode with + as space
                parse_quote! {
                    {
                        use percent_encoding::percent_decode_str;
                        let replaced = (#text).replace("+", " ");
                        percent_decode_str(&replaced).decode_utf8_lossy().to_string()
                    }
                }
            }

            // Query string encoding
            "urlencode" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.urlencode() requires exactly 1 argument");
                }
                let params = &arg_exprs[0];

                // urlencode(dict) → key1=value1&key2=value2
                parse_quote! {
                    {
                        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                        #params.iter()
                            .map(|(k, v)| {
                                let key = utf8_percent_encode(&k.to_string(), NON_ALPHANUMERIC).to_string();
                                let val = utf8_percent_encode(&v.to_string(), NON_ALPHANUMERIC).to_string();
                                format!("{}={}", key, val)
                            })
                            .collect::<Vec<_>>()
                            .join("&")
                    }
                }
            }

            // Query string parsing
            "parse_qs" => {
                if arg_exprs.len() != 1 {
                    bail!("urllib.parse.parse_qs() requires exactly 1 argument");
                }
                let qs = &arg_exprs[0];

                // parse_qs(qs) → HashMap<String, Vec<String>>
                parse_quote! {
                    {
                        use percent_encoding::percent_decode_str;
                        use std::collections::HashMap;

                        let mut result: HashMap<String, Vec<String>> = HashMap::new();
                        for pair in (#qs).split('&') {
                            if let Some((key, value)) = pair.split_once('=') {
                                let decoded_key = percent_decode_str(key).decode_utf8_lossy().to_string();
                                let decoded_value = percent_decode_str(value).decode_utf8_lossy().to_string();
                                result.entry(decoded_key).or_insert_with(Vec::new).push(decoded_value);
                            }
                        }
                        result
                    }
                }
            }

            _ => {
                bail!(
                    "urllib.parse.{} not implemented yet (available: quote, unquote, quote_plus, unquote_plus, urlencode, parse_qs)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert fnmatch module method calls
    ///
    ///
    /// Supports: fnmatch, fnmatchcase, filter, translate
    /// Shell wildcard patterns: *, ?, [seq], [!seq]
    #[inline]
    pub(super) fn try_convert_fnmatch_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // fnmatch needs regex crate for pattern matching
        self.ctx.require(crate::rust_generator::context::Import::Regex);

        let result = match method {
            // Basic pattern matching
            "fnmatch" | "fnmatchcase" => {
                if arg_exprs.len() != 2 {
                    bail!("fnmatch.{}() requires exactly 2 arguments", method);
                }
                let name = &arg_exprs[0];
                let pattern = &arg_exprs[1];

                // Simplified implementation: convert pattern to regex and match
                // NOTE: Proper fnmatch pattern translation with case sensitivity ()
                parse_quote! {
                    {
                        // Convert fnmatch pattern to regex
                        let pattern_str = #pattern;
                        let regex_pattern = pattern_str
                            .replace(".", "\\.")
                            .replace("*", ".*")
                            .replace("?", ".")
                            .replace("[!", "[^");

                        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))
                            .unwrap_or_else(|_| regex::Regex::new("^$").unwrap());

                        regex.is_match(#name)
                    }
                }
            }

            // Filter list by pattern
            "filter" => {
                if arg_exprs.len() != 2 {
                    bail!("fnmatch.filter() requires exactly 2 arguments");
                }
                let names = &arg_exprs[0];
                let pattern = &arg_exprs[1];

                // filter(names, pattern) → names matching pattern
                parse_quote! {
                    {
                        let pattern_str = #pattern;
                        let regex_pattern = pattern_str
                            .replace(".", "\\.")
                            .replace("*", ".*")
                            .replace("?", ".")
                            .replace("[!", "[^");

                        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))
                            .unwrap_or_else(|_| regex::Regex::new("^$").unwrap());

                        (#names).into_iter()
                            .filter(|name| regex.is_match(&name.to_string()))
                            .collect::<Vec<_>>()
                    }
                }
            }

            // Translate pattern to regex
            "translate" => {
                if arg_exprs.len() != 1 {
                    bail!("fnmatch.translate() requires exactly 1 argument");
                }
                let pattern = &arg_exprs[0];

                // translate(pattern) → regex string
                parse_quote! {
                    {
                        let pattern_str = #pattern;
                        let regex_pattern = pattern_str
                            .replace(".", "\\.")
                            .replace("*", ".*")
                            .replace("?", ".")
                            .replace("[!", "[^");

                        format!("(?ms)^{}$", regex_pattern)
                    }
                }
            }

            _ => {
                bail!(
                    "fnmatch.{} not implemented yet (available: fnmatch, fnmatchcase, filter, translate)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert shlex module method calls
    ///
    ///
    /// Supports: split, quote, join
    /// Security-critical: prevents shell injection
    #[inline]
    pub(super) fn try_convert_shlex_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Shell-like split (respects quotes and escapes)
            "split" => {
                if arg_exprs.len() != 1 {
                    bail!("shlex.split() requires exactly 1 argument");
                }
                let s = &arg_exprs[0];

                // Simplified shell split (handles basic quotes)
                // NOTE: Use shell-words crate for full POSIX shell compliance ()
                parse_quote! {
                    {
                        let input = #s;
                        let mut result = Vec::new();
                        let mut current = String::new();
                        let mut in_single_quote = false;
                        let mut in_double_quote = false;
                        let mut escaped = false;

                        for c in input.chars() {
                            if escaped {
                                current.push(c);
                                escaped = false;
                            } else if c == '\\' && !in_single_quote {
                                escaped = true;
                            } else if c == '\'' && !in_double_quote {
                                in_single_quote = !in_single_quote;
                            } else if c == '"' && !in_single_quote {
                                in_double_quote = !in_double_quote;
                            } else if c.is_whitespace() && !in_single_quote && !in_double_quote {
                                if !current.is_empty() {
                                    result.push(current.clone());
                                    current.clear();
                                }
                            } else {
                                current.push(c);
                            }
                        }

                        if !current.is_empty() {
                            result.push(current);
                        }

                        result
                    }
                }
            }

            // Shell-safe quoting
            "quote" => {
                if arg_exprs.len() != 1 {
                    bail!("shlex.quote() requires exactly 1 argument");
                }
                let s = &arg_exprs[0];

                // Quote string for safe shell usage
                parse_quote! {
                    {
                        let input = #s;
                        // Check if needs quoting
                        let needs_quoting = input.chars().any(|c| {
                            matches!(c, ' ' | '\t' | '\n' | '\'' | '"' | '\\' | '|' | '&' | ';' |
                                     '(' | ')' | '<' | '>' | '`' | '$' | '*' | '?' | '[' | ']' |
                                     '{' | '}' | '!' | '#' | '~')
                        });

                        if needs_quoting || input.is_empty() {
                            // Use single quotes and escape any single quotes
                            format!("'{}'", input.replace("'", "'\"'\"'"))
                        } else {
                            input.to_string()
                        }
                    }
                }
            }

            // Join list with shell-safe quoting
            "join" => {
                if arg_exprs.len() != 1 {
                    bail!("shlex.join() requires exactly 1 argument");
                }
                let args_list = &arg_exprs[0];

                // Join args with proper quoting
                parse_quote! {
                    {
                        let args = #args_list;
                        args.iter()
                            .map(|arg| {
                                let s = arg.to_string();
                                let needs_quoting = s.chars().any(|c| {
                                    matches!(c, ' ' | '\t' | '\n' | '\'' | '"' | '\\' | '|' | '&' | ';' |
                                             '(' | ')' | '<' | '>' | '`' | '$' | '*' | '?' | '[' | ']' |
                                             '{' | '}' | '!' | '#' | '~')
                                });

                                if needs_quoting || s.is_empty() {
                                    format!("'{}'", s.replace("'", "'\"'\"'"))
                                } else {
                                    s
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                }
            }

            _ => {
                bail!(
                    "shlex.{} not implemented yet (available: split, quote, join)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert textwrap module method calls
    ///
    ///
    /// Supports: wrap, fill, dedent, indent, shorten
    /// Text formatting for display and documentation
    #[inline]
    pub(super) fn try_convert_textwrap_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Wrap text into list of lines
            "wrap" => {
                if arg_exprs.len() < 2 {
                    bail!("textwrap.wrap() requires at least 2 arguments (text, width)");
                }
                let text = &arg_exprs[0];
                let width = &arg_exprs[1];

                // Simple word-wrapping algorithm
                parse_quote! {
                    {
                        let text = #text;
                        let width = #width as usize;
                        let mut lines = Vec::new();
                        let mut current_line = String::new();
                        let mut current_len = 0;

                        for word in text.split_whitespace() {
                            let word_len = word.len();
                            if current_len == 0 {
                                current_line = word.to_string();
                                current_len = word_len;
                            } else if current_len + 1 + word_len <= width {
                                current_line.push(' ');
                                current_line.push_str(word);
                                current_len += 1 + word_len;
                            } else {
                                lines.push(current_line);
                                current_line = word.to_string();
                                current_len = word_len;
                            }
                        }

                        if !current_line.is_empty() {
                            lines.push(current_line);
                        }

                        lines
                    }
                }
            }

            // Wrap and join into single string
            "fill" => {
                if arg_exprs.len() < 2 {
                    bail!("textwrap.fill() requires at least 2 arguments (text, width)");
                }
                let text = &arg_exprs[0];
                let width = &arg_exprs[1];

                // fill = wrap + join
                parse_quote! {
                    {
                        let text = #text;
                        let width = #width as usize;
                        let mut lines = Vec::new();
                        let mut current_line = String::new();
                        let mut current_len = 0;

                        for word in text.split_whitespace() {
                            let word_len = word.len();
                            if current_len == 0 {
                                current_line = word.to_string();
                                current_len = word_len;
                            } else if current_len + 1 + word_len <= width {
                                current_line.push(' ');
                                current_line.push_str(word);
                                current_len += 1 + word_len;
                            } else {
                                lines.push(current_line);
                                current_line = word.to_string();
                                current_len = word_len;
                            }
                        }

                        if !current_line.is_empty() {
                            lines.push(current_line);
                        }

                        lines.join("\n")
                    }
                }
            }

            // Remove common leading whitespace
            "dedent" => {
                if arg_exprs.len() != 1 {
                    bail!("textwrap.dedent() requires exactly 1 argument");
                }
                let text = &arg_exprs[0];

                parse_quote! {
                    {
                        let text = #text;
                        let lines: Vec<&str> = text.lines().collect();

                        // Find minimum indentation (excluding empty lines)
                        let min_indent = lines.iter()
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
                            .min()
                            .unwrap_or(0);

                        // Remove that many spaces from each line
                        lines.iter()
                            .map(|line| {
                                if line.len() >= min_indent {
                                    &line[min_indent..]
                                } else {
                                    line
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }

            // Add prefix to each line
            "indent" => {
                if arg_exprs.len() != 2 {
                    bail!("textwrap.indent() requires exactly 2 arguments (text, prefix)");
                }
                let text = &arg_exprs[0];
                let prefix = &arg_exprs[1];

                parse_quote! {
                    {
                        let text = #text;
                        let prefix = #prefix;
                        text.lines()
                            .map(|line| format!("{}{}", prefix, line))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }

            // Shorten text with ellipsis
            "shorten" => {
                if arg_exprs.len() < 2 {
                    bail!("textwrap.shorten() requires at least 2 arguments (text, width)");
                }
                let text = &arg_exprs[0];
                let width = &arg_exprs[1];

                parse_quote! {
                    {
                        let text = #text;
                        let width = #width as usize;
                        let placeholder = " [...]";

                        if text.len() <= width {
                            text.to_string()
                        } else if width < placeholder.len() {
                            text.chars().take(width).collect()
                        } else {
                            let max_len = width - placeholder.len();
                            let truncated: String = text.chars().take(max_len).collect();
                            format!("{}{}", truncated, placeholder)
                        }
                    }
                }
            }

            _ => {
                bail!(
                    "textwrap.{} not implemented yet (available: wrap, fill, dedent, indent, shorten)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert bisect module method calls
    ///
    ///
    /// Supports: bisect_left, bisect_right, insort_left, insort_right
    /// Efficient O(log n) search and insertion
    #[inline]
    pub(super) fn try_convert_bisect_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Find leftmost insertion point
            "bisect_left" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.bisect_left() requires at least 2 arguments");
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = #a;
                        let val = &#x;
                        match arr.binary_search(val) {
                            Ok(mut pos) => {
                                while pos > 0 && &arr[pos - 1] == val {
                                    pos -= 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        }
                    }
                }
            }

            // Find rightmost insertion point
            "bisect_right" | "bisect" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.{}() requires at least 2 arguments", method);
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = #a;
                        let val = &#x;
                        match arr.binary_search(val) {
                            Ok(mut pos) => {
                                pos += 1;
                                while pos < arr.len() && &arr[pos] == val {
                                    pos += 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        }
                    }
                }
            }

            // Insert at leftmost position
            "insort_left" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.insort_left() requires at least 2 arguments");
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = &mut (#a);
                        let val = #x;
                        let pos = match arr.binary_search(&val) {
                            Ok(mut pos) => {
                                while pos > 0 && arr[pos - 1] == val {
                                    pos -= 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        };
                        arr.insert(pos, val);
                    }
                }
            }

            // Insert at rightmost position
            "insort_right" | "insort" => {
                if arg_exprs.len() < 2 {
                    bail!("bisect.{}() requires at least 2 arguments", method);
                }
                let a = &arg_exprs[0];
                let x = &arg_exprs[1];

                parse_quote! {
                    {
                        let arr = &mut (#a);
                        let val = #x;
                        let pos = match arr.binary_search(&val) {
                            Ok(mut pos) => {
                                pos += 1;
                                while pos < arr.len() && arr[pos] == val {
                                    pos += 1;
                                }
                                pos
                            }
                            Err(pos) => pos,
                        };
                        arr.insert(pos, val);
                    }
                }
            }

            _ => {
                bail!(
                    "bisect.{} not implemented yet (available: bisect_left, bisect_right, insort_left, insort_right)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert heapq module method calls
    ///
    ///
    /// Supports: heapify, heappush, heappop, nlargest, nsmallest
    /// Python heapq is a MIN heap (smallest item first)
    #[inline]
    pub(super) fn try_convert_heapq_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Transform list into min-heap in-place
            "heapify" => {
                if arg_exprs.is_empty() {
                    bail!("heapq.heapify() requires at least 1 argument");
                }
                let x = &arg_exprs[0];

                parse_quote! {
                    {
                        let heap = &mut (#x);
                        // Build min-heap using bottom-up heapify
                        let len = heap.len();
                        if len > 1 {
                            for i in (0..len/2).rev() {
                                let mut pos = i;
                                loop {
                                    let left = 2 * pos + 1;
                                    let right = 2 * pos + 2;
                                    let mut smallest = pos;

                                    if left < len && heap[left] < heap[smallest] {
                                        smallest = left;
                                    }
                                    if right < len && heap[right] < heap[smallest] {
                                        smallest = right;
                                    }

                                    if smallest == pos {
                                        break;
                                    }

                                    heap.swap(pos, smallest);
                                    pos = smallest;
                                }
                            }
                        }
                    }
                }
            }

            // Push item onto min-heap
            "heappush" => {
                if arg_exprs.len() < 2 {
                    bail!("heapq.heappush() requires at least 2 arguments");
                }
                let heap = &arg_exprs[0];
                let item = &arg_exprs[1];

                parse_quote! {
                    {
                        let heap = &mut (#heap);
                        let item = #item;
                        heap.push(item);

                        // Bubble up to maintain min-heap property
                        let mut pos = heap.len() - 1;
                        while pos > 0 {
                            let parent = (pos - 1) / 2;
                            if heap[pos] >= heap[parent] {
                                break;
                            }
                            heap.swap(pos, parent);
                            pos = parent;
                        }
                    }
                }
            }

            // Pop and return smallest item from min-heap
            "heappop" => {
                if arg_exprs.is_empty() {
                    bail!("heapq.heappop() requires at least 1 argument");
                }
                let heap = &arg_exprs[0];

                parse_quote! {
                    {
                        let heap = &mut (#heap);
                        if heap.is_empty() {
                            panic!("heappop from empty heap");
                        }

                        let result = heap[0].clone();
                        let last = heap.pop().unwrap();

                        if !heap.is_empty() {
                            heap[0] = last;

                            // Bubble down to maintain min-heap property
                            let mut pos = 0;
                            loop {
                                let left = 2 * pos + 1;
                                let right = 2 * pos + 2;
                                let mut smallest = pos;

                                if left < heap.len() && heap[left] < heap[smallest] {
                                    smallest = left;
                                }
                                if right < heap.len() && heap[right] < heap[smallest] {
                                    smallest = right;
                                }

                                if smallest == pos {
                                    break;
                                }

                                heap.swap(pos, smallest);
                                pos = smallest;
                            }
                        }

                        result
                    }
                }
            }

            // Return n largest elements
            "nlargest" => {
                if arg_exprs.len() < 2 {
                    bail!("heapq.nlargest() requires at least 2 arguments");
                }
                let n = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let n = #n as usize;
                        let mut items = #iterable;
                        items.sort_by(|a, b| b.cmp(a));  // Sort descending
                        items.into_iter().take(n).collect::<Vec<_>>()
                    }
                }
            }

            // Return n smallest elements
            "nsmallest" => {
                if arg_exprs.len() < 2 {
                    bail!("heapq.nsmallest() requires at least 2 arguments");
                }
                let n = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let n = #n as usize;
                        let mut items = #iterable;
                        items.sort();  // Sort ascending
                        items.into_iter().take(n).collect::<Vec<_>>()
                    }
                }
            }

            _ => {
                bail!(
                    "heapq.{} not implemented yet (available: heapify, heappush, heappop, nlargest, nsmallest)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert copy module method calls
    ///
    ///
    /// Supports: copy, deepcopy
    /// Maps to Rust's .clone() for both (Rust clone is deep by default)
    #[inline]
    pub(super) fn try_convert_copy_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Shallow copy - in Rust, clone() is typically deep for owned data
            "copy" => {
                if arg_exprs.is_empty() {
                    bail!("copy.copy() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                parse_quote! {
                    (#obj).clone()
                }
            }

            // Deep copy - in Rust, clone() already performs deep copy
            "deepcopy" => {
                if arg_exprs.is_empty() {
                    bail!("copy.deepcopy() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                parse_quote! {
                    (#obj).clone()
                }
            }

            _ => {
                bail!(
                    "copy.{} not implemented yet (available: copy, deepcopy)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert itertools module method calls
    ///
    ///
    /// Supports: count, cycle, repeat, chain, islice, takewhile
    /// Maps to Rust's iterator adapters and std::iter methods
    #[inline]
    pub(super) fn try_convert_itertools_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Infinite counter with optional step
            "count" => {
                let start = if !arg_exprs.is_empty() {
                    &arg_exprs[0]
                } else {
                    &parse_quote!(0)
                };
                let step = if arg_exprs.len() >= 2 {
                    &arg_exprs[1]
                } else {
                    &parse_quote!(1)
                };

                parse_quote! {
                    {
                        let start = #start;
                        let step = #step;
                        std::iter::successors(Some(start), move |&n| Some(n + step))
                    }
                }
            }

            // Cycle through iterable infinitely
            "cycle" => {
                if arg_exprs.is_empty() {
                    bail!("itertools.cycle() requires at least 1 argument");
                }
                let iterable = &arg_exprs[0];

                parse_quote! {
                    {
                        let items = #iterable;
                        items.into_iter().cycle()
                    }
                }
            }

            // Repeat value n times (or infinitely if no count)
            "repeat" => {
                if arg_exprs.is_empty() {
                    bail!("itertools.repeat() requires at least 1 argument");
                }
                let value = &arg_exprs[0];

                if arg_exprs.len() >= 2 {
                    let times = &arg_exprs[1];
                    parse_quote! {
                        {
                            let val = #value;
                            let n = #times as usize;
                            std::iter::repeat(val).take(n)
                        }
                    }
                } else {
                    parse_quote! {
                        {
                            let val = #value;
                            std::iter::repeat(val)
                        }
                    }
                }
            }

            // Chain multiple iterables together
            "chain" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.chain() requires at least 2 arguments");
                }

                // Chain first two, then fold the rest
                let first = &arg_exprs[0];
                let second = &arg_exprs[1];

                if arg_exprs.len() == 2 {
                    parse_quote! {
                        {
                            let a = #first;
                            let b = #second;
                            a.into_iter().chain(b.into_iter())
                        }
                    }
                } else {
                    // For more than 2, we need to chain them all
                    let mut chain_expr: syn::Expr = parse_quote! {
                        #first.into_iter().chain(#second.into_iter())
                    };

                    for item in &arg_exprs[2..] {
                        chain_expr = parse_quote! {
                            #chain_expr.chain(#item.into_iter())
                        };
                    }

                    chain_expr
                }
            }

            // Slice iterator with start, stop, step
            "islice" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.islice() requires at least 2 arguments");
                }
                let iterable = &arg_exprs[0];

                if arg_exprs.len() == 2 {
                    // islice(iterable, stop)
                    let stop = &arg_exprs[1];
                    parse_quote! {
                        {
                            let items = #iterable;
                            let n = #stop as usize;
                            items.into_iter().take(n)
                        }
                    }
                } else {
                    // islice(iterable, start, stop)
                    let start = &arg_exprs[1];
                    let stop = &arg_exprs[2];
                    parse_quote! {
                        {
                            let items = #iterable;
                            let start_idx = #start as usize;
                            let stop_idx = #stop as usize;
                            items.into_iter().skip(start_idx).take(stop_idx - start_idx)
                        }
                    }
                }
            }

            // Take while predicate is true
            "takewhile" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.takewhile() requires at least 2 arguments");
                }
                let predicate = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let pred = #predicate;
                        let items = #iterable;
                        items.into_iter().take_while(pred)
                    }
                }
            }

            // Drop while predicate is true
            "dropwhile" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.dropwhile() requires at least 2 arguments");
                }
                let predicate = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                parse_quote! {
                    {
                        let pred = #predicate;
                        let items = #iterable;
                        items.into_iter().skip_while(pred)
                    }
                }
            }

            // Accumulate (running sum/product)
            "accumulate" => {
                if arg_exprs.is_empty() {
                    bail!("itertools.accumulate() requires at least 1 argument");
                }
                let iterable = &arg_exprs[0];

                // accumulate with default + operation
                parse_quote! {
                    {
                        let items = #iterable;
                        let mut acc = None;
                        items.into_iter().map(|x| {
                            acc = Some(match acc {
                                None => x,
                                Some(a) => a + x,
                            });
                            acc.unwrap()
                        }).collect::<Vec<_>>()
                    }
                }
            }

            // Compress - filter by selector booleans
            "compress" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.compress() requires at least 2 arguments");
                }
                let data = &arg_exprs[0];
                let selectors = &arg_exprs[1];

                parse_quote! {
                    {
                        let items = #data;
                        let sels = #selectors;
                        items.into_iter()
                            .zip(sels.into_iter())
                            .filter_map(|(item, sel)| if sel { Some(item) } else { None })
                            .collect::<Vec<_>>()
                    }
                }
            }

            // Combinations - generate k-length combinations from iterable
            "combinations" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.combinations() requires 2 arguments (iterable, r)");
                }
                let iterable = &arg_exprs[0];
                let r = &arg_exprs[1];

                parse_quote! {
                    {
                        use itertools::Itertools;
                        let items: Vec<_> = #iterable.into_iter().collect();
                        items.into_iter().combinations(#r as usize)
                    }
                }
            }

            // Permutations - generate k-length permutations from iterable
            "permutations" => {
                if arg_exprs.len() < 2 {
                    bail!("itertools.permutations() requires 2 arguments (iterable, r)");
                }
                let iterable = &arg_exprs[0];
                let r = &arg_exprs[1];

                parse_quote! {
                    {
                        use itertools::Itertools;
                        let items: Vec<_> = #iterable.into_iter().collect();
                        items.into_iter().permutations(#r as usize)
                    }
                }
            }

            // Groupby - group consecutive elements by key function
            "groupby" => {
                // First arg is the iterable
                if arg_exprs.is_empty() {
                    bail!("itertools.groupby() requires at least 1 argument (iterable)");
                }
                let iterable = &arg_exprs[0];

                // Try to find key in kwargs
                let key_func_expr =
                    if let Some((_, key_expr)) = kwargs.iter().find(|(k, _)| k == "key") {
                        key_expr.to_rust_expr(self.ctx)?
                    } else if arg_exprs.len() >= 2 {
                        arg_exprs[1].clone()
                    } else {
                        // Default key: identity function
                        parse_quote! { |x| x }
                    };

                parse_quote! {
                    {
                        use itertools::Itertools;
                        let items = #iterable;
                        let key_fn = #key_func_expr;
                        items.into_iter().group_by(key_fn)
                    }
                }
            }

            _ => {
                bail!(
                    "itertools.{} not implemented yet (available: count, cycle, repeat, chain, islice, takewhile, dropwhile, accumulate, compress, combinations, permutations, groupby)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert functools module method calls
    ///
    ///
    /// Supports: reduce
    /// Maps to Rust's Iterator::fold() method
    #[inline]
    pub(super) fn try_convert_functools_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Reduce/fold operation
            "reduce" => {
                if arg_exprs.len() < 2 {
                    bail!("functools.reduce() requires at least 2 arguments");
                }
                let function = &arg_exprs[0];
                let iterable = &arg_exprs[1];

                if arg_exprs.len() >= 3 {
                    // With initial value
                    let initial = &arg_exprs[2];
                    parse_quote! {
                        {
                            let func = #function;
                            let items = #iterable;
                            let init = #initial;
                            items.into_iter().fold(init, func)
                        }
                    }
                } else {
                    // Without initial value - use first element
                    parse_quote! {
                        {
                            let func = #function;
                            let mut items = (#iterable).into_iter();
                            let init = items.next().expect("reduce() of empty sequence with no initial value");
                            items.fold(init, func)
                        }
                    }
                }
            }

            _ => {
                bail!(
                    "functools.{} not implemented yet (available: reduce)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert warnings module method calls
    ///
    ///
    /// Supports: warn
    /// Maps to Rust's eprintln! macro for stderr output
    #[inline]
    pub(super) fn try_convert_warnings_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "warn" => {
                if arg_exprs.is_empty() {
                    bail!("warnings.warn() requires at least 1 argument");
                }
                let message = &arg_exprs[0];

                parse_quote! {
                    eprintln!("Warning: {}", #message)
                }
            }

            _ => {
                bail!("warnings.{} not implemented yet (available: warn)", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert sys module method calls
    ///
    ///
    /// Supports: exit
    /// Maps to Rust's std::process::exit
    #[inline]
    pub(super) fn try_convert_sys_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "exit" => {
                let code = if !arg_exprs.is_empty() {
                    &arg_exprs[0]
                } else {
                    &parse_quote!(0)
                };

                parse_quote! {
                    std::process::exit(#code)
                }
            }

            _ => {
                bail!("sys.{} not implemented yet (available: exit)", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert pickle module method calls
    ///
    ///
    /// Supports: dumps, loads
    /// Maps to serde/bincode for serialization (placeholder)
    #[inline]
    pub(super) fn try_convert_pickle_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "dumps" => {
                if arg_exprs.is_empty() {
                    bail!("pickle.dumps() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                // Placeholder: In real implementation, would use serde + bincode
                parse_quote! {
                    {
                        // Note: Actual pickle serialization requires serde support
                        format!("{:?}", #obj).into_bytes()
                    }
                }
            }

            "loads" => {
                if arg_exprs.is_empty() {
                    bail!("pickle.loads() requires at least 1 argument");
                }
                let data = &arg_exprs[0];

                // Placeholder: In real implementation, would use serde + bincode
                parse_quote! {
                    {
                        // Note: Actual pickle deserialization requires serde support
                        String::from_utf8_lossy(#data).to_string()
                    }
                }
            }

            _ => {
                bail!(
                    "pickle.{} not implemented yet (available: dumps, loads)",
                    method
                );
            }
        };

        Ok(Some(result))
    }

    /// Try to convert pprint module method calls
    ///
    ///
    /// Supports: pprint
    /// Maps to Rust's Debug formatting
    #[inline]
    pub(super) fn try_convert_pprint_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            "pprint" => {
                if arg_exprs.is_empty() {
                    bail!("pprint.pprint() requires at least 1 argument");
                }
                let obj = &arg_exprs[0];

                parse_quote! {
                    println!("{:#?}", #obj)
                }
            }

            _ => {
                bail!("pprint.{} not implemented yet (available: pprint)", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert fractions module method calls
    #[inline]
    pub(super) fn try_convert_fractions_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Mark that we need the num-rational crate
        self.ctx.require(crate::rust_generator::context::Import::NumRational);

        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Fraction methods
            "limit_denominator" => {
                if arg_exprs.len() != 2 {
                    bail!(
                        "Fraction.limit_denominator() requires exactly 2 arguments (self, max_denominator)"
                    );
                }
                let frac = &arg_exprs[0];
                let max_denom = &arg_exprs[1];
                // Simplified: if denominator within limit, return as-is
                parse_quote! {
                    {
                        let f = #frac;
                        let max_d = #max_denom as i32;
                        if *f.denom() <= max_d {
                            f
                        } else {
                            // Approximate by converting to float and back
                            num::rational::Ratio::approximate_float(f.to_f64().unwrap()).unwrap_or(f)
                        }
                    }
                }
            }

            "as_integer_ratio" => {
                if arg_exprs.len() != 1 {
                    bail!("Fraction.as_integer_ratio() requires exactly 1 argument (self)");
                }
                let frac = &arg_exprs[0];
                parse_quote! { (*#frac.numer(), *#frac.denom()) }
            }

            _ => return Ok(None), // Not a recognized fractions method
        };

        Ok(Some(result))
    }

    /// Try to convert pathlib module method calls
    #[inline]
    pub(super) fn try_convert_pathlib_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Path queries
            "exists" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.exists() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.exists() }
            }

            "is_file" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.is_file() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.is_file() }
            }

            "is_dir" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.is_dir() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.is_dir() }
            }

            "is_absolute" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.is_absolute() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.is_absolute() }
            }

            // Path transformations
            "absolute" | "resolve" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.{}() requires exactly 1 argument (self)", method);
                }
                let path = &arg_exprs[0];
                // Both absolute() and resolve() → canonicalize()
                parse_quote! { #path.canonicalize().unwrap() }
            }

            "with_name" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.with_name() requires exactly 2 arguments (self, name)");
                }
                let path = &arg_exprs[0];
                let name = &arg_exprs[1];
                parse_quote! { #path.with_file_name(#name) }
            }

            "with_suffix" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.with_suffix() requires exactly 2 arguments (self, suffix)");
                }
                let path = &arg_exprs[0];
                let suffix = &arg_exprs[1];
                parse_quote! { #path.with_extension(#suffix.trim_start_matches('.')) }
            }

            // Directory operations
            "mkdir" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("Path.mkdir() requires 1-2 arguments");
                }
                let path = &arg_exprs[0];

                // Check if parents=True was passed (simplified - assumes second arg is parents)
                if arg_exprs.len() == 2 {
                    // mkdir(parents=True) → create_dir_all
                    parse_quote! { std::fs::create_dir_all(#path).unwrap() }
                } else {
                    // mkdir() → create_dir
                    parse_quote! { std::fs::create_dir(#path).unwrap() }
                }
            }

            "rmdir" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.rmdir() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::remove_dir(#path).unwrap() }
            }

            "iterdir" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.iterdir() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! {
                    std::fs::read_dir(#path)
                        .unwrap()
                        .map(|e| e.unwrap().path())
                        .collect::<Vec<_>>()
                }
            }

            // File operations
            "read_text" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.read_text() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::read_to_string(#path).unwrap() }
            }

            "read_bytes" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.read_bytes() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::read(#path).unwrap() }
            }

            "write_text" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.write_text() requires exactly 2 arguments (self, content)");
                }
                let path = &arg_exprs[0];
                let content = &arg_exprs[1];
                parse_quote! { std::fs::write(#path, #content).unwrap() }
            }

            "write_bytes" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.write_bytes() requires exactly 2 arguments (self, content)");
                }
                let path = &arg_exprs[0];
                let content = &arg_exprs[1];
                parse_quote! { std::fs::write(#path, #content).unwrap() }
            }

            "unlink" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.unlink() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { std::fs::remove_file(#path).unwrap() }
            }

            "rename" => {
                if arg_exprs.len() != 2 {
                    bail!("Path.rename() requires exactly 2 arguments (self, target)");
                }
                let path = &arg_exprs[0];
                let target = &arg_exprs[1];
                parse_quote! { { std::fs::rename(&#path, #target).unwrap(); std::path::PathBuf::from(#target) } }
            }

            // Conversions
            "as_posix" => {
                if arg_exprs.len() != 1 {
                    bail!("Path.as_posix() requires exactly 1 argument (self)");
                }
                let path = &arg_exprs[0];
                parse_quote! { #path.to_str().unwrap().to_string() }
            }

            _ => return Ok(None), // Not a recognized pathlib method
        };

        Ok(Some(result))
    }

    /// Try to convert datetime module method calls
    #[inline]
    pub(super) fn try_convert_datetime_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Mark that we need the chrono crate
        self.ctx.require(crate::rust_generator::context::Import::Chrono);

        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // datetime.datetime.now() → Local::now()
            "now" => {
                if arg_exprs.is_empty() {
                    parse_quote! { chrono::Local::now().naive_local() }
                } else {
                    bail!("datetime.now() takes no arguments");
                }
            }

            // datetime.datetime.utcnow() → Utc::now()
            "utcnow" => {
                if arg_exprs.is_empty() {
                    parse_quote! { chrono::Utc::now().naive_utc() }
                } else {
                    bail!("datetime.utcnow() takes no arguments");
                }
            }

            // datetime.datetime.today() → Local::now().date()
            "today" => {
                if arg_exprs.is_empty() {
                    parse_quote! { chrono::Local::now().date_naive() }
                } else {
                    bail!("datetime.today() takes no arguments");
                }
            }

            // datetime.datetime.strftime(format) → dt.format(format).to_string()
            "strftime" => {
                if arg_exprs.len() != 2 {
                    bail!("strftime() requires exactly 2 arguments (self, format)");
                }
                let dt = &arg_exprs[0];
                let fmt = &arg_exprs[1];
                parse_quote! { #dt.format(#fmt).to_string() }
            }

            // datetime.datetime.strptime(string, format) → NaiveDateTime::parse_from_str(string, format)
            "strptime" => {
                if arg_exprs.len() != 2 {
                    bail!("strptime() requires exactly 2 arguments (string, format)");
                }
                let s = &arg_exprs[0];
                let fmt = &arg_exprs[1];
                parse_quote! {
                    chrono::NaiveDateTime::parse_from_str(#s, #fmt).unwrap()
                }
            }

            // datetime.datetime.isoformat() → dt.to_rfc3339()
            "isoformat" => {
                if arg_exprs.len() != 1 {
                    bail!("isoformat() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.to_string() }
            }

            // datetime.datetime.timestamp() → dt.timestamp()
            "timestamp" => {
                if arg_exprs.len() != 1 {
                    bail!("timestamp() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.and_utc().timestamp() as f64 }
            }

            // datetime.datetime.fromtimestamp(ts) → NaiveDateTime::from_timestamp(ts, 0)
            "fromtimestamp" => {
                if arg_exprs.len() != 1 {
                    bail!("fromtimestamp() requires exactly 1 argument (timestamp)");
                }
                let ts = &arg_exprs[0];
                parse_quote! {
                    chrono::DateTime::from_timestamp(#ts as i64, 0)
                        .unwrap()
                        .naive_local()
                }
            }

            // date.weekday() → dt.weekday().num_days_from_monday()
            "weekday" => {
                if arg_exprs.len() != 1 {
                    bail!("weekday() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.weekday().num_days_from_monday() as i32 }
            }

            // date.isoweekday() → dt.weekday().number_from_monday()
            "isoweekday" => {
                if arg_exprs.len() != 1 {
                    bail!("isoweekday() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                // ISO weekday: Monday=1, Sunday=7
                parse_quote! { (#dt.weekday().num_days_from_monday() + 1) as i32 }
            }

            // timedelta.total_seconds() → duration.num_seconds() as f64
            "total_seconds" => {
                if arg_exprs.len() != 1 {
                    bail!("total_seconds() requires exactly 1 argument (self)");
                }
                let td = &arg_exprs[0];
                parse_quote! { #td.num_seconds() as f64 }
            }

            // datetime.date() → extract date part
            "date" => {
                if arg_exprs.len() != 1 {
                    bail!("date() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.date() }
            }

            // datetime.time() → extract time part
            "time" => {
                if arg_exprs.len() != 1 {
                    bail!("time() requires exactly 1 argument (self)");
                }
                let dt = &arg_exprs[0];
                parse_quote! { #dt.time() }
            }

            // datetime.replace(year=..., month=..., day=..., ...)
            "replace" => {
                if arg_exprs.len() != 2 {
                    bail!("replace() not fully implemented (requires keyword args)");
                }
                // Simplified: assume single year replacement
                let dt = &arg_exprs[0];
                let new_year = &arg_exprs[1];
                parse_quote! { #dt.with_year(#new_year as i32).unwrap() }
            }

            _ => return Ok(None), // Not a recognized datetime method
        };

        Ok(Some(result))
    }

    /// Try to convert statistics module method calls
    #[inline]
    pub(super) fn try_convert_decimal_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Mark that we need the rust_decimal crate
        self.ctx.require(crate::rust_generator::context::Import::RustDecimal);

        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Mathematical operations
            "sqrt" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.sqrt() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.sqrt().unwrap() }
            }

            "exp" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.exp() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.exp() }
            }

            "ln" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.ln() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.ln() }
            }

            "log10" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.log10() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.log10() }
            }

            // Rounding and quantization
            "quantize" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.quantize() requires exactly 1 argument");
                }
                let value = &arg_exprs[0];
                // quantize(Decimal("0.01")) → round to 2 decimal places
                // For now, we'll use round_dp(2) as a simple approximation
                // NOTE: More sophisticated Decimal quantization based on quantum value ()
                parse_quote! { #value.round_dp(2) }
            }

            "to_integral" | "to_integral_value" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.to_integral() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.trunc() }
            }

            // Predicates
            "is_nan" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_nan() requires exactly 1 argument");
                }
                let _arg = &arg_exprs[0];
                // rust_decimal doesn't have NaN, always returns false
                parse_quote! { false }
            }

            "is_infinite" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_infinite() requires exactly 1 argument");
                }
                let _arg = &arg_exprs[0];
                // rust_decimal doesn't have infinity, always returns false
                parse_quote! { false }
            }

            "is_finite" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_finite() requires exactly 1 argument");
                }
                let _arg = &arg_exprs[0];
                // rust_decimal doesn't have infinity/NaN, always returns true
                parse_quote! { true }
            }

            "is_signed" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_signed() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.is_sign_negative() }
            }

            "is_zero" => {
                if arg_exprs.len() != 1 {
                    bail!("Decimal.is_zero() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { #arg.is_zero() }
            }

            // Sign operations
            "copy_sign" | "copysign" => {
                if arg_exprs.len() != 2 {
                    bail!("Decimal.copy_sign() requires exactly 2 arguments");
                }
                let value = &arg_exprs[0];
                let other = &arg_exprs[1];
                // Copy sign: if other is negative, return -abs(value), else abs(value)
                parse_quote! {
                    if #other.is_sign_negative() {
                        -#value.abs()
                    } else {
                        #value.abs()
                    }
                }
            }

            // Comparison
            "compare" => {
                if arg_exprs.len() != 2 {
                    bail!("Decimal.compare() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // compare() returns -1, 0, or 1
                parse_quote! {
                    match #a.cmp(&#b) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    }
                }
            }

            _ => return Ok(None), // Not a recognized decimal method
        };

        Ok(Some(result))
    }

    pub(super) fn try_convert_statistics_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Averages and central tendency
            "mean" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.mean() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.mean(data) → data.iter().sum::<f64>() / data.len() as f64
                parse_quote! {
                    {
                        let data = #data;
                        data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64
                    }
                }
            }

            "median" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.median() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.median(data) → sorted median calculation
                parse_quote! {
                    {
                        let mut sorted = #data.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let len = sorted.len();
                        if len % 2 == 0 {
                            let mid = len / 2;
                            ((sorted[mid - 1] as f64) + (sorted[mid] as f64)) / 2.0
                        } else {
                            sorted[len / 2] as f64
                        }
                    }
                }
            }

            "mode" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.mode() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.mode(data) → find most common element
                self.ctx.require(crate::rust_generator::context::Import::HashMap);
                parse_quote! {
                    {
                        let mut counts: HashMap<_, usize> = HashMap::new();
                        for &item in #data.iter() {
                            *counts.entry(item).or_insert(0) += 1;
                        }
                        *counts.iter().max_by_key(|(_, &count)| count).unwrap().0
                    }
                }
            }

            // Measures of spread
            "variance" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.variance() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.variance(data) → sample variance (n-1 denominator)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        sum_sq_diff / ((data.len() - 1) as f64)
                    }
                }
            }

            "pvariance" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.pvariance() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.pvariance(data) → population variance (n denominator)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        sum_sq_diff / (data.len() as f64)
                    }
                }
            }

            "stdev" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.stdev() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.stdev(data) → sqrt(variance)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        let variance = sum_sq_diff / ((data.len() - 1) as f64);
                        variance.sqrt()
                    }
                }
            }

            "pstdev" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.pstdev() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.pstdev(data) → sqrt(pvariance)
                parse_quote! {
                    {
                        let data = #data;
                        let mean = data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64;
                        let sum_sq_diff: f64 = data.iter()
                            .map(|&x| {
                                let diff = (x as f64) - mean;
                                diff * diff
                            })
                            .sum();
                        let pvariance = sum_sq_diff / (data.len() as f64);
                        pvariance.sqrt()
                    }
                }
            }

            // Additional means
            "harmonic_mean" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.harmonic_mean() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.harmonic_mean(data) → n / sum(1/x for x in data)
                parse_quote! {
                    {
                        let data = #data;
                        let sum_reciprocals: f64 = data.iter()
                            .map(|&x| 1.0 / (x as f64))
                            .sum();
                        (data.len() as f64) / sum_reciprocals
                    }
                }
            }

            "geometric_mean" => {
                if arg_exprs.len() != 1 {
                    bail!("statistics.geometric_mean() requires exactly 1 argument");
                }
                let data = &arg_exprs[0];
                // statistics.geometric_mean(data) → (product of all values) ^ (1/n)
                parse_quote! {
                    {
                        let data = #data;
                        let product: f64 = data.iter()
                            .map(|&x| x as f64)
                            .product();
                        product.powf(1.0 / (data.len() as f64))
                    }
                }
            }

            // Quantiles (simplified implementation)
            "quantiles" => {
                // quantiles can take n= parameter, but we'll support basic case
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("statistics.quantiles() requires 1-2 arguments");
                }
                let data = &arg_exprs[0];
                let n = if arg_exprs.len() == 2 {
                    &arg_exprs[1]
                } else {
                    // Default n=4 (quartiles)
                    &parse_quote! { 4 }
                };
                // Simplified quantiles implementation
                parse_quote! {
                    {
                        let mut sorted = #data.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let n = #n as usize;
                        let mut result = Vec::new();
                        for i in 1..n {
                            let pos = (i as f64) * (sorted.len() as f64) / (n as f64);
                            let idx = pos.floor() as usize;
                            if idx < sorted.len() {
                                result.push(sorted[idx] as f64);
                            }
                        }
                        result
                    }
                }
            }

            _ => {
                bail!("statistics.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert random module method calls
    /// Uses SmallRng with thread-local initialization for performance.
    /// RNG is initialized once per module via DEPYLER_RNG thread_local static.
    #[inline]
    pub(super) fn try_convert_random_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        // Mark that we need rand crate and SmallRng
        // The module-level DEPYLER_RNG thread_local will be generated in generate_conditional_imports
        self.ctx.require(crate::rust_generator::context::Import::Rand);
        self.ctx.require(crate::rust_generator::context::Import::SmallRng);

        // All random operations use the module-level DEPYLER_RNG thread_local
        // which is initialized once per thread when first accessed

        let result = match method {
            // Random class constructor: random.Random(seed) → SmallRng::seed_from_u64(seed)
            "Random" => {
                if arg_exprs.is_empty() {
                    // No seed - use OS entropy
                    parse_quote! { SmallRng::from_os_rng() }
                } else if arg_exprs.len() == 1 {
                    let seed = &arg_exprs[0];
                    parse_quote! { SmallRng::seed_from_u64(#seed as u64) }
                } else {
                    bail!("random.Random() takes 0 or 1 argument");
                }
            }

            // SystemRandom class constructor: random.SystemRandom() → OsRng
            "SystemRandom" => {
                if !arg_exprs.is_empty() {
                    bail!("random.SystemRandom() takes no arguments");
                }
                parse_quote! { rand::rngs::OsRng }
            }

            // Basic random generation
            "random" => {
                if !arg_exprs.is_empty() {
                    bail!("random.random() takes no arguments");
                }
                // random.random() → use module-level SmallRng
                parse_quote! {
                    DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>())
                }
            }

            // Integer range functions
            "randint" => {
                if arg_exprs.len() != 2 {
                    bail!("random.randint() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // random.randint(a, b) → SmallRng.gen_range(a..=b)
                // Python's randint is inclusive on both ends
                parse_quote! {
                    DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(#a..=#b))
                }
            }

            "randrange" => {
                // randrange can take 1, 2, or 3 arguments (like range)
                if arg_exprs.is_empty() || arg_exprs.len() > 3 {
                    bail!("random.randrange() requires 1-3 arguments");
                }

                if arg_exprs.len() == 1 {
                    // randrange(stop) → gen_range(0..stop)
                    let stop = &arg_exprs[0];
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(0..#stop))
                    }
                } else if arg_exprs.len() == 2 {
                    // randrange(start, stop) → gen_range(start..stop)
                    let start = &arg_exprs[0];
                    let stop = &arg_exprs[1];
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(#start..#stop))
                    }
                } else {
                    // randrange(start, stop, step) - stepped range selection
                    let start = &arg_exprs[0];
                    let stop = &arg_exprs[1];
                    let step = &arg_exprs[2];
                    parse_quote! {
                        {
                            let start = #start;
                            let stop = #stop;
                            let step = #step;
                            let num_steps = ((stop - start) / step).max(0);
                            let offset = DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(0..num_steps));
                            start + offset * step
                        }
                    }
                }
            }

            // Float range function
            "uniform" => {
                if arg_exprs.len() != 2 {
                    bail!("random.uniform() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                parse_quote! {
                    DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range((#a as f64)..=(#b as f64)))
                }
            }

            // Sequence functions
            "choice" => {
                if arg_exprs.len() != 1 {
                    bail!("random.choice() requires exactly 1 argument");
                }
                let seq = &arg_exprs[0];
                self.ctx.require(crate::rust_generator::context::Import::IndexedRandom);
                parse_quote! {
                    DEPYLER_RNG.with(|rng| #seq.choose(&mut *rng.borrow_mut()).cloned().unwrap())
                }
            }

            "shuffle" => {
                if arg_exprs.len() != 1 {
                    bail!("random.shuffle() requires exactly 1 argument");
                }
                let seq = &arg_exprs[0];
                self.ctx.require(crate::rust_generator::context::Import::SliceRandom);
                parse_quote! {
                    DEPYLER_RNG.with(|rng| #seq.shuffle(&mut *rng.borrow_mut()))
                }
            }

            "sample" => {
                if arg_exprs.len() != 2 {
                    bail!("random.sample() requires exactly 2 arguments");
                }
                let seq = &arg_exprs[0];
                let k = &arg_exprs[1];
                self.ctx.require(crate::rust_generator::context::Import::IndexedRandom);
                parse_quote! {
                    DEPYLER_RNG.with(|rng| {
                        #seq.choose_multiple(&mut *rng.borrow_mut(), #k as usize)
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                }
            }

            "choices" => {
                if arg_exprs.is_empty() {
                    bail!("random.choices() requires at least 1 argument");
                }
                let seq = &arg_exprs[0];
                let k = if arg_exprs.len() > 1 {
                    &arg_exprs[1]
                } else {
                    &parse_quote! { 1 }
                };
                self.ctx.require(crate::rust_generator::context::Import::IndexedRandom);
                parse_quote! {
                    DEPYLER_RNG.with(|rng| {
                        let mut rng = rng.borrow_mut();
                        (0..#k)
                            .map(|_| #seq.choose(&mut *rng).cloned().unwrap())
                            .collect::<Vec<_>>()
                    })
                }
            }

            // Distribution functions
            "gauss" | "normalvariate" => {
                if arg_exprs.len() != 2 {
                    bail!("random.{}() requires exactly 2 arguments", method);
                }
                let mu = &arg_exprs[0];
                let sigma = &arg_exprs[1];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let normal = rand_distr::Normal::new(#mu as f64, #sigma as f64).unwrap();
                        DEPYLER_RNG.with(|rng| normal.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "expovariate" => {
                if arg_exprs.len() != 1 {
                    bail!("random.expovariate() requires exactly 1 argument");
                }
                let lambd = &arg_exprs[0];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let exp = rand_distr::Exp::new(#lambd as f64).unwrap();
                        DEPYLER_RNG.with(|rng| exp.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "betavariate" => {
                if arg_exprs.len() != 2 {
                    bail!("random.betavariate() requires exactly 2 arguments");
                }
                let alpha = &arg_exprs[0];
                let beta = &arg_exprs[1];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let beta_dist = rand_distr::Beta::new(#alpha as f64, #beta as f64).unwrap();
                        DEPYLER_RNG.with(|rng| beta_dist.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "gammavariate" => {
                if arg_exprs.len() != 2 {
                    bail!("random.gammavariate() requires exactly 2 arguments");
                }
                let alpha = &arg_exprs[0];
                let beta = &arg_exprs[1];
                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let gamma = rand_distr::Gamma::new(#alpha as f64, #beta as f64).unwrap();
                        DEPYLER_RNG.with(|rng| gamma.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            // Seed function - resets the module-level RNG
            "seed" => {
                if arg_exprs.len() > 1 {
                    bail!("random.seed() requires 0 or 1 argument");
                }
                if arg_exprs.is_empty() {
                    // seed() with no args - use OS entropy
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| *rng.borrow_mut() = SmallRng::from_os_rng())
                    }
                } else {
                    let seed_val = &arg_exprs[0];
                    parse_quote! {
                        DEPYLER_RNG.with(|rng| *rng.borrow_mut() = SmallRng::seed_from_u64(#seed_val as u64))
                    }
                }
            }

            // Get/Set state (complex, simplified implementation)
            "getstate" => {
                bail!(
                    "random.getstate() not supported - Rust RNG state management differs from Python"
                );
            }
            "setstate" => {
                bail!(
                    "random.setstate() not supported - Rust RNG state management differs from Python"
                );
            }

            "triangular" => {
                if arg_exprs.len() < 2 || arg_exprs.len() > 3 {
                    bail!("random.triangular() requires 2 or 3 arguments");
                }
                let low = &arg_exprs[0];
                let high = &arg_exprs[1];
                let mode = if arg_exprs.len() == 3 {
                    &arg_exprs[2]
                } else {
                    &parse_quote! { ((#low + #high) / 2.0) }
                };

                parse_quote! {
                    {
                        use rand::distributions::Distribution;
                        let triangular = rand_distr::Triangular::new(
                            #low as f64,
                            #high as f64,
                            #mode as f64
                        ).unwrap();
                        DEPYLER_RNG.with(|rng| triangular.sample(&mut *rng.borrow_mut()))
                    }
                }
            }

            "randbytes" => {
                if arg_exprs.len() != 1 {
                    bail!("random.randbytes() requires exactly 1 argument");
                }
                let n = &arg_exprs[0];

                parse_quote! {
                    {
                        let n = #n as usize;
                        DEPYLER_RNG.with(|rng| {
                            let mut rng = rng.borrow_mut();
                            (0..n).map(|_| rng.gen::<u8>()).collect::<Vec<u8>>()
                        })
                    }
                }
            }

            _ => {
                bail!("random.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert math module method calls
    #[inline]
    pub(super) fn try_convert_math_method(
        &mut self,
        method: &str,
        args: &[HirExpr],
    ) -> Result<Option<syn::Expr>> {
        // Convert arguments first
        let arg_exprs: Vec<syn::Expr> = args
            .iter()
            .map(|arg| arg.to_rust_expr(self.ctx))
            .collect::<Result<Vec<_>>>()?;

        let result = match method {
            // Trigonometric functions - all take one f64 argument
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                parse_quote! { (#arg as f64).#method_ident() }
            }

            // atan2 takes two arguments
            "atan2" => {
                if arg_exprs.len() != 2 {
                    bail!("math.atan2() requires exactly 2 arguments");
                }
                let y = &arg_exprs[0];
                let x = &arg_exprs[1];
                parse_quote! { (#y as f64).atan2(#x as f64) }
            }

            // Hyperbolic functions
            "sinh" | "cosh" | "tanh" | "asinh" | "acosh" | "atanh" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                parse_quote! { (#arg as f64).#method_ident() }
            }

            // Power and logarithmic functions
            "sqrt" | "exp" | "ln" | "log2" | "log10" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_name = if method == "ln" { "ln" } else { method };
                let method_ident = syn::Ident::new(method_name, proc_macro2::Span::call_site());
                parse_quote! { (#arg as f64).#method_ident() }
            }

            // log() can take 1 or 2 arguments (log(x) or log(x, base))
            "log" => {
                if arg_exprs.len() == 1 {
                    let arg = &arg_exprs[0];
                    // log(x) defaults to natural logarithm
                    parse_quote! { (#arg as f64).ln() }
                } else if arg_exprs.len() == 2 {
                    let x = &arg_exprs[0];
                    let base = &arg_exprs[1];
                    // log(x, base) → x.log(base)
                    parse_quote! { (#x as f64).log(#base as f64) }
                } else {
                    bail!("math.log() requires 1 or 2 arguments");
                }
            }

            // pow() takes two arguments
            "pow" => {
                if arg_exprs.len() != 2 {
                    bail!("math.pow() requires exactly 2 arguments");
                }
                let base = &arg_exprs[0];
                let exp = &arg_exprs[1];
                // Use powf for floating point exponents
                parse_quote! { (#base as f64).powf(#exp as f64) }
            }

            // Rounding functions
            "ceil" | "floor" | "trunc" | "round" => {
                if arg_exprs.len() != 1 {
                    bail!("math.{}() requires exactly 1 argument", method);
                }
                let arg = &arg_exprs[0];
                let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
                // These return f64 in Rust, but Python's math.ceil/floor return int
                // We'll cast to i32 for ceil and floor
                if method == "ceil" || method == "floor" {
                    parse_quote! { (#arg as f64).#method_ident() as i32 }
                } else {
                    parse_quote! { (#arg as f64).#method_ident() }
                }
            }

            // Absolute value
            "fabs" => {
                if arg_exprs.len() != 1 {
                    bail!("math.fabs() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).abs() }
            }

            // copysign
            "copysign" => {
                if arg_exprs.len() != 2 {
                    bail!("math.copysign() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                parse_quote! { (#x as f64).copysign(#y as f64) }
            }

            // Degree/Radian conversion
            "degrees" => {
                if arg_exprs.len() != 1 {
                    bail!("math.degrees() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).to_degrees() }
            }
            "radians" => {
                if arg_exprs.len() != 1 {
                    bail!("math.radians() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).to_radians() }
            }

            // Special value checks
            "isnan" => {
                if arg_exprs.len() != 1 {
                    bail!("math.isnan() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).is_nan() }
            }
            "isinf" => {
                if arg_exprs.len() != 1 {
                    bail!("math.isinf() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).is_infinite() }
            }
            "isfinite" => {
                if arg_exprs.len() != 1 {
                    bail!("math.isfinite() requires exactly 1 argument");
                }
                let arg = &arg_exprs[0];
                parse_quote! { (#arg as f64).is_finite() }
            }

            // GCD - requires num crate for integers
            "gcd" => {
                if arg_exprs.len() != 2 {
                    bail!("math.gcd() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // For now, implement simple Euclidean algorithm inline
                // NOTE: Use num_integer::gcd crate for better performance ()
                parse_quote! {
                    {
                        let mut a = (#a as i64).abs();
                        let mut b = (#b as i64).abs();
                        while b != 0 {
                            let temp = b;
                            b = a % b;
                            a = temp;
                        }
                        a as i32
                    }
                }
            }

            // Factorial - compute inline for now
            "factorial" => {
                if arg_exprs.len() != 1 {
                    bail!("math.factorial() requires exactly 1 argument");
                }
                let n = &arg_exprs[0];
                parse_quote! {
                    {
                        let n = #n as i32;
                        let mut result = 1i64;
                        for i in 1..=n {
                            result *= i as i64;
                        }
                        result as i32
                    }
                }
            }

            // ldexp and frexp - less common, basic implementation
            "ldexp" => {
                if arg_exprs.len() != 2 {
                    bail!("math.ldexp() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let i = &arg_exprs[1];
                // ldexp(x, i) = x * 2^i
                parse_quote! { (#x as f64) * 2.0f64.powi(#i as i32) }
            }

            "frexp" => {
                // frexp returns (mantissa, exponent) where x = mantissa * 2^exponent
                // Rust doesn't have this built-in, so we'll implement it
                if arg_exprs.len() != 1 {
                    bail!("math.frexp() requires exactly 1 argument");
                }
                let x = &arg_exprs[0];
                parse_quote! {
                    {
                        let x = #x as f64;
                        if x == 0.0 {
                            (0.0, 0)
                        } else {
                            let exp = x.abs().log2().floor() as i32 + 1;
                            let mantissa = x / 2.0f64.powi(exp);
                            (mantissa, exp)
                        }
                    }
                }
            }

            // LCM - least common multiple
            "lcm" => {
                if arg_exprs.len() != 2 {
                    bail!("math.lcm() requires exactly 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // lcm(a, b) = abs(a * b) / gcd(a, b)
                parse_quote! {
                    {
                        let a = (#a as i64).abs();
                        let b = (#b as i64).abs();
                        if a == 0 || b == 0 {
                            0
                        } else {
                            // Compute GCD first
                            let mut gcd_a = a;
                            let mut gcd_b = b;
                            while gcd_b != 0 {
                                let temp = gcd_b;
                                gcd_b = gcd_a % gcd_b;
                                gcd_a = temp;
                            }
                            let gcd = gcd_a;
                            ((a / gcd) * b) as i32
                        }
                    }
                }
            }

            // isclose - floating point comparison with tolerance
            "isclose" => {
                if arg_exprs.len() < 2 {
                    bail!("math.isclose() requires at least 2 arguments");
                }
                let a = &arg_exprs[0];
                let b = &arg_exprs[1];
                // Default rel_tol=1e-09, abs_tol=0.0
                parse_quote! {
                    {
                        let a = #a as f64;
                        let b = #b as f64;
                        let rel_tol = 1e-9;
                        let abs_tol = 0.0;
                        let diff = (a - b).abs();
                        diff <= abs_tol.max(rel_tol * a.abs().max(b.abs()))
                    }
                }
            }

            // modf - split into fractional and integer parts
            "modf" => {
                if arg_exprs.len() != 1 {
                    bail!("math.modf() requires exactly 1 argument");
                }
                let x = &arg_exprs[0];
                parse_quote! {
                    {
                        let x = #x as f64;
                        let int_part = x.trunc();
                        let frac_part = x - int_part;
                        (frac_part, int_part)
                    }
                }
            }

            // fmod - floating point remainder
            "fmod" => {
                if arg_exprs.len() != 2 {
                    bail!("math.fmod() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                parse_quote! { (#x as f64) % (#y as f64) }
            }

            // hypot - Euclidean distance (hypotenuse)
            "hypot" => {
                if arg_exprs.len() != 2 {
                    bail!("math.hypot() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                parse_quote! { (#x as f64).hypot(#y as f64) }
            }

            // dist - distance between two points
            "dist" => {
                if arg_exprs.len() != 2 {
                    bail!("math.dist() requires exactly 2 arguments (two points)");
                }
                let p = &arg_exprs[0];
                let q = &arg_exprs[1];
                // Simplified: assume 2D points
                parse_quote! {
                    {
                        let p = #p;
                        let q = #q;
                        let dx = p[0] - q[0];
                        let dy = p[1] - q[1];
                        ((dx * dx + dy * dy) as f64).sqrt()
                    }
                }
            }

            //
            "remainder" => {
                if arg_exprs.len() != 2 {
                    bail!("math.remainder() requires exactly 2 arguments");
                }
                let x = &arg_exprs[0];
                let y = &arg_exprs[1];
                // IEEE remainder: x - n*y where n is closest integer to x/y
                parse_quote! {
                    {
                        let x = #x as f64;
                        let y = #y as f64;
                        let n = (x / y).round();
                        x - n * y
                    }
                }
            }

            //
            "comb" => {
                if arg_exprs.len() != 2 {
                    bail!("math.comb() requires exactly 2 arguments");
                }
                let n = &arg_exprs[0];
                let k = &arg_exprs[1];
                parse_quote! {
                    {
                        let n = #n as i64;
                        let k = #k as i64;
                        if k > n || k < 0 { 0 } else {
                            let k = if k > n - k { n - k } else { k };
                            let mut result = 1i64;
                            for i in 0..k {
                                result = result * (n - i) / (i + 1);
                            }
                            result as i32
                        }
                    }
                }
            }

            //
            "perm" => {
                if arg_exprs.is_empty() || arg_exprs.len() > 2 {
                    bail!("math.perm() requires 1 or 2 arguments");
                }
                let n = &arg_exprs[0];
                let k = if arg_exprs.len() == 2 {
                    &arg_exprs[1]
                } else {
                    n
                };
                parse_quote! {
                    {
                        let n = #n as i64;
                        let k = #k as i64;
                        if k > n || k < 0 { 0 } else {
                            let mut result = 1i64;
                            for i in 0..k {
                                result *= n - i;
                            }
                            result as i32
                        }
                    }
                }
            }

            //
            "expm1" => {
                if arg_exprs.len() != 1 {
                    bail!("math.expm1() requires exactly 1 argument");
                }
                let x = &arg_exprs[0];
                parse_quote! { (#x as f64).exp_m1() }
            }

            _ => {
                bail!("math.{} not implemented yet", method);
            }
        };

        Ok(Some(result))
    }

    /// Try to convert module method call (e.g., os.getcwd())
    #[inline]
    pub(super) fn try_convert_module_method(
        &mut self,
        object: &HirExpr,
        method: &str,
        args: &[HirExpr],
        kwargs: &[(String, HirExpr)],
    ) -> Result<Option<syn::Expr>> {
        // os.environ.get('VAR') → std::env::var('VAR').ok()
        // os.environ.get('VAR', 'default') → std::env::var('VAR').unwrap_or_else(|_| 'default'.to_string())
        if let HirExpr::Attribute { value, attr } = object {
            if let HirExpr::Var(module_name) = &**value {
                if module_name == "os" && attr == "environ" {
                    return self.try_convert_os_environ_method(method, args);
                }
                // os.path.exists(path) → Path::new(path).exists()
                // os.path.join(a, b) → PathBuf::from(a).join(b)
                if module_name == "os" && attr == "path" {
                    return self.try_convert_os_path_method(method, args);
                }
            }
        }

        if let HirExpr::Var(module_name) = object {
            // If this variable is declared as a local variable, don't treat it as a module
            if self.ctx.is_declared(module_name) {
                return Ok(None);
            }

            if module_name == "struct" {
                return self.try_convert_struct_method(method, args);
            }

            //
            // math.sqrt(x) → x.sqrt()
            // math.sin(x) → x.sin()
            // math.pow(x, y) → x.powf(y)
            if module_name == "math" {
                return self.try_convert_math_method(method, args);
            }

            //
            // random.random() → thread_rng().gen()
            // random.randint(a, b) → thread_rng().gen_range(a..=b)
            if module_name == "random" {
                return self.try_convert_random_method(method, args);
            }

            //
            // statistics.mean(data) → inline calculation
            // statistics.median(data) → sorted median calculation
            if module_name == "statistics" {
                return self.try_convert_statistics_method(method, args);
            }

            //
            // Fraction(1, 2) → Ratio::new(1, 2)
            // f.limit_denominator(100) → approximate with max denominator
            if module_name == "fractions" {
                return self.try_convert_fractions_method(method, args);
            }

            //
            // Path("/foo/bar").exists() → PathBuf::from("/foo/bar").exists()
            // Path("/foo").join("bar") → PathBuf::from("/foo").join("bar")
            if module_name == "pathlib" {
                return self.try_convert_pathlib_method(method, args);
            }

            //
            // datetime.datetime.now() → Local::now().naive_local()
            // datetime.datetime.utcnow() → Utc::now().naive_utc()
            // datetime.date.today() → Local::now().date_naive()
            if module_name == "datetime" {
                return self.try_convert_datetime_method(method, args);
            }

            //
            // decimal.Decimal("123.45") → Decimal::from_str("123.45")
            // Note: Decimal() constructor is handled separately in convert_call
            if module_name == "decimal" {
                return self.try_convert_decimal_method(method, args);
            }

            //
            // json.dumps(obj) → serde_json::to_string(&obj)
            // json.loads(s) → serde_json::from_str(&s)
            if module_name == "json" {
                return self.try_convert_json_method(method, args);
            }

            //
            // unicodedata.normalize("NFC", s) → s.nfc().collect::<String>()
            if module_name == "unicodedata" {
                return self.try_convert_unicodedata_method(method, args);
            }

            //
            if module_name == "re" {
                return self.try_convert_re_method(method, args);
            }

            //
            if module_name == "string" {
                return self.try_convert_string_method(method, args);
            }

            //
            if module_name == "time" {
                return self.try_convert_time_method(method, args);
            }

            //
            if module_name == "csv" {
                return self.try_convert_csv_method(method, args, kwargs);
            }

            // Must be checked before os.path to handle non-path os functions
            if module_name == "os" {
                if let Some(result) = self.try_convert_os_method(method, args)? {
                    return Ok(Some(result));
                }
                // Fall through to os.path handler if method not recognized
            }

            //
            // Only match the actual module "os.path", not variables named "path"
            // Variables named "path" are typically PathBuf instances from Path() constructor
            if module_name == "os.path" {
                return self.try_convert_os_path_method(method, args);
            }

            //
            if module_name == "base64" {
                return self.try_convert_base64_method(method, args);
            }

            //
            if module_name == "secrets" {
                return self.try_convert_secrets_method(method, args);
            }

            //
            if module_name == "hashlib" {
                return self.try_convert_hashlib_method(method, args);
            }

            //
            if module_name == "uuid" {
                return self.try_convert_uuid_method(method, args);
            }

            //
            if module_name == "hmac" {
                return self.try_convert_hmac_method(method, args);
            }

            if module_name == "platform" {
                return self.try_convert_platform_method(method, args);
            }

            //
            if module_name == "binascii" {
                return self.try_convert_binascii_method(method, args);
            }

            //
            if module_name == "urllib.parse" || module_name == "parse" {
                return self.try_convert_urllib_parse_method(method, args);
            }

            //
            if module_name == "fnmatch" {
                return self.try_convert_fnmatch_method(method, args);
            }

            //
            if module_name == "shlex" {
                return self.try_convert_shlex_method(method, args);
            }

            //
            if module_name == "textwrap" {
                return self.try_convert_textwrap_method(method, args);
            }

            //
            if module_name == "bisect" {
                return self.try_convert_bisect_method(method, args);
            }

            //
            if module_name == "heapq" {
                return self.try_convert_heapq_method(method, args);
            }

            //
            if module_name == "copy" {
                return self.try_convert_copy_method(method, args);
            }

            //
            if module_name == "itertools" {
                return self.try_convert_itertools_method(method, args, kwargs);
            }

            //
            if module_name == "functools" {
                return self.try_convert_functools_method(method, args);
            }

            //
            if module_name == "warnings" {
                return self.try_convert_warnings_method(method, args);
            }

            //
            if module_name == "sys" {
                return self.try_convert_sys_method(method, args);
            }

            //
            if module_name == "pickle" {
                return self.try_convert_pickle_method(method, args);
            }

            //
            if module_name == "pprint" {
                return self.try_convert_pprint_method(method, args);
            }

            let module_info = self
                .ctx
                .imported_modules
                .get(module_name)
                .and_then(|mapping| {
                    mapping
                        .item_map
                        .get(method)
                        .map(|rust_name| (mapping.rust_path.clone(), rust_name.clone()))
                });

            if let Some((rust_path, rust_name)) = module_info {
                // Convert args
                let arg_exprs: Vec<syn::Expr> = args
                    .iter()
                    .map(|arg| arg.to_rust_expr(self.ctx))
                    .collect::<Result<Vec<_>>>()?;

                // Python: math.sqrt(x) → Rust: x.sqrt() or f64::sqrt(x)
                if module_name == "math" && !arg_exprs.is_empty() {
                    let receiver = &arg_exprs[0];
                    let method_ident = syn::Ident::new(&rust_name, proc_macro2::Span::call_site());
                    return Ok(Some(parse_quote! { (#receiver).#method_ident() }));
                }

                // Build the Rust function path using the module's rust_path
                let path_parts: Vec<&str> = rust_name.split("::").collect();

                // Start with the module's rust_path instead of hardcoded "std"
                let base_path: syn::Path =
                    syn::parse_str(&rust_path).unwrap_or_else(|_| parse_quote! { std });
                let mut path = quote! { #base_path };

                for part in path_parts {
                    let part_ident = syn::Ident::new(part, proc_macro2::Span::call_site());
                    path = quote! { #path::#part_ident };
                }

                // Special handling for certain functions
                let result = match rust_name.as_str() {
                    "env::current_dir" => {
                        // current_dir returns Result<PathBuf>, we need to convert to String
                        parse_quote! {
                            #path().unwrap().to_string_lossy().to_string()
                        }
                    }
                    "Regex::new" => {
                        // re.compile(pattern) -> Regex::new(pattern)
                        if arg_exprs.is_empty() {
                            bail!("re.compile() requires a pattern argument");
                        }
                        let pattern = &arg_exprs[0];
                        parse_quote! {
                            regex::Regex::new(#pattern).unwrap()
                        }
                    }
                    _ => {
                        if arg_exprs.is_empty() {
                            parse_quote! { #path() }
                        } else {
                            parse_quote! { #path(#(#arg_exprs),*) }
                        }
                    }
                };
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

}
