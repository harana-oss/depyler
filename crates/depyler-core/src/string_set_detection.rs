//! String set detection for bitflags optimization.
//!
//! Detects Python sets of strings that function as enums and can be converted to Rust bitflags.

use crate::hir::{BinOp, HirExpr, HirFunction, HirModule, HirStmt, Literal, Span};
use crate::set_mutability_analysis::MutabilityAnalyzer;
use std::collections::{HashMap, HashSet};

/// Detected string set that can become bitflags.
#[derive(Debug, Clone)]
pub struct StringSetCandidate {
    /// Original variable name.
    pub name: String,
    /// String values in the set.
    pub values: Vec<String>,
    /// Source location.
    pub span: Span,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
    /// Evidence for why this is a candidate.
    pub evidence: Vec<DetectionEvidence>,
}

/// Evidence for string set detection.
#[derive(Debug, Clone, PartialEq)]
pub enum DetectionEvidence {
    /// Variable uses CONSTANT_NAMING convention.
    ConstantNaming,
    /// Type annotation suggests enum-like usage.
    TypeAnnotation(String),
    /// Used in membership check (x in SET).
    MembershipCheck { location: Span },
    /// Used in set operation.
    SetOperation { op: SetOp, location: Span },
    /// Set is never mutated.
    NeverMutated,
    /// All elements are string literals.
    AllLiterals,
    /// Is a frozenset (inherently immutable).
    FrozenSet,
}

/// Set operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
}

/// String set detector.
#[derive(Debug, Default)]
pub struct StringSetDetector {
    candidates: HashMap<String, StringSetCandidate>,
    membership_checks: HashMap<String, Vec<Span>>,
    set_operations: HashMap<String, Vec<(SetOp, Span)>>,
}

