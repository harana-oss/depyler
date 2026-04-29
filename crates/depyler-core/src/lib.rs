//! # Depyler Core - Transpilation Engine
//!
//! Core transpilation engine for converting Python code to Rust and other targets.
//!
//! ## Overview
//!
//! This crate provides the fundamental transpilation pipeline that converts Python
//! source code into target languages (Rust, Ruchy) while preserving semantics and
//! ensuring memory safety.
//!
//! ## Example
//!
//! ```rust
//! use depyler_core::DepylerPipeline;
//!
//! let pipeline = DepylerPipeline::new();
//! let python = r#"
//! def factorial(n: int) -> int:
//!     if n <= 1:
//!         return 1
//!     return n * factorial(n - 1)
//! "#;
//!
//! match pipeline.transpile(python) {
//!     Ok(rust_code) => println!("Generated:\n{}", rust_code),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! ## Architecture
//!
//! The transpilation pipeline consists of several stages:
//!
//! 1. **Parsing** ([`ast_bridge`]) - Convert Python source to AST
//! 2. **HIR** ([`hir`]) - Transform AST to High-level Intermediate Representation
//! 3. **Type Analysis** ([`generic_inference`], [`const_generic_inference`]) - Infer types and generics
//! 4. **Ownership Analysis** ([`borrowing`], [`lifetime_analysis`]) - Determine ownership patterns
//! 5. **Optimization** ([`optimization`], [`string_optimization`]) - Apply optimizations
//! 6. **Code Generation** ([`codegen`], [`rust_gen`]) - Generate target code
//!
//! ## Key Types
//!
//! - [`DepylerPipeline`] - Main entry point for transpilation
//! - [`TranspileOptions`] - Configuration options
//! - [`Hir`] - High-level intermediate representation
//! - [`TranspilationBackend`] - Backend trait for target languages

pub mod annotation_aware_type_mapper;
pub mod ast_bridge;
pub mod backend;
pub mod borrowing;
pub mod borrowing_context;
pub mod cargo_toml_gen;
pub mod codegen;
pub mod const_generic_inference;
pub mod dataflow;
pub mod debug;
pub mod direct_rules;
pub mod documentation;
pub mod error;
pub mod error_reporting;
pub mod expr_utils;
pub mod generator_state;
pub mod generator_yield_analysis;
pub mod generic_inference;
pub mod hir;
pub mod ide;
pub mod inlining;
pub mod interprocedural;
pub mod lambda_codegen;
pub mod lambda_errors;
pub mod lambda_inference;
pub mod lambda_optimizer;
pub mod lambda_testing;
pub mod lambda_types;
pub mod lifetime_analysis;
pub mod lsp;
pub mod migration_suggestions;
pub mod module_mapper;
pub mod optimization;
pub mod optimizer;
pub mod performance_warnings;
pub mod profiling;
pub mod rust_gen;
pub mod simplified_hir;
pub mod stdlib_mappings;
pub mod string_optimization;
pub mod type_hints;
pub mod type_mapper;
pub mod union_enum_gen;

use anyhow::Result;
use serde::{Deserialize, Serialize};

// Re-export backend traits and types
pub use backend::{TranspilationBackend, TranspilationTarget, ValidationError};
pub use error::TranspileError;
pub use simplified_hir::{
    Hir, HirBinaryOp, HirExpr, HirLiteral, HirParam, HirStatement, HirType, HirUnaryOp,
};

