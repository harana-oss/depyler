# String Set to Bitflags Optimization

## Overview

This document outlines an implementation plan for detecting Python sets of strings that function as enums and automatically converting them to Rust bitflags for improved performance.

## Motivation

Python code often uses sets of strings as pseudo-enums:

```python
PERMISSIONS = {"read", "write", "execute"}
STATUS_FLAGS = {"active", "pending", "archived"}

def has_permission(user_perms: set[str], required: str) -> bool:
    return required in user_perms
```

This pattern is common but inefficient. Converting to bitflags provides:
- O(1) membership checks via bitwise AND
- Reduced memory footprint (u8/u16/u32/u64 vs HashSet<String>)
- Cache-friendly operations
- Zero heap allocation

## Detection Strategy

### Phase 1: Static Analysis

#### 1.1 String Set Literal Detection

Identify set literals containing only string values:

```python
# Detectable patterns
valid_states = {"pending", "active", "completed"}
allowed_modes = frozenset(["read", "write"])
user_roles = {"admin", "editor", "viewer"}
```

**Detection Criteria:**
- Right-hand side is `set{...}` or `frozenset([...])`
- All elements are string literals (no variables or expressions)
- Finite, enumerable set of values known at compile time

#### 1.2 Type Annotation Analysis

Look for `Literal` type hints that suggest enum-like usage:

```python
from typing import Literal

Mode = Literal["read", "write", "append"]

def open_file(path: str, mode: Mode) -> File:
    ...
```

#### 1.3 Usage Pattern Analysis

Track how string sets are used throughout the module:

| Pattern | Detection Method | Confidence |
|---------|-----------------|------------|
| `x in SET` | `HirExpr::BinaryOp(In)` with set variable | High |
| `SET & other` | Set intersection operations | High |
| `SET \| other` | Set union operations | Medium |
| Function param with set membership checks | Control flow analysis | Medium |
| Match/case with string literals | Pattern matching analysis | High |

### Phase 2: Mutability Analysis

The core challenge is determining whether a string set is effectively immutable. This requires tracking all operations on the variable across its entire lifetime.

#### 2.1 Mutation Detection

Track all operations that modify a set:

```rust
#[derive(Debug, Clone)]
pub enum SetMutation {
    /// Direct method calls that mutate
    MethodCall {
        method: MutatingMethod,
        location: Span,
    },
    /// Reassignment to a different value
    Reassignment {
        location: Span,
    },
    /// Augmented assignment (|=, &=, -=, ^=)
    AugmentedAssignment {
        op: SetOp,
        location: Span,
    },
    /// Passed to function that may mutate (by reference)
    PassedToMutatingFunction {
        function: String,
        location: Span,
    },
    /// Aliased to another variable that is later mutated
    AliasedAndMutated {
        alias: String,
        mutation_location: Span,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum MutatingMethod {
    Add,        // set.add(x)
    Remove,     // set.remove(x)
    Discard,    // set.discard(x)
    Pop,        // set.pop()
    Clear,      // set.clear()
    Update,     // set.update(other)
    IntersectionUpdate,  // set.intersection_update(other)
    DifferenceUpdate,    // set.difference_update(other)
    SymmetricDifferenceUpdate,  // set.symmetric_difference_update(other)
}
```

#### 2.2 Dataflow Analysis for Immutability

Perform intra-procedural and inter-procedural analysis:

```rust
pub struct MutabilityAnalyzer {
    /// All definitions of set variables
    definitions: HashMap<String, SetDefinition>,
    /// Detected mutations per variable
    mutations: HashMap<String, Vec<SetMutation>>,
    /// Alias tracking: variable -> set of aliases
    aliases: HashMap<String, HashSet<String>>,
    /// Function signatures with mutability info
    function_params: HashMap<String, Vec<ParamMutability>>,
}

impl MutabilityAnalyzer {
    /// Determines if a set variable is immutable throughout its scope
    pub fn is_immutable(&self, name: &str) -> ImmutabilityResult {
        // Check direct mutations
        if let Some(mutations) = self.mutations.get(name) {
            if !mutations.is_empty() {
                return ImmutabilityResult::Mutable {
                    reasons: mutations.clone(),
                };
            }
        }
        
        // Check alias mutations
        if let Some(aliases) = self.aliases.get(name) {
            for alias in aliases {
                if let Some(mutations) = self.mutations.get(alias) {
                    if !mutations.is_empty() {
                        return ImmutabilityResult::MutableViaAlias {
                            alias: alias.clone(),
                            reasons: mutations.clone(),
                        };
                    }
                }
            }
        }
        
        ImmutabilityResult::Immutable
    }
}

#[derive(Debug)]
pub enum ImmutabilityResult {
    Immutable,
    Mutable { reasons: Vec<SetMutation> },
    MutableViaAlias { alias: String, reasons: Vec<SetMutation> },
    Unknown, // Cannot determine statically
}
```

