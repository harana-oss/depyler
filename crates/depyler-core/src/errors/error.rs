use std::fmt;
use thiserror::Error;

/// Source location information for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }

    /// Create a SourceLocation from a HIR Span
    pub fn from_span(span: &crate::hir::Span, file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: span.start_line as usize,
            column: span.start_col as usize,
        }
    }
}

/// Types of transpilation errors
#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("Python parse error")]
    ParseError,

    #[error("Unsupported Python feature")]
    UnsupportedFeature(String),

    #[error("Type inference error")]
    TypeInferenceError(String),

    #[error("Invalid type annotation")]
    InvalidTypeAnnotation(String),

    #[error("Type mismatch")]
    TypeMismatch {
        expected: String,
        found: String,
        context: String,
    },

    #[error("Code generation error")]
    CodeGenerationError(String),

    #[error("Verification failed")]
    VerificationError(String),

    #[error("Internal error")]
    InternalError(String),
}

/// Context-aware transpilation error
#[derive(Debug, Error)]
pub struct TranspileError {
    pub kind: ErrorKind,
    pub location: Option<SourceLocation>,
    pub context: Vec<String>,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl TranspileError {
    /// Create a new error with the given kind
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            location: None,
            context: Vec::new(),
            source: None,
        }
    }

    /// Add location information to the error
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Add location information from a HIR Span
    pub fn with_span(mut self, span: &crate::hir::Span, file: impl Into<String>) -> Self {
        self.location = Some(SourceLocation::from_span(span, file));
        self
    }

    /// Add location information from an optional span
    pub fn with_optional_span(self, span: Option<&crate::hir::Span>, file: impl Into<String>) -> Self {
        if let Some(s) = span {
            self.with_span(s, file)
        } else {
            self
        }
    }

    /// Add context to the error
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }

    /// Add source error
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl fmt::Display for TranspileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write main error message
        write!(f, "{}", self.kind)?;

        // Add location if available
        self.format_location(f)?;

        // Add context if available
        self.format_context_list(f)?;

        Ok(())
    }
}

impl TranspileError {
    /// Format location information if available
    #[inline]
    fn format_location(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(loc) = &self.location {
            write!(f, " at {loc}")?;
        }
        Ok(())
    }

    /// Format context list if not empty
    #[inline]
    fn format_context_list(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.context.is_empty() {
            write!(f, "\n\nContext:")?;
            for (i, ctx) in self.context.iter().enumerate() {
                write!(f, "\n  {}. {}", i + 1, ctx)?;
            }
        }
        Ok(())
    }
}

/// Result type alias for transpilation operations
pub type TranspileResult<T> = Result<T, TranspileError>;

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    #[allow(clippy::result_large_err)]
    fn with_context(self, ctx: impl Into<String>) -> TranspileResult<T>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<TranspileError>,
{
    fn with_context(self, ctx: impl Into<String>) -> TranspileResult<T> {
        self.map_err(|e| e.into().with_context(ctx))
    }
}

/// Convert anyhow errors to TranspileError
impl From<anyhow::Error> for TranspileError {
    fn from(err: anyhow::Error) -> Self {
        TranspileError::new(ErrorKind::InternalError(err.to_string()))
    }
}

/// Helper macro for creating errors with context
#[macro_export]
macro_rules! transpile_error {
    ($kind:expr) => {
        $crate::error::TranspileError::new($kind)
    };

    ($kind:expr, $($ctx:expr),+) => {{
        let mut err = $crate::error::TranspileError::new($kind);
        $(
            err = err.with_context($ctx);
        )+
        err
    }};
}

/// Helper macro for bailing with a transpile error
#[macro_export]
macro_rules! transpile_bail {
    ($kind:expr) => {
        return Err($crate::transpile_error!($kind))
    };

    ($kind:expr, $($ctx:expr),+) => {
        return Err($crate::transpile_error!($kind, $($ctx),+))
    };
}
