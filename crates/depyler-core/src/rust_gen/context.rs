//! Code generation context and core traits
//!
//! This module provides the central CodeGenContext struct that maintains
//! state during Rust code generation, along with core traits used across
//! the code generation pipeline.

use crate::annotation_aware_type_mapper::AnnotationAwareTypeMapper;
use crate::hir::{ExceptionScope, Type};
use crate::string_optimization::StringOptimizer;
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Error type classification for Result<T, E> return types
///
/// or a concrete error type (single type). This determines if raise statements
/// need Box::new() wrapper.
///
/// # Complexity
/// N/A (enum definition)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType {
    /// Concrete error type (e.g., ValueError, ZeroDivisionError)
    /// No wrapping needed: `return Err(ValueError::new(...))`
    Concrete(String),
    /// Box<dyn Error> - mixed or generic error types
    /// Needs wrapping: `return Err(Box::new(ValueError::new(...)))`
    DynBox,
}

/// Code generation context
///
/// Maintains all state needed during Rust code generation including:
/// - Type mapping and string optimization
/// - Import tracking (needs_hashmap, needs_cow, etc.)
/// - Variable scoping and mutability analysis
/// - Generator state management
///
/// # Complexity
/// N/A (data structure)
pub struct CodeGenContext<'a> {
    pub type_mapper: &'a crate::type_mapper::TypeMapper,
    pub annotation_aware_mapper: AnnotationAwareTypeMapper,
    pub string_optimizer: StringOptimizer,
    pub union_enum_generator: crate::union_enum_gen::UnionEnumGenerator,
    pub generated_enums: Vec<proc_macro2::TokenStream>,
    pub needs_hashmap: bool,
    pub needs_hashset: bool,
    pub needs_vecdeque: bool,
    pub needs_fnv_hashmap: bool,
    pub needs_ahash_hashmap: bool,
    pub needs_arc: bool,
    pub needs_rc: bool,
    pub needs_cow: bool,
    pub needs_rand: bool,
    pub needs_slice_random: bool,
    pub needs_serde_json: bool,
    pub needs_regex: bool,
    pub needs_chrono: bool,
    pub needs_clap: bool, // Track clap dependency for ArgumentParser
    pub needs_csv: bool,
    pub needs_rust_decimal: bool,
    pub needs_num_rational: bool,
    pub needs_base64: bool,
    pub needs_md5: bool,
    pub needs_sha2: bool,
    pub needs_sha3: bool,
    pub needs_blake2: bool,
    pub needs_hex: bool,
    pub needs_uuid: bool,
    pub needs_hmac: bool,
    pub needs_crc32: bool,
    pub needs_url_encoding: bool,
    pub needs_lazy_static: bool,
    pub declared_vars: Vec<HashSet<String>>,
    pub current_function_can_fail: bool,
    pub current_return_type: Option<Type>,
    pub module_mapper: crate::module_mapper::ModuleMapper,
    pub imported_modules: std::collections::HashMap<String, crate::module_mapper::ModuleMapping>,
    pub imported_items: std::collections::HashMap<String, String>,
    pub mutable_vars: HashSet<String>,
    pub needs_zerodivisionerror: bool,
    pub needs_indexerror: bool,
    pub needs_valueerror: bool,
    pub needs_argumenttypeerror: bool,
    pub is_classmethod: bool,
    pub in_generator: bool,
    pub generator_state_vars: HashSet<String>,
    pub var_types: HashMap<String, Type>,
    pub class_names: HashSet<String>,
    /// Map from class name to map of field name to field type
    pub class_field_types: HashMap<String, HashMap<String, Type>>,
    pub mutating_methods: HashMap<String, HashSet<String>>,
    pub function_return_types: HashMap<String, Type>,
    pub function_param_borrows: HashMap<String, Vec<bool>>,
    /// Track function parameters that need mutable borrows (&mut T)
    pub function_param_muts: HashMap<String, Vec<bool>>,
    pub tuple_iter_vars: HashSet<String>,
    pub is_final_statement: bool,
    pub result_bool_functions: HashSet<String>,
    pub result_returning_functions: HashSet<String>,
    pub current_error_type: Option<ErrorType>,
    pub exception_scopes: Vec<ExceptionScope>,
    pub argparser_tracker: crate::rust_gen::argparse_transform::ArgParserTracker,
    pub generated_args_struct: Option<proc_macro2::TokenStream>,
    pub generated_commands_enum: Option<proc_macro2::TokenStream>,
    pub current_subcommand_fields: Option<std::collections::HashSet<String>>,

    pub validator_functions: std::collections::HashSet<String>,

    pub stdlib_mappings: crate::stdlib_mappings::StdlibMappings,

    /// Track parameters in the current function that are &mut references
    pub current_func_mut_ref_params: HashSet<String>,

    /// Track parameters in the current function that are & references (immutable)
    pub current_func_ref_params: HashSet<String>,

    pub function_param_names: HashMap<String, Vec<String>>,

    /// Track function parameter types for Optional unwrap analysis
    pub function_param_types: HashMap<String, Vec<Type>>,

    /// Track how many times each variable is used in the current function (for clone analysis)
    pub var_usage_counts: HashMap<String, usize>,
    /// Track how many times we've seen each variable during code generation
    pub var_usage_current: HashMap<String, usize>,
}