#### 2.3 Scope-Based Analysis

Track variable lifetimes and scope:

```python
# Example: Set is immutable within function scope
def process():
    states = {"a", "b", "c"}  # Definition
    if x in states:           # Read-only usage
        return True
    return False              # states goes out of scope, never mutated

# Example: Set is mutable - CANNOT optimize
def process_mutable():
    states = {"a", "b"}
    states.add("c")           # Mutation detected
    return "c" in states
```

#### 2.4 Alias Tracking

Detect when sets are aliased and track mutations through aliases:

```python
# Alias creates shared mutability
original = {"a", "b"}
alias = original          # alias now points to same set
alias.add("c")            # Mutates original through alias

# Copy breaks the alias chain - original is still immutable
original = {"a", "b"}
copy = original.copy()    # copy is independent
copy.add("c")             # Does NOT affect original
```

#### 2.5 Function Parameter Analysis

Analyze whether functions mutate set parameters:

```python
def mutates_set(s: set[str]) -> None:
    s.add("new")  # Mutates the parameter

def reads_set(s: set[str]) -> bool:
    return "x" in s  # Read-only

my_set = {"a", "b"}
reads_set(my_set)    # Safe - my_set still immutable candidate
mutates_set(my_set)  # my_set is now mutable, cannot optimize
```

### Phase 3: Value Domain Analysis

#### 3.1 String Value Validation

Verify all string values are:
- Valid Rust identifiers (or can be normalized)
- Unique within the set
- Small enough for efficient bitflags (≤64 values for u64)

#### 3.2 Cross-Reference Analysis

Track all locations where set values appear:

```python
# All these should reference the same enum
STATUS = {"active", "pending"}

def check(s: str) -> bool:
    return s in STATUS

def is_active(s: str) -> bool:
    return s == "active"  # Should also map to bitflag
```

## Edge Cases

### 1. Conditional Mutation

The set may be mutated only in certain code paths:

```python
flags = {"read", "write"}

if admin_mode:
    flags.add("execute")  # Conditional mutation

# Cannot optimize: mutation is possible even if not always executed
```

**Resolution:** Any reachable mutation path disqualifies the candidate.

### 2. Loop-Based Mutation

Sets built incrementally in loops:

```python
# Cannot optimize - set is built dynamically
permissions = set()
for p in user.permissions:
    permissions.add(p)

# CAN optimize - comprehension creates immutable set
permissions = {p for p in ["read", "write", "execute"]}
```

**Resolution:** Distinguish between mutation and construction. Set comprehensions with literal elements are valid candidates.

### 3. Exception Handlers

Mutations in exception handlers:

```python
flags = {"a", "b"}
try:
    process()
except Error:
    flags.add("error")  # Mutation in exception path
```

**Resolution:** Treat exception handlers as reachable code paths.

### 4. Closure Capture

Sets captured by closures may be mutated later:

```python
states = {"active", "pending"}

def make_adder():
    def add_state(s):
        states.add(s)  # Captures and mutates outer set
    return add_state

adder = make_adder()
adder("completed")  # Mutates states
```

**Resolution:** Track closure captures and mark captured sets as potentially mutable if any captured closure performs mutations.

### 5. Class Instance Attributes

Sets as instance attributes have complex lifetimes:

```python
class Config:
    def __init__(self):
        self.valid_modes = {"read", "write"}
    
    def add_mode(self, mode: str):
        self.valid_modes.add(mode)  # Method mutates attribute
```

**Resolution:** For class attributes, require explicit annotation or analyze all methods to determine mutability.

### 6. Module-Level Sets with Deferred Mutation

Sets defined at module level but mutated at runtime:

```python
# module.py
MODES = {"a", "b"}

def init():
    MODES.add("c")  # Called at runtime, mutates module-level set

# Somewhere else
import module
module.init()
```

**Resolution:** Track all call sites and module initialization patterns.

### 7. Dynamic String Values

Sets where values are not all literals:

```python
PREFIX = "mode_"
modes = {f"{PREFIX}read", f"{PREFIX}write"}  # f-strings with variables

# Cannot determine values at compile time
dynamic = {get_mode()}  # Function call
```

