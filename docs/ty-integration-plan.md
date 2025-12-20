# Comprehensive Plan: Integrating `ty` Type Checker into Depyler

## Executive Summary

This document outlines a comprehensive plan for integrating [ty](https://github.com/astral-sh/ty), Astral's extremely fast Python type checker, to replace Depyler's current custom type inference system. `ty` is 10-100x faster than mypy/Pyright and is written in Rust, making it an ideal candidate for integration.

## Current State Analysis

### Depyler's Existing Type System Components

1. **`depyler-core/src/type_hints.rs`** - `TypeHintProvider`
   - Usage pattern analysis (iterator, numeric, string-like, container)
   - Confidence-based type inference (Low, Medium, High, Certain)
   - Constraint collection and type hint generation

2. **`depyler-core/src/generic_inference.rs`** - `TypeVarRegistry`
   - Generic type parameter tracking
   - Type variable constraints (bounds, variance)
   - Rust trait bound generation

3. **`depyler-analysis/src/metrics/type_flow.rs`** - `TypeInferencer`
   - Type environment tracking
   - Function signature inference
   - Type flow analysis across statements

4. **`depyler-core/src/const_generic_inference.rs`** - `ConstGenericInferencer`
   - Fixed-size array pattern detection
   - Const generic inference

5. **`depyler-core/src/lambda_inference.rs`** - `LambdaTypeInferencer`
   - AWS Lambda event type inference

### Current Parser: RustPython

- Uses `rustpython-parser` v0.4 and `rustpython-ast` v0.4
- AST bridge in `depyler-core/src/ast_bridge.rs` converts to HIR

## ty Architecture Overview

> **Reference Commit:** `fa57253980c317cce7ff1f35691e3d850c0fb58b` (astral-sh/ruff)

### Complete Crate Structure (from ruff repository)

```
crates/
├── ty/                          # CLI binary (main entry point)
├── ty_combine/                  # Combine utilities for options
├── ty_completion_eval/          # Completion evaluation
├── ty_ide/                      # IDE features (inlay hints, completions)
├── ty_project/                  # Project configuration & database
├── ty_python_semantic/          # Core semantic analysis (THE KEY CRATE)
├── ty_server/                   # Language server implementation
├── ty_static/                   # Static environment variables
├── ty_test/                     # Testing utilities (mdtest framework)
├── ty_vendored/                 # Bundled typeshed
├── ty_wasm/                     # WebAssembly bindings
│
# Supporting ruff crates we'll also need:
├── ruff_db/                     # Core database traits (Db, Files, System)
├── ruff_python_ast/             # Python AST types
├── ruff_python_parser/          # Python parser
├── ruff_text_size/              # Text size utilities
├── ruff_source_file/            # Source file handling
├── ruff_diagnostics/            # Diagnostic types
└── ruff_notebook/               # Jupyter notebook support
```

### Core Database Trait Hierarchy

ty uses Salsa for incremental computation. The trait hierarchy is:

```rust
// ruff_db/src/lib.rs
#[salsa::db]
pub trait Db: salsa::Database {
    fn vendored(&self) -> &VendoredFileSystem;
    fn system(&self) -> &dyn System;
    fn files(&self) -> &Files;
    fn python_version(&self) -> PythonVersion;
}

// ty_python_semantic/src/db.rs
#[salsa::db]
pub trait Db: ruff_db::Db {
    fn should_check_file(&self, file: File) -> bool;
    fn rule_selection(&self, file: File) -> &RuleSelection;
    fn lint_registry(&self) -> &LintRegistry;
    fn verbose(&self) -> bool;
}

// ty_project/src/db.rs
#[salsa::db]
pub trait Db: ty_python_semantic::Db {
    fn project(&self) -> Project;
    fn dyn_clone(&self) -> Box<dyn Db>;
}
```

### ty's Type System Capabilities

- **Intersection types** (first-class support)
- **Advanced type narrowing** with top/bottom materializations
- **Sophisticated reachability analysis**
- **Gradual typing support** with redeclarations
- **Comprehensive diagnostics** with rich context
- **Incremental analysis** for IDE integration
- **Protocol/structural typing** with synthesized protocols
- **Generics** with TypeVar, ParamSpec, TypeVarTuple support
- **Literal types** (IntLiteral, StringLiteral, BooleanLiteral)
- **Enum literal types** with member tracking

## Integration Strategy

### Phase 1: Dependency Addition and Parser Migration (Weeks 1-2)

#### 1.1 Add ty Crates as Dependencies

Since ty's Rust code lives in the ruff repository, we need to add it as a git dependency. Key requirements from ruff's `Cargo.toml`:

```toml
# Cargo.toml [workspace.dependencies]

# Pin to specific commit for stability
[workspace.dependencies]
# Core ty crates
ty_python_semantic = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }
ty_project = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }
ty_vendored = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }

# Required ruff infrastructure crates
ruff_db = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }
ruff_python_ast = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }
ruff_python_parser = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }
ruff_text_size = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }
ruff_source_file = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }
ruff_diagnostics = { git = "https://github.com/astral-sh/ruff", rev = "fa57253980c317cce7ff1f35691e3d850c0fb58b" }

# Salsa (ty's incremental computation framework) - REQUIRED
# From ruff's Cargo.toml:
salsa = { git = "https://github.com/salsa-rs/salsa.git", rev = "55e5e7d32fa3fc189276f35bb04c9438f9aedbd1", default-features = false, features = [
    "compact_str",
    "macros",
    "salsa_unstable",
    "inventory",
] }
```

**Critical Note:** The Rust edition in ruff is `2024` with `rust-version = "1.90"`. Depyler currently uses `edition = "2024"`. This may require upgrading Depyler's Rust toolchain.

#### 1.2 Replace RustPython Parser with Ruff Parser

ty uses `ruff_python_parser` which is faster and better maintained:

```rust
// Before (current)
use rustpython_parser::{parse, Mode};
use rustpython_ast as ast;

// After (with ty/ruff)
use ruff_python_parser::{parse_module, ParseError};
use ruff_python_ast as ast;
```

#### 1.3 Create Adapter Layer

Create a new module to bridge ty's types to Depyler's HIR. Based on analysis of ty's actual API:

```rust
// crates/depyler-core/src/ty_bridge.rs

use ruff_db::Db as SourceDb;
use ruff_db::files::{File, Files, system_path_to_file};
use ruff_db::system::{MemoryFileSystem, System, SystemPath, SystemPathBuf};
use ruff_db::vendored::VendoredFileSystem;
use ty_python_semantic::{Db as SemanticDb, SemanticModel, HasType};
use ty_python_semantic::types::Type;
use crate::hir::{Type as HirType, HirModule};

/// Depyler's implementation of the ty database traits
#[salsa::db]
pub struct DepylerDb {
    storage: salsa::Storage<Self>,
    files: Files,
    system: MemoryFileSystem,  // or OsSystem for real files
    vendored: VendoredFileSystem,
}

// Implement the required traits
#[salsa::db]
impl SourceDb for DepylerDb {
    fn vendored(&self) -> &VendoredFileSystem {
        &self.vendored
    }
    
    fn system(&self) -> &dyn System {
        &self.system
    }
    
    fn files(&self) -> &Files {
        &self.files
    }
    
    fn python_version(&self) -> ruff_python_ast::PythonVersion {
        ty_python_semantic::Program::get(self).python_version(self)
    }
}

#[salsa::db]
impl SemanticDb for DepylerDb {
    fn should_check_file(&self, file: File) -> bool {
        !file.path(self).is_vendored_path()
    }
    
    fn rule_selection(&self, _file: File) -> &ty_python_semantic::lint::RuleSelection {
        // Return default rules or configure as needed
        &DEFAULT_RULES
    }
    
    fn lint_registry(&self) -> &ty_python_semantic::lint::LintRegistry {
        ty_python_semantic::default_lint_registry()
    }
    
    fn verbose(&self) -> bool {
        false
    }
}

#[salsa::db]
impl salsa::Database for DepylerDb {}

pub struct TyBridge {
    db: DepylerDb,
}

impl TyBridge {
    pub fn new() -> Self { 
        let db = DepylerDb {
            storage: salsa::Storage::default(),
            files: Files::default(),
            system: MemoryFileSystem::new(),
            vendored: ty_vendored::file_system().clone(),
        };
        Self { db }
    }
    
    /// Analyze source code and return type information
    pub fn analyze(&mut self, source: &str, filename: &str) -> Result<TypeMap> {
        // Write source to virtual file system
        self.db.system.write_file(
            SystemPath::new(filename),
            source,
        )?;
        
        // Get file handle
        let file = system_path_to_file(&self.db, filename)?;
        
        // Create semantic model for the file
        let model = SemanticModel::new(&self.db, file);
        
        // Extract types using the model
        // ...
    }
}
```

### Phase 2: Type Inference Integration (Weeks 3-4)

#### 2.1 Create `TyTypeInferencer`

Replace `TypeHintProvider` with ty-backed inference:

```rust
// crates/depyler-core/src/ty_inference.rs

use ty_python_semantic::{
    Db, SemanticModel, HasType, Module,
    resolve_module, infer_types_for_file,
};

pub struct TyTypeInferencer {
    semantic_model: SemanticModel<'static>,
}

impl TyTypeInferencer {
    pub fn infer_function_types(
        &self, 
        func_name: &str
    ) -> Result<FunctionTypes> {
        // Use ty's type inference
        let func_type = self.semantic_model.resolve_function(func_name)?;
        
        FunctionTypes {
            params: self.convert_params(func_type.parameters()),
            return_type: self.convert_type(func_type.return_type()),
        }
    }
}
```

#### 2.2 Integrate with HIR Pipeline

Modify `DepylerPipeline` to use ty for type inference:

```rust
// In depyler-core/src/pipeline.rs

pub struct DepylerPipeline {
    ty_inferencer: TyTypeInferencer,
    // ... other fields
}

impl DepylerPipeline {
    pub fn parse_to_hir(&self, source: &str) -> Result<HirModule> {
        // 1. Parse with ruff_python_parser
        let ast = ruff_python_parser::parse_module(source)?;
        
        // 2. Get type information from ty
        let types = self.ty_inferencer.analyze(source)?;
        
        // 3. Convert to HIR with enriched type info
        let hir = self.ast_bridge.convert_with_types(ast, types)?;
        
        Ok(hir)
    }
}
```

### Phase 3: Advanced Type Features (Weeks 5-6)

#### 3.1 Generic Type Inference

Leverage ty's superior generic handling:

```rust
pub fn infer_generics(&self, func: &HirFunction) -> Vec<TypeParameter> {
    // ty automatically handles:
    // - TypeVar bounds
    // - ParamSpec
    // - TypeVarTuple
    // - Generic aliases
    
    self.semantic_model
        .type_parameters(func_name)
        .map(|tp| convert_type_param(tp))
        .collect()
}
```

#### 3.2 Protocol and Structural Typing

ty provides first-class protocol support:

```rust
pub fn check_protocol_conformance(
    &self,
    ty: Type<'_>,
    protocol: &str
) -> bool {
    self.semantic_model.is_protocol_member(ty, protocol)
}
```

### Phase 4: Deprecation and Migration (Weeks 7-8)

#### 4.1 Components to Deprecate

| Component | Location | Replacement |
|-----------|----------|-------------|
| `TypeHintProvider` | `type_hints.rs` | `TyTypeInferencer` |
| `TypeVarRegistry` | `generic_inference.rs` | ty's generic inference |
| `TypeInferencer` | `type_flow.rs` | `TyTypeInferencer` |
| `ConstGenericInferencer` | `const_generic_inference.rs` | Custom + ty |
| RustPython parser | `ast_bridge.rs` | `ruff_python_parser` |

#### 4.2 Migration Path

1. **Feature flag approach:**
   ```toml
   [features]
   ty-type-checker = ["ty_python_semantic", "ruff_python_parser"]
   legacy-type-checker = []  # Keep old code temporarily
   default = ["ty-type-checker"]
   ```

2. **Gradual rollout:**
   - Week 7: Enable ty behind feature flag
   - Week 8: Remove legacy code after validation

### Phase 5: Testing and Validation (Week 9-10)

#### 5.1 Type Inference Test Suite

```rust
#[test]
fn test_ty_basic_inference() {
    let source = r#"
def add(x: int, y: int) -> int:
    return x + y
"#;
    let inferencer = TyTypeInferencer::new();
    let types = inferencer.infer_module_types(source).unwrap();
    
    assert_eq!(types.get_return_type("add"), Some(HirType::Int));
}

#[test]
fn test_ty_generic_inference() {
    let source = r#"
from typing import TypeVar, List

T = TypeVar('T')

def first(items: List[T]) -> T:
    return items[0]
"#;
    let inferencer = TyTypeInferencer::new();
    let generics = inferencer.infer_generics("first").unwrap();
    
    assert_eq!(generics.len(), 1);
    assert_eq!(generics[0].name, "T");
}
```

#### 5.2 Benchmark Comparisons

```rust
// benches/type_inference_comparison.rs

fn bench_type_inference(c: &mut Criterion) {
    let source = generate_complex_python(100);
    
    let mut group = c.benchmark_group("type_inference");
    
    group.bench_function("legacy_type_hints", |b| {
        let provider = TypeHintProvider::new();
        b.iter(|| provider.analyze(black_box(&source)))
    });
    
    group.bench_function("ty_inference", |b| {
        let inferencer = TyTypeInferencer::new();
        b.iter(|| inferencer.infer_module_types(black_box(&source)))
    });
}
```

## Technical Challenges and Mitigations

### Challenge 1: ty Crates Not Published

**Issue:** ty's Rust crates are not on crates.io yet.

**Mitigation:**
- Use git dependencies with pinned commit (`rev = "fa57253..."`)
- Monitor astral-sh/ty for crates.io publication
- Consider vendoring if needed for release stability

**Key Requirements from ruff's Cargo.toml:**
- Rust edition: `2024`
- Minimum Rust version: `1.90`
- Salsa version: Git dependency with specific features

### Challenge 2: Database Pattern

**Issue:** ty uses Salsa-style incremental computation (the `Db` trait pattern). This requires implementing multiple database traits.

**Mitigation:**

Based on the actual trait hierarchy from the ruff codebase:

```rust
// Required trait implementations (from ty_test/src/db.rs as reference)

use ruff_db::Db as SourceDb;
use ruff_db::files::{File, Files};
use ruff_db::system::{System, MemoryFileSystem, SystemPath};
use ruff_db::vendored::VendoredFileSystem;
use ty_python_semantic::{Db as SemanticDb, Program, default_lint_registry};
use ty_python_semantic::lint::{LintRegistry, RuleSelection};

#[salsa::db]
#[derive(Clone)]
pub struct DepylerDb {
    storage: salsa::Storage<Self>,
    files: Files,
    system: MemoryFileSystem,
    vendored: VendoredFileSystem,
    rule_selection: RuleSelection,
}

impl DepylerDb {
    pub fn new() -> Self {
        Self {
            storage: salsa::Storage::default(),
            files: Files::default(),
            system: MemoryFileSystem::new(),
            vendored: ty_vendored::file_system().clone(),
            rule_selection: RuleSelection::default(),
        }
    }
    
    /// Write Python source to virtual filesystem
    pub fn write_source(&mut self, path: &str, content: &str) -> std::io::Result<()> {
        self.system.write_file(SystemPath::new(path), content)
    }
}

#[salsa::db]
impl SourceDb for DepylerDb {
    fn vendored(&self) -> &VendoredFileSystem { &self.vendored }
    fn system(&self) -> &dyn System { &self.system }
    fn files(&self) -> &Files { &self.files }
    fn python_version(&self) -> ruff_python_ast::PythonVersion {
        Program::get(self).python_version(self)
    }
}

#[salsa::db]
impl SemanticDb for DepylerDb {
    fn should_check_file(&self, file: File) -> bool {
        !file.path(self).is_vendored_path()
    }
    
    fn rule_selection(&self, _file: File) -> &RuleSelection {
        &self.rule_selection
    }
    
    fn lint_registry(&self) -> &LintRegistry {
        default_lint_registry()
    }
    
    fn verbose(&self) -> bool { false }
}

#[salsa::db]
impl salsa::Database for DepylerDb {}
```

### Challenge 3: HIR Type Mapping

**Issue:** ty's `Type<'db>` is complex with many variants. Based on actual code analysis from `ty_python_semantic/src/types/`:

**ty's Type enum variants (from property_tests/type_generation.rs):**
- `Type::Never` - Bottom type
- `Type::Unknown` - Unknown/inferred type
- `Type::Any` - Any type (gradual typing)
- `Type::IntLiteral(i64)` - Integer literal types
- `Type::BooleanLiteral(bool)` - Boolean literal types
- `Type::StringLiteral` - String literal types
- `Type::LiteralString` - Arbitrary string literals
- `Type::BytesLiteral` - Bytes literal types
- `Type::NominalInstance` - Class instances
- `Type::ProtocolInstance` - Protocol instances
- `Type::ClassLiteral` - Class types themselves
- `Type::Callable` - Callable types
- `Type::FunctionLiteral` - Function types
- `Type::Tuple` - Tuple types (heterogeneous, homogeneous, variable-length)
- `Type::Union` - Union types
- `Type::SubclassOf` - type[X] types
- `Type::EnumLiteral` - Enum member types
- `Type::KnownInstance` - Known standard library instances
- `Type::TypedDict` - TypedDict types
- `Type::Dynamic` - Dynamic types
- `Type::TypeIs` / `Type::TypeGuard` - Type narrowing

**Mitigation:**
```rust
use ty_python_semantic::types::{Type, KnownClass, NominalInstanceType};
use ty_python_semantic::Db;

fn map_ty_to_hir<'db>(db: &'db dyn Db, ty: Type<'db>) -> HirType {
    match ty {
        Type::Never => HirType::Never,
        Type::Unknown | Type::Any => HirType::Unknown,
        
        // Literal types
        Type::IntLiteral(_) => HirType::Int,
        Type::BooleanLiteral(_) => HirType::Bool,
        Type::StringLiteral(_) | Type::LiteralString => HirType::String,
        Type::BytesLiteral(_) => HirType::Bytes,
        
        // Instance types - check against known classes
        Type::NominalInstance(instance) => {
            match instance.known_class(db) {
                Some(KnownClass::Int) => HirType::Int,
                Some(KnownClass::Str) => HirType::String,
                Some(KnownClass::Bool) => HirType::Bool,
                Some(KnownClass::Float) => HirType::Float,
                Some(KnownClass::List) => {
                    // Extract element type from specialization
                    HirType::List(Box::new(HirType::Unknown))
                }
                Some(KnownClass::Dict) => {
                    HirType::Dict(Box::new(HirType::Unknown), Box::new(HirType::Unknown))
                }
                Some(KnownClass::Tuple) => {
                    // Handle tuple via TupleType
                    if let Some(tuple_type) = instance.tuple_spec(db) {
                        map_tuple_to_hir(db, tuple_type)
                    } else {
                        HirType::Tuple(vec![])
                    }
                }
                Some(KnownClass::NoneType) => HirType::None,
                _ => {
                    // Custom class - get qualified name
                    let qname = instance.class(db).qualified_name(db);
                    HirType::Custom(qname.to_string())
                }
            }
        }
        
        // Tuple types
        Type::Tuple(tuple) => map_tuple_to_hir(db, tuple),
        
        // Union types  
        Type::Union(union) => {
            let members: Vec<HirType> = union
                .elements(db)
                .iter()
                .map(|t| map_ty_to_hir(db, *t))
                .collect();
            
            // Check for Optional pattern (Union with None)
            if members.len() == 2 && members.iter().any(|t| matches!(t, HirType::None)) {
                let inner = members.iter()
                    .find(|t| !matches!(t, HirType::None))
                    .cloned()
                    .unwrap_or(HirType::Unknown);
                HirType::Optional(Box::new(inner))
            } else {
                HirType::Union(members)
            }
        }
        
        // Callable types
        Type::Callable(callable) => {
            let sig = callable.signature(db);
            let params: Vec<HirType> = sig.parameters()
                .iter()
                .map(|p| map_ty_to_hir(db, p.annotated_type()))
                .collect();
            let ret = sig.return_ty()
                .map(|t| map_ty_to_hir(db, t))
                .unwrap_or(HirType::None);
            HirType::Function {
                params,
                ret: Box::new(ret),
            }
        }
        
        // Subclass types (type[X])
        Type::SubclassOf(subclass) => {
            if let Some(class) = subclass.into_class(db) {
                HirType::Type(Box::new(map_class_to_hir(db, class)))
            } else {
                HirType::Unknown
            }
        }
        
        _ => HirType::Unknown,
    }
}

