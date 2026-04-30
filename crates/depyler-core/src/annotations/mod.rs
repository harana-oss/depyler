#![allow(clippy::missing_errors_doc)] // Parse methods have obvious error conditions

pub mod applicator;
pub mod parser;
pub mod types;

// Re-export all public items so existing `use crate::annotations::*` paths keep working.
pub use applicator::{AnnotationExtractor, AnnotationValidator};
pub use parser::AnnotationParser;
pub use types::{
    AnnotationError, Architecture, BoundsChecking, CompatibilityLayer, ErrorStrategy,
    FallbackStrategy, GlobalStrategy, HashStrategy, InteriorMutability, LambdaAnnotations,
    LambdaEventType, LambdaRuntime, MigrationStrategy, OptimizationLevel, OwnershipModel,
    PanicBehavior, PerformanceHint, SafetyLevel, ServiceType, StringStrategy, Termination,
    ThreadSafety, TranspilationAnnotations, TypeStrategy,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_annotations() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: type_strategy = "conservative"
# @depyler: ownership = "borrowed"
def test_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.type_strategy, TypeStrategy::Conservative);
        assert_eq!(annotations.ownership_model, OwnershipModel::Borrowed);
    }

    #[test]
    fn test_parse_performance_annotations() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: performance_critical = "true"
# @depyler: vectorize = "true"
# @depyler: unroll_loops = "4"
def fast_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert!(
            annotations
                .performance_hints
                .contains(&PerformanceHint::PerformanceCritical)
        );
        assert!(annotations.performance_hints.contains(&PerformanceHint::Vectorize));
        assert!(annotations.performance_hints.contains(&PerformanceHint::UnrollLoops(4)));
    }

    #[test]
    fn test_parse_safety_annotations() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: safety_level = "unsafe_allowed"
# @depyler: bounds_checking = "disabled"
def unsafe_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.safety_level, SafetyLevel::UnsafeAllowed);
        assert_eq!(annotations.bounds_checking, BoundsChecking::Disabled);
    }

    #[test]
    fn test_parse_fallback_strategy() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: fallback = "mcp"
def complex_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.fallback_strategy, FallbackStrategy::Mcp);
    }

    #[test]
    fn test_parse_thread_safety() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: thread_safety = "required"
# @depyler: interior_mutability = "arc_mutex"
def thread_safe_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.thread_safety, ThreadSafety::Required);
        assert_eq!(annotations.interior_mutability, InteriorMutability::ArcMutex);
    }

    #[test]
    fn test_invalid_annotation_key() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: invalid_key = "value"
def test_function():
    pass
        "#;

        let result = parser.parse_annotations(source);
        assert!(matches!(result, Err(AnnotationError::UnknownKey(_))));
    }

    #[test]
    fn test_invalid_annotation_value() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: type_strategy = "invalid_value"
def test_function():
    pass
        "#;

        let result = parser.parse_annotations(source);
        assert!(matches!(result, Err(AnnotationError::InvalidValue { .. })));
    }

    #[test]
    fn test_default_annotations() {
        let annotations = TranspilationAnnotations::default();
        assert_eq!(annotations.type_strategy, TypeStrategy::Conservative);
        assert_eq!(annotations.ownership_model, OwnershipModel::Owned);
        assert_eq!(annotations.safety_level, SafetyLevel::Safe);
        assert_eq!(annotations.fallback_strategy, FallbackStrategy::Error);
    }

    #[test]
    fn test_optimization_hints() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: optimization_hint = "vectorize"
# @depyler: optimization_level = "aggressive"
def optimized_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert!(annotations.performance_hints.contains(&PerformanceHint::Vectorize));
        assert_eq!(annotations.optimization_level, OptimizationLevel::Aggressive);
    }

    #[test]
    fn test_string_and_hash_strategies() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: string_strategy = "zero_copy"
# @depyler: hash_strategy = "fnv"
def string_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.string_strategy, StringStrategy::ZeroCopy);
        assert_eq!(annotations.hash_strategy, HashStrategy::Fnv);
    }

    #[test]
    fn test_error_handling_annotations() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: panic_behavior = "return_error"
# @depyler: error_strategy = "result_type"
def error_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.panic_behavior, PanicBehavior::ReturnError);
        assert_eq!(annotations.error_strategy, ErrorStrategy::ResultType);
    }

    #[test]
    fn test_service_and_migration_annotations() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: service_type = "web_api"
# @depyler: migration_strategy = "incremental"
# @depyler: compatibility_layer = "pyo3"
def service_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.service_type, Some(ServiceType::WebApi));
        assert_eq!(annotations.migration_strategy, Some(MigrationStrategy::Incremental));
        assert_eq!(annotations.compatibility_layer, Some(CompatibilityLayer::PyO3));
    }

    #[test]
    fn test_verification_annotations() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: termination = "proven"
# @depyler: invariant = "left <= right"
# @depyler: verify_bounds = "true"
def verified_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.termination, Termination::Proven);
        assert!(annotations.invariants.contains(&"left <= right".to_string()));
        assert!(annotations.verify_bounds);
    }

    #[test]
    fn test_global_strategy() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: global_strategy = "lazy_static"
def global_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.global_strategy, GlobalStrategy::LazyStatic);
    }

    #[test]
    fn test_lambda_annotations_basic() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: lambda_runtime = "provided.al2"
