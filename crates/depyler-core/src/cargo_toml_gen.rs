//! Cargo.toml generation from CodeGenContext dependencies
//!
//! Automatically generates Cargo.toml with correct dependencies
//! based on the needs_* flags tracked during code generation.

use crate::rust_gen::CodeGenContext;

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

/// Extract dependencies from CodeGenContext needs_* flags
pub fn extract_dependencies(ctx: &CodeGenContext) -> Vec<Dependency> {
    let mut deps = Vec::new();

    // Standard library collections (no external deps needed)
    // HashMap, HashSet, VecDeque are in std::collections

    // External crate mappings
    if ctx.needs_serde_json {
        deps.push(Dependency::new("serde_json", "1.0"));
        deps.push(Dependency::new("serde", "1.0").with_features(vec!["derive".to_string()]));
    }

    if ctx.needs_regex {
        deps.push(Dependency::new("regex", "1.0"));
    }

    if ctx.needs_chrono {
        deps.push(Dependency::new("chrono", "0.4"));
    }

    if ctx.needs_unicode_normalization {
        deps.push(Dependency::new("unicode-normalization", "0.1"));
    }

    if ctx.needs_csv {
        deps.push(Dependency::new("csv", "1.0"));
    }

    if ctx.needs_rust_decimal {
        deps.push(Dependency::new("rust_decimal", "1.0"));
    }

    if ctx.needs_num_rational {
        deps.push(Dependency::new("num-rational", "0.4"));
    }

    if ctx.needs_base64 {
        deps.push(Dependency::new("base64", "0.21"));
    }

    if ctx.needs_md5 {
        deps.push(Dependency::new("md-5", "0.10"));
    }

    if ctx.needs_sha2 {
        deps.push(Dependency::new("sha2", "0.10"));
    }

    if ctx.needs_sha3 {
        deps.push(Dependency::new("sha3", "0.10"));
    }

    if ctx.needs_blake2 {
        deps.push(Dependency::new("blake2", "0.10"));
    }

    if ctx.needs_hex {
        deps.push(Dependency::new("hex", "0.4"));
    }

    if ctx.needs_uuid {
        deps.push(Dependency::new("uuid", "1.0"));
    }

    if ctx.needs_hmac {
        deps.push(Dependency::new("hmac", "0.12"));
    }

    if ctx.needs_crc32 {
        deps.push(Dependency::new("crc32fast", "1.3"));
    }

    if ctx.needs_url_encoding {
        deps.push(Dependency::new("percent-encoding", "2.3"));
    }

    if ctx.needs_rand {
        deps.push(Dependency::new("rand", "0.9"));
    }

    if ctx.needs_clap {
        deps.push(Dependency::new("clap", "4.5").with_features(vec!["derive".to_string()]));
    }

    if ctx.needs_lazy_static {
        deps.push(Dependency::new("lazy_static", "1.4"));
    }

    if ctx.needs_smallvec {
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
