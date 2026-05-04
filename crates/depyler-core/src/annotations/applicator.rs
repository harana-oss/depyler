use regex::Regex;

use crate::annotations::types::{
    BoundsChecking, ErrorStrategy, InteriorMutability, OwnershipModel, PanicBehavior,
    PerformanceHint, ServiceType, StringStrategy, ThreadSafety, TranspilationAnnotations,
};

// ---------------------------------------------------------------------------
// AnnotationValidator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AnnotationValidator;

impl AnnotationValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validates the consistency of annotation settings.
    ///
    /// # Errors
    ///
    /// Returns a vector of error messages if any validation rules are violated.
    pub fn validate(&self, annotations: &TranspilationAnnotations) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate conflicting strategies
        if annotations.string_strategy == StringStrategy::ZeroCopy
            && annotations.ownership_model == OwnershipModel::Owned
        {
            errors
                .push("Zero-copy string strategy conflicts with owned ownership model".to_string());
        }

        if annotations.thread_safety == ThreadSafety::Required
            && annotations.interior_mutability == InteriorMutability::RefCell
        {
            errors.push("RefCell is not thread-safe, use Arc<Mutex<T>> instead".to_string());
        }

        if annotations.panic_behavior == PanicBehavior::ReturnError
            && annotations.error_strategy == ErrorStrategy::Panic
        {
            errors.push("Conflicting panic behavior and error strategy".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn suggest_improvements(&self, annotations: &TranspilationAnnotations) -> Vec<String> {
        let mut suggestions = Vec::new();

        if annotations
            .performance_hints
            .contains(&PerformanceHint::PerformanceCritical)
        {
            suggestions
                .push("Ensure all hot paths are covered for performance critical code".to_string());
        }

        if annotations.thread_safety == ThreadSafety::Required
            && annotations.ownership_model != OwnershipModel::Shared
        {
            suggestions
                .push("Consider using ownership = \"shared\" for thread-safe code".to_string());
        }

        if annotations.service_type == Some(ServiceType::WebApi)
            && !annotations
                .performance_hints
                .contains(&PerformanceHint::OptimizeForLatency)
        {
            suggestions
                .push("Consider adding optimization_hint = \"latency\" for web APIs".to_string());
        }

        suggestions
    }
}

// ---------------------------------------------------------------------------
// AnnotationExtractor
// ---------------------------------------------------------------------------

/// Extracts raw annotation text from Python source code surrounding function
/// and class definitions, ready for `AnnotationParser::parse_annotations`.
#[derive(Debug, Clone)]
pub struct AnnotationExtractor {
    function_pattern: Regex,
    class_pattern: Regex,
}

impl Default for AnnotationExtractor {
    fn default() -> Self {
        Self {
            function_pattern: Regex::new(r"(?m)^def\s+(\w+)\s*\(").unwrap(),
            class_pattern: Regex::new(r"(?m)^class\s+(\w+)\s*[\(:]").unwrap(),
        }
    }
}

impl AnnotationExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extracts annotations for a specific function from source code.
    ///
    /// # Panics
    ///
    /// Panics if regex patterns fail to match (should not happen with valid regex).
    pub fn extract_function_annotations(
        &self,
        source: &str,
        function_name: &str,
    ) -> Option<String> {
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if let Some(captures) = self.function_pattern.captures(line) {
                if captures.get(1).unwrap().as_str() == function_name {
                    // Collect annotations above the function
                    let mut annotations = Vec::new();
                    let mut j = i.saturating_sub(1);

                    while j < i && (lines[j].trim().starts_with('#') || lines[j].trim().is_empty())
                    {
                        if lines[j].contains("@quantsim:") {
                            annotations.push(lines[j]);
                        }
                        if j == 0 {
                            break;
                        }
                        j = j.saturating_sub(1);
                    }

                    if !annotations.is_empty() {
                        annotations.reverse();
                        return Some(annotations.join("\n"));
                    }
                }
            }
        }
        None
    }

    /// Extracts annotations for a specific class from source code.
    ///
    /// # Panics
    ///
    /// Panics if regex patterns fail to match (should not happen with valid regex).
    pub fn extract_class_annotations(&self, source: &str, class_name: &str) -> Option<String> {
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if let Some(captures) = self.class_pattern.captures(line) {
                if captures.get(1).unwrap().as_str() == class_name {
                    // Collect annotations above the class (may be above decorators)
                    let mut annotations = Vec::new();
                    let mut j = i.saturating_sub(1);

                    // Walk backwards through comments, empty lines, and decorators
                    while j < i {
                        let trimmed = lines[j].trim();
                        if trimmed.starts_with('#')
                            || trimmed.is_empty()
                            || trimmed.starts_with('@')
                        {
                            if trimmed.contains("@quantsim:") {
                                annotations.push(lines[j]);
                            }
                            if j == 0 {
                                break;
                            }
                            j = j.saturating_sub(1);
                        } else {
                            // Hit a non-comment, non-empty, non-decorator line - stop
                            break;
                        }
                    }

                    if !annotations.is_empty() {
                        annotations.reverse();
                        return Some(annotations.join("\n"));
                    }
                }
            }
        }
        None
    }
}
