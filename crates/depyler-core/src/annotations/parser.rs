use std::collections::HashMap;

use regex::Regex;

use crate::annotations::types::{
    AnnotationError, BoundsChecking, CompatibilityLayer, ErrorStrategy,
    FallbackStrategy, GlobalStrategy, HashStrategy, InteriorMutability,
    MigrationStrategy, OwnershipModel,
    PanicBehavior, PerformanceHint, SafetyLevel, ServiceType, StringStrategy, Termination,
    ThreadSafety, TranspilationAnnotations, TypeStrategy,
};

pub struct AnnotationParser {
    pattern: Regex,
}

impl Default for AnnotationParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnotationParser {
    /// Creates a new annotation parser.
    ///
    /// # Panics
    ///
    /// Panics if the internal regex pattern fails to compile (should never happen).
    pub fn new() -> Self {
        let pattern =
            // This regex is statically known to be valid
            // Match both comment-style (# @quantsim:) and docstring-style (@quantsim:) annotations
            Regex::new(r"(?:#\s*)?@quantsim:\s*(\w+)\s*=\s*(.+)")
                .unwrap_or_else(|e| panic!("Failed to compile annotation regex: {e}"));
        Self { pattern }
    }

    /// Parses annotations from source code comments.
    ///
    /// # Errors
    ///
    /// Returns `AnnotationError` if unknown keys or invalid values are encountered.
    ///
    /// # Panics
    ///
    /// Panics if the regex fails to capture groups (should not happen with valid regex).
    pub fn parse_annotations(
        &self,
        source: &str,
    ) -> Result<TranspilationAnnotations, AnnotationError> {
        let mut annotations = TranspilationAnnotations::default();
        let mut parsed_values: HashMap<String, String> = HashMap::new();

        for line in source.lines() {
            if let Some(captures) = self.pattern.captures(line) {
                let key = captures.get(1).unwrap().as_str().to_string();
                let value = captures.get(2).unwrap().as_str().trim_matches('"').trim();

                // Special handling for custom_attribute - accumulate instead of replace
                if key == "custom_attribute" {
                    annotations.custom_attributes.push(value.to_string());
                } else if key == "additional_derives" {
                    // Parse comma-separated derives and accumulate
                    for derive in value.split(',') {
                        let derive = derive.trim();
                        if !derive.is_empty() {
                            annotations.additional_derives.push(derive.to_string());
                        }
                    }
                } else {
                    parsed_values.insert(key, value.to_string());
                }
            }
        }

        self.apply_annotations(&mut annotations, parsed_values)?;
        Ok(annotations)
    }

    /// Parses annotations from function-specific source code.
    ///
    /// # Errors
    ///
    /// Returns `AnnotationError` if parsing fails.
    pub fn parse_function_annotations(
        &self,
        function_source: &str,
    ) -> Result<TranspilationAnnotations, AnnotationError> {
        self.parse_annotations(function_source)
    }

    fn apply_annotations(
        &self,
        annotations: &mut TranspilationAnnotations,
        values: HashMap<String, String>,
    ) -> Result<(), AnnotationError> {
        for (key, value) in values {
            // Dispatch to category handlers
            match key.as_str() {
                // Core annotations (5)
                "type_strategy" | "ownership" | "safety_level" | "fallback" | "bounds_checking" => {
                    self.apply_core_annotation(annotations, &key, &value)?;
                }

                // Optimization annotations (4)
                "performance_critical"
                | "vectorize"
                | "unroll_loops"
                | "optimization_hint" => {
                    self.apply_optimization_annotation(annotations, &key, &value)?;
                }

                // Thread safety annotations (2)
                "thread_safety" | "interior_mutability" => {
                    self.apply_thread_safety_annotation(annotations, &key, &value)?;
                }

                // String/Hash strategy (2)
                "string_strategy" | "hash_strategy" => {
                    self.apply_string_hash_annotation(annotations, &key, &value)?;
                }

                // Error handling (2)
                "panic_behavior" | "error_strategy" => {
                    self.apply_error_handling_annotation(annotations, &key, &value)?;
                }

                // Global strategy (1)
                "global_strategy" => {
                    self.apply_global_strategy_annotation(annotations, &value)?;
                }

                // Verification (3)
                "termination" | "invariant" | "verify_bounds" => {
                    self.apply_verification_annotation(annotations, &key, &value)?;
                }

                // Service metadata (4)
                "service_type" | "migration_strategy" | "compatibility_layer" | "pattern" => {
                    self.apply_service_metadata_annotation(annotations, &key, &value)?;
                }

                _ => return Err(AnnotationError::UnknownKey(key)),
            }
        }
        Ok(())
    }