# @depyler: event_type = "APIGatewayProxyRequest"
# @depyler: cold_start_optimize = "true"
def handler(event, context):
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert!(annotations.lambda_annotations.is_some());

        let lambda_annotations = annotations.lambda_annotations.unwrap();
        assert_eq!(lambda_annotations.runtime, LambdaRuntime::ProvidedAl2);
        assert_eq!(
            lambda_annotations.event_type,
            Some(LambdaEventType::ApiGatewayProxyRequest)
        );
        assert!(lambda_annotations.cold_start_optimize);
    }

    #[test]
    fn test_lambda_annotations_memory_and_architecture() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: memory_size = "256"
# @depyler: architecture = "arm64"
# @depyler: timeout = "30"
def handler(event, context):
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        let lambda_annotations = annotations.lambda_annotations.unwrap();
        assert_eq!(lambda_annotations.memory_size, 256);
        assert_eq!(lambda_annotations.architecture, Architecture::Arm64);
        assert_eq!(lambda_annotations.timeout, Some(30));
    }

    #[test]
    fn test_lambda_eventbridge_with_custom_type() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: event_type = "EventBridgeEvent<OrderEvent>"
# @depyler: custom_serialization = "true"
def handler(event, context):
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        let lambda_annotations = annotations.lambda_annotations.unwrap();
        assert_eq!(
            lambda_annotations.event_type,
            Some(LambdaEventType::EventBridgeEvent(Some("OrderEvent".to_string())))
        );
        assert!(lambda_annotations.custom_serialization);
    }

    #[test]
    fn test_lambda_sqs_batch_processing() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: event_type = "SqsEvent"
# @depyler: batch_failure_reporting = "true"
# @depyler: tracing = "Active"
def handler(event, context):
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        let lambda_annotations = annotations.lambda_annotations.unwrap();
        assert_eq!(lambda_annotations.event_type, Some(LambdaEventType::SqsEvent));
        assert!(lambda_annotations.batch_failure_reporting);
        assert!(lambda_annotations.tracing_enabled);
    }

    #[test]
    fn test_lambda_auto_event_type() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: event_type = "auto"
# @depyler: cold_start_optimize = "true"
def handler(event, context):
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        let lambda_annotations = annotations.lambda_annotations.unwrap();
        assert_eq!(lambda_annotations.event_type, Some(LambdaEventType::Auto));
        assert!(lambda_annotations.cold_start_optimize);
    }

    #[test]
    fn test_lambda_custom_runtime() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: lambda_runtime = "rust-runtime-1.0"
def handler(event, context):
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        let lambda_annotations = annotations.lambda_annotations.unwrap();
        assert_eq!(
            lambda_annotations.runtime,
            LambdaRuntime::Custom("rust-runtime-1.0".to_string())
        );
    }

    #[test]
    fn test_custom_custom_attribute_single() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: custom_attribute = "inline"
def my_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.custom_attributes.len(), 1);
        assert_eq!(annotations.custom_attributes[0], "inline");
    }

    #[test]
    fn test_custom_custom_attribute_multiple() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: custom_attribute = "inline"
# @depyler: custom_attribute = "must_use"
# @depyler: custom_attribute = "cold"
def my_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.custom_attributes.len(), 3);
        assert_eq!(annotations.custom_attributes[0], "inline");
        assert_eq!(annotations.custom_attributes[1], "must_use");
        assert_eq!(annotations.custom_attributes[2], "cold");
    }

    #[test]
    fn test_custom_custom_attribute_with_other_annotations() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: optimization_level = "aggressive"
# @depyler: custom_attribute = "inline(always)"
# @depyler: performance_critical = "true"
def hot_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.optimization_level, OptimizationLevel::Aggressive);
        assert_eq!(annotations.custom_attributes.len(), 1);
        assert_eq!(annotations.custom_attributes[0], "inline(always)");
        assert!(
            annotations
                .performance_hints
                .contains(&PerformanceHint::PerformanceCritical)
        );
    }

    #[test]
    fn test_custom_custom_attribute_empty() {
        let parser = AnnotationParser::new();
        let source = r#"
def my_function():
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.custom_attributes.len(), 0);
    }

    #[test]
    fn test_additional_derives_single() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: additional_derives = "Serialize"
class MyClass:
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.additional_derives.len(), 1);
        assert_eq!(annotations.additional_derives[0], "Serialize");
    }

    #[test]
    fn test_additional_derives_comma_separated() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: additional_derives = "Serialize, Deserialize, Hash"
class MyClass:
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.additional_derives.len(), 3);
        assert_eq!(annotations.additional_derives[0], "Serialize");
        assert_eq!(annotations.additional_derives[1], "Deserialize");
        assert_eq!(annotations.additional_derives[2], "Hash");
    }

    #[test]
    fn test_additional_derives_multiple_lines() {
        let parser = AnnotationParser::new();
        let source = r#"
# @depyler: additional_derives = "Serialize"
# @depyler: additional_derives = "Deserialize"
class MyClass:
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.additional_derives.len(), 2);
        assert_eq!(annotations.additional_derives[0], "Serialize");
        assert_eq!(annotations.additional_derives[1], "Deserialize");
    }

    #[test]
    fn test_additional_derives_empty() {
        let parser = AnnotationParser::new();
        let source = r#"
class MyClass:
    pass
        "#;

        let annotations = parser.parse_annotations(source).unwrap();
        assert_eq!(annotations.additional_derives.len(), 0);
    }
}
