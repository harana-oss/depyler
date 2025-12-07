# Test Migration Index

## Overview

This index provides a complete listing of all migration tasks for converting Rust tests to TOML format.

## Quick Navigation

| Task | File | Status | Est. Time | Risk |
|------|------|--------|-----------|------|
| [00 - Overview](./00-overview.md) | - | Reference | - | - |
| [01 - Basic Types](./01-basic-types.md) | `01-basic-types.toml` | Not Started | 2h | Low |
| [02 - Strings](./02-strings.md) | `02-strings.toml` | Not Started | 3h | Low |
| [03 - Lists](./03-lists.md) | `03-lists.toml` | Not Started | 3h | Low |
| [04 - Dictionaries](./04-dictionaries.md) | `04-dictionaries.toml` | Not Started | 2-3h | Low |
| [05 - Tuples and Sets](./05-tuples-sets.md) | `05-tuples-sets.toml` | Not Started | 2h | Low |
| [06 - Operators](./06-operators.md) | `06-operators.toml` | Not Started | 2h | Low |
| [07 - Assignment](./07-assignment.md) | `07-assignment.toml` | Not Started | 1.5h | Low |
| [08 - Control Flow](./08-control-flow.md) | `08-control-flow.toml` | Not Started | 2-3h | Low |
| [09 - Loops](./09-loops.md) | `09-loops.toml` | Not Started | 2-3h | Low |
| [10 - Functions](./10-functions.md) | `10-functions.toml` | Not Started | 3-4h | Low |
| [11 - Lambdas](./11-lambdas.md) | `11-lambdas.toml` | Not Started | 2-3h | Low-Medium |
| [12 - Classes](./12-classes.md) | `12-classes.toml` | Not Started | 3-4h | Medium |
| [13 - Generators](./13-generators.md) | `13-generators.toml` | Not Started | 2-3h | Medium |
| [14 - Comprehensions](./14-comprehensions.md) | `14-comprehensions.toml` | Not Started | 2-3h | Low-Medium |
| [15 - Error Handling](./15-error-handling.md) | `15-error-handling.toml` | Not Started | 2-3h | Low |
| [16 - Type Inference](./16-type-inference.md) | `16-type-inference.toml` | Not Started | 2-3h | Low |
| [17 - Ownership](./17-ownership.md) | `17-ownership.toml` | Not Started | 3-4h | Medium |
| [18 - Mutability](./18-mutability.md) | `18-mutability.toml` | Not Started | 2-3h | Medium |
| [19 - Optional Types](./19-optional-types.md) | `19-optional-types.toml` | Not Started | 2h | Low |
| [20 - Result Types](./20-result-types.md) | `20-result-types.toml` | Not Started | 2h | Low |
| [21 - Math Module](./21-math-module.md) | `21-math-stdlib.toml` | Not Started | 2h | Low |
| [22 - JSON Module](./22-json-module.md) | `22-json-stdlib.toml` | Not Started | 2h | Low |
| [23 - OS/Sys Modules](./23-os-sys-modules.md) | `23-os-sys-modules.toml` | Not Started | 2-3h | Low-Medium |
| [24 - Datetime/Time](./24-datetime-time-modules.md) | `24-datetime-time-modules.toml` | Not Started | 2h | Low |
| [25 - Collections](./25-collections-module.md) | `25-collections-module.toml` | Not Started | 2-3h | Low |
| [26 - Itertools/Functools](./26-itertools-functools-modules.md) | `26-itertools-functools.toml` | Not Started | 3-4h | Low-Medium |
| [27 - Random](./27-random-module.md) | `27-random-stdlib.toml` | Not Started | 2h | Low |
| [28 - Regex](./28-regex-module.md) | `28-regex-stdlib.toml` | Not Started | 2-3h | Low |
| [29 - AST/HIR/Codegen](./29-ast-hir-codegen.md) | `29-ast-hir-codegen.toml` | Not Started | 4-5h | Medium |
| [30 - CLI/Quality](./30-cli-quality-tests.md) | `30-cli-integration.toml` | Not Started | 3-4h | Medium |