    /// Apply core annotation (type_strategy, ownership, safety_level, fallback, bounds_checking)
    #[inline]
    fn apply_core_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        key: &str,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match key {
            "type_strategy" => {
                annotations.type_strategy = self.parse_type_strategy(value)?;
            }
            "ownership" => {
                annotations.ownership_model = self.parse_ownership_model(value)?;
            }
            "safety_level" => {
                annotations.safety_level = self.parse_safety_level(value)?;
            }
            "fallback" => {
                annotations.fallback_strategy = self.parse_fallback_strategy(value)?;
            }
            "bounds_checking" => {
                annotations.bounds_checking = self.parse_bounds_checking(value)?;
            }
            _ => unreachable!("apply_core_annotation called with non-core key"),
        }
        Ok(())
    }

    /// Apply optimization annotation (performance_critical, vectorize, unroll_loops, optimization_hint)
    #[inline]
    fn apply_optimization_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        key: &str,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match key {
            "performance_critical" => {
                if value == "true" {
                    annotations
                        .performance_hints
                        .push(PerformanceHint::PerformanceCritical);
                }
            }
            "vectorize" => {
                if value == "true" {
                    annotations
                        .performance_hints
                        .push(PerformanceHint::Vectorize);
                }
            }
            "unroll_loops" => {
                let count: u32 = value.parse().map_err(|_| AnnotationError::InvalidValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })?;
                annotations
                    .performance_hints
                    .push(PerformanceHint::UnrollLoops(count));
            }
            "optimization_hint" => {
                self.apply_optimization_hint(annotations, value)?;
            }
            _ => unreachable!("apply_optimization_annotation called with non-optimization key"),
        }
        Ok(())
    }

    /// Apply optimization hint sub-handler
    #[inline]
    fn apply_optimization_hint(
        &self,
        annotations: &mut TranspilationAnnotations,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match value {
            "vectorize" => annotations
                .performance_hints
                .push(PerformanceHint::Vectorize),
            "latency" => annotations
                .performance_hints
                .push(PerformanceHint::OptimizeForLatency),
            "throughput" => annotations
                .performance_hints
                .push(PerformanceHint::OptimizeForThroughput),
            "async_ready" => {
                eprintln!("Warning: async_ready is experimental and not yet fully supported");
            }
            _ => {
                return Err(AnnotationError::InvalidValue {
                    key: "optimization_hint".to_string(),
                    value: value.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Apply thread safety annotation (thread_safety, interior_mutability)
    #[inline]
    fn apply_thread_safety_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        key: &str,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match key {
            "thread_safety" => {
                annotations.thread_safety = self.parse_thread_safety(value)?;
            }
            "interior_mutability" => {
                annotations.interior_mutability = self.parse_interior_mutability(value)?;
            }
            _ => unreachable!("apply_thread_safety_annotation called with non-thread-safety key"),
        }
        Ok(())
    }

    /// Apply global strategy annotation
    #[inline]
    fn apply_global_strategy_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        value: &str,
    ) -> Result<(), AnnotationError> {
        annotations.global_strategy = self.parse_global_strategy(value)?;
        Ok(())
    }

    /// Apply string/hash strategy annotation (string_strategy, hash_strategy)
    #[inline]
    fn apply_string_hash_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        key: &str,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match key {
            "string_strategy" => {
                annotations.string_strategy = self.parse_string_strategy(value)?;
            }
            "hash_strategy" => {
                annotations.hash_strategy = self.parse_hash_strategy(value)?;
            }
            _ => unreachable!("apply_string_hash_annotation called with non-string/hash key"),
        }
        Ok(())
    }

    /// Apply error handling annotation (panic_behavior, error_strategy)
    #[inline]
    fn apply_error_handling_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        key: &str,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match key {
            "panic_behavior" => {
                annotations.panic_behavior = self.parse_panic_behavior(value)?;
            }
            "error_strategy" => {
                annotations.error_strategy = self.parse_error_strategy(value)?;
            }
            _ => unreachable!("apply_error_handling_annotation called with non-error key"),
        }
        Ok(())
    }

    /// Apply verification annotation (termination, invariant, verify_bounds)
    #[inline]
    fn apply_verification_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        key: &str,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match key {
            "termination" => {
                annotations.termination = self.parse_termination(value)?;
            }
            "invariant" => {
                annotations.invariants.push(value.to_string());
            }
            "verify_bounds" => {
                annotations.verify_bounds = value == "true";
            }
            _ => unreachable!("apply_verification_annotation called with non-verification key"),
        }
        Ok(())
    }

    /// Apply service metadata annotation (service_type, migration_strategy, compatibility_layer, pattern)
    #[inline]
    fn apply_service_metadata_annotation(
        &self,
        annotations: &mut TranspilationAnnotations,
        key: &str,
        value: &str,
    ) -> Result<(), AnnotationError> {
        match key {
            "service_type" => {
                annotations.service_type = Some(self.parse_service_type(value)?);
            }
            "migration_strategy" => {
                annotations.migration_strategy = Some(self.parse_migration_strategy(value)?);
            }
            "compatibility_layer" => {
                annotations.compatibility_layer = Some(self.parse_compatibility_layer(value)?);
            }
            "pattern" => {
                annotations.pattern = Some(value.to_string());
            }
            _ => unreachable!("apply_service_metadata_annotation called with non-service key"),
        }
        Ok(())
    }

    fn parse_type_strategy(&self, value: &str) -> Result<TypeStrategy, AnnotationError> {
        match value {
            "conservative" => Ok(TypeStrategy::Conservative),
            "aggressive" => Ok(TypeStrategy::Aggressive),
            "zero_copy" => Ok(TypeStrategy::ZeroCopy),
            "always_owned" => Ok(TypeStrategy::AlwaysOwned),
            _ => Err(AnnotationError::InvalidValue {
                key: "type_strategy".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_ownership_model(&self, value: &str) -> Result<OwnershipModel, AnnotationError> {
        match value {
            "owned" => Ok(OwnershipModel::Owned),
            "borrowed" => Ok(OwnershipModel::Borrowed),
            "shared" => Ok(OwnershipModel::Shared),
            _ => Err(AnnotationError::InvalidValue {
                key: "ownership".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_safety_level(&self, value: &str) -> Result<SafetyLevel, AnnotationError> {
        match value {
            "safe" => Ok(SafetyLevel::Safe),
            "unsafe_allowed" => Ok(SafetyLevel::UnsafeAllowed),
            _ => Err(AnnotationError::InvalidValue {
                key: "safety_level".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_fallback_strategy(&self, value: &str) -> Result<FallbackStrategy, AnnotationError> {
        match value {
            "mcp" => Ok(FallbackStrategy::Mcp),
            "manual" => Ok(FallbackStrategy::Manual),
            "error" => Ok(FallbackStrategy::Error),
            _ => Err(AnnotationError::InvalidValue {
                key: "fallback".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_bounds_checking(&self, value: &str) -> Result<BoundsChecking, AnnotationError> {
        match value {
            "explicit" => Ok(BoundsChecking::Explicit),
            "implicit" => Ok(BoundsChecking::Implicit),
            "disabled" => Ok(BoundsChecking::Disabled),
            _ => Err(AnnotationError::InvalidValue {
                key: "bounds_checking".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_thread_safety(&self, value: &str) -> Result<ThreadSafety, AnnotationError> {
        match value {
            "required" => Ok(ThreadSafety::Required),
            "not_required" => Ok(ThreadSafety::NotRequired),
            _ => Err(AnnotationError::InvalidValue {
                key: "thread_safety".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_interior_mutability(
        &self,
        value: &str,
    ) -> Result<InteriorMutability, AnnotationError> {
        match value {
            "none" => Ok(InteriorMutability::None),
            "arc_mutex" => Ok(InteriorMutability::ArcMutex),
            "ref_cell" => Ok(InteriorMutability::RefCell),
            "cell" => Ok(InteriorMutability::Cell),
            _ => Err(AnnotationError::InvalidValue {
                key: "interior_mutability".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_string_strategy(&self, value: &str) -> Result<StringStrategy, AnnotationError> {
        match value {
            "conservative" => Ok(StringStrategy::Conservative),
            "always_owned" => Ok(StringStrategy::AlwaysOwned),
            "zero_copy" => Ok(StringStrategy::ZeroCopy),
            _ => Err(AnnotationError::InvalidValue {
                key: "string_strategy".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_hash_strategy(&self, value: &str) -> Result<HashStrategy, AnnotationError> {
        match value {
            "standard" => Ok(HashStrategy::Standard),
            "fnv" => Ok(HashStrategy::Fnv),
            "ahash" => Ok(HashStrategy::AHash),
            _ => Err(AnnotationError::InvalidValue {
                key: "hash_strategy".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_panic_behavior(&self, value: &str) -> Result<PanicBehavior, AnnotationError> {
        match value {
            "propagate" => Ok(PanicBehavior::Propagate),
            "return_error" => Ok(PanicBehavior::ReturnError),
            "abort" => Ok(PanicBehavior::Abort),
            _ => Err(AnnotationError::InvalidValue {
                key: "panic_behavior".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_error_strategy(&self, value: &str) -> Result<ErrorStrategy, AnnotationError> {
        match value {
            "panic" => Ok(ErrorStrategy::Panic),
            "result_type" => Ok(ErrorStrategy::ResultType),
            "option_type" => Ok(ErrorStrategy::OptionType),
            _ => Err(AnnotationError::InvalidValue {
                key: "error_strategy".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_global_strategy(&self, value: &str) -> Result<GlobalStrategy, AnnotationError> {
        match value {
            "none" => Ok(GlobalStrategy::None),
            "lazy_static" => Ok(GlobalStrategy::LazyStatic),
            "once_cell" => Ok(GlobalStrategy::OnceCell),
            _ => Err(AnnotationError::InvalidValue {
                key: "global_strategy".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_termination(&self, value: &str) -> Result<Termination, AnnotationError> {
        match value {
            "unknown" => Ok(Termination::Unknown),
            "proven" => Ok(Termination::Proven),
            _ => {
                if value.starts_with("bounded_") {
                    if let Some(num_str) = value.strip_prefix("bounded_") {
                        if let Ok(bound) = num_str.parse::<u32>() {
                            return Ok(Termination::BoundedLoop(bound));
                        }
                    }
                }
                Err(AnnotationError::InvalidValue {
                    key: "termination".to_string(),
                    value: value.to_string(),
                })
            }
        }
    }

    fn parse_service_type(&self, value: &str) -> Result<ServiceType, AnnotationError> {
        match value {
            "web_api" => Ok(ServiceType::WebApi),
            "cli" => Ok(ServiceType::Cli),
            "library" => Ok(ServiceType::Library),
            _ => Err(AnnotationError::InvalidValue {
                key: "service_type".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_migration_strategy(&self, value: &str) -> Result<MigrationStrategy, AnnotationError> {
        match value {
            "incremental" => Ok(MigrationStrategy::Incremental),
            "big_bang" => Ok(MigrationStrategy::BigBang),
            "hybrid" => Ok(MigrationStrategy::Hybrid),
            _ => Err(AnnotationError::InvalidValue {
                key: "migration_strategy".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn parse_compatibility_layer(
        &self,
        value: &str,
    ) -> Result<CompatibilityLayer, AnnotationError> {
        match value {
            "pyo3" => Ok(CompatibilityLayer::PyO3),
            "ctypes" => Ok(CompatibilityLayer::CTypes),
            "none" => Ok(CompatibilityLayer::None),
            _ => Err(AnnotationError::InvalidValue {
                key: "compatibility_layer".to_string(),
                value: value.to_string(),
            }),
        }
    }
}