impl<'a> CodeGenContext<'a> {
    /// Enter a new lexical scope
    ///
    /// # Complexity
    /// 1 (simple push)
    pub fn enter_scope(&mut self) {
        self.declared_vars.push(HashSet::new());
    }

    /// Exit the current lexical scope
    ///
    /// # Complexity
    /// 1 (simple pop)
    pub fn exit_scope(&mut self) {
        self.declared_vars.pop();
    }

    /// Check if a variable is declared in any scope
    ///
    /// # Complexity
    /// 2 (iterator + any)
    pub fn is_declared(&self, var_name: &str) -> bool {
        self.declared_vars.iter().any(|scope| scope.contains(var_name))
    }

    /// Declare a variable in the current scope
    ///
    /// # Complexity
    /// 2 (if let + insert)
    pub fn declare_var(&mut self, var_name: &str) {
        if let Some(current_scope) = self.declared_vars.last_mut() {
            current_scope.insert(var_name.to_string());
        }
    }

    /// Process a Union type and generate an enum if needed
    ///
    /// Returns the enum name and optionally generates an enum definition
    /// that is added to generated_enums.
    ///
    /// # Complexity
    /// 2 (if + push)
    pub fn process_union_type(&mut self, types: &[crate::hir::Type]) -> String {
        let (enum_name, enum_def) = self.union_enum_generator.generate_union_enum(types);
        if !enum_def.is_empty() {
            self.generated_enums.push(enum_def);
        }
        enum_name
    }

    // ========================================================================
    // ========================================================================

    /// Get the current exception scope
    ///
    /// Returns the most recent scope from the stack, or Unhandled if stack is empty.
    ///
    /// # Complexity
    /// 2 (last + unwrap_or)
    pub fn current_exception_scope(&self) -> &ExceptionScope {
        self.exception_scopes.last().unwrap_or(&ExceptionScope::Unhandled)
    }

    /// Check if currently inside a try block
    ///
    /// # Complexity
    /// 2 (current_exception_scope + matches)
    pub fn is_in_try_block(&self) -> bool {
        matches!(self.current_exception_scope(), ExceptionScope::TryCaught { .. })
    }

    /// Check if a specific exception type is handled by current try block
    ///
    /// Returns true if:
    /// - Inside a try block with bare except (empty handled_types)
    /// - Inside a try block that explicitly handles this exception type
    ///
    /// # Complexity
    /// 4 (match + is_empty + contains + comparison)
    pub fn is_exception_handled(&self, exception_type: &str) -> bool {
        if let ExceptionScope::TryCaught { handled_types } = self.current_exception_scope() {
            // Empty list = bare except (catches all)
            handled_types.is_empty() || handled_types.contains(&exception_type.to_string())
        } else {
            false
        }
    }