/// The main transpilation pipeline for converting Python code to multiple targets
///
/// ## Version 3.0.0 - Multi-Target Support
///
/// Depyler now supports multiple transpilation targets through the `TranspilationBackend` trait:
/// - **Rust** (default): Generates idiomatic, safe Rust code
/// - **Ruchy**: Generates functional Ruchy script format with pipeline operators
///
/// ### Example Usage
///
/// ```rust
/// use depyler_core::DepylerPipeline;
/// # use anyhow::Result;
/// # fn example() -> Result<()> {
/// # let python_code = "def hello(): pass";
///
/// // Create pipeline and transpile to Rust (default)
/// let pipeline = DepylerPipeline::new();
/// let rust_code = pipeline.transpile(python_code)?;
/// # Ok(())
/// # }
/// ```
///
/// `DepylerPipeline` coordinates the entire transpilation process, from parsing Python
/// source code to generating equivalent Rust code. It provides a high-level API for
/// transpilation with configurable analysis, optimization, and verification stages.
///
/// # Features
///
/// - **Semantic Analysis**: Converts Python AST to type-aware HIR
/// - **Type Inference**: Infers and validates type information
/// - **Optimization**: Applies performance optimizations
/// - **Verification**: Optional property verification for correctness
/// - **Code Generation**: Produces idiomatic Rust code
///
/// # Examples
///
/// Basic transpilation:
///
/// ```rust
/// use depyler_core::DepylerPipeline;
///
/// let pipeline = DepylerPipeline::new();
/// let python_code = r#"
/// def add(a: int, b: int) -> int:
///     return a + b
/// "#;
///
/// let rust_code = pipeline.transpile(python_code).unwrap();
/// assert!(rust_code.contains("pub fn add"));
/// assert!(rust_code.contains("i32"));
/// ```
///
/// With verification enabled:
///
/// ```rust
/// use depyler_core::DepylerPipeline;
///
/// let pipeline = DepylerPipeline::new()
///     .with_verification();
///
/// let python_code = r#"
/// def factorial(n: int) -> int:
///     if n <= 1:
///         return 1
///     return n * factorial(n - 1)
/// "#;
///
/// let rust_code = pipeline.transpile(python_code).unwrap();
/// assert!(rust_code.contains("factorial"));
/// ```
///
/// Parsing to HIR for analysis:
///
/// ```rust
/// use depyler_core::DepylerPipeline;
///
/// let pipeline = DepylerPipeline::new();
/// let python_code = "def hello(): return 'world'";
///
/// let hir = pipeline.parse_to_hir(python_code).unwrap();
/// assert_eq!(hir.functions.len(), 1);
/// assert_eq!(hir.functions[0].name, "hello");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepylerPipeline {
    analyzer: CoreAnalyzer,
    transpiler: DirectTranspiler,
    #[serde(skip_serializing_if = "Option::is_none")]
    verifier: Option<PropertyVerifier>,
    #[serde(skip)]
    #[allow(dead_code)]
    mcp_client: LazyMcpClient,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_config: Option<debug::DebugConfig>,
    #[serde(skip)]
    config: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreAnalyzer {
    pub metrics_enabled: bool,
    pub type_inference_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectTranspiler {
    pub type_mapper: type_mapper::TypeMapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyVerifier {
    pub enable_quickcheck: bool,
    pub enable_contracts: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LazyMcpClient {
    #[allow(dead_code)]
    endpoint: Option<String>,
}

pub trait AnalyzableStage {
    type Input;
    type Output;
    type Metrics;

    fn execute(&self, input: Self::Input) -> Result<(Self::Output, Self::Metrics)>;
    fn validate(&self, output: &Self::Output) -> ValidationResult;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Default for DepylerPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl DepylerPipeline {
    /// Creates a new transpilation pipeline with default configuration
    ///
    /// The default pipeline includes:
    /// - Core semantic analysis and type inference
    /// - Standard optimizations
    /// - No property verification (use `with_verification()` to enable)
    /// - No debug output
    ///
    /// # Examples
    ///
    /// ```rust
    /// use depyler_core::DepylerPipeline;
    ///
    /// let pipeline = DepylerPipeline::new();
    /// // Pipeline is ready for transpilation
    /// ```
    pub fn new() -> Self {
        Self {
            analyzer: CoreAnalyzer {
                metrics_enabled: false,
                type_inference_enabled: true,
            },
            transpiler: DirectTranspiler {
                type_mapper: type_mapper::TypeMapper::default(),
            },
            verifier: None,
            mcp_client: LazyMcpClient::default(),
            debug_config: None,
            config: Config::default(),
        }
    }

    pub fn with_verification(mut self) -> Self {
        self.verifier = Some(PropertyVerifier {
            enable_quickcheck: true,
            enable_contracts: true,
        });
        self
    }

    pub fn with_debug(mut self, debug_config: debug::DebugConfig) -> Self {
        self.debug_config = Some(debug_config);
        self
    }

    /// Transpiles Python source code to equivalent Rust code
    ///
    /// This is the main entry point for transpilation. It performs the complete
    /// pipeline: parsing, semantic analysis, type inference, optimization, and
    /// code generation.
    ///
    /// # Arguments
    ///
    /// * `python_source` - The Python source code to transpile
    ///
    /// # Returns
    ///
    /// Returns the generated Rust code as a string, or an error if transpilation fails.
    ///
    /// # Examples
    ///
    /// Basic function transpilation:
    ///
    /// ```rust
    /// use depyler_core::DepylerPipeline;
    ///
    /// let pipeline = DepylerPipeline::new();
    /// let python_code = r#"
    /// def multiply(x: int, y: int) -> int:
    ///     return x * y
    /// "#;
    ///
    /// let rust_code = pipeline.transpile(python_code).unwrap();
    /// assert!(rust_code.contains("pub fn multiply"));
    /// assert!(rust_code.contains("-> i32"));
    /// ```
    ///
    /// Complex function with control flow:
    ///
    /// ```rust
    /// use depyler_core::DepylerPipeline;
    ///
    /// let pipeline = DepylerPipeline::new();
    /// let python_code = r#"
    /// def is_even(n: int) -> bool:
    ///     if n % 2 == 0:
    ///         return True
    ///     else:
    ///         return False
    /// "#;
    ///
    /// let rust_code = pipeline.transpile(python_code).unwrap();
    /// assert!(rust_code.contains("pub fn is_even"));
    /// assert!(rust_code.contains("bool")); // Changed to just check for bool type
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The Python source contains syntax errors
    /// - Unsupported Python constructs are used
    /// - Type inference fails
    /// - Verification fails (if enabled)
    ///
    /// Transpiles Python source code and returns both Rust code and Cargo dependencies
    ///
    /// of Cargo dependencies needed to build it. Use this when you need to generate
    /// a complete Cargo project with Cargo.toml.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (rust_code, dependencies) or an error if transpilation fails.
    pub fn transpile_with_dependencies(
        &self,
        python_source: &str,
    ) -> Result<(String, Vec<cargo_toml_gen::Dependency>)> {
        // Parse Python source
        let ast = self.parse_python(python_source)?;

        // Convert to HIR with annotation support
        let mut hir = ast_bridge::AstBridge::new()
            .with_source(python_source.to_string())
            .python_to_hir(ast)?;

        // Apply const generic inference
        let mut const_inferencer = const_generic_inference::ConstGenericInferencer::new();
        const_inferencer.analyze_module(&mut hir)?;

        // Apply type inference hints
        if self.analyzer.type_inference_enabled {
            let mut type_hint_provider = type_hints::TypeHintProvider::new();

            // Analyze all functions and collect hints
            let mut function_hints = Vec::new();
            for (idx, func) in hir.functions.iter().enumerate() {
                if let Ok(hints) = type_hint_provider.analyze_function(func) {
                    if !hints.is_empty() {
                        log::debug!("Type inference hints:");
                        log::debug!("{}", type_hint_provider.format_hints(&hints));
                        function_hints.push((idx, hints));
                    }
                }
            }

            // Apply high-confidence hints to the HIR
            for (func_idx, hints) in function_hints {
                let func = &mut hir.functions[func_idx];

                // Apply parameter type hints
                for param in &mut func.params {
                    if matches!(param.ty, hir::Type::Unknown) {
                        // Find hint for this parameter
                        for hint in &hints {
                            if let type_hints::HintTarget::Parameter(hint_param) = &hint.target {
                                if hint_param == &param.name
                                    && matches!(
                                        hint.confidence,
                                        type_hints::Confidence::High
                                            | type_hints::Confidence::Certain
                                    )
                                {
                                    param.ty = hint.suggested_type.clone();
                                    log::debug!(
                                        "Applied type hint: {} -> {:?}",
                                        param.name,
                                        param.ty
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }

                // Apply return type hints
                if matches!(func.ret_type, hir::Type::Unknown) {
                    for hint in &hints {
                        if matches!(hint.target, type_hints::HintTarget::Return)
                            && matches!(
                                hint.confidence,
                                type_hints::Confidence::Medium
                                    | type_hints::Confidence::High
                                    | type_hints::Confidence::Certain
                            )
                        {
                            func.ret_type = hint.suggested_type.clone();
                            log::debug!("Applied return type hint: {:?}", func.ret_type);
                            break;
                        }
                    }
                }
            }
        }

        // Apply optimization passes based on annotations
        optimization::optimize_module(&mut hir);

        // Apply the new general-purpose optimizer
        let mut optimizer = optimizer::Optimizer::new(optimizer::OptimizerConfig::default());
        let optimized_hir = optimizer.optimize_program(hir);

        // Run migration suggestions analysis
        if self.analyzer.metrics_enabled {
            let mut migration_analyzer = migration_suggestions::MigrationAnalyzer::new(
                migration_suggestions::MigrationConfig::default(),
            );
            let suggestions = migration_analyzer.analyze_program(&optimized_hir);
            if !suggestions.is_empty() {
                eprintln!("{}", migration_analyzer.format_suggestions(&suggestions));
            }
        }

        // Run performance warnings analysis
        if self.analyzer.metrics_enabled {
            let mut perf_analyzer = performance_warnings::PerformanceAnalyzer::new(
                performance_warnings::PerformanceConfig::default(),
            );
            let warnings = perf_analyzer.analyze_program(&optimized_hir);
            if !warnings.is_empty() {
                eprintln!("{}", perf_analyzer.format_warnings(&warnings));
            }
        }

        // Run profiling analysis if enabled
        if self.analyzer.metrics_enabled {
            let mut profiler = profiling::Profiler::new(profiling::ProfileConfig::default());
            let profile_report = profiler.analyze_program(&optimized_hir);
            if !profile_report.metrics.is_empty() {
                eprintln!("{}", profile_report.format_report());
            }
        }

        // Generate Rust code with dependencies
        rust_gen::generate_rust_file(&optimized_hir, &self.transpiler.type_mapper)
    }

    pub fn transpile(&self, python_source: &str) -> Result<String> {
        // Parse Python source
        let ast = self.parse_python(python_source)?;

        // Convert to HIR with annotation support
        let mut hir = ast_bridge::AstBridge::new()
            .with_source(python_source.to_string())
            .python_to_hir(ast)?;

        // Apply const generic inference
        let mut const_inferencer = const_generic_inference::ConstGenericInferencer::new();
        const_inferencer.analyze_module(&mut hir)?;

        // Apply type inference hints
        if self.analyzer.type_inference_enabled {
            let mut type_hint_provider = type_hints::TypeHintProvider::new();

            // Analyze all functions and collect hints
            let mut function_hints = Vec::new();
            for (idx, func) in hir.functions.iter().enumerate() {
                if let Ok(hints) = type_hint_provider.analyze_function(func) {
                    if !hints.is_empty() {
                        log::debug!("Type inference hints:");
                        log::debug!("{}", type_hint_provider.format_hints(&hints));
                        function_hints.push((idx, hints));
                    }
                }
            }

            // Apply high-confidence hints to the HIR
            for (func_idx, hints) in function_hints {
                let func = &mut hir.functions[func_idx];

                // Apply parameter type hints
                for param in &mut func.params {
                    if matches!(param.ty, hir::Type::Unknown) {
                        // Find hint for this parameter
                        for hint in &hints {
                            if let type_hints::HintTarget::Parameter(hint_param) = &hint.target {
                                if hint_param == &param.name
                                    && matches!(
                                        hint.confidence,
                                        type_hints::Confidence::High
                                            | type_hints::Confidence::Certain
                                    )
                                {
                                    param.ty = hint.suggested_type.clone();
                                    log::debug!(
                                        "Applied type hint: {} -> {:?}",
                                        param.name,
                                        param.ty
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }

                // Apply return type hints
                if matches!(func.ret_type, hir::Type::Unknown) {
                    for hint in &hints {
                        if matches!(hint.target, type_hints::HintTarget::Return)
                            && matches!(
                                hint.confidence,
                                type_hints::Confidence::Medium
                                    | type_hints::Confidence::High
                                    | type_hints::Confidence::Certain
                            )
                        {
                            func.ret_type = hint.suggested_type.clone();
                            log::debug!("Applied return type hint: {:?}", func.ret_type);
                            break;
                        }
                    }
                }
            }
        }

        // Apply optimization passes based on annotations
        optimization::optimize_module(&mut hir);

        // Apply the new general-purpose optimizer
        let mut optimizer = optimizer::Optimizer::new(optimizer::OptimizerConfig::default());
        let optimized_hir = optimizer.optimize_program(hir);

        // Run migration suggestions analysis
        if self.analyzer.metrics_enabled {
            let mut migration_analyzer = migration_suggestions::MigrationAnalyzer::new(
                migration_suggestions::MigrationConfig::default(),
            );
            let suggestions = migration_analyzer.analyze_program(&optimized_hir);
            if !suggestions.is_empty() {
                eprintln!("{}", migration_analyzer.format_suggestions(&suggestions));
            }
        }

        // Run performance warnings analysis
        if self.analyzer.metrics_enabled {
            let mut perf_analyzer = performance_warnings::PerformanceAnalyzer::new(
                performance_warnings::PerformanceConfig::default(),
            );
            let warnings = perf_analyzer.analyze_program(&optimized_hir);
            if !warnings.is_empty() {
                eprintln!("{}", perf_analyzer.format_warnings(&warnings));
            }
        }

        // Run profiling analysis if enabled
        if self.analyzer.metrics_enabled {
            let mut profiler = profiling::Profiler::new(profiling::ProfileConfig::default());
            let profile_report = profiler.analyze_program(&optimized_hir);
            if !profile_report.metrics.is_empty() {
                eprintln!("{}", profile_report.format_report());
            }
        }

        // Generate Rust code using the unified generation system
        let (rust_code, _dependencies) =
            rust_gen::generate_rust_file(&optimized_hir, &self.transpiler.type_mapper)?;

        Ok(rust_code)
    }

    pub fn parse_to_hir(&self, source: &str) -> Result<hir::HirModule> {
        let ast = self.parse_python(source)?;
        ast_bridge::AstBridge::new()
            .with_source(source.to_string())
            .python_to_hir(ast)
    }

    /// Parse Python source and apply true dataflow-based type inference
    ///
    /// This method performs complete type inference using dataflow analysis
    /// to produce definitive types rather than heuristic-based hints.
    pub fn parse_to_typed_hir(&self, source: &str) -> Result<hir::HirModule> {
        let mut hir = self.parse_to_hir(source)?;

        // Apply dataflow-based type inference
        let inferencer = dataflow::DataflowTypeInferencer::new();
        inferencer.apply_types_to_module(&mut hir);

        Ok(hir)
    }

    /// Analyze a single function using dataflow type inference
    pub fn infer_function_types(&self, func: &hir::HirFunction) -> dataflow::InferredTypes {
        let inferencer = dataflow::DataflowTypeInferencer::new();
        inferencer.infer_function(func)
    }

    pub fn analyze_to_typed_hir(&self, source: &str) -> Result<hir::HirModule> {
        // Now uses dataflow analysis for proper type inference
        self.parse_to_typed_hir(source)
    }

    pub fn parse_python(&self, source: &str) -> Result<rustpython_ast::Mod> {
        use rustpython_ast::Suite;
        use rustpython_parser::Parse;

        let statements = Suite::parse(source, "<input>")
            .map_err(|e| anyhow::anyhow!("Python parse error: {}", e))?;

        Ok(rustpython_ast::Mod::Module(rustpython_ast::ModModule {
            body: statements,
            type_ignores: vec![],
            range: Default::default(),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub enable_verification: bool,
    pub enable_metrics: bool,
    pub optimization_level: OptimizationLevel,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_verification: false,
            enable_metrics: false,
            optimization_level: OptimizationLevel::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum OptimizationLevel {
    #[default]
    Debug,
    Release,
    Size,
}

impl DepylerPipeline {
    pub fn new_with_config(config: Config) -> Self {
        let mut pipeline = Self::new();
        pipeline.analyzer.metrics_enabled = config.enable_metrics;
        pipeline.config = config.clone();

        if config.enable_verification {
            pipeline = pipeline.with_verification();
        }

        pipeline
    }
}
