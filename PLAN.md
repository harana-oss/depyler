# Depyler Test Plan

## Current Test Coverage

The `tests/toml/` directory contains **184 TOML test files** with **3,924 individual tests** covering:

| Category | Test Files |
|----------|-----------|
| Core Language | `assert-statement`, `assignment`, `augmented-assignment`, `binary-literals`, `complex-numbers`, `control-flow`, `global-nonlocal` |
| Modern Python | `type-parameter-lists`, `type-alias-statement`, `keyword-only-args`, `positional-only-args`, `exception-groups`, `match-*` patterns, `walrus-operator` |
| Types & Inference | `basic-types`, `type-inference`, `constant-type-inference`, `optional-types`, `result-types`, `cast-expressions` |
| Classes & OOP | `classes`, `class-methods`, `properties`, `descriptors`, `multiple-inheritance`, `metaclasses`, `init-subclass`, `new-vs-init`, `slots` |
| Dataclasses | `dataclasses-basic`, `dataclasses-advanced`, `dataclasses-comparison`, `dataclasses-field`, `dataclasses-frozen` |
| Enums | `enum-basic`, `enum-advanced`, `enum-int`, `enum-methods`, `enum-copy-semantics` |
| Functions & Lambdas | `functions`, `lambdas`, `closures`, `closures-late-binding`, `closures-mutable` |
| Async | `async-functions`, `async-for`, `async-with`, `async-generators`, `async-comprehensions`, `asyncio-tasks`, `await-expressions` |
| Generators | `generators`, `generator-liveness` |
| Iterators & Comprehensions | `iterators`, `comprehensions` |
| Decorators | `decorators-basic`, `decorators-class`, `decorators-with-args` |
| Context Managers | `context-managers`, `nested-context-managers`, `parenthesized-context`, `contextlib-*` |
| Collections | `lists-arrays`, `dictionaries`, `set-operations`, `collections`, `deque`, `counter`, `chainmap`, `ordereddict`, `defaultdict`, `namedtuple` |
| Strings | `string-operations`, `string-methods-all`, `fstrings`, `fstrings-debug`, `fstrings-nested`, `unicode-strings`, `raw-strings`, `template-strings`, `bytes`, `bytes-strings`, `format-spec` |
| Ownership & Mutability | `ownership`, `mutability`, `mutating-method-names`, `mutable-variable-detection`, `parameter-mutability`, `parameter-borrowing`, `field-alias-mutation`, `interprocedural-mutation`, `for-loop-iterator-mutation`, `return-value-mutation` |
| Lifetimes | `lifetimes`, `lifetime-parameters` |
| Imports | `imports-basic`, `imports-from`, `imports-relative`, `imports-dynamic`, `conditional-imports`, `import-deduplication`, `packages-init`, `packages-all`, `namespace-packages` |
| Stdlib Modules | `modules-math`, `modules-json`, `modules-csv`, `modules-datetime-time`, `modules-regex`, `modules-random`, `modules-os-sys`, `modules-collections`, `modules-itertools-functools` |
| Error Handling | `exceptions`, `exception-groups`, `error-handling`, `result-error-propagation` |
| Dunder Methods | `dunder-arithmetic`, `dunder-attribute`, `dunder-bitwise`, `dunder-callable`, `dunder-comparison`, `dunder-container`, `dunder-context`, `dunder-format`, `dunder-inplace`, `dunder-iteration`, `dunder-lifecycle`, `dunder-reflected`, `dunder-unary` |
| CLI & Integration | `cli-integration`, `argparse` |
| Codegen | `ast-hir-codegen`, `lazy-static-detection`, `reference-return-types`, `cse-optimization`, `dead-code-elimination`, `optimizer-inlining` |
| Examples | `examples-*` (algorithms, arrays, data-processing, data-structures, etc.) |
| Regressions | `regressions` |

## Test Execution Strategy

### TOML-Based Integration Tests

The primary test suite uses TOML files in `tests/toml/`. Run with the `depyler test` command:

```bash
# Run all TOML tests (parallel mode - fast)
cargo run -- test -j

# Run all TOML tests (sequential - better error visibility)
cargo run -- test

# Run with verbose output
cargo run -- test -v

# Run with compilation verification (slower, validates Rust output compiles)
cargo run -- test -c

# Run a specific test file
cargo run -- test -p tests/toml/basic-types.toml

# Filter tests by name
cargo run -- test -f "string_constants"

# Combine options
cargo run -- test -j -v -f "dataclass"
```

### Python-Language-Only Tests

Pure Python tests that verify CPython reference semantics. Located in `tests/python/`:

```bash
# Run all Python tests with pytest
python -m pytest tests/python/ -v

# Run individual suites
python -m pytest tests/python/semantics/ -v      # CPython semantic edge cases
python -m pytest tests/python/data_model/ -v     # Data model / dunder conformance
python -m pytest tests/python/stdlib/ -v         # Stdlib tiny conformance
python -m pytest tests/python/parser/ -v         # Parsing & syntax round-trip
python -m pytest tests/python/version_gates/ -v  # Version-gated behavior (3.11+)

# Or with unittest
python -m unittest discover tests/python -v
```

### Unit Tests (per crate)
```bash
cargo test -p depyler-core
cargo test -p depyler-analysis
cargo test -p depyler-verify
cargo test -p depyler-annotations
```

### Property-Based Tests
```bash
cargo test --features quickcheck
```

### Benchmark Suite
```bash
cargo bench --bench transpilation
```