impl StringSetDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a module for string set candidates.
    pub fn analyze_module(&mut self, module: &HirModule) -> Vec<StringSetCandidate> {
        // First pass: collect set/list definitions
        for constant in &module.constants {
            self.analyze_constant_definition(&constant.name, &constant.value);
        }

        for func in &module.functions {
            self.analyze_function(func);
        }

        // Second pass: analyze mutability using MutabilityAnalyzer
        let mut mutability_analyzer = MutabilityAnalyzer::new();
        mutability_analyzer.analyze_module(module);

        // Mark immutable candidates with NeverMutated evidence
        for (name, candidate) in self.candidates.iter_mut() {
            if mutability_analyzer.is_immutable(name).is_immutable() {
                candidate.evidence.push(DetectionEvidence::NeverMutated);
                candidate.confidence = calculate_confidence(&candidate.evidence);
            }
        }

        // Filter out candidates that ARE mutated
        self.candidates
            .retain(|name, _| mutability_analyzer.is_immutable(name).is_immutable());

        // Third pass: refine candidates with usage analysis
        self.refine_candidates();

        self.candidates.values().cloned().collect()
    }

    /// Analyze a constant definition for set literals.
    fn analyze_constant_definition(&mut self, name: &str, expr: &HirExpr) {
        if let Some(values) = self.extract_string_set_values(expr) {
            if !values.is_empty() && values.len() <= 64 {
                let mut evidence = vec![DetectionEvidence::AllLiterals];

                // Check for CONSTANT_NAMING
                if is_constant_naming(name) {
                    evidence.push(DetectionEvidence::ConstantNaming);
                }

                // Check if it's a frozenset
                if matches!(expr, HirExpr::FrozenSet { .. }) {
                    evidence.push(DetectionEvidence::FrozenSet);
                }

                let confidence = calculate_initial_confidence(&evidence);

                self.candidates.insert(
                    name.to_string(),
                    StringSetCandidate {
                        name: name.to_string(),
                        values,
                        span: Span::default(),
                        confidence,
                        evidence,
                    },
                );
            }
        }
    }

    /// Analyze a function for set usage patterns.
    fn analyze_function(&mut self, func: &HirFunction) {
        for stmt in &func.body {
            self.analyze_statement(stmt);
        }
    }

    /// Analyze a statement for set usage.
    fn analyze_statement(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                if let crate::hir::AssignTarget::Symbol(name) = target {
                    self.analyze_constant_definition(name, value);
                }
                self.analyze_expression(value);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.analyze_expression(condition);
                for s in then_body {
                    self.analyze_statement(s);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.analyze_statement(s);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                self.analyze_expression(condition);
                for s in body {
                    self.analyze_statement(s);
                }
            }
            HirStmt::For { iter, body, .. } => {
                self.analyze_expression(iter);
                for s in body {
                    self.analyze_statement(s);
                }
            }
            HirStmt::Return(Some(expr)) => {
                self.analyze_expression(expr);
            }
            HirStmt::Expr(expr) => {
                self.analyze_expression(expr);
            }
            _ => {}
        }
    }

    /// Analyze an expression for set membership checks and operations.
    fn analyze_expression(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::Binary { op, left, right } => {
                // Check for membership: x in SET
                if matches!(op, BinOp::In) {
                    if let HirExpr::Var(set_name) = right.as_ref() {
                        self.membership_checks
                            .entry(set_name.clone())
                            .or_default()
                            .push(Span::default());
                    }
                }
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            HirExpr::Call { args, .. } => {
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            HirExpr::MethodCall {
                object, method, args, ..
            } => {
                // Check for set operations
                if let HirExpr::Var(set_name) = object.as_ref() {
                    let op = match method.as_str() {
                        "union" => Some(SetOp::Union),
                        "intersection" => Some(SetOp::Intersection),
                        "difference" => Some(SetOp::Difference),
                        "symmetric_difference" => Some(SetOp::SymmetricDifference),
                        _ => None,
                    };
                    if let Some(op) = op {
                        self.set_operations
                            .entry(set_name.clone())
                            .or_default()
                            .push((op, Span::default()));
                    }
                }
                self.analyze_expression(object);
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            HirExpr::List(items) | HirExpr::Tuple(items) => {
                for item in items {
                    self.analyze_expression(item);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    self.analyze_expression(k);
                    self.analyze_expression(v);
                }
            }
            HirExpr::Set(items) => {
                for item in items {
                    self.analyze_expression(item);
                }
            }
            HirExpr::Index { base, index } => {
                self.analyze_expression(base);
                self.analyze_expression(index);
            }
            HirExpr::Attribute { value, .. } => {
                self.analyze_expression(value);
            }
            HirExpr::Unary { operand, .. } => {
                self.analyze_expression(operand);
            }
            HirExpr::IfExpr { test, body, orelse } => {
                self.analyze_expression(test);
                self.analyze_expression(body);
                self.analyze_expression(orelse);
            }
            HirExpr::Lambda { body, .. } => {
                self.analyze_expression(body);
            }
            _ => {}
        }
    }

    /// Extract string values from a set, frozenset, or list literal.
    fn extract_string_set_values(&self, expr: &HirExpr) -> Option<Vec<String>> {
        match expr {
            HirExpr::Set(items) | HirExpr::FrozenSet(items) | HirExpr::List(items) => {
                let mut values = Vec::new();
                for item in items {
                    if let HirExpr::Literal(Literal::String(s)) = item {
                        values.push(s.clone());
                    } else {
                        return None;
                    }
                }
                Some(values)
            }
            _ => None,
        }
    }

    /// Refine candidates based on usage analysis.
    fn refine_candidates(&mut self) {
        let membership_checks = std::mem::take(&mut self.membership_checks);
        let set_operations = std::mem::take(&mut self.set_operations);

        for (name, checks) in &membership_checks {
            if let Some(candidate) = self.candidates.get_mut(name) {
                for span in checks {
                    candidate
                        .evidence
                        .push(DetectionEvidence::MembershipCheck { location: *span });
                }
                candidate.confidence = calculate_confidence(&candidate.evidence);
            }
        }

        for (name, ops) in &set_operations {
            if let Some(candidate) = self.candidates.get_mut(name) {
                for (op, span) in ops {
                    candidate.evidence.push(DetectionEvidence::SetOperation {
                        op: *op,
                        location: *span,
                    });
                }
                candidate.confidence = calculate_confidence(&candidate.evidence);
            }
        }
    }

    /// Get candidates above a confidence threshold.
    pub fn get_high_confidence_candidates(&self, threshold: f64) -> Vec<&StringSetCandidate> {
        self.candidates.values().filter(|c| c.confidence >= threshold).collect()
    }
}

/// Check if a name follows CONSTANT_NAMING convention.
fn is_constant_naming(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
        && name.chars().next().map_or(false, |c| c.is_ascii_uppercase())
}

/// Calculate initial confidence based on evidence.
fn calculate_initial_confidence(evidence: &[DetectionEvidence]) -> f64 {
    let mut confidence: f64 = 0.3; // Base confidence for being a string set literal

    for ev in evidence {
        match ev {
            DetectionEvidence::ConstantNaming => confidence += 0.2,
            DetectionEvidence::FrozenSet => confidence += 0.3,
            DetectionEvidence::AllLiterals => confidence += 0.1,
            _ => {}
        }
    }

    confidence.min(1.0)
}

/// Calculate confidence based on all evidence.
fn calculate_confidence(evidence: &[DetectionEvidence]) -> f64 {
    let mut confidence: f64 = calculate_initial_confidence(evidence);

    let membership_count = evidence
        .iter()
        .filter(|e| matches!(e, DetectionEvidence::MembershipCheck { .. }))
        .count();
    let set_op_count = evidence
        .iter()
        .filter(|e| matches!(e, DetectionEvidence::SetOperation { .. }))
        .count();

    // Membership checks increase confidence
    confidence += (membership_count as f64 * 0.1).min(0.3);

    // Set operations increase confidence
    confidence += (set_op_count as f64 * 0.05).min(0.1);

    // Never mutated is a strong signal
    if evidence.contains(&DetectionEvidence::NeverMutated) {
        confidence += 0.2;
    }

    confidence.min(1.0)
}

/// Validates that all string values can be converted to valid Rust identifiers.
pub fn validate_string_values(values: &[String]) -> Result<Vec<String>, InvalidStringValue> {
    let mut rust_idents = Vec::with_capacity(values.len());
    let mut seen = HashSet::new();

    for value in values {
        let ident = normalize_to_rust_identifier(value);
        if seen.contains(&ident) {
            return Err(InvalidStringValue::Duplicate {
                original: value.clone(),
                normalized: ident,
            });
        }
        seen.insert(ident.clone());
        rust_idents.push(ident);
    }

    Ok(rust_idents)
}

/// Normalize a string to a valid Rust identifier.
pub fn normalize_to_rust_identifier(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_uppercase());
        } else if c == '-' || c == ' ' || c == '.' {
            result.push('_');
        } else if c == '_' {
            result.push('_');
        } else {
            // For non-ASCII or special chars, use hex encoding
            result.push_str(&format!("_{:04X}", c as u32));
        }
    }

    // Ensure it doesn't start with a digit
    if result.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        result.insert(0, '_');
    }

    // Handle empty strings
    if result.is_empty() {
        result = "_EMPTY".to_string();
    }

    result
}

