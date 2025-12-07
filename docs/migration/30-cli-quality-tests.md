# Migration Task: CLI, Coverage, and Quality Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_cli.rs | ~200 | ~10 | Integration |
| test_cli_edge_cases.rs | ~150 | ~8 | Integration |
| test_depyler.rs | ~180 | ~9 | Integration |
| test_quality.rs | ~120 | ~6 | Quality metrics |
| test_coverage_basic.rs | ~100 | ~5 | Coverage |

## Target Files

- `tests/toml/30-cli-integration.toml`
- `tests/toml/31-quality-metrics.toml`

Note: CLI and integration tests may not be suitable for TOML migration due to their nature. This document outlines what can be migrated and what should remain as Rust tests.

## CLI Tests to Keep as Rust

The following tests involve subprocess execution, file I/O, and complex setup that don't fit the TOML pattern:

```rust
// These should remain as Rust integration tests:
// - CLI argument parsing
// - File input/output operations
// - Error message formatting
// - Exit code verification
// - Multi-file transpilation
```

## Tests That CAN Be Migrated

### Simple Transpilation Tests (30-cli-integration.toml)

```toml
[[test]]
name = "cli_basic_function"
description = "Basic function transpilation via CLI"

[test.python]
code =  '''
def hello() -> str:
    return "Hello, World!"
'''

[test.assertions]
any_of = ["fn hello()", "String", "Hello, World!"]
compiles = true
```

```toml
[[test]]
name = "cli_type_annotation"
description = "Type annotation preservation"

[test.python]
code =  '''
def typed_func(x: int, y: str) -> bool:
    return len(y) > x
'''

[test.assertions]
any_of = ["i64", "String", "bool"]
compiles = true
```

```toml
[[test]]
name = "cli_multiple_functions"
description = "Multiple functions in single file"

[test.python]
code =  '''
def first() -> int:
    return 1

def second() -> int:
    return 2

def combined() -> int:
    return first() + second()
'''

[test.assertions]
any_of = ["fn first()", "fn second()", "fn combined()"]
compiles = true
```

### Quality Metrics Tests (31-quality-metrics.toml)

```toml
[[test]]
name = "quality_type_safety_score"
description = "Type safety scoring for fully typed code"

[test.python]
code =  '''
def fully_typed(x: int, y: int) -> int:
    z: int = x + y
    return z
'''

[test.assertions]
quality_score_min = 90
category = "type_safety"
```

```toml
[[test]]
name = "quality_memory_safety_score"
description = "Memory safety scoring"

[test.python]
code =  '''
def safe_list_access(items: list[int], index: int) -> int:
    if 0 <= index < len(items):
        return items[index]
    return 0
'''

[test.assertions]
quality_score_min = 85
category = "memory_safety"
```

```toml
[[test]]
name = "quality_energy_efficiency"
description = "Energy efficiency scoring"

[test.python]
code =  '''
def efficient_sum(nums: list[int]) -> int:
    return sum(nums)
'''

[test.assertions]
quality_score_min = 80
category = "energy_efficiency"
```

```toml
[[test]]
name = "quality_code_complexity"
description = "Code complexity measurement"

[test.python]
code =  '''
def complex_function(x: int) -> int:
    if x > 0:
        if x > 10:
            if x > 100:
                return 3
            return 2
        return 1
    return 0
'''

[test.assertions]
complexity_max = 10
category = "complexity"
```

## Tests to Remain as Rust

### CLI Integration (Keep as Rust)

```rust
// test_cli.rs - Keep for:
#[test]
fn test_cli_file_input() {
    // Tests actual file I/O
}

#[test]
fn test_cli_error_handling() {
    // Tests error messages and exit codes
}

#[test]
fn test_cli_output_file() {
    // Tests file writing
}

#[test]
fn test_cli_multiple_files() {
    // Tests batch processing
}
```

### Coverage Tests (Keep as Rust)

```rust
// test_coverage_basic.rs - Keep for:
#[test]
fn test_coverage_report_generation() {
    // Tests coverage data collection
}

#[test]
fn test_coverage_line_tracking() {
    // Tests line-by-line coverage
}
```

## Recommended Approach

### Phase 1: Keep as Rust
- CLI argument parsing tests
- File I/O tests
- Coverage report generation tests
- Error handling tests

### Phase 2: Migrate to TOML
- Simple transpilation quality checks
- Quality metric calculations
- Basic compilation verification tests

## Extended TOML Format for Quality Tests

```toml
[[test]]
name = "quality_test_name"
category = "quality"

[test.python]
code =  '''
# Python code
'''

[test.quality_assertions]
type_safety_score_min = 90
memory_safety_score_min = 85
energy_efficiency_min = 80
complexity_max = 10
coverage_min = 80

[test.compilation]
must_compile = true
no_warnings = true
```

## Notes

- CLI tests often require actual subprocess execution
- Coverage tests need instrumentation and runtime tracking
- Quality metrics tests can be TOML-migrated with custom assertion types
- Consider a hybrid approach: keep integration tests in Rust, migrate unit-style tests to TOML

## Acceptance Criteria

- [ ] Identify CLI tests suitable for migration
- [ ] Migrate quality metric scoring tests
- [ ] Document tests that must remain as Rust
- [ ] Create extended TOML format for quality assertions
- [ ] Update test runner to support quality assertions

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Medium (hybrid approach needed)