    /// Enter a try block scope with specified exception handlers
    ///
    /// # Complexity
    /// 1 (simple push)
    pub fn enter_try_scope(&mut self, handled_types: Vec<String>) {
        self.exception_scopes.push(ExceptionScope::TryCaught { handled_types });
    }

    /// Enter an exception handler scope
    ///
    /// # Complexity
    /// 1 (simple push)
    pub fn enter_handler_scope(&mut self) {
        self.exception_scopes.push(ExceptionScope::Handler);
    }

    /// Exit the current exception scope
    ///
    /// # Complexity
    /// 1 (simple pop)
    pub fn exit_exception_scope(&mut self) {
        self.exception_scopes.pop();
    }

    /// Check if an expression evaluates to a float type
    ///
    /// Used by math builtins to determine whether to use float or integer methods.
    pub fn is_expr_float_type(&self, expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::{BinOp, HirExpr, Literal};
        match expr {
            HirExpr::Var(var_name) => {
                matches!(self.var_types.get(var_name), Some(Type::Float))
            }
            HirExpr::Literal(Literal::Float(_)) => true,
            HirExpr::Attribute { value, attr } => self.get_attribute_field_type(value, attr) == Some(Type::Float),
            HirExpr::Unary { operand, .. } => self.is_expr_float_type(operand),
            HirExpr::Binary { op, left, right } => {
                // Division always produces float, or if either operand is float
                matches!(op, BinOp::Div) || self.is_expr_float_type(left) || self.is_expr_float_type(right)
            }
            HirExpr::Call { func, args, .. } => {
                // Check if function has a known float return type
                if matches!(self.function_return_types.get(func), Some(Type::Float)) {
                    return true;
                }
                // float() always returns float
                if func == "float" {
                    return true;
                }
                // Functions that return float when given float args (type-preserving)
                // abs(), min(), max(), sum() return the same type as their arguments
                if matches!(func.as_str(), "abs" | "min" | "max" | "sum") {
                    return args.iter().any(|arg| self.is_expr_float_type(arg));
                }
                false
            }
            HirExpr::IfExpr { body, orelse, .. } => self.is_expr_float_type(body) || self.is_expr_float_type(orelse),
            _ => false,
        }
    }

    /// Check if an expression evaluates to an integer type
    ///
    /// Used by arithmetic operations to handle mixed int/float types.
    pub fn is_expr_int_type(&self, expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::{BinOp, HirExpr, Literal};
        match expr {
            HirExpr::Var(var_name) => {
                matches!(self.var_types.get(var_name), Some(Type::Int))
            }
            HirExpr::Literal(Literal::Int(_)) => true,
            HirExpr::Attribute { value, attr } => self.get_attribute_field_type(value, attr) == Some(Type::Int),
            HirExpr::Unary { operand, .. } => self.is_expr_int_type(operand),
            // Binary operations are int type if:
            // - Division (/) always produces float, so exclude
            // - Both operands are int types for other operations
            HirExpr::Binary { op, left, right } => {
                // Division always produces float in Python
                if matches!(op, BinOp::Div) {
                    return false;
                }
                // For other arithmetic operations, result is int if both operands are int
                self.is_expr_int_type(left) && self.is_expr_int_type(right)
            }
            // Function calls - check if the function returns int
            HirExpr::Call { func, args, .. } => {
                if matches!(self.function_return_types.get(func), Some(Type::Int)) {
                    return true;
                }
                // Built-in functions that always return int
                if matches!(func.as_str(), "len" | "int" | "ord" | "round") {
                    return true;
                }
                // abs() returns the same type as its argument
                if func == "abs" && !args.is_empty() {
                    return self.is_expr_int_type(&args[0]);
                }
                false
            }
            HirExpr::IfExpr { body, orelse, .. } => self.is_expr_int_type(body) && self.is_expr_int_type(orelse),
            _ => false,
        }
    }