fn map_tuple_to_hir<'db>(db: &'db dyn Db, tuple: TupleType<'db>) -> HirType {
    if let Some(elements) = tuple.elements(db) {
        HirType::Tuple(elements.iter().map(|t| map_ty_to_hir(db, *t)).collect())
    } else {
        HirType::Tuple(vec![])
    }
}
```

### Challenge 4: Incremental vs Batch

**Issue:** ty is optimized for incremental IDE usage; Depyler does batch transpilation.

**Mitigation:**
- For CLI: Use batch mode, dispose database after transpilation
- For future LSP: Leverage ty's incremental capabilities

### Challenge 5: KnownClass and KnownModule Enums

**Issue:** ty uses `KnownClass` and `KnownModule` enums to identify standard library types. These need to be mapped to Depyler's HIR types.

**KnownClass variants (from module_resolver/module.rs):**
```rust
pub enum KnownClass {
    // Builtins
    Bool, Int, Float, Complex, Str, Bytes, ByteArray,
    List, Tuple, Set, FrozenSet, Dict,
    Slice, Range, Type, Object, NoneType,
    
    // Typing module
    TypeVar, ParamSpec, TypeVarTuple,
    TypeAliasType, Generic, Protocol,
    
    // Collections
    Deque, DefaultDict, OrderedDict, Counter, ChainMap,
    
