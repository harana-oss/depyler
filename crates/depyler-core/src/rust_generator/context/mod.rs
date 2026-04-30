//! Code generation context and core traits
//!
//! This module provides the central CodeGenContext struct that maintains
//! state during Rust code generation, along with core traits used across
//! the code generation pipeline.
//!
//! The implementation is split across focused sub-modules:
//! - `exception`   – `ErrorType` + exception-scope stack management
//! - `var_scope`   – lexical scope, variable declaration, clone analysis
//! - `borrow`      – borrow / reference flag helpers
//! - `type_query`  – `expr_has_type` / `get_expr_type` family
//! - `crate::analysis::usage_analysis` – `analyze_var_usage` and friends

pub mod borrow;
pub mod exception;
pub mod imports;
pub mod type_query;
pub mod var_scope;

// Re-export the ErrorType so downstream code keeps using the same path.
pub use exception::ErrorType;
// Re-export Import so callers can write `use context::Import` or `context::Import::HashMap`.
pub use imports::Import;

use crate::hir::{ExceptionScope, Type};
use crate::optimizations::string_optimization::StringOptimizer;
use crate::types::type_mapper::AnnotationAwareTypeMapper;
use anyhow::Result;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Parameter borrow information for interprocedural analysis
///
/// Contains information about how a parameter should be borrowed
/// based on interprocedural analysis results.
#[derive(Debug, Copy, Clone)]
pub struct ParamBorrowInfo {
    pub should_borrow: bool,
    pub needs_mut: bool,
    pub takes_ownership: bool,
}

/// Code generation context
///
/// Maintains all state needed during Rust code generation including:
/// - Type mapping and string optimization
/// - Import tracking (needs_hashmap, needs_cow, etc.)
/// - Variable scoping and mutability analysis
/// - Generator state management
pub struct CodeGenContext<'a> {
    pub type_mapper: &'a crate::types::type_mapper::TypeMapper,
    pub annotation_aware_mapper: AnnotationAwareTypeMapper,
    pub string_optimizer: StringOptimizer,
    pub union_enum_generator: super::union_enum_gen::UnionEnumGenerator,
    pub generated_enums: Vec<proc_macro2::TokenStream>,
    /// All Rust imports (crate or std items) that the generated code requires.
    ///
    /// Use `ctx.require(Import::HashMap)` to record a dependency and
    /// `ctx.requires(Import::HashMap)` to query it.  This replaces the former
    /// ~37 individual `needs_*: bool` fields.
    pub required_imports: BTreeSet<Import>,
    pub declared_vars: Vec<HashSet<String>>,
    pub current_function_can_fail: bool,
    pub current_function_name: Option<String>,
    pub current_return_type: Option<Type>,
    /// The inferred effective return type (may differ from annotation when inference fills in details)
    pub effective_return_type: Option<Type>,
    pub module_mapper: crate::mappings::module_mapper::ModuleMapper,
    pub imported_modules:
        std::collections::HashMap<String, crate::mappings::module_mapper::ModuleMapping>,
    pub imported_items: std::collections::HashMap<String, String>,
    pub mutable_vars: HashSet<String>,
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
        HashMap<String, Vec<crate::analysis::borrowing_context::BorrowingStrategy>>,
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
    /// Functions that should NOT use reference return optimization because callers
    /// pass `&mut` for the source parameter, creating cross-statement borrow conflicts.
    pub functions_suppress_ref_return: HashSet<String>,
    pub tuple_iter_vars: HashSet<String>,
    pub is_final_statement: bool,
    pub result_bool_functions: HashSet<String>,
    pub result_returning_functions: HashSet<String>,
    pub current_error_type: Option<ErrorType>,
    pub exception_scopes: Vec<ExceptionScope>,
    pub argparser_tracker: crate::rust_generator::argparse_transform::ArgParserTracker,
    pub generated_args_struct: Option<proc_macro2::TokenStream>,
    pub generated_commands_enum: Option<proc_macro2::TokenStream>,
    pub current_subcommand_fields: Option<std::collections::HashSet<String>>,

    pub validator_functions: std::collections::HashSet<String>,

    pub stdlib_mappings: crate::mappings::stdlib_mappings::StdlibMappings,

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

    /// Positions in a return tuple that should be references (from indexed field access
    /// on borrowed parameters). E.g., `player = state.players[idx]` → `player: &Player`.
    pub return_reference_positions: Vec<bool>,

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

    /// Variables assigned from a subscript expression that are later mutated through
    /// field access. These should be mutable references (`&mut T`) instead of clones.
    /// Pattern: `player = state.players[idx]` then `player.field += value`
    /// Generated as: `let player = state.players.get_mut(idx).unwrap()`
    pub mut_ref_index_vars: HashSet<String>,
}

impl<'a> CodeGenContext<'a> {
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
    pub fn process_union_type(&mut self, types: &[crate::hir::Type]) -> String {
        let (enum_name, enum_def) = self.union_enum_generator.generate_union_enum(types);
        if !enum_def.is_empty() {
            self.generated_enums.push(enum_def);
        }
        enum_name
    }

    /// Mark that complex numbers are used (triggers num-complex import)
    pub fn mark_complex_used(&mut self) {
        self.require(Import::Complex);
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
