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

/// Parameter borrow information for interprocedural analysis
///
/// Contains information about how a parameter should be borrowed
/// based on interprocedural analysis results.
///
/// # Complexity
/// N/A (data structure)
#[derive(Debug, Copy, Clone)]
pub struct ParamBorrowInfo {
    /// Whether the parameter should be borrowed instead of owned
    pub should_borrow: bool,
    /// Whether the parameter needs a mutable borrow (&mut T)
    pub needs_mut: bool,
    /// Whether the parameter takes ownership (no borrowing)
    pub takes_ownership: bool,
}

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
    pub needs_smallvec: bool,
    pub needs_rand: bool,
    pub needs_small_rng: bool,
    pub needs_slice_random: bool,
    pub needs_indexed_random: bool,
    pub needs_serde_json: bool,
    pub needs_regex: bool,
    pub needs_chrono: bool,
    pub needs_unicode_normalization: bool,
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
    pub needs_complex: bool,
    pub declared_vars: Vec<HashSet<String>>,
    pub current_function_can_fail: bool,
    pub current_function_name: Option<String>,
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
    pub enum_names: HashSet<String>,
    /// Struct names that derive Copy (all fields are Copy types)
    pub copy_structs: HashSet<String>,
    /// Map from class name to map of field name to field type
    pub class_field_types: HashMap<String, HashMap<String, Type>>,
    pub mutating_methods: HashMap<String, HashSet<String>>,
    pub function_return_types: HashMap<String, Type>,
    /// Track parameter borrow information for interprocedural analysis
    pub function_param_borrows: HashMap<String, Vec<ParamBorrowInfo>>,
    /// Track parameter borrow information for interprocedural analysis
    pub function_param_strategies:
        HashMap<String, Vec<crate::borrowing_context::BorrowingStrategy>>,
    /// Track current function parameter ownership
    pub current_function_param_ownership: HashMap<String, bool>,
    /// Track parameters that require cloning
    pub param_clone_requirements: HashSet<String>,
    /// Track function parameters that need mutable borrows (&mut T)
    pub function_param_muts: HashMap<String, Vec<bool>>,
    /// Track functions whose return values are mutated at call sites (need &mut return type)
    pub functions_with_mutated_return: HashSet<String>,
    /// Track functions that return references (have params_with_field_return non-empty)
    /// These are functions like _get_players that return &Vec<T> borrowing from a param
    pub functions_returning_refs: HashSet<String>,
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

    /// Interprocedural analysis results for cross-function mutation tracking
    pub interprocedural_analysis: Option<&'a crate::interprocedural::InterproceduralAnalysis<'a>>,

    /// Track classes that need _get_field/_set_field methods for dynamic attribute access
    pub classes_needing_dynamic_access: HashSet<String>,

    /// Track parameters in the current function that are &mut references
    pub current_func_mut_ref_params: HashSet<String>,

    /// Track parameters in the current function that are & references (immutable)
    pub current_func_ref_params: HashSet<String>,

    /// Track variables that shadow ref params (e.g., for-loop variables with same name)
    pub shadowed_ref_params: HashSet<String>,

    pub function_param_names: HashMap<String, Vec<String>>,

    /// Track function parameter types for Optional unwrap analysis
    pub function_param_types: HashMap<String, Vec<Type>>,

    /// Track how many times each variable is used in the current function (for clone analysis)
    pub var_usage_counts: HashMap<String, usize>,

    /// Track how many times we've seen each variable during code generation
    pub var_usage_current: HashMap<String, usize>,

    /// Track variables that were declared with Option<T> type in Rust (even if HIR narrows them)
    pub optional_vars: HashSet<String>,

    /// Track module-level constants that use lazy_static (need dereferencing to get actual type)
    pub lazy_static_constants: HashSet<String>,

    /// Track module-level constants that are static arrays (use direct indexing)
    pub static_array_constants: HashSet<String>,

    /// Flag to indicate we're generating code for an assignment target (LHS)
    /// When true, use get_mut() for array access and as_mut() for Optional access
    pub is_assignment_target: bool,

    /// Flag to temporarily prevent cloning during expression generation
    /// Unlike is_assignment_target, this does NOT affect get() vs get_mut() choice
    pub prevent_clone: bool,

    /// Flag to indicate the current function returns a reference (to avoid cloning in return expressions)
    pub returns_reference: bool,

    /// Flag to indicate the current function returns a mutable reference (&mut T)
    pub returns_mutable_reference: bool,

    /// Variables that can be borrowed instead of cloned (from usage analysis)
    /// Key: variable name, Value: true if the variable should be borrowed
    pub borrowable_vars: HashSet<String>,

    /// Variables that need mutable borrowing (from usage analysis)
    /// These are field-source variables that are mutated and whose source is &mut T
    pub mut_borrowable_vars: HashSet<String>,

    /// Flag to indicate we should generate a borrow instead of clone for the current expression
    pub generate_borrow: bool,

    /// Flag to indicate we should generate a mutable borrow (&mut) instead of immutable (&)
    pub generate_mut_borrow: bool,

    /// Flag to indicate that a .clone() has already been added to the current expression.
    /// This prevents duplicate .clone() calls when multiple code paths try to add cloning.
    /// Reset to false at the start of each new expression conversion.
    pub clone_already_applied: bool,

    /// Flag to indicate we're converting an expression inside a primitive cast (int(), float()).
    /// When true, prevents adding references to if-expression branches.
    pub in_primitive_cast: bool,

    /// Variables that need to be cloned at assignment to avoid borrow conflicts.
    /// Pattern: var1 = f(&state) returns &T, then var2 = g(&mut state), then var1 used later.
    /// The immutable borrow in var1 would conflict with the mutable borrow for var2.
    /// Solution: clone var1 at assignment so the borrow ends immediately.
    pub vars_needing_clone_at_assign: HashSet<String>,

    /// Variables that are consumed in multiple move positions (e.g., assigned to
    /// a struct field AND passed to push/append). All uses except the last need .clone().
    pub vars_needing_clone_for_move: HashSet<String>,

    /// Counter tracking how many consuming (move) uses have been generated so far per variable.
    pub move_consume_current: HashMap<String, usize>,

    /// Total consuming (move) uses per variable, populated by analyze_move_consuming_uses.
    pub move_consume_totals: HashMap<String, usize>,

    /// Variables that need automatic `*` dereference when used standalone.
    /// Set inside `.filter()` closures where the iterator variable is `&T`.
    /// NOT applied for `.field` or `.method()` access (Rust auto-deref handles those).
    pub filter_deref_vars: HashSet<String>,
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
        self.declared_vars
            .iter()
            .any(|scope| scope.contains(var_name))
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

    /// Set the generate_borrow flag
    pub fn set_generate_borrow(&mut self, value: bool) {
        self.generate_borrow = value;
    }

    /// Set the generate_mut_borrow flag
    pub fn set_generate_mut_borrow(&mut self, value: bool) {
        self.generate_mut_borrow = value;
    }

    /// Mark that complex numbers are used (triggers num-complex import)
    pub fn mark_complex_used(&mut self) {
        self.needs_complex = true;
    }

    /// Check if a variable should be borrowed instead of cloned
    pub fn should_borrow_var(&self, var_name: &str) -> bool {
        self.borrowable_vars.contains(var_name)
    }

    /// Check if a variable should be mutably borrowed instead of cloned
    pub fn should_mut_borrow_var(&self, var_name: &str) -> bool {
        self.mut_borrowable_vars.contains(var_name)
    }

    /// Mark a variable as borrowable
    pub fn mark_as_borrowable(&mut self, var_name: &str) {
        self.borrowable_vars.insert(var_name.to_string());
    }

    /// Check if an attribute field is a Copy type (doesn't need borrowing or cloning)
    /// Returns true if the field is definitely a Copy type, false otherwise.
    /// For unknown types, returns true to avoid incorrect borrowing of primitives.
    pub fn is_attribute_copy_type(&self, value: &Box<crate::hir::HirExpr>, attr: &str) -> bool {
        if let Some(field_type) = self.get_attribute_field_type(value, attr) {
            !self.type_needs_clone(&field_type)
        } else {
            // Unknown type - assume Copy to avoid incorrectly borrowing primitives
            // like nested field access (e.g., state.ball_location.x)
            true
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
        self.exception_scopes
            .last()
            .unwrap_or(&ExceptionScope::Unhandled)
    }

    /// Check if currently inside a try block
    ///
    /// # Complexity
    /// 2 (current_exception_scope + matches)
    pub fn is_in_try_block(&self) -> bool {
        matches!(
            self.current_exception_scope(),
            ExceptionScope::TryCaught { .. }
        )
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
        self.exception_scopes
            .push(ExceptionScope::TryCaught { handled_types });
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
            HirExpr::Attribute { value, attr } => {
                self.get_attribute_field_type(value, attr) == Some(Type::Float)
            }
            HirExpr::Unary { operand, .. } => self.is_expr_float_type(operand),
            HirExpr::Binary { op, left, right } => {
                // Division always produces float, or if either operand is float
                matches!(op, BinOp::Div)
                    || self.is_expr_float_type(left)
                    || self.is_expr_float_type(right)
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
            HirExpr::IfExpr { body, orelse, .. } => {
                self.is_expr_float_type(body) || self.is_expr_float_type(orelse)
            }
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
            HirExpr::Attribute { value, attr } => {
                self.get_attribute_field_type(value, attr) == Some(Type::Int)
            }
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
            HirExpr::IfExpr { body, orelse, .. } => {
                self.is_expr_int_type(body) && self.is_expr_int_type(orelse)
            }
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
            HirExpr::Attribute { value, attr } => {
                self.get_attribute_field_type(value, attr) == Some(Type::String)
            }
            HirExpr::Call { func, .. } => {
                if matches!(self.function_return_types.get(func), Some(Type::String)) {
                    return true;
                }
                // Built-in functions that return string
                matches!(func.as_str(), "str" | "repr" | "chr" | "format")
            }
            HirExpr::IfExpr { body, orelse, .. } => {
                self.is_expr_string_type(body) && self.is_expr_string_type(orelse)
            }
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
            HirExpr::Attribute { value, attr } => {
                self.get_attribute_field_type(value, attr) == Some(Type::Bool)
            }
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
            HirExpr::IfExpr { body, orelse, .. } => {
                self.is_expr_bool_type(body) && self.is_expr_bool_type(orelse)
            }
            _ => false,
        }
    }

    /// Get the type of a field access expression (e.g., state.val1)
    pub fn get_attribute_field_type(
        &self,
        value: &Box<crate::hir::HirExpr>,
        attr: &str,
    ) -> Option<Type> {
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
                crate::hir::Literal::Ellipsis => Type::None,
                crate::hir::Literal::Complex(_, _) => Type::Custom("num::Complex<f64>".to_string()),
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
        *self
            .var_usage_counts
            .entry(var_name.to_string())
            .or_insert(0) += 1;
    }

    /// Check if this is the last use of a variable (can move instead of clone)
    pub fn is_last_var_use(&mut self, var_name: &str) -> bool {
        let total = self.var_usage_counts.get(var_name).copied().unwrap_or(1);
        let current = self
            .var_usage_current
            .entry(var_name.to_string())
            .or_insert(0);
        *current += 1;
        *current >= total
    }

    /// Check if a variable needs cloning (non-Copy type with multiple uses)
    pub fn var_needs_clone(&mut self, var_name: &str) -> bool {
        // Variables from tuple iteration over string literals are &str (Copy type)
        // They don't need .clone() - .to_string() will be added when needed
        if self.tuple_iter_vars.contains(var_name) {
            return false;
        }

        // Check if the variable type is non-Copy
        let var_type = self.var_types.get(var_name).cloned();
        let is_non_copy = var_type.as_ref().is_some_and(|t| self.type_needs_clone(t));

        if !is_non_copy {
            return false;
        }

        // Check if this is the last use
        !self.is_last_var_use(var_name)
    }

    /// Check if a type needs clone (is not Copy)
    fn type_needs_clone(&self, ty: &Type) -> bool {
        match ty {
            // Copy types - don't need clone
            Type::Int | Type::Float | Type::Bool | Type::None => false,
            // Non-Copy types - need clone
            Type::String | Type::List(_) | Type::Dict(_, _) | Type::Set(_) => true,
            // Custom types: enums and all-Copy-field structs are Copy
            Type::Custom(name) => {
                !self.enum_names.contains(name) && !self.copy_structs.contains(name)
            }
            // Optional needs clone if inner type needs clone
            Type::Optional(inner) => self.type_needs_clone(inner),
            // Tuple needs clone if any element needs clone
            Type::Tuple(types) => types.iter().any(|t| self.type_needs_clone(t)),
            // Arrays, Generics, Functions, etc. - assume need clone for safety
            Type::Array { .. } | Type::Generic { .. } | Type::Function { .. } | Type::Union(_) => {
                true
            }
            // TypeVar and Unknown - assume need clone
            Type::TypeVar(_) | Type::Unknown => true,
            // Final wraps another type
            Type::Final(inner) => self.type_needs_clone(inner),
        }
    }

    /// Get the type of a field in the current class (when accessed via self.field)
    pub fn get_self_field_type(&self, field_name: &str) -> Option<Type> {
        // Find the class we're currently in by looking at class_field_types
        // The current class is tracked when we enter a class impl block
        for (_, field_types) in &self.class_field_types {
            if let Some(field_type) = field_types.get(field_name) {
                return Some(field_type.clone());
            }
        }
        None
    }

    /// Analyze variable usage in function body before code generation
    pub fn analyze_var_usage(&mut self, stmts: &[crate::hir::HirStmt]) {
        self.reset_var_usage();
        self.borrowable_vars.clear();
        self.mut_borrowable_vars.clear();

        // First pass: identify field-source variables and count all uses
        let field_source_vars = self.collect_field_source_vars(stmts);
        for stmt in stmts {
            self.count_var_uses_in_stmt(stmt);
        }

        // Second pass: analyze if field-source variables can be borrowed
        for var_name in &field_source_vars {
            // Check for mutable borrowing first (requires &mut source AND mutation)
            if self.can_var_mut_borrow(stmts, var_name) {
                self.mut_borrowable_vars.insert(var_name.clone());
            } else if self.can_var_borrow(stmts, var_name) {
                self.borrowable_vars.insert(var_name.clone());
            }
        }
    }

    /// Analyze only borrow eligibility for field-source variables without
    /// resetting var_usage_counts (avoids side effects on clone decisions).
    pub fn analyze_field_borrowing(&mut self, stmts: &[crate::hir::HirStmt]) {
        self.borrowable_vars.clear();
        self.mut_borrowable_vars.clear();

        let field_source_vars = self.collect_field_source_vars(stmts);
        for var_name in &field_source_vars {
            if self.can_var_mut_borrow(stmts, var_name) {
                self.mut_borrowable_vars.insert(var_name.clone());
            } else if self.can_var_borrow(stmts, var_name) {
                self.borrowable_vars.insert(var_name.clone());
            }
        }
    }

    /// Detect variables consumed in multiple move positions and mark them for cloning.
    /// Consuming positions: attribute assignment RHS, push/append/extend args, return values,
    /// and function call args where the callee takes ownership.
    pub fn analyze_move_consuming_uses(&mut self, stmts: &[crate::hir::HirStmt]) {
        self.vars_needing_clone_for_move.clear();
        self.move_consume_current.clear();
        self.move_consume_totals.clear();
        let mut consuming_counts: HashMap<String, usize> = HashMap::new();
        for stmt in stmts {
            self.count_move_consuming_uses_in_stmt(stmt, &mut consuming_counts);
        }
        for (name, count) in &consuming_counts {
            if *count > 1 {
                // Don't clone borrowed function parameters (&T is Copy)
                let is_borrowed_param = self
                    .current_function_param_ownership
                    .get(name)
                    .is_some_and(|takes_ownership| !takes_ownership);
                if !is_borrowed_param {
                    self.vars_needing_clone_for_move.insert(name.clone());
                }
            }
        }
        self.move_consume_totals = consuming_counts;
    }

    /// Check if a variable at a consuming position needs .clone() (not the last move use).
    pub fn should_clone_for_move(&mut self, var_name: &str) -> bool {
        if !self.vars_needing_clone_for_move.contains(var_name) {
            return false;
        }
        // Check type at codegen time (var_types populated by earlier assignments)
        let is_non_copy = self
            .var_types
            .get(var_name)
            .is_some_and(|t| self.type_needs_clone(t));
        if !is_non_copy {
            return false;
        }
        let total = self.move_consume_totals.get(var_name).copied().unwrap_or(1);
        let current = self
            .move_consume_current
            .entry(var_name.to_string())
            .or_insert(0);
        *current += 1;
        // Clone all uses except the last
        *current < total
    }

    /// Count consuming (move) uses of variables in a statement.
    fn count_move_consuming_uses_in_stmt(
        &self,
        stmt: &crate::hir::HirStmt,
        counts: &mut HashMap<String, usize>,
    ) {
        use crate::hir::{AssignTarget, HirExpr, HirStmt};
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                match target {
                    // obj.field = var → consumes var (moves into field)
                    AssignTarget::Attribute { .. } => {
                        self.collect_consuming_vars(value, counts);
                    }
                    // let x = var → consumes var (moves into binding)
                    AssignTarget::Symbol(_) => {
                        self.collect_consuming_vars(value, counts);
                    }
                    _ => {}
                }
            }
            // Bare expression: e.g., list.append(var) or func(var)
            HirStmt::Expr(expr) => {
                if let HirExpr::MethodCall { args, .. } = expr {
                    for arg in args {
                        self.collect_consuming_vars(arg, counts);
                    }
                } else if let HirExpr::Call { args, .. } = expr {
                    for arg in args {
                        self.collect_consuming_vars(arg, counts);
                    }
                }
            }
            HirStmt::Return(Some(expr)) => {
                self.collect_consuming_vars(expr, counts);
            }
            // Recurse into nested blocks
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                for s in then_body {
                    self.count_move_consuming_uses_in_stmt(s, counts);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.count_move_consuming_uses_in_stmt(s, counts);
                    }
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                for s in body {
                    self.count_move_consuming_uses_in_stmt(s, counts);
                }
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                for s in body {
                    self.count_move_consuming_uses_in_stmt(s, counts);
                }
                for handler in handlers {
                    for s in &handler.body {
                        self.count_move_consuming_uses_in_stmt(s, counts);
                    }
                }
                if let Some(els) = orelse {
                    for s in els {
                        self.count_move_consuming_uses_in_stmt(s, counts);
                    }
                }
                if let Some(fin) = finalbody {
                    for s in fin {
                        self.count_move_consuming_uses_in_stmt(s, counts);
                    }
                }
            }
            HirStmt::With { body, .. } => {
                for s in body {
                    self.count_move_consuming_uses_in_stmt(s, counts);
                }
            }
            _ => {}
        }
    }

    /// Collect top-level variable references that represent a consuming (move) use.
    fn collect_consuming_vars(
        &self,
        expr: &crate::hir::HirExpr,
        counts: &mut HashMap<String, usize>,
    ) {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Var(name) => {
                *counts.entry(name.clone()).or_insert(0) += 1;
            }
            // Call args are consuming (the function receives ownership or the codegen
            // will handle borrowing separately)
            HirExpr::Call { args, .. } => {
                for arg in args {
                    if let HirExpr::Var(name) = arg {
                        *counts.entry(name.clone()).or_insert(0) += 1;
                    }
                }
            }
            _ => {}
        }
    }

    /// Collect variable names that are assigned from field access (e.g., `players = state.all_players`)
    fn collect_field_source_vars(&self, stmts: &[crate::hir::HirStmt]) -> Vec<String> {
        let mut result = Vec::new();
        for stmt in stmts {
            self.collect_field_source_vars_in_stmt(stmt, &mut result);
        }
        result
    }

    fn collect_field_source_vars_in_stmt(
        &self,
        stmt: &crate::hir::HirStmt,
        result: &mut Vec<String>,
    ) {
        use crate::hir::{AssignTarget, HirStmt};
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                if let AssignTarget::Symbol(var_name) = target {
                    if self.is_attribute_sourced(value) {
                        result.push(var_name.clone());
                    }
                }
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                for s in then_body {
                    self.collect_field_source_vars_in_stmt(s, result);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.collect_field_source_vars_in_stmt(s, result);
                    }
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                for s in body {
                    self.collect_field_source_vars_in_stmt(s, result);
                }
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                for s in body {
                    self.collect_field_source_vars_in_stmt(s, result);
                }
                for handler in handlers {
                    for s in &handler.body {
                        self.collect_field_source_vars_in_stmt(s, result);
                    }
                }
                if let Some(els) = orelse {
                    for s in els {
                        self.collect_field_source_vars_in_stmt(s, result);
                    }
                }
                if let Some(fin) = finalbody {
                    for s in fin {
                        self.collect_field_source_vars_in_stmt(s, result);
                    }
                }
            }
            HirStmt::With { body, .. } => {
                for s in body {
                    self.collect_field_source_vars_in_stmt(s, result);
                }
            }
            HirStmt::FunctionDef { body, .. } => {
                for s in body {
                    self.collect_field_source_vars_in_stmt(s, result);
                }
            }
            _ => {}
        }
    }

    /// Check if an expression is "attribute-sourced" - either a direct attribute access
    /// or a conditional expression where both branches are attribute-sourced.
    fn is_attribute_sourced(&self, expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Attribute { .. } => true,
            HirExpr::Index { base, .. } => self.is_attribute_sourced(base),
            HirExpr::IfExpr { body, orelse, .. } => {
                self.is_attribute_sourced(body) && self.is_attribute_sourced(orelse)
            }
            _ => false,
        }
    }

    /// Check if an expression is an empty collection initialization.
    /// These should not block borrowability since they're just placeholder values.
    fn is_empty_collection_init(expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::HirExpr;
        match expr {
            // Empty list literal: []
            HirExpr::List(items) => items.is_empty(),
            // Empty dict literal: {}
            HirExpr::Dict(pairs) => pairs.is_empty(),
            // Empty set literal: set()
            HirExpr::Set(items) => items.is_empty(),
            // Built-in constructors: list(), dict(), set(), Vec::new(), etc.
            HirExpr::Call { func, args, .. } => {
                let is_empty_constructor = matches!(
                    func.as_str(),
                    "list" | "dict" | "set" | "Vec" | "HashMap" | "HashSet"
                );
                is_empty_constructor && args.is_empty()
            }
            // Default values that are common placeholder initializations
            HirExpr::Literal(lit) => {
                use crate::hir::Literal;
                match lit {
                    Literal::Int(0) => true,
                    Literal::Float(f) => *f == 0.0,
                    Literal::String(s) => s.is_empty(),
                    Literal::Bool(false) | Literal::None => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Check if an expression is sourced from a &mut T parameter's field.
    fn is_mut_ref_attribute_sourced(&self, expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Attribute { value, .. } => self.is_mut_ref_base(value),
            HirExpr::Index { base, .. } => self.is_mut_ref_attribute_sourced(base),
            HirExpr::IfExpr { body, orelse, .. } => {
                self.is_mut_ref_attribute_sourced(body) && self.is_mut_ref_attribute_sourced(orelse)
            }
            _ => false,
        }
    }

    /// Check if the base of an attribute access chain is a &mut T parameter.
    fn is_mut_ref_base(&self, expr: &crate::hir::HirExpr) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Var(name) => self.current_func_mut_ref_params.contains(name),
            HirExpr::Attribute { value, .. } | HirExpr::Index { base: value, .. } => {
                self.is_mut_ref_base(value)
            }
            _ => false,
        }
    }

    /// Check if a variable can be mutably borrowed instead of cloned.
    /// Requires: source is &mut T, variable is mutated, no moves/captures.
    fn can_var_mut_borrow(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        // Must have mutation use
        if !self.var_has_mut_use(stmts, var_name) {
            return false;
        }
        // Must be assigned from a &mut T source
        if !self.var_has_mut_ref_source(stmts, var_name) {
            return false;
        }
        // No move uses or closure captures
        !self.var_has_move_use(stmts, var_name)
            && !self.var_captured_in_closure(stmts, var_name)
            && !self.var_has_non_attribute_assignment(stmts, var_name)
    }

    /// Check if variable is assigned from a &mut T source (field of mutable reference param)
    fn var_has_mut_ref_source(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        for stmt in stmts {
            if self.stmt_has_mut_ref_source(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_mut_ref_source(&self, stmt: &crate::hir::HirStmt, var_name: &str) -> bool {
        use crate::hir::{AssignTarget, HirStmt};
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                if let AssignTarget::Symbol(name) = target {
                    if name == var_name && self.is_mut_ref_attribute_sourced(value) {
                        return true;
                    }
                }
                false
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                self.var_has_mut_ref_source(then_body, var_name)
                    || else_body
                        .as_ref()
                        .is_some_and(|e| self.var_has_mut_ref_source(e, var_name))
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                self.var_has_mut_ref_source(body, var_name)
            }
            _ => false,
        }
    }

    /// Check if a variable can be borrowed instead of cloned.
    /// Field-source variables (List/Dict/Set from attribute access) are passed by reference
    /// when used as function arguments, so function calls are NOT move uses.
    fn can_var_borrow(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        // Only check for actual moves (returns), not function calls (passed by reference)
        !self.var_has_return_move(stmts, var_name)
            && !self.var_field_access_returned(stmts, var_name)
            && !self.var_has_mut_use(stmts, var_name)
            && !self.var_captured_in_closure(stmts, var_name)
            && !self.var_has_non_attribute_assignment(stmts, var_name)
    }

    /// Check if a field of the variable is returned (e.g., `return team_stats.name`).
    /// Borrowing the variable won't work if we need to return an owned field from it.
    fn var_field_access_returned(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        for stmt in stmts {
            if self.stmt_has_field_return(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_field_return(&self, stmt: &crate::hir::HirStmt, var_name: &str) -> bool {
        use crate::hir::HirStmt;
        match stmt {
            HirStmt::Return(Some(expr)) => self.expr_accesses_var_field(expr, var_name),
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                self.var_field_access_returned(then_body, var_name)
                    || else_body
                        .as_ref()
                        .is_some_and(|e| self.var_field_access_returned(e, var_name))
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                self.var_field_access_returned(body, var_name)
            }
            _ => false,
        }
    }

    /// Check if an expression accesses a field on the given variable.
    fn expr_accesses_var_field(&self, expr: &crate::hir::HirExpr, var_name: &str) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Attribute { value, .. } | HirExpr::Index { base: value, .. } => {
                self.expr_is_or_contains_var(value, var_name)
            }
            HirExpr::IfExpr { body, orelse, .. } => {
                self.expr_accesses_var_field(body, var_name)
                    || self.expr_accesses_var_field(orelse, var_name)
            }
            _ => false,
        }
    }

    /// Check if variable is returned (actual move), not including function call args
    /// which are passed by reference for List/Dict/Set types.
    fn var_has_return_move(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        for stmt in stmts {
            if self.stmt_has_return_move(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_return_move(&self, stmt: &crate::hir::HirStmt, var_name: &str) -> bool {
        use crate::hir::HirStmt;
        match stmt {
            HirStmt::Return(Some(expr)) => self.expr_is_var_move(expr, var_name),
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                self.var_has_return_move(then_body, var_name)
                    || else_body
                        .as_ref()
                        .is_some_and(|e| self.var_has_return_move(e, var_name))
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                self.var_has_return_move(body, var_name)
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.var_has_return_move(body, var_name)
                    || handlers
                        .iter()
                        .any(|h| self.var_has_return_move(&h.body, var_name))
                    || orelse
                        .as_ref()
                        .is_some_and(|e| self.var_has_return_move(e, var_name))
                    || finalbody
                        .as_ref()
                        .is_some_and(|e| self.var_has_return_move(e, var_name))
            }
            HirStmt::With { body, .. } => self.var_has_return_move(body, var_name),
            _ => false,
        }
    }

    /// Check if variable is assigned from a non-attribute expression anywhere.
    /// If a var is assigned both from attributes and non-attributes in different branches,
    /// we cannot borrow consistently (types would mismatch: &T vs T).
    fn var_has_non_attribute_assignment(
        &self,
        stmts: &[crate::hir::HirStmt],
        var_name: &str,
    ) -> bool {
        for stmt in stmts {
            if self.stmt_has_non_attribute_assignment(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_non_attribute_assignment(
        &self,
        stmt: &crate::hir::HirStmt,
        var_name: &str,
    ) -> bool {
        use crate::hir::{AssignTarget, HirStmt};
        match stmt {
            HirStmt::Assign {
                target,
                value,
                type_annotation,
            } => {
                if let AssignTarget::Symbol(name) = target {
                    // Skip empty collection initializations WITHOUT type annotations
                    // Pattern: `players = list()` followed by `players = state.home_players`
                    // But DON'T skip if there's a type annotation like `players: list[Player] = list()`
                    // because that fixes the type and we can't later assign a reference
                    let is_empty_init_without_annotation =
                        Self::is_empty_collection_init(value) && type_annotation.is_none();
                    if name == var_name
                        && !self.is_attribute_sourced(value)
                        && !is_empty_init_without_annotation
                    {
                        return true;
                    }
                }
                false
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                self.var_has_non_attribute_assignment(then_body, var_name)
                    || else_body
                        .as_ref()
                        .is_some_and(|e| self.var_has_non_attribute_assignment(e, var_name))
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                self.var_has_non_attribute_assignment(body, var_name)
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.var_has_non_attribute_assignment(body, var_name)
                    || handlers
                        .iter()
                        .any(|h| self.var_has_non_attribute_assignment(&h.body, var_name))
                    || orelse
                        .as_ref()
                        .is_some_and(|e| self.var_has_non_attribute_assignment(e, var_name))
                    || finalbody
                        .as_ref()
                        .is_some_and(|e| self.var_has_non_attribute_assignment(e, var_name))
            }
            HirStmt::With { body, .. } => self.var_has_non_attribute_assignment(body, var_name),
            _ => false,
        }
    }

    /// Check if variable is moved (returned, passed to ownership-taking function)
    fn var_has_move_use(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        for stmt in stmts {
            if self.stmt_has_move_use(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_move_use(&self, stmt: &crate::hir::HirStmt, var_name: &str) -> bool {
        use crate::hir::HirStmt;
        match stmt {
            HirStmt::Return(Some(expr)) => self.expr_is_var_move(expr, var_name),
            HirStmt::Assign { value, .. } => {
                // Check if variable is passed as argument (potentially moved)
                self.expr_has_var_as_call_arg(value, var_name)
            }
            HirStmt::Expr(expr) => self.expr_has_var_as_call_arg(expr, var_name),
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                self.var_has_move_use(then_body, var_name)
                    || else_body
                        .as_ref()
                        .map(|e| self.var_has_move_use(e, var_name))
                        .unwrap_or(false)
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                self.var_has_move_use(body, var_name)
            }
            HirStmt::Raise { exception, .. } => exception
                .as_ref()
                .map(|e| self.expr_is_var_move(e, var_name))
                .unwrap_or(false),
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.var_has_move_use(body, var_name)
                    || handlers
                        .iter()
                        .any(|h| self.var_has_move_use(&h.body, var_name))
                    || orelse
                        .as_ref()
                        .map(|e| self.var_has_move_use(e, var_name))
                        .unwrap_or(false)
                    || finalbody
                        .as_ref()
                        .map(|e| self.var_has_move_use(e, var_name))
                        .unwrap_or(false)
            }
            HirStmt::With { body, .. } => self.var_has_move_use(body, var_name),
            _ => false,
        }
    }

    fn expr_is_var_move(&self, expr: &crate::hir::HirExpr, var_name: &str) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Var(name) => name == var_name,
            HirExpr::IfExpr { body, orelse, .. } => {
                self.expr_is_var_move(body, var_name) || self.expr_is_var_move(orelse, var_name)
            }
            _ => false,
        }
    }

    fn expr_has_var_as_call_arg(&self, expr: &crate::hir::HirExpr, var_name: &str) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Call { args, kwargs, .. } => {
                args.iter().any(|a| self.expr_is_var_move(a, var_name))
                    || kwargs
                        .iter()
                        .any(|(_, v)| self.expr_is_var_move(v, var_name))
            }
            HirExpr::MethodCall { args, kwargs, .. } => {
                args.iter().any(|a| self.expr_is_var_move(a, var_name))
                    || kwargs
                        .iter()
                        .any(|(_, v)| self.expr_is_var_move(v, var_name))
            }
            _ => false,
        }
    }

    /// Check if variable is mutated
    fn var_has_mut_use(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        for stmt in stmts {
            if self.stmt_has_mut_use(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_mut_use(&self, stmt: &crate::hir::HirStmt, var_name: &str) -> bool {
        use crate::hir::{AssignTarget, HirStmt};
        match stmt {
            HirStmt::Expr(expr) => self.expr_is_mutating_method_call(expr, var_name),
            HirStmt::Assign { target, .. } => {
                // Check if assignment target mutates our variable through attribute/index access
                self.assign_target_mutates_var(target, var_name)
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                self.var_has_mut_use(then_body, var_name)
                    || else_body
                        .as_ref()
                        .map(|e| self.var_has_mut_use(e, var_name))
                        .unwrap_or(false)
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                self.var_has_mut_use(body, var_name)
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.var_has_mut_use(body, var_name)
                    || handlers
                        .iter()
                        .any(|h| self.var_has_mut_use(&h.body, var_name))
                    || orelse
                        .as_ref()
                        .map(|e| self.var_has_mut_use(e, var_name))
                        .unwrap_or(false)
                    || finalbody
                        .as_ref()
                        .map(|e| self.var_has_mut_use(e, var_name))
                        .unwrap_or(false)
            }
            HirStmt::With { body, .. } => self.var_has_mut_use(body, var_name),
            _ => false,
        }
    }

    /// Check if an assignment target mutates a variable (attribute or index assignment)
    fn assign_target_mutates_var(&self, target: &crate::hir::AssignTarget, var_name: &str) -> bool {
        use crate::hir::AssignTarget;
        match target {
            AssignTarget::Symbol(_) => false, // Direct assignment doesn't mutate the var
            AssignTarget::Attribute { value, .. } => self.expr_is_or_contains_var(value, var_name),
            AssignTarget::Index { base, .. } => self.expr_is_or_contains_var(base, var_name),
            AssignTarget::Slice { base, .. } => self.expr_is_or_contains_var(base, var_name),
            AssignTarget::Tuple(targets) => targets
                .iter()
                .any(|t| self.assign_target_mutates_var(t, var_name)),
            AssignTarget::Starred(_) => false, // Starred target doesn't mutate a var
        }
    }

    /// Check if an expression is or contains a specific variable
    fn expr_is_or_contains_var(&self, expr: &crate::hir::HirExpr, var_name: &str) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Var(name) => name == var_name,
            HirExpr::Attribute { value, .. } => self.expr_is_or_contains_var(value, var_name),
            HirExpr::Index { base, .. } => self.expr_is_or_contains_var(base, var_name),
            _ => false,
        }
    }

    fn expr_is_mutating_method_call(&self, expr: &crate::hir::HirExpr, var_name: &str) -> bool {
        use crate::hir::HirExpr;
        if let HirExpr::MethodCall { object, method, .. } = expr {
            if let HirExpr::Var(name) = object.as_ref() {
                if name == var_name && self.is_mutating_method(method) {
                    return true;
                }
            }
        }
        false
    }

    fn is_mutating_method(&self, method: &str) -> bool {
        matches!(
            method,
            "append"
                | "extend"
                | "insert"
                | "remove"
                | "pop"
                | "clear"
                | "sort"
                | "reverse"
                | "update"
                | "add"
                | "discard"
                | "push"
                | "push_back"
                | "push_front"
                | "pop_back"
                | "pop_front"
        )
    }

    /// Check if variable is captured in a closure (lambda)
    fn var_captured_in_closure(&self, stmts: &[crate::hir::HirStmt], var_name: &str) -> bool {
        for stmt in stmts {
            if self.stmt_has_closure_capture(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_closure_capture(&self, stmt: &crate::hir::HirStmt, var_name: &str) -> bool {
        use crate::hir::HirStmt;
        match stmt {
            HirStmt::Assign { value, .. } => self.expr_has_closure_capture(value, var_name),
            HirStmt::Expr(expr) => self.expr_has_closure_capture(expr, var_name),
            HirStmt::Return(Some(expr)) => self.expr_has_closure_capture(expr, var_name),
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.expr_has_closure_capture(condition, var_name)
                    || self.var_captured_in_closure(then_body, var_name)
                    || else_body
                        .as_ref()
                        .map(|e| self.var_captured_in_closure(e, var_name))
                        .unwrap_or(false)
            }
            HirStmt::While { condition, body } => {
                self.expr_has_closure_capture(condition, var_name)
                    || self.var_captured_in_closure(body, var_name)
            }
            HirStmt::For { iter, body, .. } => {
                self.expr_has_closure_capture(iter, var_name)
                    || self.var_captured_in_closure(body, var_name)
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.var_captured_in_closure(body, var_name)
                    || handlers
                        .iter()
                        .any(|h| self.var_captured_in_closure(&h.body, var_name))
                    || orelse
                        .as_ref()
                        .map(|e| self.var_captured_in_closure(e, var_name))
                        .unwrap_or(false)
                    || finalbody
                        .as_ref()
                        .map(|e| self.var_captured_in_closure(e, var_name))
                        .unwrap_or(false)
            }
            HirStmt::With { context, body, .. } => {
                self.expr_has_closure_capture(context, var_name)
                    || self.var_captured_in_closure(body, var_name)
            }
            _ => false,
        }
    }

    fn expr_has_closure_capture(&self, expr: &crate::hir::HirExpr, var_name: &str) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Lambda { body, params } => {
                // Check if var_name is captured (used but not a param)
                !params.contains(&var_name.to_string()) && self.expr_references_var(body, var_name)
            }
            HirExpr::ListComp {
                element,
                iter,
                condition,
                ..
            }
            | HirExpr::SetComp {
                element,
                iter,
                condition,
                ..
            } => {
                self.expr_has_closure_capture(element, var_name)
                    || self.expr_has_closure_capture(iter, var_name)
                    || condition
                        .as_ref()
                        .map(|c| self.expr_has_closure_capture(c, var_name))
                        .unwrap_or(false)
            }
            HirExpr::Binary { left, right, .. } => {
                self.expr_has_closure_capture(left, var_name)
                    || self.expr_has_closure_capture(right, var_name)
            }
            HirExpr::Call { args, kwargs, .. } => {
                args.iter()
                    .any(|a| self.expr_has_closure_capture(a, var_name))
                    || kwargs
                        .iter()
                        .any(|(_, v)| self.expr_has_closure_capture(v, var_name))
            }
            HirExpr::MethodCall {
                object,
                args,
                kwargs,
                ..
            } => {
                self.expr_has_closure_capture(object, var_name)
                    || args
                        .iter()
                        .any(|a| self.expr_has_closure_capture(a, var_name))
                    || kwargs
                        .iter()
                        .any(|(_, v)| self.expr_has_closure_capture(v, var_name))
            }
            _ => false,
        }
    }

    fn expr_references_var(&self, expr: &crate::hir::HirExpr, var_name: &str) -> bool {
        use crate::hir::HirExpr;
        match expr {
            HirExpr::Var(name) => name == var_name,
            HirExpr::Binary { left, right, .. } => {
                self.expr_references_var(left, var_name)
                    || self.expr_references_var(right, var_name)
            }
            HirExpr::Unary { operand, .. } => self.expr_references_var(operand, var_name),
            HirExpr::Call { args, kwargs, .. } => {
                args.iter().any(|a| self.expr_references_var(a, var_name))
                    || kwargs
                        .iter()
                        .any(|(_, v)| self.expr_references_var(v, var_name))
            }
            HirExpr::MethodCall {
                object,
                args,
                kwargs,
                ..
            } => {
                self.expr_references_var(object, var_name)
                    || args.iter().any(|a| self.expr_references_var(a, var_name))
                    || kwargs
                        .iter()
                        .any(|(_, v)| self.expr_references_var(v, var_name))
            }
            HirExpr::Attribute { value, .. } => self.expr_references_var(value, var_name),
            HirExpr::Index { base, index } => {
                self.expr_references_var(base, var_name)
                    || self.expr_references_var(index, var_name)
            }
            HirExpr::List(elts) | HirExpr::Tuple(elts) | HirExpr::Set(elts) => {
                elts.iter().any(|e| self.expr_references_var(e, var_name))
            }
            HirExpr::Dict(pairs) => pairs.iter().any(|(k, v)| {
                self.expr_references_var(k, var_name) || self.expr_references_var(v, var_name)
            }),
            HirExpr::IfExpr { test, body, orelse } => {
                self.expr_references_var(test, var_name)
                    || self.expr_references_var(body, var_name)
                    || self.expr_references_var(orelse, var_name)
            }
            _ => false,
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
                object,
                args,
                kwargs,
                ..
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
