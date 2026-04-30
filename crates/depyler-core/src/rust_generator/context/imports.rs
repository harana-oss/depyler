//! Import tracking for code generation.
//!
//! Replaces ~37 individual `needs_*: bool` fields on `CodeGenContext` with a
//! single `required_imports: BTreeSet<Import>` so the full dependency set is
//! inspectable in one place and new imports only require one new enum variant
//! instead of a new struct field everywhere.

use super::CodeGenContext;
use std::collections::BTreeSet;

/// Every Rust import (crate or std item) that code generation may need.
///
/// Add a new variant here when a new external dependency is introduced; the
/// compiler will then point to every `match` that needs updating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Import {
    // ---- std collections ----
    HashMap,
    HashSet,
    VecDeque,
    // ---- external collections ----
    FnvHashMap,
    AHashMap,
    SmallVec,
    // ---- std smart-pointers / borrowing ----
    Arc,
    Rc,
    Cow,
    // ---- rand ----
    Rand,
    SmallRng,
    SliceRandom,
    IndexedRandom,
    // ---- serde ----
    SerdeJson,
    // ---- regex / text ----
    Regex,
    UnicodeNormalization,
    // ---- date/time ----
    Chrono,
    // ---- CSV / data formats ----
    Csv,
    // ---- numeric ----
    RustDecimal,
    NumRational,
    Complex,
    // ---- crypto / encoding ----
    Base64,
    Md5,
    Sha2,
    Sha3,
    Blake2,
    Hex,
    Uuid,
    Hmac,
    Crc32,
    UrlEncoding,
    // ---- CLI ----
    Clap,
    // ---- lazy initialisation ----
    LazyStatic,
    // ---- Python exception types emitted as Rust structs ----
    ZeroDivisionError,
    IndexError,
    ValueError,
    ArgumentTypeError,
}

impl<'a> CodeGenContext<'a> {
    // ------------------------------------------------------------------ //
    // Mutation helpers                                                     //
    // ------------------------------------------------------------------ //

    /// Record that the generated code requires `import`.
    ///
    /// Idempotent: calling it multiple times with the same variant is a no-op.
    pub fn require(&mut self, import: Import) {
        self.required_imports.insert(import);
    }

    // ------------------------------------------------------------------ //
    // Query helpers                                                        //
    // ------------------------------------------------------------------ //

    /// Returns `true` if `import` has been required.
    pub fn requires(&self, import: Import) -> bool {
        self.required_imports.contains(&import)
    }

    /// Return the full set of required imports (for iteration in codegen).
    pub fn all_required_imports(&self) -> &BTreeSet<Import> {
        &self.required_imports
    }
}
