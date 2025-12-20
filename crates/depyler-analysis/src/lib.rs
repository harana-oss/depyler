//! # Depyler Analysis
//!
//! Unified analysis, quality gates, and verification engine for Depyler.
//!
//! This crate consolidates functionality from the former `depyler-analyzer`,
//! `depyler-quality`, and `depyler-verify` crates into a single cohesive package.
//!
//! ## Modules
//!
//! - **[`metrics`]** - Code metrics (complexity, type coverage, etc.)
//! - **[`quality`]** - Quality gates and PMAT scoring
//! - **[`verify`]** - Property verification and memory safety analysis
//!
//! ## Quick Start
//!
//! ```rust
//! use depyler_analysis::prelude::*;
//!
//! // Create an analyzer for metrics
//! let analyzer = Analyzer::new();
//!
//! // Quality analysis
//! let quality_analyzer = QualityAnalyzer::new();
//!
//! // Property verification
//! let verifier = PropertyVerifier::new();
//! ```

pub mod metrics;
pub mod quality;
pub mod usage_analysis;
pub mod verify;

/// Prelude for convenient imports
pub mod prelude {
    // From metrics
    pub use crate::metrics::complexity::{
        calculate_cognitive, calculate_cyclomatic, calculate_max_nesting, count_statements,
    };
    pub use crate::metrics::type_flow::{FunctionSignature, TypeEnvironment, TypeInferencer};
    pub use crate::metrics::{
        AnalysisResult, Analyzer, ComplexityDistribution, FunctionMetrics, ModuleMetrics,
        PerformanceProfile, QualityMetrics as MetricsQualityMetrics, TranspilationMetrics,
        TypeCoverage,
    };

    // From quality
    pub use crate::quality::{
        ComplexityMetrics, CoverageMetrics, PmatMetrics, QualityAnalyzer, QualityError,
        QualityGate, QualityGateResult, QualityReport, QualityRequirement, QualityStatus, Severity,
    };

    // From verify
    pub use crate::verify::{
        PropertyStatus, PropertyVerifier, TestCase, VerificationMethod, VerificationResult,
    };

    // From usage_analysis
    pub use crate::usage_analysis::{
        get_borrowable_field_vars, UsageAnalyzer, UsageContext, VariableUsage,
    };
}

// Re-export main types at crate root for convenience
pub use metrics::complexity::{
    calculate_cognitive, calculate_cyclomatic, calculate_max_nesting, count_statements,
};
pub use metrics::type_flow::{FunctionSignature, TypeEnvironment, TypeInferencer};
pub use metrics::{AnalysisResult, Analyzer, FunctionMetrics, ModuleMetrics, TypeCoverage};
pub use quality::{
    ComplexityMetrics, CoverageMetrics, PmatMetrics, QualityAnalyzer, QualityError, QualityGate,
    QualityGateResult, QualityReport, QualityRequirement, QualityStatus, Severity,
};
pub use usage_analysis::{get_borrowable_field_vars, UsageAnalyzer, UsageContext, VariableUsage};
pub use verify::{PropertyStatus, PropertyVerifier, VerificationResult};