/// Error for invalid string values.
#[derive(Debug, Clone)]
pub enum InvalidStringValue {
    Duplicate { original: String, normalized: String },
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_constant_naming() {
        assert!(is_constant_naming("PERMISSIONS"));
        assert!(is_constant_naming("STATUS_FLAGS"));
        assert!(is_constant_naming("MAX_SIZE_123"));
        assert!(!is_constant_naming("permissions"));
        assert!(!is_constant_naming("Permissions"));
        assert!(!is_constant_naming("_PRIVATE"));
        assert!(!is_constant_naming(""));
    }

    #[test]
    fn test_normalize_to_rust_identifier() {
        assert_eq!(normalize_to_rust_identifier("read"), "READ");
        assert_eq!(normalize_to_rust_identifier("with-dash"), "WITH_DASH");
        assert_eq!(normalize_to_rust_identifier("with space"), "WITH_SPACE");
        assert_eq!(normalize_to_rust_identifier("123numeric"), "_123NUMERIC");
        assert_eq!(normalize_to_rust_identifier(""), "_EMPTY");
    }

    #[test]
    fn test_validate_string_values_success() {
        let values = vec!["read".to_string(), "write".to_string(), "execute".to_string()];
        let result = validate_string_values(&values);
        assert!(result.is_ok());
        let idents = result.unwrap();
        assert_eq!(idents, vec!["READ", "WRITE", "EXECUTE"]);
    }

    #[test]
    fn test_validate_string_values_duplicate() {
        let values = vec!["read".to_string(), "READ".to_string()];
        let result = validate_string_values(&values);
        assert!(matches!(result, Err(InvalidStringValue::Duplicate { .. })));
    }

    #[test]
    fn test_calculate_confidence() {
        let evidence = vec![
            DetectionEvidence::AllLiterals,
            DetectionEvidence::ConstantNaming,
            DetectionEvidence::FrozenSet,
        ];
        let conf = calculate_confidence(&evidence);
        assert!(conf > 0.8);

        let minimal = vec![DetectionEvidence::AllLiterals];
        let conf_min = calculate_confidence(&minimal);
        assert!(conf_min < 0.5);
    }

    #[test]
    fn test_string_set_candidate_creation() {
        let candidate = StringSetCandidate {
            name: "PERMISSIONS".to_string(),
            values: vec!["read".to_string(), "write".to_string()],
            span: Span::default(),
            confidence: 0.9,
            evidence: vec![DetectionEvidence::AllLiterals, DetectionEvidence::ConstantNaming],
        };
        assert_eq!(candidate.name, "PERMISSIONS");
        assert_eq!(candidate.values.len(), 2);
    }
}
