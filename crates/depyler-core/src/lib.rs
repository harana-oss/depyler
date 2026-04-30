pub mod analysis;
pub mod annotations;
pub mod ast_bridge;
pub mod dataflow;
pub mod errors;
pub mod generators;
pub mod hir;
pub mod interprocedural;
pub mod mappings;
pub mod optimizations;
pub mod rust_generator;
pub mod types;

use anyhow::Result;

pub use errors::error::TranspileError;

/// The main transpilation pipeline for converting Python code to multiple targets
#[derive(Debug, Clone)]
pub struct DepylerPipeline {
    analyzer: CoreAnalyzer,
    transpiler: DirectTranspiler,
    pub enable_verification: bool,
}

#[derive(Debug, Clone)]
pub struct CoreAnalyzer {
    pub type_inference_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct DirectTranspiler {
    pub type_mapper: types::type_mapper::TypeMapper,
}

impl Default for DepylerPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl DepylerPipeline {
    /// Creates a new transpilation pipeline with default configuration
    pub fn new() -> Self {
        Self {
            analyzer: CoreAnalyzer {
                type_inference_enabled: true,
            },
            transpiler: DirectTranspiler {
                type_mapper: types::type_mapper::TypeMapper::default(),
            },
            enable_verification: false,
        }
    }

    /// Transpiles Python source code to equivalent Rust code
    fn build_rust_output(
        &self,
        python_source: &str,
    ) -> Result<(String, Vec<rust_generator::cargo_toml_gen::Dependency>)> {
        // Parse Python source
        let ast = self.parse_python(python_source)?;

        // Convert to HIR with annotation support
        let mut hir = ast_bridge::AstBridge::new()
            .with_source(python_source.to_string())
            .python_to_hir(ast)?;

        // Apply const generic inference
        let mut const_inferencer = types::type_inference::ConstGenericInferencer::new();
        const_inferencer.analyze_module(&mut hir)?;

        // Apply dataflow-based type inference
        if self.analyzer.type_inference_enabled {
            let inferencer = dataflow::DataflowTypeInferencer::new();
            inferencer.apply_types_to_module(&mut hir);
        }

        // Apply optimization passes based on annotations
        let mut optimizer = optimizations::optimizer::Optimizer::new(
            optimizations::optimizer::OptimizerConfig::default(),
        );
        let optimized_hir = optimizer.optimize_program(hir);

        // Generate Rust code with dependencies
        rust_generator::generate_rust_file(&optimized_hir, &self.transpiler.type_mapper)
    }

    /// Transpiles Python source code and returns both Rust code and the list
    /// of Cargo dependencies needed to build it. Use this when you need to
    /// generate a complete Cargo project with `Cargo.toml`.
    ///
    /// # Returns
    ///
    /// Returns a tuple of `(rust_code, dependencies)` or an error if
    /// transpilation fails.
    pub fn transpile_with_dependencies(
        &self,
        python_source: &str,
    ) -> Result<(String, Vec<rust_generator::cargo_toml_gen::Dependency>)> {
        self.build_rust_output(python_source)
    }

    /// Transpiles Python source code to Rust, discarding the dependency list.
    ///
    /// For most use-cases where only the generated Rust source is needed.
    /// Use [`transpile_with_dependencies`] if you also need the Cargo
    /// dependency information.
    pub fn transpile(&self, python_source: &str) -> Result<String> {
        self.build_rust_output(python_source).map(|(code, _)| code)
    }

    pub fn parse_to_hir(&self, source: &str) -> Result<hir::HirModule> {
        let ast = self.parse_python(source)?;
        ast_bridge::AstBridge::new()
            .with_source(source.to_string())
            .python_to_hir(ast)
    }

    /// Parse Python source and apply full type inference (const generics + dataflow).
    ///
    /// Runs the same inference passes as `transpile` but stops before
    /// optimisation and code generation, returning the typed HIR for inspection.
    pub fn parse_to_typed_hir(&self, source: &str) -> Result<hir::HirModule> {
        let mut hir = self.parse_to_hir(source)?;

        // Apply const generic inference (same as transpile)
        let mut const_inferencer = types::type_inference::ConstGenericInferencer::new();
        const_inferencer.analyze_module(&mut hir)?;

        // Apply dataflow-based type inference
        if self.analyzer.type_inference_enabled {
            let inferencer = dataflow::DataflowTypeInferencer::new();
            inferencer.apply_types_to_module(&mut hir);
        }

        Ok(hir)
    }

    /// Analyze a single function using dataflow type inference
    pub fn infer_function_types(&self, func: &hir::HirFunction) -> dataflow::InferredTypes {
        let inferencer = dataflow::DataflowTypeInferencer::new();
        inferencer.infer_function(func)
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

impl DepylerPipeline {
    /// Creates a new transpilation pipeline with verification enabled.
    ///
    /// Verification is not yet wired into the pipeline; this is a
    /// forward-compatible constructor for when it is.
    pub fn new_with_verification() -> Self {
        Self {
            enable_verification: true,
            ..Self::new()
        }
    }
}
