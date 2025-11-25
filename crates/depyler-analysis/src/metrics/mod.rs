//! Code metrics and analysis
//!
//! This module provides code complexity metrics, type coverage analysis,
//! and performance profiling for transpiled code.

pub mod complexity;
pub mod type_flow;

use anyhow::Result;
use depyler_core::hir::{HirFunction, HirModule};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(test)]
use depyler_annotations::TranspilationAnnotations;

// Re-export complexity functions for easier use
pub use complexity::{calculate_cognitive, calculate_cyclomatic, calculate_max_nesting, count_statements};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub module_metrics: ModuleMetrics,
    pub function_metrics: Vec<FunctionMetrics>,
    pub type_coverage: TypeCoverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetrics {
    pub total_functions: usize,
    pub total_lines: usize,
    pub avg_cyclomatic_complexity: f64,
    pub max_cyclomatic_complexity: u32,
    pub avg_cognitive_complexity: f64,
    pub max_cognitive_complexity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetrics {
    pub name: String,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub lines_of_code: usize,
    pub parameters: usize,
    pub max_nesting_depth: usize,
    pub has_type_annotations: bool,
    pub return_type_annotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCoverage {
    pub total_parameters: usize,
    pub annotated_parameters: usize,
    pub total_functions: usize,
    pub functions_with_return_type: usize,
    pub coverage_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranspilationMetrics {
    pub parse_time: Duration,
    pub analysis_time: Duration,
    pub transpilation_time: Duration,
    pub total_time: Duration,
    pub source_size_bytes: usize,
    pub output_size_bytes: usize,
    pub functions_transpiled: usize,
    pub direct_transpilation_rate: f64,
    pub mcp_fallback_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub cyclomatic_distribution: ComplexityDistribution,
    pub cognitive_distribution: ComplexityDistribution,
    pub type_coverage: f64,
    pub panic_free_functions: usize,
    pub terminating_functions: usize,
    pub pure_functions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityDistribution {
    pub low: usize,       // complexity <= 5
    pub medium: usize,    // 5 < complexity <= 10
    pub high: usize,      // 10 < complexity <= 20
    pub very_high: usize, // complexity > 20
}

impl Default for ComplexityDistribution {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplexityDistribution {
    pub fn new() -> Self {
        Self {
            low: 0,
            medium: 0,
            high: 0,
            very_high: 0,
        }
    }

    pub fn add(&mut self, complexity: u32) {
        match complexity {
            0..=5 => self.low += 1,
            6..=10 => self.medium += 1,
            11..=20 => self.high += 1,
            _ => self.very_high += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.low + self.medium + self.high + self.very_high
    }

    pub fn average(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }

        let weighted_sum = (self.low * 3) + (self.medium * 8) + (self.high * 15) + (self.very_high * 25);
        weighted_sum as f64 / total as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub parsing_throughput_mbps: f64,
    pub hir_generation_throughput_mbps: f64,
    pub transpilation_throughput_mbps: f64,
    pub memory_peak_mb: f64,
}

impl PerformanceProfile {
    pub fn calculate(metrics: &TranspilationMetrics, memory_peak_bytes: usize) -> Self {
        let source_mb = metrics.source_size_bytes as f64 / (1024.0 * 1024.0);

        Self {
            parsing_throughput_mbps: if metrics.parse_time.as_secs_f64() > 0.0 {
                source_mb / metrics.parse_time.as_secs_f64()
            } else {
                0.0
            },
            hir_generation_throughput_mbps: if metrics.analysis_time.as_secs_f64() > 0.0 {
                source_mb / metrics.analysis_time.as_secs_f64()
            } else {
                0.0
            },
            transpilation_throughput_mbps: if metrics.transpilation_time.as_secs_f64() > 0.0 {
                source_mb / metrics.transpilation_time.as_secs_f64()
            } else {
                0.0
            },
            memory_peak_mb: memory_peak_bytes as f64 / (1024.0 * 1024.0),
        }
    }
}

/// Main analyzer for code metrics
pub struct Analyzer {
    #[allow(dead_code)]
    enable_type_inference: bool,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            enable_type_inference: true,
        }
    }

    pub fn analyze(&self, module: &HirModule) -> Result<AnalysisResult> {
        let function_metrics: Vec<FunctionMetrics> = module
            .functions
            .iter()
            .map(|f| self.analyze_function(f))
            .collect::<Result<Vec<_>>>()?;

        let module_metrics = self.calculate_module_metrics(&function_metrics);
        let type_coverage = self.calculate_type_coverage(module);

        Ok(AnalysisResult {
            module_metrics,
            function_metrics,
            type_coverage,
        })
    }

    fn analyze_function(&self, func: &HirFunction) -> Result<FunctionMetrics> {
        let cyclomatic = complexity::calculate_cyclomatic(&func.body);
        let cognitive = complexity::calculate_cognitive(&func.body);
        let max_nesting = complexity::calculate_max_nesting(&func.body);
        let loc = complexity::count_statements(&func.body);

        let has_type_annotations = func
            .params
            .iter()
            .all(|param| !matches!(param.ty, depyler_core::hir::Type::Unknown));
        let return_type_annotated = !matches!(func.ret_type, depyler_core::hir::Type::Unknown);

        Ok(FunctionMetrics {
            name: func.name.clone(),
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            lines_of_code: loc,
            parameters: func.params.len(),
            max_nesting_depth: max_nesting,
            has_type_annotations,
            return_type_annotated,
        })
    }

    fn calculate_module_metrics(&self, functions: &[FunctionMetrics]) -> ModuleMetrics {
        let total_functions = functions.len();
        let total_lines: usize = functions.iter().map(|f| f.lines_of_code).sum();

        let avg_cyclomatic = if total_functions > 0 {
            functions.iter().map(|f| f.cyclomatic_complexity as f64).sum::<f64>() / total_functions as f64
        } else {
            0.0
        };

        let max_cyclomatic = functions.iter().map(|f| f.cyclomatic_complexity).max().unwrap_or(0);

        let avg_cognitive = if total_functions > 0 {
            functions.iter().map(|f| f.cognitive_complexity as f64).sum::<f64>() / total_functions as f64
        } else {
            0.0
        };

        let max_cognitive = functions.iter().map(|f| f.cognitive_complexity).max().unwrap_or(0);

        ModuleMetrics {
            total_functions,
            total_lines,
            avg_cyclomatic_complexity: avg_cyclomatic,
            max_cyclomatic_complexity: max_cyclomatic,
            avg_cognitive_complexity: avg_cognitive,
            max_cognitive_complexity: max_cognitive,
        }
    }

    fn calculate_type_coverage(&self, module: &HirModule) -> TypeCoverage {
        let mut total_parameters = 0;
        let mut annotated_parameters = 0;
        let mut functions_with_return_type = 0;

        for func in &module.functions {
            total_parameters += func.params.len();
            annotated_parameters += func
                .params
                .iter()
                .filter(|param| !matches!(param.ty, depyler_core::hir::Type::Unknown))
                .count();

            if !matches!(func.ret_type, depyler_core::hir::Type::Unknown) {
                functions_with_return_type += 1;
            }
        }

        let total_annotations = annotated_parameters + functions_with_return_type;
        let total_possible = total_parameters + module.functions.len();
        let coverage_percentage = if total_possible > 0 {
            (total_annotations as f64 / total_possible as f64) * 100.0
        } else {
            100.0
        };

        TypeCoverage {
            total_parameters,
            annotated_parameters,
            total_functions: module.functions.len(),
            functions_with_return_type,
            coverage_percentage,
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use depyler_core::hir::*;

    fn create_test_function() -> HirFunction {
        use smallvec::smallvec;
        HirFunction {
            name: "test_func".to_string(),
            params: smallvec![
                HirParam {
                    name: Symbol::from("x"),
                    ty: Type::Int,
                    default: None
                },
                HirParam {
                    name: Symbol::from("y"),
                    ty: Type::String,
                    default: None
                }
            ],
            ret_type: Type::Int,
            body: vec![HirStmt::Return(Some(HirExpr::Literal(Literal::Int(42))))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        }
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = Analyzer::new();
        assert!(analyzer.enable_type_inference);

        let default_analyzer = Analyzer::default();
        assert!(default_analyzer.enable_type_inference);
    }

    #[test]
    fn test_analyze_empty_module() {
        let analyzer = Analyzer::new();
        let module = HirModule {
            functions: vec![],
            imports: vec![],
            type_aliases: vec![],
            protocols: vec![],
            classes: vec![],
            constants: vec![],
        };

        let result = analyzer.analyze(&module).unwrap();
        assert_eq!(result.module_metrics.total_functions, 0);
        assert_eq!(result.function_metrics.len(), 0);
        assert_eq!(result.type_coverage.total_functions, 0);
        assert_eq!(result.type_coverage.coverage_percentage, 100.0);
    }

    #[test]
    fn test_analyze_single_function() {
        let analyzer = Analyzer::new();
        let func = create_test_function();
        let module = HirModule {
            functions: vec![func],
            imports: vec![],
            type_aliases: vec![],
            protocols: vec![],
            classes: vec![],
            constants: vec![],
        };

        let result = analyzer.analyze(&module).unwrap();
        assert_eq!(result.module_metrics.total_functions, 1);
        assert_eq!(result.function_metrics.len(), 1);

        let func_metrics = &result.function_metrics[0];
        assert_eq!(func_metrics.name, "test_func");
        assert_eq!(func_metrics.parameters, 2);
        assert!(func_metrics.has_type_annotations);
        assert!(func_metrics.return_type_annotated);
    }

    #[test]
    fn test_complexity_distribution() {
        let mut dist = ComplexityDistribution::new();

        dist.add(3); // low
        dist.add(8); // medium
        dist.add(15); // high
        dist.add(25); // very_high

        assert_eq!(dist.low, 1);
        assert_eq!(dist.medium, 1);
        assert_eq!(dist.high, 1);
        assert_eq!(dist.very_high, 1);
        assert_eq!(dist.total(), 4);
    }
}
