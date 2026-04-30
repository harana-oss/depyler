//! Borrow / reference tracking flag helpers for `CodeGenContext`.

use super::CodeGenContext;

impl<'a> CodeGenContext<'a> {
    // ========================================================================
    // Borrow generation flags
    // ========================================================================

    /// Set the generate_borrow flag
    pub fn set_generate_borrow(&mut self, value: bool) {
        self.generate_borrow = value;
    }

    /// Set the generate_mut_borrow flag
    pub fn set_generate_mut_borrow(&mut self, value: bool) {
        self.generate_mut_borrow = value;
    }

    // ========================================================================
    // Borrow eligibility queries
    // ========================================================================

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

    // ========================================================================
    // Move-clone analysis at codegen time
    // ========================================================================

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
}
