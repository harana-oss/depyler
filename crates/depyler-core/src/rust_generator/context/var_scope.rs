//! Lexical scope management, variable declaration, mutability, and clone analysis
//! for `CodeGenContext`.

use super::CodeGenContext;
use crate::hir::Type;
use std::collections::HashSet;

impl<'a> CodeGenContext<'a> {
    // ========================================================================
    // Lexical scope
    // ========================================================================

    /// Enter a new lexical scope
    pub fn enter_scope(&mut self) {
        self.declared_vars.push(HashSet::new());
    }

    /// Exit the current lexical scope
    pub fn exit_scope(&mut self) {
        self.declared_vars.pop();
    }

    /// Check if a variable is declared in any scope
    pub fn is_declared(&self, var_name: &str) -> bool {
        self.declared_vars
            .iter()
            .any(|scope| scope.contains(var_name))
    }

    /// Declare a variable in the current scope
    pub fn declare_var(&mut self, var_name: &str) {
        if let Some(current_scope) = self.declared_vars.last_mut() {
            current_scope.insert(var_name.to_string());
        }
    }

    // ========================================================================
    // Variable usage counting (feeds clone analysis)
    // ========================================================================

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

    // ========================================================================
    // Clone analysis
    // ========================================================================

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
    pub fn type_needs_clone(&self, ty: &Type) -> bool {
        match ty {
            // Copy types (delegates to Type::is_copy for the primitive cases)
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
}
