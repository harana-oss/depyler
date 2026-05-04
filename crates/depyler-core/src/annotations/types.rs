use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnnotationError {
    #[error("Invalid annotation syntax: {0}")]
    InvalidSyntax(String),
    #[error("Unknown annotation key: {0}")]
    UnknownKey(String),
    #[error("Invalid value for key {key}: {value}")]
    InvalidValue { key: String, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // Configuration struct requires multiple boolean flags
pub struct TranspilationAnnotations {
    pub type_strategy: TypeStrategy,
    pub ownership_model: OwnershipModel,
    pub safety_level: SafetyLevel,
    pub performance_hints: Vec<PerformanceHint>,
    pub fallback_strategy: FallbackStrategy,
    pub bounds_checking: BoundsChecking,
    pub thread_safety: ThreadSafety,
    pub interior_mutability: InteriorMutability,
    pub string_strategy: StringStrategy,
    pub hash_strategy: HashStrategy,
    pub panic_behavior: PanicBehavior,
    pub error_strategy: ErrorStrategy,
    pub global_strategy: GlobalStrategy,
    pub termination: Termination,
    pub invariants: Vec<String>,
    pub verify_bounds: bool,
    pub service_type: Option<ServiceType>,
    pub migration_strategy: Option<MigrationStrategy>,
    pub compatibility_layer: Option<CompatibilityLayer>,
    pub pattern: Option<String>,
    pub custom_attributes: Vec<String>,
    pub additional_derives: Vec<String>,
}

impl Default for TranspilationAnnotations {
    fn default() -> Self {
        Self {
            type_strategy: TypeStrategy::Conservative,
            ownership_model: OwnershipModel::Owned,
            safety_level: SafetyLevel::Safe,
            performance_hints: Vec::new(),
            fallback_strategy: FallbackStrategy::Error,
            bounds_checking: BoundsChecking::Explicit,
            thread_safety: ThreadSafety::NotRequired,
            interior_mutability: InteriorMutability::None,
            string_strategy: StringStrategy::Conservative,
            hash_strategy: HashStrategy::Standard,
            panic_behavior: PanicBehavior::Propagate,
            error_strategy: ErrorStrategy::Panic,
            global_strategy: GlobalStrategy::None,
            termination: Termination::Unknown,
            invariants: Vec::new(),
            verify_bounds: false,
            service_type: None,
            migration_strategy: None,
            compatibility_layer: None,
            pattern: None,
            custom_attributes: Vec::new(),
            additional_derives: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeStrategy {
    Conservative,
    Aggressive,
    ZeroCopy,
    AlwaysOwned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnershipModel {
    Owned,
    Borrowed,
    Shared,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyLevel {
    Safe,
    UnsafeAllowed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceHint {
    Vectorize,
    UnrollLoops(u32),
    OptimizeForLatency,
    OptimizeForThroughput,
    PerformanceCritical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackStrategy {
    Mcp,
    Manual,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundsChecking {
    Explicit,
    Implicit,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadSafety {
    Required,
    NotRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteriorMutability {
    None,
    ArcMutex,
    RefCell,
    Cell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringStrategy {
    Conservative,
    AlwaysOwned,
    ZeroCopy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashStrategy {
    Standard,
    Fnv,
    AHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanicBehavior {
    Propagate,
    ReturnError,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorStrategy {
    Panic,
    ResultType,
    OptionType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlobalStrategy {
    None,
    LazyStatic,
    OnceCell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Termination {
    Unknown,
    Proven,
    BoundedLoop(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    WebApi,
    Cli,
    Library,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStrategy {
    Incremental,
    BigBang,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityLayer {
    PyO3,
    CTypes,
    None,
}
