/// Migration suggestions for Python-to-Rust idiom transitions
use crate::hir::{HirExpr, HirFunction, HirModule, HirStmt, Type};
use colored::Colorize;

pub struct MigrationAnalyzer {
    suggestions: Vec<MigrationSuggestion>,
    config: MigrationConfig,
}

#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub suggest_iterators: bool,
    pub suggest_error_handling: bool,
    pub suggest_ownership: bool,
    pub suggest_performance: bool,
    pub verbosity: u8,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            suggest_iterators: true,
            suggest_error_handling: true,
            suggest_ownership: true,
            suggest_performance: true,
            verbosity: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MigrationSuggestion {
    pub category: SuggestionCategory,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub python_example: String,
    pub rust_suggestion: String,
    pub notes: Vec<String>,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionCategory {
    Iterator,
    ErrorHandling,
    Ownership,
    Performance,
    TypeSystem,
    Concurrency,
    ApiDesign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Important,
    Critical,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub function: String,
    pub line: usize,
}

impl MigrationAnalyzer {
    pub fn new(config: MigrationConfig) -> Self {
        Self {
            suggestions: Vec::new(),
            config,
        }
    }

    /// Analyze a program and generate migration suggestions
    ///
    /// # Example
    /// ```
    /// use depyler_core::migration_suggestions::{MigrationAnalyzer, MigrationConfig};
    /// use depyler_core::hir::HirModule;
    ///
    /// let mut analyzer = MigrationAnalyzer::new(MigrationConfig::default());
    /// let program = HirModule {
    ///     imports: vec![],
    ///     functions: vec![],
    ///     classes: vec![],
    /// };
    /// let suggestions = analyzer.analyze_program(&program);
    /// assert!(suggestions.is_empty());
    /// ```
    pub fn analyze_program(&mut self, program: &HirModule) -> Vec<MigrationSuggestion> {
        self.suggestions.clear();

        for func in &program.functions {
            self.analyze_function(func);
        }

        self.suggestions.sort_by(|a, b| b.severity.cmp(&a.severity));

        self.suggestions.clone()
    }

    fn analyze_function(&mut self, func: &HirFunction) {
        self.check_function_patterns(func);

        for (idx, stmt) in func.body.iter().enumerate() {
            self.analyze_stmt(stmt, func, idx);
        }
    }

    fn check_function_patterns(&mut self, func: &HirFunction) {
        if self.has_accumulator_pattern(&func.body) {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::Iterator,
                severity: Severity::Warning,
                title: format!("Consider using iterator methods in '{}'", func.name),
                description: "This function uses an accumulator pattern that could be replaced with iterator methods"
                    .to_string(),
                python_example: r#"result = []
for item in items:
    if condition(item):
        result.append(transform(item))"#
                    .to_string(),
                rust_suggestion: r#"let result: Vec<_> = items.iter()
    .filter(|item| condition(item))
    .map(|item| transform(item))
    .collect();"#
                    .to_string(),
                notes: vec![
                    "Iterator chains are more idiomatic and often more efficient".to_string(),
                    "They avoid intermediate allocations".to_string(),
                ],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line: 0,
                }),
            });
        }

        // Check for error handling patterns
        if self.uses_none_as_error(&func.body, &func.ret_type) {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::ErrorHandling,
                severity: Severity::Important,
                title: format!("Use Result<T, E> instead of Option<T> for errors in '{}'", func.name),
                description: "Returning None for errors loses error information".to_string(),
                python_example: r#"def process(data):
    if not valid(data):
        return None
    return result"#
                    .to_string(),
                rust_suggestion: r#"fn process(data: &Data) -> Result<T, ProcessError> {
    if !valid(data) {
        return Err(ProcessError::InvalidData);
    }
    Ok(result)
}"#
                .to_string(),
                notes: vec![
                    "Result provides rich error information".to_string(),
                    "Errors can be propagated with the ? operator".to_string(),
                ],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line: 0,
                }),
            });
        }

        if self.has_mutable_parameter_pattern(func) {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::Ownership,
                severity: Severity::Important,
                title: format!("Consider ownership transfer or mutable reference in '{}'", func.name),
                description: "This function appears to modify its parameters".to_string(),
                python_example: r#"def modify_list(lst):
    lst.append(42)
    return lst"#
                    .to_string(),
                rust_suggestion: r#"// Option 1: Take mutable reference
fn modify_list(lst: &mut Vec<i32>) {
    lst.push(42);
}

// Option 2: Take ownership and return
fn modify_list(mut lst: Vec<i32>) -> Vec<i32> {
    lst.push(42);
    lst
}"#
                .to_string(),
                notes: vec![
                    "Rust's ownership system requires explicit mutability".to_string(),
                    "Choose based on whether callers need the original".to_string(),
                ],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line: 0,
                }),
            });
        }
    }

    fn analyze_stmt(&mut self, stmt: &HirStmt, func: &HirFunction, line: usize) {
        match stmt {
            HirStmt::For { target, iter, body } => {
                self.analyze_for_loop(target, iter, body, func, line);
            }
            HirStmt::While { condition, body } => {
                self.analyze_while_loop(condition, body, func, line);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_if_statement(condition, then_body, else_body, func, line);
            }
            HirStmt::Assign { target, value, .. } => {
                self.analyze_assignment(target, value, func, line);
            }
            _ => {}
        }
    }

    fn analyze_for_loop(
        &mut self,
        _target: &crate::hir::AssignTarget,
        iter: &HirExpr,
        body: &[HirStmt],
        func: &HirFunction,
        line: usize,
    ) {
        if let HirExpr::Call { func: fname, args, .. } = iter {
            if fname == "enumerate" && !args.is_empty() {
                self.add_suggestion(MigrationSuggestion {
                    category: SuggestionCategory::Iterator,
                    severity: Severity::Info,
                    title: "Use .enumerate() iterator method".to_string(),
                    description: "Rust's enumerate() is an iterator method, not a function".to_string(),
                    python_example: "for i, item in enumerate(items):".to_string(),
                    rust_suggestion: "for (i, item) in items.iter().enumerate() {".to_string(),
                    notes: vec!["Iterator methods are more idiomatic in Rust".to_string()],
                    location: Some(SourceLocation {
                        function: func.name.clone(),
                        line,
                    }),
                });
            }
        }

        if self.has_filter_map_pattern(body) {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::Iterator,
                severity: Severity::Warning,
                title: "Consider filter_map() for conditional transformation".to_string(),
                description: "Combining filter and map operations can be more efficient".to_string(),
                python_example: r#"result = []
for item in items:
    if condition(item):
        result.append(transform(item))"#
                    .to_string(),
                rust_suggestion: r#"let result: Vec<_> = items.iter()
    .filter_map(|item| {
        if condition(item) {
            Some(transform(item))
        } else {
            None
        }
    })
    .collect();"#
                    .to_string(),
                notes: vec!["filter_map avoids intermediate Option wrapping".to_string()],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line,
                }),
            });
        }
    }

    fn analyze_while_loop(&mut self, condition: &HirExpr, _body: &[HirStmt], func: &HirFunction, line: usize) {
        if let HirExpr::Literal(crate::hir::Literal::Bool(true)) = condition {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::Iterator,
                severity: Severity::Info,
                title: "Consider 'loop' instead of 'while true'".to_string(),
                description: "Rust has a dedicated 'loop' construct for infinite loops".to_string(),
                python_example: "while True:".to_string(),
                rust_suggestion: "loop {".to_string(),
                notes: vec![
                    "'loop' is more idiomatic and clearer in intent".to_string(),
                    "The compiler can better optimize 'loop' constructs".to_string(),
                ],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line,
                }),
            });
        }
    }

    fn analyze_if_statement(
        &mut self,
        condition: &HirExpr,
        _then_body: &[HirStmt],
        else_body: &Option<Vec<HirStmt>>,
        func: &HirFunction,
        line: usize,
    ) {
        if self.is_type_check(condition) {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::TypeSystem,
                severity: Severity::Important,
                title: "Use Rust's type system instead of runtime type checks".to_string(),
                description: "Rust's static typing eliminates the need for runtime type checks".to_string(),
                python_example: r#"if isinstance(value, str):
    process_string(value)
elif isinstance(value, int):
    process_number(value)"#
                    .to_string(),
                rust_suggestion: r#"// Use enums for sum types
enum Value {
    String(String),
    Number(i32),
}

match value {
    Value::String(s) => process_string(s),
    Value::Number(n) => process_number(n),
}"#
                .to_string(),
                notes: vec![
                    "Enums provide compile-time guarantees".to_string(),
                    "Pattern matching ensures exhaustive handling".to_string(),
                ],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line,
                }),
            });
        }

        if self.is_none_check(condition) && else_body.is_some() {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::ErrorHandling,
                severity: Severity::Warning,
                title: "Use pattern matching or if-let for Option handling".to_string(),
                description: "Rust provides ergonomic ways to handle Option values".to_string(),
                python_example: r#"if value is not None:
    process(value)
else:
    handle_none()"#
                    .to_string(),
                rust_suggestion: r#"// Option 1: if let
if let Some(v) = value {
    process(v);
} else {
    handle_none();
}

// Option 2: match
match value {
    Some(v) => process(v),
    None => handle_none(),
}"#
                .to_string(),
                notes: vec!["Pattern matching is more idiomatic and safer".to_string()],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line,
                }),
            });
        }
    }

    fn analyze_assignment(
        &mut self,
        _target: &crate::hir::AssignTarget,
        value: &HirExpr,
        func: &HirFunction,
        line: usize,
    ) {
        if let HirExpr::Call { func: fname, .. } = value {
            if fname == "list" || fname == "dict" {
                self.add_suggestion(MigrationSuggestion {
                    category: SuggestionCategory::Performance,
                    severity: Severity::Info,
                    title: "Consider using collect() for building collections".to_string(),
                    description: "Rust's collect() is more efficient than repeated push operations".to_string(),
                    python_example: "[x * 2 for x in range(10)]".to_string(),
                    rust_suggestion: "(0..10).map(|x| x * 2).collect::<Vec<_>>()".to_string(),
                    notes: vec!["collect() can optimize capacity allocation".to_string()],
                    location: Some(SourceLocation {
                        function: func.name.clone(),
                        line,
                    }),
                });
            }
        }

        if self.is_string_concatenation(value) {
            self.add_suggestion(MigrationSuggestion {
                category: SuggestionCategory::Performance,
                severity: Severity::Warning,
                title: "Use format! or String::push_str for string building".to_string(),
                description: "String concatenation with + is inefficient in Rust".to_string(),
                python_example: r#"result = ""
for item in items:
    result = result + str(item)"#
                    .to_string(),
                rust_suggestion: r#"// Option 1: format!
let result = format!("{}{}{}", a, b, c);

// Option 2: String::push_str (for loops)
let mut result = String::new();
for item in items {
    result.push_str(&item.to_string());
}"#
                .to_string(),
                notes: vec![
                    "String concatenation creates new allocations".to_string(),
                    "Use String::with_capacity() if size is known".to_string(),
                ],
                location: Some(SourceLocation {
                    function: func.name.clone(),
                    line,
                }),
            });
        }
    }

    // Helper methods for pattern detection

    fn has_accumulator_pattern(&self, body: &[HirStmt]) -> bool {
        let has_empty_list = self.has_empty_list_initialization(body);
        let has_append_in_loop = self.has_append_in_for_loop(body);
        has_empty_list && has_append_in_loop
    }

    fn has_empty_list_initialization(&self, body: &[HirStmt]) -> bool {
        body.iter().any(|stmt| {
            matches!(
                stmt,
                HirStmt::Assign {
                    value: HirExpr::List(v),
                    ..
                } if v.is_empty()
            )
        })
    }

    fn has_append_in_for_loop(&self, body: &[HirStmt]) -> bool {
        body.iter().any(|stmt| {
            if let HirStmt::For { body, .. } = stmt {
                self.contains_append_call(body)
            } else {
                false
            }
        })
    }

    fn contains_append_call(&self, body: &[HirStmt]) -> bool {
        body.iter().any(|stmt| {
            matches!(
                stmt,
                HirStmt::Expr(HirExpr::MethodCall { method, .. }) if method == "append"
            )
        })
    }

    fn uses_none_as_error(&self, body: &[HirStmt], ret_type: &Type) -> bool {
        // Check if function returns Optional and has early None returns
        if !matches!(ret_type, Type::Optional(_)) {
            return false;
        }

        for stmt in body {
            if let HirStmt::Return(Some(HirExpr::Literal(crate::hir::Literal::None))) = stmt {
                // Check if this is in an error condition (simplified check)
                return true;
            }
        }

        false
    }

    fn has_mutable_parameter_pattern(&self, func: &HirFunction) -> bool {
        func.body
            .iter()
            .any(|stmt| self.is_mutating_method_on_param(stmt, func))
    }

    fn is_mutating_method_on_param(&self, stmt: &HirStmt, func: &HirFunction) -> bool {
        if let HirStmt::Expr(HirExpr::MethodCall { object, method, .. }) = stmt {
            if let HirExpr::Var(var) = object.as_ref() {
                return self.is_param_mutated(var, method, func);
            }
        }
        false
    }

    fn is_param_mutated(&self, var: &str, method: &str, func: &HirFunction) -> bool {
        let is_parameter = func.params.iter().any(|p| p.name == var);
        let mutating_methods = ["append", "extend", "push", "insert", "remove", "clear"];
        let is_mutating = mutating_methods.contains(&method);
        is_parameter && is_mutating
    }

    fn has_filter_map_pattern(&self, body: &[HirStmt]) -> bool {
        body.iter().any(|stmt| {
            if let HirStmt::If { then_body, .. } = stmt {
                self.contains_append_call(then_body)
            } else {
                false
            }
        })
    }

    fn is_type_check(&self, expr: &HirExpr) -> bool {
        // Check for isinstance() calls
        if let HirExpr::Call { func, .. } = expr {
            return func == "isinstance";
        }
        false
    }

    fn is_none_check(&self, expr: &HirExpr) -> bool {
        // Check for "x == None" patterns (Python's is/is not would be transpiled to ==/!=)
        if let HirExpr::Binary { left: _, right, op } = expr {
            if let HirExpr::Literal(crate::hir::Literal::None) = right.as_ref() {
                return matches!(op, crate::hir::BinOp::Eq | crate::hir::BinOp::NotEq);
            }
        }
        false
    }

    fn is_string_concatenation(&self, expr: &HirExpr) -> bool {
        // Check for string + operations
        if let HirExpr::Binary {
            op: crate::hir::BinOp::Add,
            left,
            right,
        } = expr
        {
            // Simplified check - would need type info for accuracy
            return matches!(left.as_ref(), HirExpr::Var(_)) || matches!(right.as_ref(), HirExpr::Var(_));
        }
        false
    }

    fn add_suggestion(&mut self, suggestion: MigrationSuggestion) {
        self.suggestions.push(suggestion);
    }

    /// Format suggestions for display
    ///
    /// # Example
    /// ```
    /// use depyler_core::migration_suggestions::{
    ///     MigrationAnalyzer, MigrationConfig, MigrationSuggestion,
    ///     SuggestionCategory, Severity
    /// };
    ///
    /// let analyzer = MigrationAnalyzer::new(MigrationConfig::default());
    /// let output = analyzer.format_suggestions(&[]);
    /// assert!(output.contains("No migration suggestions"));
    /// ```
    pub fn format_suggestions(&self, suggestions: &[MigrationSuggestion]) -> String {
        if suggestions.is_empty() {
            return self.format_empty_suggestions();
        }

        let mut output = self.format_header();

        for (idx, suggestion) in suggestions.iter().enumerate() {
            output.push_str(&self.format_single_suggestion(suggestion, idx));
        }

        output.push_str(&self.format_summary(suggestions));
        output
    }

    fn format_empty_suggestions(&self) -> String {
        "✨ No migration suggestions found - code is already idiomatic!\n"
            .green()
            .to_string()
    }

    fn format_header(&self) -> String {
        format!("\n{}\n{}\n\n", "Migration Suggestions".bold().blue(), "═".repeat(50))
    }

    fn format_single_suggestion(&self, suggestion: &MigrationSuggestion, idx: usize) -> String {
        let mut output = String::new();

        output.push_str(&self.format_suggestion_title(suggestion, idx));
        output.push_str(&self.format_suggestion_metadata(suggestion));
        output.push_str(&self.format_suggestion_examples(suggestion));
        output.push_str(&self.format_suggestion_notes(suggestion));
        output.push('\n');

        output
    }

    fn format_suggestion_title(&self, suggestion: &MigrationSuggestion, idx: usize) -> String {
        let severity_color = Self::get_severity_color(suggestion.severity);
        format!(
            "{} {} {}\n",
            format!("[{}]", idx + 1).dimmed(),
            format!("[{:?}]", suggestion.severity).color(severity_color),
            suggestion.title.bold()
        )
    }

    fn get_severity_color(severity: Severity) -> &'static str {
        match severity {
            Severity::Critical => "red",
            Severity::Important => "yellow",
            Severity::Warning => "bright yellow",
            Severity::Info => "bright blue",
        }
    }

    fn format_suggestion_metadata(&self, suggestion: &MigrationSuggestion) -> String {
        let mut output = format!("   {} {:?}\n", "Category:".dimmed(), suggestion.category);

        output.push_str(&format!("   {} {}\n", "Why:".dimmed(), suggestion.description));

        if let Some(loc) = &suggestion.location {
            output.push_str(&format!(
                "   {} {} line {}\n",
                "Location:".dimmed(),
                loc.function,
                loc.line
            ));
        }

        output
    }

    fn format_suggestion_examples(&self, suggestion: &MigrationSuggestion) -> String {
        if self.config.verbosity == 0 {
            return String::new();
        }

        let mut output = String::new();

        output.push_str(&format!("\n   {}:\n", "Python pattern".yellow()));
        for line in suggestion.python_example.lines() {
            output.push_str(&format!("   │ {}\n", line));
        }

        output.push_str(&format!("\n   {}:\n", "Rust idiom".green()));
        for line in suggestion.rust_suggestion.lines() {
            output.push_str(&format!("   │ {}\n", line));
        }

        output
    }

    fn format_suggestion_notes(&self, suggestion: &MigrationSuggestion) -> String {
        if suggestion.notes.is_empty() || self.config.verbosity <= 1 {
            return String::new();
        }

        let mut output = format!("\n   {}:\n", "Notes".dimmed());
        for note in &suggestion.notes {
            output.push_str(&format!("   • {}\n", note.dimmed()));
        }

        output
    }

    fn format_summary(&self, suggestions: &[MigrationSuggestion]) -> String {
        let critical_count = suggestions.iter().filter(|s| s.severity == Severity::Critical).count();
        let important_count = suggestions.iter().filter(|s| s.severity == Severity::Important).count();

        format!(
            "{} {} suggestions ({} critical, {} important)\n",
            "Summary:".bold(),
            suggestions.len(),
            critical_count,
            important_count
        )
    }
}
