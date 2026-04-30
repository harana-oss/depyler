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
    pub optimization_level: OptimizationLevel,
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
    // Lambda-specific annotations
    pub lambda_annotations: Option<LambdaAnnotations>,
    pub custom_attributes: Vec<String>,
    /// Additional derive macros to add to generated structs (e.g., "Serialize", "Hash")
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
            optimization_level: OptimizationLevel::Standard,
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
            lambda_annotations: None,
            custom_attributes: Vec::new(),
            additional_derives: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // Lambda configuration requires many boolean flags
pub struct LambdaAnnotations {
    pub runtime: LambdaRuntime,
    pub event_type: Option<LambdaEventType>,
    pub cold_start_optimize: bool,
    pub memory_size: u16,
    pub architecture: Architecture,
    pub pre_warm_paths: Vec<String>,
    pub custom_serialization: bool,
    pub batch_failure_reporting: bool,
    pub timeout: Option<u16>,
    pub tracing_enabled: bool,
    pub environment_variables: Vec<(String, String)>,
}

impl Default for LambdaAnnotations {
    fn default() -> Self {
        Self {
            runtime: LambdaRuntime::ProvidedAl2,
            event_type: None,
            cold_start_optimize: true,
            memory_size: 128,
            architecture: Architecture::Arm64,
            pre_warm_paths: vec![],
            custom_serialization: false,
            batch_failure_reporting: false,
            timeout: None,
            tracing_enabled: false,
            environment_variables: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LambdaRuntime {
    ProvidedAl2,
    ProvidedAl2023,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LambdaEventType {
    Auto,
    S3Event,
    ApiGatewayProxyRequest,
    ApiGatewayV2HttpRequest,
    SqsEvent,
    SnsEvent,
    DynamodbEvent,
    EventBridgeEvent(Option<String>),
    CloudwatchEvent,
    KinesisEvent,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    Arm64,
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
pub enum OptimizationLevel {
    Standard,
    Aggressive,
    Conservative,
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
