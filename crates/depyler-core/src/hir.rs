use crate::annotations::TranspilationAnnotations;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// An interned, cheaply-cloneable identifier.
///
pub type Symbol = String;

/// Source span tracking the original Python source location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Span {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Create a span from a rustpython_ast TextRange using source code for line/col calculation
    pub fn from_text_range(range: rustpython_ast::text_size::TextRange, source: &str) -> Self {
        let (start_line, start_col) = offset_to_line_col(source, range.start().into());
        let (end_line, end_col) = offset_to_line_col(source, range.end().into());
        Self {
            start_line: start_line as u32,
            start_col: start_col as u32,
            end_line: end_line as u32,
            end_col: end_col as u32,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.start_line, self.start_col)
    }
}

/// Convert byte offset to (line, column), both 1-indexed
fn offset_to_line_col(source: &str, offset: u32) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    let offset = offset as usize;

    for (i, ch) in source.chars().enumerate() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Wrapper that attaches a source span to any HIR node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub inner: T,
    pub span: Option<Span>,
}

impl<T> Spanned<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, span: None }
    }

    pub fn with_span(inner: T, span: Span) -> Self {
        Self {
            inner,
            span: Some(span),
        }
    }

    /// Get the inner node, discarding span information.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// DerefMut intentionally omitted: mutation through the span wrapper silently
// discards span information. Callers that need to mutate the inner value should
// destructure the `Spanned` explicitly: `let Spanned { inner, span } = spanned;`

impl<T: Default> Default for Spanned<T> {
    fn default() -> Self {
        Self {
            inner: T::default(),
            span: None,
        }
    }
}

/// A statement with optional source location tracking
pub type SpannedStmt = Spanned<HirStmt>;

/// An expression with optional source location tracking  
pub type SpannedExpr = Spanned<HirExpr>;

/// High-level Intermediate Representation of a Python module
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirModule {
    pub functions: Vec<HirFunction>,
    pub imports: Vec<Import>,
    pub type_aliases: Vec<TypeAlias>,
    pub protocols: Vec<Protocol>,
    pub classes: Vec<HirClass>,
    pub constants: Vec<HirConstant>,
    /// Module-level executable statements (e.g., unpacking, print calls)
    /// These will be wrapped in a main() or module initialization function
    pub statements: Vec<HirStmt>,
}

