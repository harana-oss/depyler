use crate::annotations::{AnnotationExtractor, AnnotationParser};
use crate::hir::*;
use anyhow::{Result, bail};
use rustpython_ast::{self as ast};

mod converters;
pub mod expr_utils;
mod properties;
mod type_extraction;

pub use converters::{ExprConverter, SpanContext, StmtConverter};
pub use properties::FunctionAnalyzer;
pub use type_extraction::TypeExtractor;

// Re-export helpers that converters.rs defines (they were previously defined here
// and some external code within the crate may still reference them through this path).
pub(crate) use converters::{
    convert_aug_op, convert_binop, convert_body, convert_cmpop, convert_expr, convert_import,
    convert_import_from, convert_parameters, convert_stmt, convert_unaryop, extract_assign_target,
    extract_docstring_and_body, infer_parameter_types_from_classes,
};

/// Bridge between Python AST and Depyler HIR
pub struct AstBridge {
    source_code: Option<String>,
    annotation_extractor: AnnotationExtractor,
    annotation_parser: AnnotationParser,
}

impl Default for AstBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl AstBridge {
    /// Creates a new AST bridge with default configuration
    pub fn new() -> Self {
        Self {
            source_code: None,
            annotation_extractor: AnnotationExtractor::new(),
            annotation_parser: AnnotationParser::new(),
        }
    }

    /// Sets the source code for better error reporting and debugging
    pub fn with_source(mut self, source: String) -> Self {
        self.source_code = Some(source);
        self
    }

    /// Converts a Python AST module to Depyler HIR
    pub fn python_to_hir(&self, module: ast::Mod) -> Result<HirModule> {
        match module {
            ast::Mod::Module(m) => self.convert_module(m),
            _ => bail!("Only module-level code is supported"),
        }
    }

    fn convert_module(&self, module: ast::ModModule) -> Result<HirModule> {
        let mut functions = Vec::new();
        let mut imports = Vec::new();
        let mut type_aliases = Vec::new();
        let mut protocols = Vec::new();
        let mut classes = Vec::new();
        let mut constants = Vec::new();
        let mut statements = Vec::new();

        // If we have mixed constants and executable statements, we'll treat all
        // module-level assignments as executable code (local variables in main).
        let has_executable_statements = module.body.iter().any(|stmt| {
            !matches!(
                stmt,
                ast::Stmt::FunctionDef(_)
                    | ast::Stmt::AsyncFunctionDef(_)
                    | ast::Stmt::ClassDef(_)
                    | ast::Stmt::Import(_)
                    | ast::Stmt::ImportFrom(_)
                    | ast::Stmt::Assign(_)
                    | ast::Stmt::AnnAssign(_)
            )
        });

        // Also check if there are any type aliases - if so, treat as having executable statements
        // so that type aliases and their usage are both put in the main function
        let has_type_aliases = module.body.iter().any(|stmt| {
            matches!(stmt, ast::Stmt::TypeAlias(_))
                || matches!(stmt, ast::Stmt::Assign(assign) if Self::looks_like_type_alias(assign))
        });

        let has_executable_statements = has_executable_statements || has_type_aliases;

        for stmt in module.body {
            match stmt {
                ast::Stmt::FunctionDef(f) => {
                    functions.push(self.convert_function(f, false)?);
                }
                ast::Stmt::Import(i) => {
                    imports.extend(convert_import(i)?);
                }
                ast::Stmt::ImportFrom(i) => {
                    imports.extend(convert_import_from(i)?);
                }
                ast::Stmt::AsyncFunctionDef(f) => {
                    functions.push(self.convert_async_function(f)?);
                }
                ast::Stmt::ClassDef(class) => {
                    // Try to parse as protocol first
                    if let Some(protocol) = self.try_convert_protocol(&class)? {
                        protocols.push(protocol);
                    } else {
                        // Convert regular class
                        if let Some(hir_class) = self.try_convert_class(&class)? {
                            classes.push(hir_class);
                        }
                    }
                }
                ast::Stmt::Assign(assign) => {
                    // Skip TypeVar assignments - they're only for generic type parameters
                    if Self::is_typevar_assignment(&ast::Stmt::Assign(assign.clone())) {
                        continue;
                    }

                    // Try to parse as type alias first
                    if let Some(type_alias) = self.try_convert_type_alias(&assign)? {
                        type_aliases.push(type_alias);
                    } else if !has_executable_statements {
                        // Only treat as constant if there are NO executable statements
                        if let Some(constant) = self.try_convert_constant(&assign)? {
                            constants.push(constant);
                        } else {
                            // Otherwise, treat as executable statement
                            statements.push(convert_stmt(ast::Stmt::Assign(assign))?);
                        }
                    } else {
                        // If there are executable statements, treat ALL assignments as executable
                        statements.push(convert_stmt(ast::Stmt::Assign(assign))?);
                    }
                }
                ast::Stmt::AnnAssign(ann_assign) => {
                    // Skip TypeVar assignments - they're only for generic type parameters
                    if Self::is_typevar_assignment(&ast::Stmt::AnnAssign(ann_assign.clone())) {
                        continue;
                    }

                    // Try to parse annotated assignment as type alias first
                    if let Some(type_alias) = self.try_convert_annotated_type_alias(&ann_assign)? {
                        type_aliases.push(type_alias);
                    } else if !has_executable_statements {
                        // Only treat as constant if there are NO executable statements
                        if let Some(constant) = self.try_convert_annotated_constant(&ann_assign)? {
                            constants.push(constant);
                        } else {
                            // Otherwise, treat as executable statement
                            statements.push(convert_stmt(ast::Stmt::AnnAssign(ann_assign))?);
                        }
                    } else {
                        // If there are executable statements, treat ALL assignments as executable
                        statements.push(convert_stmt(ast::Stmt::AnnAssign(ann_assign))?);
                    }
                }
                ast::Stmt::TypeAlias(type_alias_stmt) => {
                    if let Some(type_alias) = self.convert_type_alias_stmt(&type_alias_stmt)? {
                        type_aliases.push(type_alias);
                    }
                }
                _ => {
                    // Other statements (e.g., print, expression statements) are executable
                    statements.push(convert_stmt(stmt)?);
                }
            }
        }

        // Post-process: Infer parameter types from class names
        infer_parameter_types_from_classes(&mut classes);

        Ok(HirModule {
            functions,
            imports,
            type_aliases,
            protocols,
            classes,
            constants,
            statements,
        })
    }
}
