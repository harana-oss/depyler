//! Exception scope stack and error-type tracking for `CodeGenContext`.

use super::CodeGenContext;
use crate::hir::ExceptionScope;

/// Error type classification for Result<T, E> return types
///
/// Determines whether a concrete type or a boxed trait object is used,
/// which in turn controls whether `raise` statements need a `Box::new()` wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType {
    /// Concrete error type (e.g., ValueError, ZeroDivisionError)
    /// No wrapping needed: `return Err(ValueError::new(...))`
    Concrete(String),
    /// Box<dyn Error> - mixed or generic error types
    /// Needs wrapping: `return Err(Box::new(ValueError::new(...)))`
    DynBox,
}

impl<'a> CodeGenContext<'a> {
    // ========================================================================
    // Exception scope management
    // ========================================================================

    /// Get the current exception scope
    ///
    /// Returns the most recent scope from the stack, or Unhandled if stack is empty.
    pub fn current_exception_scope(&self) -> &ExceptionScope {
        self.exception_scopes
            .last()
            .unwrap_or(&ExceptionScope::Unhandled)
    }

    /// Check if currently inside a try block
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
    pub fn is_exception_handled(&self, exception_type: &str) -> bool {
        if let ExceptionScope::TryCaught { handled_types } = self.current_exception_scope() {
            // Empty list = bare except (catches all)
            handled_types.is_empty() || handled_types.contains(&exception_type.to_string())
        } else {
            false
        }
    }

    /// Enter a try block scope with specified exception handlers
    pub fn enter_try_scope(&mut self, handled_types: Vec<String>) {
        self.exception_scopes
            .push(ExceptionScope::TryCaught { handled_types });
    }

    /// Enter an exception handler scope
    pub fn enter_handler_scope(&mut self) {
        self.exception_scopes.push(ExceptionScope::Handler);
    }

    /// Exit the current exception scope
    pub fn exit_exception_scope(&mut self) {
        self.exception_scopes.pop();
    }
}