    // Contextlib
    AbstractContextManager, AbstractAsyncContextManager,
    
    // IO
    IOBase, TextIOBase, BufferedIOBase, RawIOBase,
    
    // Special
    Property, Classmethod, Staticmethod,
    Super, ModuleType, FunctionType,
    
    // Enum
    Enum, Flag, IntEnum, IntFlag, StrEnum,
    
    // ... and many more
}
```

**KnownModule variants:**
```rust
pub enum KnownModule {
    Builtins, Typing, TypingExtensions,
    Types, Sys, Os, Pathlib,
    Abc, Contextlib, Dataclasses,
    Collections, Enum, Inspect,
    // ... etc
}
```

**Mitigation:**
```rust
fn map_known_class_to_hir(class: KnownClass) -> Option<HirType> {
    match class {
        KnownClass::Int => Some(HirType::Int),
        KnownClass::Float => Some(HirType::Float),
        KnownClass::Str => Some(HirType::String),
        KnownClass::Bool => Some(HirType::Bool),
        KnownClass::Bytes => Some(HirType::Bytes),
        KnownClass::List => Some(HirType::List(Box::new(HirType::Unknown))),
        KnownClass::Dict => Some(HirType::Dict(
            Box::new(HirType::Unknown), 
            Box::new(HirType::Unknown)
        )),
        KnownClass::Set => Some(HirType::Set(Box::new(HirType::Unknown))),
        KnownClass::Tuple => Some(HirType::Tuple(vec![])),
        KnownClass::NoneType => Some(HirType::None),
        KnownClass::Object => Some(HirType::Object),
        _ => None, // Custom handling needed
    }
}
```

## Benefits of Integration

### 1. Performance
- 10-100x faster type checking (benchmarked against mypy/Pyright)
- Fine-grained incrementality for future IDE integration

### 2. Correctness
- Better adherence to Python typing spec
- Comprehensive diagnostics with context
- Proper handling of edge cases

### 3. Maintenance
- Leverage Astral's ongoing development
- Benefit from community contributions
- Reduced internal maintenance burden

### 4. Features
- First-class intersection types
- Advanced type narrowing
- Protocol support
- Reachability analysis

## Implementation Timeline

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1-2 | Dependency Setup | Parser migration, initial ty integration |
| 3-4 | Core Integration | `TyTypeInferencer`, pipeline changes |
| 5-6 | Advanced Features | Generics, protocols, diagnostics |
| 7-8 | Migration | Feature flags, legacy deprecation |
| 9-10 | Validation | Testing, benchmarking, documentation |

## Risks and Contingencies

| Risk | Likelihood | Impact | Contingency |
|------|------------|--------|-------------|
| ty API changes | Medium | High | Pin to specific commit |
| crates.io unavailable | High (current) | Medium | Git dependency |
| Performance regression | Low | Medium | Benchmark gates |
| Feature gaps | Low | Low | Hybrid approach |
| Rust version incompatibility | High | High | Upgrade Depyler's MSRV |
| Salsa learning curve | Medium | Medium | Reference ty_test examples |

### Critical: Rust Version Requirement

**Depyler currently uses:**
- `edition = "2024"`
- `rust-version = "1.83"`

**Ruff/ty requires:**
- `edition = "2024"`
- `rust-version = "1.90"`

**Action Required:** Upgrade Depyler's Rust toolchain before integration:
```toml
# Cargo.toml
[workspace.package]
edition = "2024"
rust-version = "1.90"
```

## Conclusion

Integrating ty into Depyler represents a significant upgrade to our type inference capabilities. The primary blockers are:

1. **ty crates not on crates.io** - Use git dependencies
2. **Database pattern complexity** - Implement minimal `Db` trait
3. **Type mapping complexity** - Incremental mapping implementation

The benefits (performance, correctness, maintainability) significantly outweigh the integration costs. Recommended approach: start with Phase 1 to validate feasibility before committing to full migration.

## Appendix A: API Surface Comparison

### Current Depyler Type API

```rust
// Current
let provider = TypeHintProvider::new();
provider.analyze(&hir_function)?;
let hints = provider.get_parameter_hints("func")?;
```

### Proposed ty-backed API

```rust
// Proposed
let inferencer = TyTypeInferencer::new();
let types = inferencer.infer_function("func")?;
// types.params, types.return_type, types.generics
```

## Appendix B: ty Crate Structure

### Complete ty_python_semantic Structure

```
ty_python_semantic/src/
├── lib.rs                      # Public exports, Db trait definition
├── db.rs                       # Database trait (Db) definition
├── module_name.rs              # ModuleName, ModuleNameResolutionError
├── module_resolver/            # Module resolution system
│   ├── mod.rs
│   ├── module.rs              # Module, KnownModule enum
│   └── resolver.rs            # resolve_module, ModuleResolveMode
├── program.rs                  # Program settings, PythonVersion
├── python_platform.rs          # PythonPlatform enum
├── semantic_model.rs           # SemanticModel - MAIN API FOR QUERIES
├── semantic_index/             # Semantic indexing
│   ├── mod.rs
│   ├── builder.rs
│   └── definition.rs          # Definition types
├── place.rs                    # Symbol/place resolution
├── lint.rs                     # Lint infrastructure
├── suppression.rs              # Comment suppressions
├── types/                      # THE TYPE SYSTEM
│   ├── mod.rs                 # Type enum definition
│   ├── infer.rs               # Type inference (infer_scope_types, infer_definition_types)
│   ├── display.rs             # Type display/formatting
│   ├── diagnostic.rs          # Type diagnostics
│   ├── class.rs               # ClassType, ClassLiteral
│   ├── class_base.rs          # ClassBase enum
│   ├── instance.rs            # NominalInstanceType, ProtocolInstanceType
│   ├── function.rs            # FunctionType, KnownFunction
│   ├── signatures.rs          # Signature, Parameters, Parameter
│   ├── generics.rs            # GenericContext, BoundTypeVarInstance
│   ├── tuple.rs               # TupleType, TupleSpec
│   ├── enums.rs               # EnumLiteralType, enum_metadata
│   ├── subclass_of.rs         # SubclassOfType
│   ├── newtype.rs             # NewType handling
│   ├── protocol_class.rs      # ProtocolClass
│   ├── special_form.rs        # SpecialFormType (typing module forms)
│   ├── member.rs              # Member lookup
│   ├── context.rs             # InferContext, TypeContext
│   ├── definition.rs          # TypeDefinition
│   ├── cyclic.rs              # Cycle detection
│   └── property_tests/        # Property-based tests with type generation
│       └── type_generation.rs # Ty enum for test type generation
└── ...
```

### Key Public APIs (from lib.rs exports)

```rust
// Main database trait
pub use db::Db;