**Resolution:** Only optimize sets where ALL values are compile-time string literals.

### 8. Set Operations That Return New Sets

Non-mutating operations that look like mutations:

```python
a = {"x", "y"}
b = {"y", "z"}

# These create NEW sets, don't mutate originals
c = a | b      # Union - a and b unchanged
d = a & b      # Intersection - a and b unchanged
e = a - b      # Difference - a and b unchanged
f = a ^ b      # Symmetric difference - a and b unchanged

# These MUTATE in place
a |= b         # a is mutated
a &= b         # a is mutated
```

**Resolution:** Distinguish between operators that return new values vs augmented assignment operators that mutate in place.

### 9. frozenset vs set

`frozenset` is inherently immutable:

```python
# Always safe to optimize - frozenset cannot be mutated
MODES = frozenset(["read", "write", "execute"])
MODES.add("x")  # AttributeError at runtime - no add method
```

**Resolution:** `frozenset` literals are automatically valid candidates (assuming literal string values).

### 10. Set Passed to External/Unknown Functions

```python
modes = {"a", "b"}
external_library.process(modes)  # Unknown if this mutates
```

**Resolution:** Conservative approach - treat as mutable unless function signature indicates immutability (e.g., `def process(modes: frozenset[str])` or `def process(modes: Sequence[str])`).

### 11. Shadowing

Variable shadowing can complicate tracking:

```python
modes = {"a", "b"}  # Original set

def inner():
    modes = {"x", "y"}  # Shadows outer, different set
    modes.add("z")      # Mutates inner, not outer

# Outer modes is still immutable
```

**Resolution:** Scope-aware tracking distinguishes shadowed variables.

### 12. Global/Nonlocal Declarations

```python
modes = {"a", "b"}

def mutate():
    global modes
    modes.add("c")  # Mutates the global set
```

**Resolution:** Track `global` and `nonlocal` declarations and their effects.

### 13. Large Sets Exceeding Bitflag Capacity

```python
# 100 values - exceeds u64 capacity (64 bits)
many_flags = {"flag_" + str(i) for i in range(100)}
```

**Resolution:** Fall back to `HashSet` for sets with >64 unique values. Emit diagnostic suggesting breaking into multiple flag groups if appropriate.

### 14. Unicode and Special Characters in String Values

```python
modes = {"читать", "писать"}  # Cyrillic
emoji_flags = {"🔴", "🟢", "🔵"}
special = {"with-dash", "with space", "123numeric"}
```

**Resolution:** Normalize to valid Rust identifiers:
- `"with-dash"` → `WITH_DASH`
- `"with space"` → `WITH_SPACE`  
- `"123numeric"` → `_123NUMERIC`
- Non-ASCII → transliterate or use hash-based naming

### 15. Empty Sets

```python
empty = set()  # Empty set literal
also_empty: set[str] = set()
```

**Resolution:** Empty sets are trivially valid but generate no-op bitflags. Consider warning or skipping.

## Code Generation


### Phase 3: Bitflag Generation

#### 3.1 Enum Definition

Generate a Rust enum with `bitflags` derive:

```rust
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Permissions: u8 {
        const READ = 0b001;
        const WRITE = 0b010;
        const EXECUTE = 0b100;
    }
}
```

#### 3.2 Conversion Helpers

Generate bidirectional conversion functions:

```rust
impl Permissions {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::READ),
            "write" => Some(Self::WRITE),
            "execute" => Some(Self::EXECUTE),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::READ => "read",
            Self::WRITE => "write",
            Self::EXECUTE => "execute",
            _ => "",
        }
    }
}
```

#### 3.3 Operation Mapping

| Python Operation | Rust Bitflags |
|-----------------|---------------|
| `x in set` | `flags.contains(Flag::X)` |
| `set1 & set2` | `flags1 & flags2` |
| `set1 \| set2` | `flags1 \| flags2` |
| `set1 - set2` | `flags1 & !flags2` |
| `len(set)` | `flags.bits().count_ones()` |
| `set.add(x)` | `flags.insert(Flag::X)` |
| `set.remove(x)` | `flags.remove(Flag::X)` |

## Implementation Architecture

### New Components

```
crates/depyler-core/src/
├── string_set_detection.rs      # Pattern detection
├── bitflags_gen.rs              # Code generation
└── optimizations/
    └── string_set_optimizer.rs  # Optimization pass
```

### Data Structures

