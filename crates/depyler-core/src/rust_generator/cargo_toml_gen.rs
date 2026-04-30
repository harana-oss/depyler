//! Cargo.toml generation from CodeGenContext dependencies
//!
//! Automatically generates Cargo.toml with correct dependencies
//! based on the needs_* flags tracked during code generation.

use crate::rust_generator::CodeGenContext;

/// Dependency specification with version and optional features
#[derive(Debug, Clone)]
pub struct Dependency {
    pub crate_name: String,
    pub version: String,
    pub features: Vec<String>,
}

impl Dependency {
    pub fn new(crate_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            crate_name: crate_name.into(),
            version: version.into(),
            features: vec![],
        }
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    /// Generate TOML dependency line
    pub fn to_toml_line(&self) -> String {
        if self.features.is_empty() {
            format!("{} = \"{}\"", self.crate_name, self.version)
        } else {
            let features_str = self
                .features
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{} = {{ version = \"{}\", features = [{}] }}",
                self.crate_name, self.version, features_str
            )
        }
    }
}

/// Extract dependencies from CodeGenContext required_imports set
pub fn extract_dependencies(ctx: &CodeGenContext) -> Vec<Dependency> {
    use super::context::Import;

    let mut deps = Vec::new();

    // Standard library collections (no external deps needed)
    // HashMap, HashSet, VecDeque are in std::collections

    // External crate mappings
    if ctx.requires(Import::SerdeJson) {
        deps.push(Dependency::new("serde_json", "1.0"));
        deps.push(Dependency::new("serde", "1.0").with_features(vec!["derive".to_string()]));
    }

    if ctx.requires(Import::Regex) {
        deps.push(Dependency::new("regex", "1.0"));
    }

    if ctx.requires(Import::Chrono) {
        deps.push(Dependency::new("chrono", "0.4"));
    }

    if ctx.requires(Import::UnicodeNormalization) {
        deps.push(Dependency::new("unicode-normalization", "0.1"));
    }

    if ctx.requires(Import::Csv) {
        deps.push(Dependency::new("csv", "1.0"));
    }

    if ctx.requires(Import::RustDecimal) {
        deps.push(Dependency::new("rust_decimal", "1.0"));
    }

    if ctx.requires(Import::NumRational) {
        deps.push(Dependency::new("num-rational", "0.4"));
    }

    if ctx.requires(Import::Base64) {
        deps.push(Dependency::new("base64", "0.21"));
    }

    if ctx.requires(Import::Md5) {
        deps.push(Dependency::new("md-5", "0.10"));
    }

    if ctx.requires(Import::Sha2) {
        deps.push(Dependency::new("sha2", "0.10"));
    }

    if ctx.requires(Import::Sha3) {
        deps.push(Dependency::new("sha3", "0.10"));
    }

    if ctx.requires(Import::Blake2) {
        deps.push(Dependency::new("blake2", "0.10"));
    }

    if ctx.requires(Import::Hex) {
        deps.push(Dependency::new("hex", "0.4"));
    }

    if ctx.requires(Import::Uuid) {
        deps.push(Dependency::new("uuid", "1.0"));
    }

    if ctx.requires(Import::Hmac) {
        deps.push(Dependency::new("hmac", "0.12"));
    }

    if ctx.requires(Import::Crc32) {
        deps.push(Dependency::new("crc32fast", "1.3"));
    }

    if ctx.requires(Import::UrlEncoding) {
        deps.push(Dependency::new("percent-encoding", "2.3"));
    }

    if ctx.requires(Import::Rand) {
        deps.push(Dependency::new("rand", "0.9"));
    }

    if ctx.requires(Import::Clap) {
        deps.push(Dependency::new("clap", "4.5").with_features(vec!["derive".to_string()]));
    }

    if ctx.requires(Import::LazyStatic) {
        deps.push(Dependency::new("lazy_static", "1.4"));
    }

    if ctx.requires(Import::SmallVec) {
        deps.push(Dependency::new("smallvec", "1.0"));
    }

    deps.push(Dependency::new("bevy_reflect", "0.17"));
    deps.push(Dependency::new("log", "0.4"));

    deps
}

/// Generate complete Cargo.toml content
///
/// are complete and can be built by Cargo without manual editing.
pub fn generate_cargo_toml(
    package_name: &str,
    source_file_path: &str,
    dependencies: &[Dependency],
) -> String {
    let mut toml = String::new();

    // Package section
    toml.push_str("[package]\n");
    toml.push_str(&format!("name = \"{}\"\n", package_name));
    toml.push_str("version = \"0.1.0\"\n");
    toml.push_str("edition = \"2021\"\n");
    toml.push('\n');

    // Binary section
    toml.push_str("[[bin]]\n");
    toml.push_str(&format!("name = \"{}\"\n", package_name));
    toml.push_str(&format!("path = \"{}\"\n", source_file_path));
    toml.push('\n');

    // Dependencies section
    if !dependencies.is_empty() {
        toml.push_str("[dependencies]\n");
        for dep in dependencies {
            toml.push_str(&dep.to_toml_line());
            toml.push('\n');
        }
    }

    toml
}