/// Module-level constant declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirConstant {
    pub name: String,
    pub value: HirExpr,
    pub type_annotation: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<ImportItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportItem {
    Named(String),
    Aliased { name: String, alias: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeAlias {
    pub name: String,
    pub target_type: Type,
    pub is_newtype: bool, // true for NewType, false for simple alias
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Protocol {
    pub name: String,
    pub type_params: Vec<String>, // Generic type parameters like T, U
    pub methods: Vec<ProtocolMethod>,
    pub is_runtime_checkable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolMethod {
    pub name: String,
    pub params: SmallVec<[HirParam; 4]>,
    pub ret_type: Type,
    pub is_optional: bool,
    pub has_default: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirClass {
    pub name: String,
    pub base_classes: Vec<String>,
    pub methods: Vec<HirMethod>,
    pub fields: Vec<HirField>,
    pub is_dataclass: bool,
    pub is_enum: bool,
    pub is_intflag: bool,
    pub is_abc: bool,
    pub docstring: Option<String>,
    pub annotations: TranspilationAnnotations,
    /// Set to true if the class uses dynamic attribute access (getattr/setattr with variable names)
    pub needs_dynamic_field_access: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirMethod {
    pub name: String,
    pub params: SmallVec<[HirParam; 4]>,
    pub ret_type: Type,
    pub body: Vec<HirStmt>,
    pub is_static: bool,
    pub is_classmethod: bool,
    pub is_property: bool,
    pub is_async: bool,
    pub is_abstract: bool,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirField {
    pub name: String,
    pub field_type: Type,
    pub default_value: Option<HirExpr>,
    pub is_class_var: bool,
}

/// Function parameter with optional default value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirParam {
    pub name: Symbol,
    pub ty: Type,
    pub default: Option<HirExpr>,
}

impl HirParam {
    /// Create a required parameter (no default value)
    pub fn new(name: Symbol, ty: Type) -> Self {
        Self {
            name,
            ty,
            default: None,
        }
    }

    /// Create a parameter with a default value
    pub fn with_default(name: Symbol, ty: Type, default: HirExpr) -> Self {
        Self {
            name,
            ty,
            default: Some(default),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirFunction {
    pub name: Symbol,
    pub params: SmallVec<[HirParam; 4]>, // Most functions have < 4 params
    pub ret_type: Type,
    pub body: Vec<HirStmt>,
    pub properties: FunctionProperties,
    pub annotations: TranspilationAnnotations,
    pub docstring: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionProperties {
    pub is_pure: bool,
    pub max_stack_depth: Option<usize>,
    pub always_terminates: bool,
    pub panic_free: bool,
    pub can_fail: bool,
    pub error_types: Vec<String>,
    pub is_async: bool,
    pub is_generator: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssignTarget {
    /// Simple variable assignment: x = value
    Symbol(Symbol),
    /// Subscript assignment: x[key] = value
    Index {
        base: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    /// Slice assignment: x[:] = value or x[start:stop] = value
    Slice {
        base: Box<HirExpr>,
        start: Option<Box<HirExpr>>,
        stop: Option<Box<HirExpr>>,
        step: Option<Box<HirExpr>>,
    },
    /// Attribute assignment: x.attr = value (for future use)
    Attribute { value: Box<HirExpr>, attr: Symbol },
    /// Tuple unpacking: (a, b) = value or a, b = value
    Tuple(Vec<AssignTarget>),
    /// Starred expression in unpacking: *rest (captures remaining elements)
    Starred(Symbol),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirStmt {
    Assign {
        target: AssignTarget,
        value: HirExpr,
        type_annotation: Option<Type>,
    },
    Return(Option<HirExpr>),
    If {
        condition: HirExpr,
        then_body: Vec<HirStmt>,
        else_body: Option<Vec<HirStmt>>,
    },
    While {
        condition: HirExpr,
        body: Vec<HirStmt>,
    },
    For {
        target: AssignTarget,
        iter: HirExpr,
        body: Vec<HirStmt>,
    },
    Expr(HirExpr),
    // Basic exception support
    Raise {
        exception: Option<HirExpr>,
        cause: Option<HirExpr>,
    },
    Break {
        label: Option<Symbol>,
    },
    Continue {
        label: Option<Symbol>,
    },
    With {
        context: HirExpr,
        target: Option<Symbol>,
        body: Vec<HirStmt>,
    },
    Try {
        body: Vec<HirStmt>,
        handlers: Vec<ExceptHandler>,
        orelse: Option<Vec<HirStmt>>,
        finalbody: Option<Vec<HirStmt>>,
    },
    Assert {
        test: HirExpr,
        msg: Option<HirExpr>,
    },
    Pass,
    /// Nested function definition (inner functions)
    FunctionDef {
        name: Symbol,
        params: Box<SmallVec<[HirParam; 4]>>,
        ret_type: Type,
        body: Vec<HirStmt>,
        docstring: Option<String>,
    },
    /// Global variable declaration - marks variables as global scope
    Global {
        names: Vec<Symbol>,
    },
    /// Nonlocal variable declaration - marks variables as enclosing scope
    Nonlocal {
        names: Vec<Symbol>,
    },
    /// Async for loop - iterates over async iterators/streams
    AsyncFor {
        target: AssignTarget,
        iter: HirExpr,
        body: Vec<HirStmt>,
    },
    /// Async with statement - async context manager
    AsyncWith {
        context: HirExpr,
        target: Option<Symbol>,
        body: Vec<HirStmt>,
    },
    /// Delete statement - removes variables or collection items
    Delete {
        targets: Vec<AssignTarget>,
    },
    /// Import statement - local import inside function
    Import {
        modules: Vec<(Symbol, Option<Symbol>)>, // (module_name, alias)
    },
    /// Import-from statement - from x import y
    ImportFrom {
        module: Option<Symbol>,
        names: Vec<(Symbol, Option<Symbol>)>, // (name, alias)
    },
    /// Async nested function definition
    AsyncFunctionDef {
        name: Symbol,
        params: Box<SmallVec<[HirParam; 4]>>,
        ret_type: Type,
        body: Vec<HirStmt>,
        docstring: Option<String>,
    },
    /// Match statement (Python 3.10+)
    Match {
        subject: HirExpr,
        cases: Vec<MatchCase>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptHandler {
    pub exception_type: Option<String>,
    pub name: Option<Symbol>,
    pub body: Vec<HirStmt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchCase {
    pub pattern: HirPattern,
    pub guard: Option<HirExpr>,
    pub body: Vec<HirStmt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirPattern {
    /// Match a literal value: case 1:, case "hello":
    Value(HirExpr),
    /// Match singleton: case None:, case True:, case False:
    Singleton(Literal),
    /// Match a sequence: case [a, b, c]:
    Sequence(Vec<HirPattern>),
    /// Match a mapping: case {"key": value}:
    Mapping {
        keys: Vec<HirExpr>,
        patterns: Vec<HirPattern>,
        rest: Option<Symbol>,
    },
    /// Match a class: case Point(x, y):
    Class {
        cls: String,
        patterns: Vec<HirPattern>,
        kwd_attrs: Vec<Symbol>,
        kwd_patterns: Vec<HirPattern>,
    },
    /// Star pattern in sequence: case [first, *rest]:
    Star(Option<Symbol>),
    /// As pattern: case _ as x: or case pattern as name:
    As {
        pattern: Option<Box<HirPattern>>,
        name: Option<Symbol>,
    },
    /// Or pattern: case 1 | 2 | 3:
    Or(Vec<HirPattern>),
    /// Wildcard pattern: case _:
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirExpr {
    Literal(Literal),
    Var(Symbol),
    Binary {
        op: BinOp,
        left: Box<HirExpr>,
        right: Box<HirExpr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<HirExpr>,
    },
    Call {
        func: Symbol,
        args: Vec<HirExpr>,
        kwargs: Vec<(Symbol, HirExpr)>,
        /// Explicit type parameters for generic calls like `func[int]()`
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        type_params: Vec<Type>,
    },
    MethodCall {
        object: Box<HirExpr>,
        method: Symbol,
        args: Vec<HirExpr>,
        /// Format: Vec<(arg_name, value_expr)>
        /// Empty for calls without kwargs
        kwargs: Vec<(Symbol, HirExpr)>,
        /// Explicit type parameters for generic method calls like `obj.method[int]()`
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        type_params: Vec<Type>,
    },
    Index {
        base: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    Slice {
        base: Box<HirExpr>,
        start: Option<Box<HirExpr>>,
        stop: Option<Box<HirExpr>>,
        step: Option<Box<HirExpr>>,
    },
    Attribute {
        value: Box<HirExpr>,
        attr: Symbol,
    },
    List(Vec<HirExpr>),
    Dict(Vec<(HirExpr, HirExpr)>),
    Tuple(Vec<HirExpr>),
    Set(Vec<HirExpr>),
    FrozenSet(Vec<HirExpr>),
    // Ownership hints from analysis
    Borrow {
        expr: Box<HirExpr>,
        mutable: bool,
    },
    // List comprehension
    ListComp {
        element: Box<HirExpr>,
        target: Symbol,
        iter: Box<HirExpr>,
        condition: Option<Box<HirExpr>>,
    },
    // Flattened list comprehension with multiple generators
    // [item for row in matrix for item in row]
    FlattenedListComp {
        element: Box<HirExpr>,
        generators: Vec<HirComprehension>,
    },
    // Set comprehension
    SetComp {
        element: Box<HirExpr>,
        target: Symbol,
        iter: Box<HirExpr>,
        condition: Option<Box<HirExpr>>,
    },
    // Dict comprehension
    DictComp {
        key: Box<HirExpr>,
        value: Box<HirExpr>,
        target: Symbol,
        iter: Box<HirExpr>,
        condition: Option<Box<HirExpr>>,
    },
    // Lambda function
    Lambda {
        params: Vec<Symbol>,
        body: Box<HirExpr>,
    },
    // Await expression
    Await {
        value: Box<HirExpr>,
    },
    // F-string (format string)
    FString {
        parts: Vec<FStringPart>,
    },
    // Yield expression for generators
    Yield {
        value: Option<Box<HirExpr>>,
    },
    /// Marker for annotated declarations without an initializer (Python: "x: T")
    ///
    /// Used to represent a variable that has a type annotation but no value. This
    /// allows the code generator to emit a declaration with a type but no
    /// initializer (e.g., `let mut x: T;`).
    Uninitialized,
    // Ternary/conditional expression (Python: x if cond else y)
    IfExpr {
        test: Box<HirExpr>,
        body: Box<HirExpr>,
        orelse: Box<HirExpr>,
    },
    // sorted() with key parameter (Python: sorted(iterable, key=lambda x: ..., reverse=True))
    SortByKey {
        iterable: Box<HirExpr>,
        key_params: Vec<Symbol>,
        key_body: Box<HirExpr>,
        reverse: bool,
    },
    // Generator expression (Python: (x * 2 for x in range(5)))
    GeneratorExp {
        element: Box<HirExpr>,
        generators: Vec<HirComprehension>,
    },
    // Named expression / walrus operator (Python: (x := expr))
    NamedExpr {
        target: Symbol,
        value: Box<HirExpr>,
    },
}

/// Comprehension generator (used in list/set/dict/generator comprehensions)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirComprehension {
    pub target: Symbol,
    pub iter: Box<HirExpr>,
    pub conditions: Vec<HirExpr>,
}

/// Part of an f-string - either literal text or an expression to interpolate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FStringPart {
    /// Literal text in the f-string
    Literal(String),
    /// Expression to be formatted and inserted
    Expr(Box<HirExpr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Bool(bool),
    None,
    Ellipsis,
    Complex(f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    MatMul,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    LShift,
    RShift,
    In,
    NotIn,
    Is,
    IsNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
    Pos,
    BitNot,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstGeneric {
    /// Literal constant value (e.g., 5 in [T; 5])
    Literal(usize),
    /// Const generic parameter (e.g., N in [T; N])
    Parameter(String),
    /// Expression involving const generics (e.g., N + 1)
    Expression(String),
}

///
/// Tracks whether code is executing inside a try/except block to determine
/// appropriate error handling strategy:
/// - Unhandled: Exceptions propagate to caller (use ? operator or Result return)
/// - TryCaught: Exceptions are caught by handlers (use .unwrap_or() or control flow)
/// - Handler: Inside except/finally block (exceptions may propagate)
///
/// # Complexity
/// N/A (enum definition)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExceptionScope {
    /// Code outside any try/except block - exceptions propagate to caller
    /// Functions with unhandled exceptions should return Result<T, E>
    Unhandled,

    /// Code inside try block - exceptions are caught by handlers
    /// Contains list of exception types that are handled
    /// e.g., `try: ... except ValueError: ...` → TryCaught { handled_types: ["ValueError"] }
    TryCaught {
        /// Exception types caught by handlers (e.g., ["ValueError", "ZeroDivisionError"])
        /// Empty list means bare except clause (catches all)
        handled_types: Vec<String>,
    },

    /// Code inside except or finally block
    /// Exceptions in handlers may propagate to outer scope
    Handler,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Type {
    Unknown,
    Int,
    Float,
    String,
    Bool,
    None,
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Set(Box<Type>),
    Optional(Box<Type>),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Custom(String),
    /// Type variable for generics (e.g., T, U)
    TypeVar(String),
    /// Generic type with parameters (e.g., List<T>, Dict<K, V>)
    Generic {
        base: String,
        params: Vec<Type>,
    },
    /// Union type (e.g., Union[int, str])
    Union(Vec<Type>),
    /// Fixed-size array with const generic size (e.g., [T; N])
    Array {
        element_type: Box<Type>,
        size: ConstGeneric,
    },
    /// Final type annotation from typing.Final[T] - marks constants
    Final(Box<Type>),
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    pub fn is_container(&self) -> bool {
        matches!(
            self,
            Type::List(_) | Type::Dict(_, _) | Type::Tuple(_) | Type::Set(_) | Type::Array { .. }
        )
    }

    /// Returns `true` if this type implements `Copy` (i.e. does not need an
    /// explicit `.clone()` when moved).  Primitive scalars and fixed-size
    /// arrays / tuples whose elements are all `Copy` are considered `Copy`.
    /// `Custom` types are conservatively considered non-`Copy` here; callers
    /// that have access to struct/enum name sets should use the context-aware
    /// `CodeGenContext::type_needs_clone` for those.
    pub fn is_copy(&self) -> bool {
        match self {
            Type::Int | Type::Float | Type::Bool | Type::None => true,
            Type::Optional(inner) | Type::Final(inner) => inner.is_copy(),
            Type::Tuple(elements) => elements.iter().all(|t| t.is_copy()),
            Type::Array { element_type, .. } => element_type.is_copy(),
            _ => false,
        }
    }

    /// Returns `true` if this HIR type is a reference / borrowed type.
    /// The HIR `Type` enum currently has no explicit reference variant, so
    /// this always returns `false`.  It exists as a stable, grep-able
    /// predicate so that future reference variants can be handled in one place.
    pub fn is_ref(&self) -> bool {
        false
    }
}