    /// Check if an expression evaluates to a string type
    ///
    /// Used by string concatenation vs arithmetic addition detection.
    pub fn is_expr_string_type(&self, expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::{HirExpr, Literal};
        match expr {
            HirExpr::Var(var_name) => {
                matches!(self.var_types.get(var_name), Some(Type::String))
            }
            HirExpr::Literal(Literal::String(_)) => true,
            HirExpr::Attribute { value, attr } => self.get_attribute_field_type(value, attr) == Some(Type::String),
            HirExpr::Call { func, .. } => {
                if matches!(self.function_return_types.get(func), Some(Type::String)) {
                    return true;
                }
                // Built-in functions that return string
                matches!(func.as_str(), "str" | "repr" | "chr" | "format")
            }
            HirExpr::IfExpr { body, orelse, .. } => self.is_expr_string_type(body) && self.is_expr_string_type(orelse),
            HirExpr::FString { .. } => true,
            _ => false,
        }
    }

    /// Check if an expression evaluates to a boolean type
    ///
    /// Used by logical operations type inference.
    pub fn is_expr_bool_type(&self, expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::{BinOp, HirExpr, Literal};
        match expr {
            HirExpr::Var(var_name) => {
                matches!(self.var_types.get(var_name), Some(Type::Bool))
            }
            HirExpr::Literal(Literal::Bool(_)) => true,
            HirExpr::Attribute { value, attr } => self.get_attribute_field_type(value, attr) == Some(Type::Bool),
            // Comparison operations always return bool
            HirExpr::Binary { op, .. } => {
                matches!(
                    op,
                    BinOp::Eq
                        | BinOp::NotEq
                        | BinOp::Lt
                        | BinOp::LtEq
                        | BinOp::Gt
                        | BinOp::GtEq
                        | BinOp::In
                        | BinOp::NotIn
                        | BinOp::Is
                        | BinOp::IsNot
                        | BinOp::And
                        | BinOp::Or
                )
            }
            HirExpr::Call { func, .. } => {
                if matches!(self.function_return_types.get(func), Some(Type::Bool)) {
                    return true;
                }
                // Built-in functions that return bool
                matches!(
                    func.as_str(),
                    "bool" | "isinstance" | "issubclass" | "callable" | "hasattr"
                )
            }
            HirExpr::IfExpr { body, orelse, .. } => self.is_expr_bool_type(body) && self.is_expr_bool_type(orelse),
            _ => false,
        }
    }