// Module resolution
pub use module_name::{ModuleName, ModuleNameResolutionError};
pub use module_resolver::{
    KnownModule, Module, SearchPath, SearchPathValidationError, SearchPaths,
    all_modules, list_modules, resolve_module, resolve_module_confident,
    resolve_real_module, resolve_real_module_confident,
    resolve_real_shadowable_module, system_module_search_paths,
};

// Program configuration
pub use program::{
    MisconfigurationMode, Program, ProgramSettings,
    PythonVersionFileSource, PythonVersionSource,
    PythonVersionWithSource, SearchPathSettings,
};

// Platform
pub use python_platform::PythonPlatform;

// Semantic model for querying types
pub use semantic_model::{
    Completion, Definition, HasDefinition, HasType,
    ResolvedModule, SemanticModel,
};
```

### SemanticModel API (from semantic_model.rs)

```rust
impl<'db> SemanticModel<'db> {
    /// Create a new semantic model for a file
    pub fn new(db: &'db dyn Db, file: File) -> Self;
    
    /// Get the database
    pub fn db(&self) -> &'db dyn Db;
    
    /// Get the file being analyzed
    pub fn file(&self) -> File;
    
    /// Resolve a module import
    pub fn resolve_module(&self, module: Option<&str>, level: u32) -> Option<Module<'db>>;
    
    /// Resolve a module to its Type
    pub fn resolve_module_type(&self, module: Option<&str>, level: u32) -> Option<Type<'db>>;
}

/// Trait for AST nodes that have an inferred type
pub trait HasType {
    fn inferred_type<'db>(&self, model: &SemanticModel<'db>) -> Option<Type<'db>>;
}

