//! High-level API for dataflow-based type inference

use super::cfg::CfgBuilder;
use super::lattice::{LatticeType, TypeState};
use super::solver::{FixpointSolver, TypePropagation};
use crate::ast_bridge::AstBridge;
use crate::hir::{HirFunction, HirModule, HirStmt, Type};
use anyhow::Result;
use rustpython_ast::Suite;
use rustpython_parser::Parse;
use std::collections::HashMap;

/// Infer types for all functions in a Python source string.
///
/// This is a convenience function that parses Python code and runs
/// dataflow-based type inference on all functions.
///
/// # Example
/// ```ignore
/// use depyler_core::dataflow::infer_python;
///
/// let result = infer_python(r#"
/// def greet(name: str) -> str:
///     result = []
///     result.append("Hello, ")
///     result.append(name)
///     return "".join(result)
/// "#)?;
///
/// for (func_name, types) in result {
///     println!("Function: {}", func_name);
///     println!("  Return type: {:?}", types.return_type);
///     for (var, ty) in types.all_variables() {
///         println!("  {}: {:?}", var, ty);
///     }
/// }
/// ```
pub fn infer_python(source: &str) -> Result<HashMap<String, InferredTypes>> {
    // Parse Python source to AST
    let statements = Suite::parse(source, "<input>").map_err(|e| anyhow::anyhow!("Python parse error: {}", e))?;

    let ast = rustpython_ast::Mod::Module(rustpython_ast::ModModule {
        body: statements,
        type_ignores: vec![],
        range: Default::default(),
    });

    // Convert to HIR
    let hir_module = AstBridge::new().with_source(source.to_string()).python_to_hir(ast)?;

    // Use infer_module which handles interprocedural analysis
    let inferencer = DataflowTypeInferencer::new();
    Ok(inferencer.infer_module(&hir_module))
}