## Statistics

- **Total Tasks**: 30
- **Total Source Files**: ~234
- **Target TOML Files**: ~30
- **Estimated Total Time**: 70-90 hours
- **Tests to Migrate**: ~191 (using transpilation patterns)
- **Tests to Keep as Rust**: ~43 (integration, HIR-only, coverage)

## Migration Phases

### Phase 1: Core Language (Tasks 01-10)
Foundation tests covering basic Python-to-Rust mappings.

| Task | Priority | Dependencies |
|------|----------|--------------|
| 01 - Basic Types | P1 | None |
| 02 - Strings | P1 | 01 |
| 03 - Lists | P1 | 01 |
| 04 - Dictionaries | P1 | 01 |
| 05 - Tuples/Sets | P1 | 01 |
| 06 - Operators | P1 | 01-05 |
| 07 - Assignment | P1 | 01-06 |
| 08 - Control Flow | P1 | 01-07 |
| 09 - Loops | P1 | 01-08 |
| 10 - Functions | P1 | 01-09 |

### Phase 2: Advanced Features (Tasks 11-20)
Advanced Python features and Rust-specific concepts.

| Task | Priority | Dependencies |
|------|----------|--------------|
| 11 - Lambdas | P2 | 10 |
| 12 - Classes | P2 | 10 |
| 13 - Generators | P2 | 09, 11 |
| 14 - Comprehensions | P2 | 03, 09 |
| 15 - Error Handling | P2 | 08 |
| 16 - Type Inference | P2 | 01-10 |
| 17 - Ownership | P2 | 01-16 |
| 18 - Mutability | P2 | 01-16 |
| 19 - Optional Types | P2 | 15 |
| 20 - Result Types | P2 | 15 |

### Phase 3: Standard Library (Tasks 21-28)
Python stdlib to Rust crate mappings.

| Task | Priority | Dependencies |
|------|----------|--------------|
| 21 - Math Module | P3 | 01, 06 |
| 22 - JSON Module | P3 | 02, 04 |
| 23 - OS/Sys Modules | P3 | 02, 03 |
| 24 - Datetime/Time | P3 | 01, 02 |
| 25 - Collections | P3 | 03, 04, 05 |
| 26 - Itertools/Functools | P3 | 09, 11, 14 |
| 27 - Random | P3 | 01, 03 |
| 28 - Regex | P3 | 02, 03 |

### Phase 4: Infrastructure (Tasks 29-30)
AST, code generation, and tooling tests.

| Task | Priority | Dependencies |
|------|----------|--------------|
| 29 - AST/HIR/Codegen | P4 | All |
| 30 - CLI/Quality | P4 | All |

## TOML Format Quick Reference

```toml
[[test]]
name = "test_name"
description = "What this test verifies"

[test.python]
code =  '''
def example(x: int) -> int:
    return x * 2
'''

[test.assertions]
any_of = ["pattern1", "pattern2"]  # At least one must match
all_of = ["pattern1", "pattern2"]  # All must match
none_of = ["anti_pattern"]         # None should match
compiles = true                    # Rust must compile
```

## Test Runner Commands

```bash
# Run all TOML tests
cargo test --test toml_runner

# Run specific category
cargo test --test toml_runner -- --filter "strings"

# Run with verbose output
cargo test --test toml_runner -- --verbose

# Validate TOML files only (no execution)
cargo test --test toml_runner -- --validate-only
```

## Getting Started

1. Read [00-overview.md](./00-overview.md) for full context
2. Implement TOML test runner (see overview)
3. Start with Phase 1 tasks (01-10)
4. Migrate one file at a time, verify tests pass
5. Delete original Rust test files after migration

## Notes

- Each task file contains detailed test cases ready for TOML conversion
- Use the `tags` field for filtering and organization
- Maintain backwards compatibility during migration
- Consider parallel execution for TOML tests