    /// Get the type of a field access expression (e.g., state.val1)
    pub fn get_attribute_field_type(&self, value: &Box<crate::hir::HirExpr>, attr: &str) -> Option<Type> {
        // Get the class name from the value expression
        let class_name = match value.as_ref() {
            crate::hir::HirExpr::Var(var_name) => {
                // Look up the variable's type to find the class name
                self.var_types.get(var_name).and_then(|ty| {
                    if let Type::Custom(name) = ty {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
            }
            _ => None,
        };

        // Look up the field type in class_field_types
        if let Some(class_name) = class_name {
            if let Some(field_types) = self.class_field_types.get(&class_name) {
                return field_types.get(attr).cloned();
            }
        }

        None
    }

    /// Get the inferred type of an expression.
    /// Returns None if the type cannot be determined.
    pub fn get_expr_type(&self, expr: &crate::hir::HirExpr) -> Option<Type> {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Var(name) => self.var_types.get(name).cloned(),
            HirExpr::Attribute { value, attr } => self.get_attribute_field_type(value, attr),
            HirExpr::Literal(lit) => Some(match lit {
                crate::hir::Literal::Int(_) => Type::Int,
                crate::hir::Literal::Float(_) => Type::Float,
                crate::hir::Literal::String(_) => Type::String,
                crate::hir::Literal::Bool(_) => Type::Bool,
                crate::hir::Literal::None => Type::None,
                crate::hir::Literal::Bytes(_) => Type::List(Box::new(Type::Int)),
            }),
            _ => None,
        }
    }

    /// Check if an expression is Optional type and return the inner type if so.
    pub fn get_optional_inner_type(&self, expr: &crate::hir::HirExpr) -> Option<Type> {
        if let Some(Type::Optional(inner)) = self.get_expr_type(expr) {
            Some((*inner).clone())
        } else {
            None
        }
    }

    /// Reset variable usage tracking for a new function
    pub fn reset_var_usage(&mut self) {
        self.var_usage_counts.clear();
        self.var_usage_current.clear();
    }

    /// Increment usage count for a variable during analysis
    pub fn count_var_use(&mut self, var_name: &str) {
        *self.var_usage_counts.entry(var_name.to_string()).or_insert(0) += 1;
    }

    /// Check if this is the last use of a variable (can move instead of clone)
    pub fn is_last_var_use(&mut self, var_name: &str) -> bool {
        let total = self.var_usage_counts.get(var_name).copied().unwrap_or(1);
        let current = self.var_usage_current.entry(var_name.to_string()).or_insert(0);
        *current += 1;
        *current >= total
    }

    /// Check if a variable needs cloning (non-Copy type with multiple uses)
    pub fn var_needs_clone(&mut self, var_name: &str) -> bool {
        // Check if the variable type is non-Copy
        let var_type = self.var_types.get(var_name);
        let is_non_copy = var_type.is_some_and(|t| Self::type_needs_clone(t));

        if !is_non_copy {
            return false;
        }

        // Check if this is the last use
        !self.is_last_var_use(var_name)
    }

    /// Check if a type needs clone (is not Copy)
    fn type_needs_clone(ty: &Type) -> bool {
        match ty {
            // Copy types - don't need clone
            Type::Int | Type::Float | Type::Bool | Type::None => false,
            // Non-Copy types - need clone
            Type::String | Type::List(_) | Type::Dict(_, _) | Type::Set(_) | Type::Custom(_) => true,
            // Optional needs clone if inner type needs clone
            Type::Optional(inner) => Self::type_needs_clone(inner),
            // Tuple needs clone if any element needs clone
            Type::Tuple(types) => types.iter().any(Self::type_needs_clone),
            // Arrays, Generics, Functions, etc. - assume need clone for safety
            Type::Array { .. } | Type::Generic { .. } | Type::Function { .. } | Type::Union(_) => true,
            // TypeVar and Unknown - assume need clone
            Type::TypeVar(_) | Type::Unknown => true,
            // Final wraps another type
            Type::Final(inner) => Self::type_needs_clone(inner),
        }
    }

    /// Analyze variable usage in function body before code generation
    pub fn analyze_var_usage(&mut self, stmts: &[crate::hir::HirStmt]) {
        self.reset_var_usage();
        for stmt in stmts {
            self.count_var_uses_in_stmt(stmt);
        }
    }

    fn count_var_uses_in_stmt(&mut self, stmt: &crate::hir::HirStmt) {
        use crate::hir::HirStmt;
        match stmt {
            HirStmt::Assign { value, .. } => {
                self.count_var_uses_in_expr(value);
            }
            HirStmt::Expr(expr) => {
                self.count_var_uses_in_expr(expr);
            }
            HirStmt::Return(Some(expr)) => {
                self.count_var_uses_in_expr(expr);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.count_var_uses_in_expr(condition);
                for s in then_body {
                    self.count_var_uses_in_stmt(s);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.count_var_uses_in_stmt(s);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                self.count_var_uses_in_expr(condition);
                for s in body {
                    self.count_var_uses_in_stmt(s);
                }
            }
            HirStmt::For { iter, body, .. } => {
                self.count_var_uses_in_expr(iter);
                for s in body {
                    self.count_var_uses_in_stmt(s);
                }
            }
            HirStmt::Assert { test, msg, .. } => {
                self.count_var_uses_in_expr(test);
                if let Some(m) = msg {
                    self.count_var_uses_in_expr(m);
                }
            }
            HirStmt::Raise { exception, .. } => {
                if let Some(e) = exception {
                    self.count_var_uses_in_expr(e);
                }
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                for s in body {
                    self.count_var_uses_in_stmt(s);
                }
                for handler in handlers {
                    for s in &handler.body {
                        self.count_var_uses_in_stmt(s);
                    }
                }
                if let Some(els) = orelse {
                    for s in els {
                        self.count_var_uses_in_stmt(s);
                    }
                }
                if let Some(fin) = finalbody {
                    for s in fin {
                        self.count_var_uses_in_stmt(s);
                    }
                }
            }
            HirStmt::With { context, body, .. } => {
                self.count_var_uses_in_expr(context);
                for s in body {
                    self.count_var_uses_in_stmt(s);
                }
            }
            _ => {}
        }
    }

    fn count_var_uses_in_expr(&mut self, expr: &crate::hir::HirExpr) {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Var(name) => {
                self.count_var_use(name);
            }
            HirExpr::Binary { left, right, .. } => {
                self.count_var_uses_in_expr(left);
                self.count_var_uses_in_expr(right);
            }
            HirExpr::Unary { operand, .. } => {
                self.count_var_uses_in_expr(operand);
            }
            HirExpr::Call { args, kwargs, .. } => {
                for arg in args {
                    self.count_var_uses_in_expr(arg);
                }
                for (_, v) in kwargs {
                    self.count_var_uses_in_expr(v);
                }
            }
            HirExpr::MethodCall {
                object, args, kwargs, ..
            } => {
                // Method calls borrow the receiver, so if the object is just a variable,
                // don't count it as a consuming use. Only count nested expressions.
                if !matches!(**object, HirExpr::Var(_)) {
                    self.count_var_uses_in_expr(object);
                }
                for arg in args {
                    self.count_var_uses_in_expr(arg);
                }
                for (_, v) in kwargs {
                    self.count_var_uses_in_expr(v);
                }
            }
            HirExpr::Attribute { value, .. } => {
                // Attribute access borrows the object (e.g., person.name gives &String),
                // so accessing person.first_name and person.last_name doesn't consume person.
                // Only count nested expressions, not direct variable access.
                if !matches!(**value, HirExpr::Var(_)) {
                    self.count_var_uses_in_expr(value);
                }
            }
            HirExpr::Index { base, index } => {
                self.count_var_uses_in_expr(base);
                self.count_var_uses_in_expr(index);
            }
            HirExpr::Slice {
                base,
                start,
                stop,
                step,
            } => {
                self.count_var_uses_in_expr(base);
                if let Some(l) = start {
                    self.count_var_uses_in_expr(l);
                }
                if let Some(u) = stop {
                    self.count_var_uses_in_expr(u);
                }
                if let Some(s) = step {
                    self.count_var_uses_in_expr(s);
                }
            }
            HirExpr::List(elts) | HirExpr::Tuple(elts) | HirExpr::Set(elts) => {
                for e in elts {
                    self.count_var_uses_in_expr(e);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    self.count_var_uses_in_expr(k);
                    self.count_var_uses_in_expr(v);
                }
            }
            HirExpr::IfExpr { test, body, orelse } => {
                self.count_var_uses_in_expr(test);
                self.count_var_uses_in_expr(body);
                self.count_var_uses_in_expr(orelse);
            }
            HirExpr::ListComp {
                element,
                target: _,
                iter,
                condition,
            } => {
                self.count_var_uses_in_expr(element);
                self.count_var_uses_in_expr(iter);
                if let Some(c) = condition {
                    self.count_var_uses_in_expr(c);
                }
            }
            HirExpr::Lambda { body, .. } => {
                self.count_var_uses_in_expr(body);
            }
            _ => {}
        }
    }
}

/// Trait for converting HIR elements to Rust tokens
///
/// This is the main trait for code generation. All HIR types that can
/// be converted to Rust code implement this trait.
pub trait RustCodeGen {
    fn to_rust_tokens(&self, ctx: &mut CodeGenContext) -> Result<proc_macro2::TokenStream>;
}

/// Extension trait for converting expressions to Rust syn::Expr
///
/// Used internally for expression-to-expression conversions where
/// we need syn::Expr specifically rather than TokenStream.
pub trait ToRustExpr {
    fn to_rust_expr(&self, ctx: &mut CodeGenContext) -> Result<syn::Expr>;
}