/// Infer types for a single Python function string.
///
/// # Example
/// ```ignore
/// use depyler_core::dataflow::infer_python_function;
///
/// let types = infer_python_function(r#"
/// def process(items: list[int]) -> int:
///     total = 0
///     for item in items:
///         total += item
///     return total
/// "#)?;
///
/// println!("Return type: {:?}", types.return_type);
/// println!("total: {:?}", types.get_variable_type("total"));
/// ```
pub fn infer_python_function(source: &str) -> Result<InferredTypes> {
    // Parse Python source to AST
    let statements = Suite::parse(source, "<input>").map_err(|e| anyhow::anyhow!("Python parse error: {}", e))?;

    let ast = rustpython_ast::Mod::Module(rustpython_ast::ModModule {
        body: statements,
        type_ignores: vec![],
        range: Default::default(),
    });

    // Convert to HIR
    let hir_module = AstBridge::new().with_source(source.to_string()).python_to_hir(ast)?;

    // Get the last function (typically what the user wants when testing)
    let last_func = hir_module
        .functions
        .last()
        .ok_or_else(|| anyhow::anyhow!("No function found in source"))?;

    // Run type inference with module context
    let inferencer = DataflowTypeInferencer::new();
    let results = inferencer.infer_module(&hir_module);

    results
        .get(&last_func.name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Failed to infer types for function"))
}

/// Result of dataflow type inference
#[derive(Debug, Clone)]
pub struct InferredTypes {
    /// Types for each variable at the function's end
    pub variable_types: HashMap<String, Type>,
    /// Inferred parameter types (if not annotated)
    pub parameter_types: HashMap<String, Type>,
    /// Inferred return type
    pub return_type: Option<Type>,
    /// Number of iterations to reach fixpoint
    pub iterations: usize,
    /// Whether all types were definitively inferred (no Unknown)
    pub is_complete: bool,
}

impl InferredTypes {
    /// Get the type of a variable
    pub fn get_variable_type(&self, name: &str) -> Option<&Type> {
        self.variable_types.get(name)
    }

    /// Get all variable names with their types
    pub fn all_variables(&self) -> impl Iterator<Item = (&String, &Type)> {
        self.variable_types.iter()
    }
}

/// Dataflow-based type inferencer that produces definitive types
pub struct DataflowTypeInferencer {
    /// Whether to use annotations when available
    use_annotations: bool,
    /// Whether to infer return types from return statements
    infer_return_types: bool,
}

impl DataflowTypeInferencer {
    pub fn new() -> Self {
        Self {
            use_annotations: true,
            infer_return_types: true,
        }
    }

    /// Create inferencer that ignores existing annotations
    pub fn ignore_annotations(mut self) -> Self {
        self.use_annotations = false;
        self
    }

    /// Disable return type inference
    pub fn without_return_inference(mut self) -> Self {
        self.infer_return_types = false;
        self
    }

    /// Infer types for a single function
    /// Infer types for a single function
    pub fn infer_function(&self, func: &HirFunction) -> InferredTypes {
        self.infer_function_with_context(func, &HashMap::new())
    }

    /// Infer types for a single function with knowledge of other function signatures
    pub fn infer_function_with_context(
        &self,
        func: &HirFunction,
        user_functions: &HashMap<String, Type>,
    ) -> InferredTypes {
        // Collect initial parameter types
        let mut param_types: HashMap<String, Type> = HashMap::new();
        for param in &func.params {
            let ty = if self.use_annotations && !matches!(param.ty, Type::Unknown) {
                param.ty.clone()
            } else {
                Type::Unknown
            };
            param_types.insert(param.name.clone(), ty);
        }

        // Build CFG and run dataflow analysis
        let cfg = CfgBuilder::new().build_function(func);
        let analysis = TypePropagation::new(param_types.clone()).with_user_functions(user_functions.clone());
        let result = FixpointSolver::solve(&analysis, &cfg);

        // Extract final types from all exit points
        let mut variable_types: HashMap<String, Type> = HashMap::new();

        // Merge types from all blocks (take most specific from any reachable point)
        for (_, state) in &result.out_facts {
            for (var, ty) in &state.vars {
                let hir_ty = ty.to_hir_type();
                if !matches!(hir_ty, Type::Unknown) {
                    variable_types
                        .entry(var.clone())
                        .and_modify(|existing| {
                            // Join types to find common type
                            let existing_lattice = LatticeType::from_hir_type(existing);
                            let new_lattice = LatticeType::from_hir_type(&hir_ty);
                            *existing = existing_lattice.join(&new_lattice).to_hir_type();
                        })
                        .or_insert(hir_ty);
                }
            }
        }

        // Infer parameter types from usage if not annotated
        let mut inferred_param_types: HashMap<String, Type> = HashMap::new();
        for param in &func.params {
            if matches!(param.ty, Type::Unknown) {
                // Try to infer from variable_types
                if let Some(ty) = variable_types.get(&param.name) {
                    if !matches!(ty, Type::Unknown) {
                        inferred_param_types.insert(param.name.clone(), ty.clone());
                    }
                }
            }
        }

        // Infer return type from return statements
        let return_type = if self.infer_return_types {
            self.infer_return_type(func, &result.out_facts)
        } else {
            None
        };

        // Check completeness
        let is_complete = variable_types.values().all(|ty| !matches!(ty, Type::Unknown))
            && return_type.as_ref().map_or(true, |ty| !matches!(ty, Type::Unknown));

        InferredTypes {
            variable_types,
            parameter_types: inferred_param_types,
            return_type,
            iterations: result.iterations,
            is_complete,
        }
    }

    /// Infer types for all functions in a module
    pub fn infer_module(&self, module: &HirModule) -> HashMap<String, InferredTypes> {
        // First pass: collect all function return types from annotations
        let mut user_functions: HashMap<String, Type> = HashMap::new();
        for func in &module.functions {
            if self.use_annotations && !matches!(func.ret_type, Type::Unknown) {
                user_functions.insert(func.name.clone(), func.ret_type.clone());
            }
        }

        // Second pass: infer types with knowledge of other functions
        let mut results = HashMap::new();
        for func in &module.functions {
            results.insert(
                func.name.clone(),
                self.infer_function_with_context(func, &user_functions),
            );
        }
        results
    }

    /// Apply inferred types back to a function (mutates the HIR)
    pub fn apply_types(&self, func: &mut HirFunction, inferred: &InferredTypes) {
        // Apply parameter types
        for param in &mut func.params {
            if matches!(param.ty, Type::Unknown) {
                if let Some(ty) = inferred.parameter_types.get(&param.name) {
                    param.ty = ty.clone();
                }
            }
        }

        // Apply return type
        if matches!(func.ret_type, Type::Unknown) {
            if let Some(ty) = &inferred.return_type {
                func.ret_type = ty.clone();
            }
        }

        // Apply variable types to type annotations in assignments
        self.apply_types_to_body(&mut func.body, inferred);
    }

    /// Apply inferred types to all functions in a module
    pub fn apply_types_to_module(&self, module: &mut HirModule) {
        let inferred = self.infer_module(module);
        for func in &mut module.functions {
            if let Some(func_inferred) = inferred.get(&func.name) {
                self.apply_types(func, func_inferred);
            }
        }
    }

    fn apply_types_to_body(&self, body: &mut [HirStmt], inferred: &InferredTypes) {
        for stmt in body {
            match stmt {
                HirStmt::Assign {
                    target,
                    type_annotation,
                    ..
                } => {
                    if type_annotation.is_none() {
                        if let crate::hir::AssignTarget::Symbol(name) = target {
                            if let Some(ty) = inferred.variable_types.get(name) {
                                if !matches!(ty, Type::Unknown) {
                                    *type_annotation = Some(ty.clone());
                                }
                            }
                        }
                    }
                }
                HirStmt::If {
                    then_body, else_body, ..
                } => {
                    self.apply_types_to_body(then_body, inferred);
                    if let Some(else_stmts) = else_body {
                        self.apply_types_to_body(else_stmts, inferred);
                    }
                }
                HirStmt::While { body, .. } => {
                    self.apply_types_to_body(body, inferred);
                }
                HirStmt::For { body, .. } => {
                    self.apply_types_to_body(body, inferred);
                }
                HirStmt::Try {
                    body,
                    handlers,
                    orelse,
                    finalbody,
                } => {
                    self.apply_types_to_body(body, inferred);
                    for handler in handlers {
                        self.apply_types_to_body(&mut handler.body, inferred);
                    }
                    if let Some(else_stmts) = orelse {
                        self.apply_types_to_body(else_stmts, inferred);
                    }
                    if let Some(finally_stmts) = finalbody {
                        self.apply_types_to_body(finally_stmts, inferred);
                    }
                }
                HirStmt::With { body, .. } => {
                    self.apply_types_to_body(body, inferred);
                }
                HirStmt::FunctionDef { body, .. } => {
                    // Nested functions get their own inference
                    self.apply_types_to_body(body, inferred);
                }
                _ => {}
            }
        }
    }

    fn infer_return_type(
        &self,
        func: &HirFunction,
        out_facts: &HashMap<super::cfg::BlockId, TypeState>,
    ) -> Option<Type> {
        // Collect all return types from return statements
        let mut return_types: Vec<Type> = Vec::new();
        self.collect_return_types(&func.body, out_facts, &mut return_types);

        if return_types.is_empty() {
            return Some(Type::None);
        }

        // Join all return types
        let mut result = LatticeType::from_hir_type(&return_types[0]);
        for ty in &return_types[1..] {
            result = result.join(&LatticeType::from_hir_type(ty));
        }

        let final_type = result.to_hir_type();
        if matches!(final_type, Type::Unknown) {
            None
        } else {
            Some(final_type)
        }
    }

    fn collect_return_types(
        &self,
        body: &[HirStmt],
        out_facts: &HashMap<super::cfg::BlockId, TypeState>,
        return_types: &mut Vec<Type>,
    ) {
        // Get a merged state from all facts for expression type inference
        let merged_state = out_facts.values().fold(TypeState::new(), |acc, state| acc.join(state));
        let analysis = TypePropagation::new(HashMap::new());

        for stmt in body {
            match stmt {
                HirStmt::Return(Some(expr)) => {
                    let ty = analysis.infer_expr_type(expr, &merged_state);
                    return_types.push(ty);
                }
                HirStmt::Return(None) => {
                    return_types.push(Type::None);
                }
                HirStmt::If {
                    then_body, else_body, ..
                } => {
                    self.collect_return_types(then_body, out_facts, return_types);
                    if let Some(else_stmts) = else_body {
                        self.collect_return_types(else_stmts, out_facts, return_types);
                    }
                }
                HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                    self.collect_return_types(body, out_facts, return_types);
                }
                HirStmt::Try {
                    body,
                    handlers,
                    orelse,
                    finalbody,
                } => {
                    self.collect_return_types(body, out_facts, return_types);
                    for handler in handlers {
                        self.collect_return_types(&handler.body, out_facts, return_types);
                    }
                    if let Some(else_stmts) = orelse {
                        self.collect_return_types(else_stmts, out_facts, return_types);
                    }
                    if let Some(finally_stmts) = finalbody {
                        self.collect_return_types(finally_stmts, out_facts, return_types);
                    }
                }
                HirStmt::With { body, .. } => {
                    self.collect_return_types(body, out_facts, return_types);
                }
                _ => {}
            }
        }
    }
}