/// Trait for AST nodes that have a definition
pub trait HasDefinition {
    fn definition<'db>(&self, model: &SemanticModel<'db>) -> Definition<'db>;
}
```

### Type Inference Entry Points (from types/infer.rs)

```rust
/// Infer all types for a scope (file/class/function body)
#[salsa::tracked]
pub(crate) fn infer_scope_types<'db>(
    db: &'db dyn Db, 
    scope: ScopeId<'db>
) -> ScopeInference<'db>;

/// Infer types for a single definition
#[salsa::tracked]
pub(crate) fn infer_definition_types<'db>(
    db: &'db dyn Db,
    definition: Definition<'db>,
) -> DefinitionInference<'db>;

impl<'db> DefinitionInference<'db> {
    /// Get the type of an expression within this definition
    pub fn expression_type(&self, expression: impl Into<ExpressionNodeKey>) -> Type<'db>;
    
    /// Get the type bound to a definition
    pub fn binding_type(&self, definition: Definition<'db>) -> Type<'db>;
    
    /// Get the declared type and qualifiers
    pub fn declaration_type(&self, definition: Definition<'db>) -> TypeAndQualifiers<'db>;
}
```

## Appendix C: Practical Usage Examples

### Example 1: Basic Type Inference

```rust
use ruff_db::files::system_path_to_file;
use ruff_db::parsed::parsed_module;
use ty_python_semantic::{HasType, SemanticModel};

fn infer_function_types(db: &DepylerDb, source: &str) -> Result<()> {
    // Write source to test file
    db.system.write_file(SystemPath::new("/test.py"), source)?;
    
    // Get the file
    let file = system_path_to_file(db, "/test.py")?;
    
    // Create semantic model
    let model = SemanticModel::new(db, file);
    
    // Parse the module to get AST
    let parsed = parsed_module(db, file).load(db);
    
    // Iterate over function definitions
    for stmt in parsed.ast().body() {
        if let ast::Stmt::FunctionDef(func_def) = stmt {
            // Get inferred return type
            if let Some(return_type) = func_def.inferred_type(&model) {
                println!("Function {} returns: {}", 
                    func_def.name, 
                    return_type.display(db));
            }
        }
    }
    
    Ok(())
}
```

### Example 2: Module Type Resolution

```rust
use ty_python_semantic::{resolve_module, SemanticModel};

fn resolve_import(db: &DepylerDb, file: File) -> Option<Type<'_>> {
    let model = SemanticModel::new(db, file);
    
    // Resolve "from typing import List"
    let typing_module = model.resolve_module(Some("typing"), 0)?;
    
    // Get the module type
    let module_type = model.resolve_module_type(Some("typing"), 0)?;
    
    Some(module_type)
}
```

### Example 3: Full Integration Pattern

```rust
use ruff_db::Db as SourceDb;
use ruff_db::files::{File, Files, system_path_to_file};
use ruff_db::system::{MemoryFileSystem, System, SystemPath};
use ruff_db::vendored::VendoredFileSystem;
use ty_python_semantic::{
    Db as SemanticDb, Program, ProgramSettings, SemanticModel,
    SearchPathSettings, PythonVersionWithSource, PythonPlatform,
};
use ty_python_semantic::types::infer::infer_scope_types;
use ty_python_semantic::semantic_index::{global_scope, semantic_index};

/// Complete example of using ty for type inference in Depyler
pub struct TyTypeChecker {
    db: DepylerDb,
}

impl TyTypeChecker {
    pub fn new() -> Self {
        let mut db = DepylerDb::new();
        
        // Initialize Program settings
        let settings = ProgramSettings {
            python_version: PythonVersionWithSource::default(),
            python_platform: PythonPlatform::default(),
            search_paths: SearchPathSettings::default(),
        };
        Program::from_settings(&db, settings).unwrap();
        
        Self { db }
    }
    