```rust
/// Detected string set that can become bitflags
#[derive(Debug, Clone)]
pub struct StringSetCandidate {
    /// Original variable name
    pub name: String,
    /// String values in the set
    pub values: Vec<String>,
    /// Source location
    pub span: Span,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Evidence for why this is a candidate
    pub evidence: Vec<DetectionEvidence>,
}

#[derive(Debug, Clone)]
pub enum DetectionEvidence {
    ConstantNaming,
    TypeAnnotation(String),
    MembershipCheck { location: Span },
    SetOperation { op: SetOp, location: Span },
    NeverMutated,
    AllLiterals,
}

/// Optimization decision
#[derive(Debug, Clone)]
pub struct BitflagsDecision {
    pub candidate: StringSetCandidate,
    pub target_type: BitflagsTargetType,
    pub flag_mapping: HashMap<String, u64>,
}

#[derive(Debug, Clone, Copy)]
pub enum BitflagsTargetType {
    U8,   // ≤8 values
    U16,  // ≤16 values
    U32,  // ≤32 values
    U64,  // ≤64 values
}
```

### Integration Points

1. **HIR Construction** (`crates/depyler-core/src/hir/`)
   - Add `StringSetLiteral` variant to `HirExpr`
   - Track set definitions in `HirModule`

2. **Type Inference** (`crates/depyler-core/src/type_hints.rs`)
   - Add `UsagePattern::StringSetMembership`
   - Propagate set type information

3. **Optimization Pass** (`crates/depyler-core/src/optimizations/`)
   - Run after HIR construction, before code generation
   - Replace `HirExpr::Set` with `HirExpr::Bitflags` where applicable

4. **Code Generation** (`crates/depyler-core/src/rust_gen/`)
   - Generate bitflags definitions in module preamble
   - Translate operations to bitflag equivalents

## Configuration

### Annotation Support

Allow explicit opt-in/opt-out via depyler annotations:

```python
# @depyler: bitflags
PERMISSIONS = {"read", "write", "execute"}

# @depyler: no_bitflags
DYNAMIC_SET = {"a", "b"}  # Keep as HashSet
```

### Threshold Configuration

```toml
# depyler.toml
[optimizations.string_sets]
enabled = true
min_confidence = 0.8
max_values = 64
require_annotation = false
```

## Testing Strategy

### Unit Tests

1. Detection accuracy for various patterns
2. Immutability analysis correctness
3. Code generation output validation
4. Edge cases (empty sets, single-value sets, 64-value sets)

### Integration Tests

1. Round-trip: Python → Rust → compile → execute
2. Semantic equivalence verification
3. Performance benchmarks vs HashSet

### Benchmark Suite

```rust
#[bench]
fn bench_hashset_contains(b: &mut Bencher) {
    let set: HashSet<&str> = ["read", "write", "execute"].into();
    b.iter(|| set.contains("write"));
}

#[bench]
fn bench_bitflags_contains(b: &mut Bencher) {
    let flags = Permissions::READ | Permissions::WRITE;
    b.iter(|| flags.contains(Permissions::WRITE));
}
```

## Rollout Plan

### Phase 1: Detection Only (v0.3.0)
- Implement detection logic
- Add diagnostic output showing candidates
- No code generation changes

### Phase 2: Opt-in Generation (v0.4.0)
- Generate bitflags when `@depyler: bitflags` annotation present
- Validate generated code compiles and runs

### Phase 3: Automatic Optimization (v0.5.0)
- Enable automatic detection with high-confidence threshold
- Add configuration options
- Performance benchmarks in CI

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| False positives | Incorrect optimization | High confidence threshold, opt-out annotation |
| String interning | Different semantics | Skip if strings used with `id()` or interning |
| Dynamic set modification | Runtime errors | Full mutation analysis |
| Large sets (>64 values) | Can't use bitflags | Fall back to HashSet, warn user |
| Non-ASCII strings | Invalid Rust identifiers | Normalize or skip candidate |

## Dependencies

- `bitflags` crate (already in ecosystem)
- No new external dependencies required

## Success Metrics

1. **Detection Rate**: >90% of eligible string sets detected
2. **False Positive Rate**: <5% of detected candidates
3. **Performance Improvement**: >10x faster membership checks
4. **Memory Reduction**: >90% for typical flag sets

## References

- [bitflags crate documentation](https://docs.rs/bitflags/)
- [Python typing.Literal](https://docs.python.org/3/library/typing.html#typing.Literal)
- Existing pattern detection in `crates/depyler-core/src/type_hints.rs`
- String optimization in `crates/depyler-core/src/string_optimization.rs`