impl Default for DataflowTypeInferencer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{AssignTarget, BinOp, FunctionProperties, HirExpr, HirParam, Literal};
    use depyler_annotations::TranspilationAnnotations;

    fn make_function(name: &str, params: Vec<HirParam>, ret_type: Type, body: Vec<HirStmt>) -> HirFunction {
        HirFunction {
            name: name.to_string(),
            params: smallvec::SmallVec::from_vec(params),
            ret_type,
            body,
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        }
    }

    #[test]
    fn test_infer_simple_function() {
        let func = make_function(
            "add",
            vec![
                HirParam::new("a".to_string(), Type::Int),
                HirParam::new("b".to_string(), Type::Int),
            ],
            Type::Unknown,
            vec![HirStmt::Return(Some(HirExpr::Binary {
                op: BinOp::Add,
                left: Box::new(HirExpr::Var("a".to_string())),
                right: Box::new(HirExpr::Var("b".to_string())),
            }))],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        assert_eq!(result.return_type, Some(Type::Int));
        assert!(result.is_complete);
    }

    #[test]
    fn test_infer_variable_types() {
        let func = make_function(
            "test",
            vec![],
            Type::Unknown,
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("x".to_string()),
                    value: HirExpr::Literal(Literal::Int(42)),
                    type_annotation: None,
                },
                HirStmt::Assign {
                    target: AssignTarget::Symbol("y".to_string()),
                    value: HirExpr::Literal(Literal::String("hello".to_string())),
                    type_annotation: None,
                },
                HirStmt::Assign {
                    target: AssignTarget::Symbol("z".to_string()),
                    value: HirExpr::Binary {
                        op: BinOp::Add,
                        left: Box::new(HirExpr::Var("x".to_string())),
                        right: Box::new(HirExpr::Literal(Literal::Int(1))),
                    },
                    type_annotation: None,
                },
                HirStmt::Return(Some(HirExpr::Var("z".to_string()))),
            ],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        assert_eq!(result.variable_types.get("x"), Some(&Type::Int));
        assert_eq!(result.variable_types.get("y"), Some(&Type::String));
        assert_eq!(result.variable_types.get("z"), Some(&Type::Int));
        assert_eq!(result.return_type, Some(Type::Int));
    }

    #[test]
    fn test_infer_with_branches() {
        let func = make_function(
            "test",
            vec![HirParam::new("cond".to_string(), Type::Bool)],
            Type::Unknown,
            vec![
                HirStmt::If {
                    condition: HirExpr::Var("cond".to_string()),
                    then_body: vec![HirStmt::Assign {
                        target: AssignTarget::Symbol("result".to_string()),
                        value: HirExpr::Literal(Literal::Int(1)),
                        type_annotation: None,
                    }],
                    else_body: Some(vec![HirStmt::Assign {
                        target: AssignTarget::Symbol("result".to_string()),
                        value: HirExpr::Literal(Literal::Int(0)),
                        type_annotation: None,
                    }]),
                },
                HirStmt::Return(Some(HirExpr::Var("result".to_string()))),
            ],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        // Both branches assign Int, so result should be Int
        assert_eq!(result.variable_types.get("result"), Some(&Type::Int));
        assert_eq!(result.return_type, Some(Type::Int));
    }

    #[test]
    fn test_infer_list_operations() {
        let func = make_function(
            "test",
            vec![],
            Type::Unknown,
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("items".to_string()),
                    value: HirExpr::List(vec![
                        HirExpr::Literal(Literal::Int(1)),
                        HirExpr::Literal(Literal::Int(2)),
                        HirExpr::Literal(Literal::Int(3)),
                    ]),
                    type_annotation: None,
                },
                HirStmt::Return(Some(HirExpr::Var("items".to_string()))),
            ],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        assert_eq!(
            result.variable_types.get("items"),
            Some(&Type::List(Box::new(Type::Int)))
        );
    }

    #[test]
    fn test_infer_dict_operations() {
        let func = make_function(
            "test",
            vec![],
            Type::Unknown,
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("data".to_string()),
                    value: HirExpr::Dict(vec![(
                        HirExpr::Literal(Literal::String("key".to_string())),
                        HirExpr::Literal(Literal::Int(42)),
                    )]),
                    type_annotation: None,
                },
                HirStmt::Return(Some(HirExpr::Var("data".to_string()))),
            ],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        assert_eq!(
            result.variable_types.get("data"),
            Some(&Type::Dict(Box::new(Type::String), Box::new(Type::Int)))
        );
    }

    #[test]
    fn test_apply_types() {
        let mut func = make_function(
            "test",
            vec![HirParam::new("x".to_string(), Type::Unknown)],
            Type::Unknown,
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("y".to_string()),
                    value: HirExpr::Binary {
                        op: BinOp::Add,
                        left: Box::new(HirExpr::Var("x".to_string())),
                        right: Box::new(HirExpr::Literal(Literal::Int(1))),
                    },
                    type_annotation: None,
                },
                HirStmt::Return(Some(HirExpr::Var("y".to_string()))),
            ],
        );

        let inferencer = DataflowTypeInferencer::new();
        let inferred = inferencer.infer_function(&func);
        inferencer.apply_types(&mut func, &inferred);

        // Return type should be inferred (will be Unknown or Int depending on inference)
        // Variable y should have type annotation applied
    }

    #[test]
    fn test_infer_module() {
        let module = HirModule {
            functions: vec![
                make_function(
                    "func1",
                    vec![HirParam::new("x".to_string(), Type::Int)],
                    Type::Unknown,
                    vec![HirStmt::Return(Some(HirExpr::Binary {
                        op: BinOp::Mul,
                        left: Box::new(HirExpr::Var("x".to_string())),
                        right: Box::new(HirExpr::Literal(Literal::Int(2))),
                    }))],
                ),
                make_function(
                    "func2",
                    vec![HirParam::new("s".to_string(), Type::String)],
                    Type::Unknown,
                    vec![HirStmt::Return(Some(HirExpr::Var("s".to_string())))],
                ),
            ],
            imports: vec![],
            type_aliases: vec![],
            protocols: vec![],
            classes: vec![],
            constants: vec![],
        };

        let inferencer = DataflowTypeInferencer::new();
        let results = inferencer.infer_module(&module);

        assert!(results.contains_key("func1"));
        assert!(results.contains_key("func2"));
        assert_eq!(results.get("func1").unwrap().return_type, Some(Type::Int));
        assert_eq!(results.get("func2").unwrap().return_type, Some(Type::String));
    }

    #[test]
    fn test_infer_no_return() {
        let func = make_function(
            "no_return",
            vec![],
            Type::Unknown,
            vec![HirStmt::Assign {
                target: AssignTarget::Symbol("x".to_string()),
                value: HirExpr::Literal(Literal::Int(42)),
                type_annotation: None,
            }],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        // No explicit return means None
        assert_eq!(result.return_type, Some(Type::None));
    }

    #[test]
    fn test_infer_multiple_returns() {
        let func = make_function(
            "multi_return",
            vec![HirParam::new("x".to_string(), Type::Int)],
            Type::Unknown,
            vec![HirStmt::If {
                condition: HirExpr::Binary {
                    op: BinOp::Gt,
                    left: Box::new(HirExpr::Var("x".to_string())),
                    right: Box::new(HirExpr::Literal(Literal::Int(0))),
                },
                then_body: vec![HirStmt::Return(Some(HirExpr::Literal(Literal::Int(1))))],
                else_body: Some(vec![HirStmt::Return(Some(HirExpr::Literal(Literal::Int(-1))))]),
            }],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        // Both returns are Int
        assert_eq!(result.return_type, Some(Type::Int));
    }

    #[test]
    fn test_infer_loop_variable() {
        let func = make_function(
            "sum_list",
            vec![HirParam::new("items".to_string(), Type::List(Box::new(Type::Int)))],
            Type::Unknown,
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("total".to_string()),
                    value: HirExpr::Literal(Literal::Int(0)),
                    type_annotation: None,
                },
                HirStmt::For {
                    target: AssignTarget::Symbol("item".to_string()),
                    iter: HirExpr::Var("items".to_string()),
                    body: vec![HirStmt::Assign {
                        target: AssignTarget::Symbol("total".to_string()),
                        value: HirExpr::Binary {
                            op: BinOp::Add,
                            left: Box::new(HirExpr::Var("total".to_string())),
                            right: Box::new(HirExpr::Var("item".to_string())),
                        },
                        type_annotation: None,
                    }],
                },
                HirStmt::Return(Some(HirExpr::Var("total".to_string()))),
            ],
        );

        let inferencer = DataflowTypeInferencer::new();
        let result = inferencer.infer_function(&func);

        assert_eq!(result.variable_types.get("total"), Some(&Type::Int));
        assert_eq!(result.return_type, Some(Type::Int));
    }
}