    /// Analyze Python source and return inferred types
    pub fn analyze(&mut self, source: &str) -> Result<AnalysisResult> {
        // Write source to virtual file
        let path = SystemPath::new("/src/main.py");
        self.db.system.write_file(path, source)?;
        
        // Get file handle
        let file = system_path_to_file(&self.db, path)?;
        
        // Get the global scope
        let index = semantic_index(&self.db, file);
        let scope = global_scope(&self.db, file);
        
        // Infer all types in the scope
        let inference = infer_scope_types(&self.db, scope);
        
        // Create semantic model for additional queries
        let model = SemanticModel::new(&self.db, file);
        
        // Extract results
        let mut result = AnalysisResult::default();
        
        // ... process inference results
        
        Ok(result)
    }
}

#[derive(Default)]
pub struct AnalysisResult {
    pub function_types: HashMap<String, HirType>,
    pub variable_types: HashMap<String, HirType>,
    pub diagnostics: Vec<String>,
}
```

### Example 4: Using ty's Known Classes

```rust
use ty_python_semantic::types::{Type, KnownClass};
use ty_python_semantic::module_resolver::KnownModule;
use ty_python_semantic::place::builtins_symbol;

/// Check if a type is a specific known type
fn is_list_type<'db>(db: &'db dyn Db, ty: Type<'db>) -> bool {
    match ty {
        Type::NominalInstance(instance) => {
            instance.known_class(db) == Some(KnownClass::List)
        }
        _ => false,
    }
}

/// Get the builtin 'int' type
fn get_int_type<'db>(db: &'db dyn Db) -> Type<'db> {
    builtins_symbol(db, "int")
        .place
        .expect_type()
        .expect_class_literal()
        .to_non_generic_instance(db)
}
```

## Appendix D: Resources

- [ty Documentation](https://docs.astral.sh/ty/)
- [ty GitHub Repository](https://github.com/astral-sh/ty)
- [Ruff Repository (ty source)](https://github.com/astral-sh/ruff)
- [ty Playground](https://play.ty.dev/)
- [Salsa Documentation](https://salsa-rs.github.io/salsa/)
- [Reference Commit](https://github.com/astral-sh/ruff/tree/fa57253980c317cce7ff1f35691e3d850c0fb58b/crates)
