use crate::test_helpers::transpile;
use depyler_core::DepylerPipeline;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[derive(Debug, Deserialize)]
struct TomlTestFile {
    #[serde(default)]
    metadata: TomlMetadata,
    test: Vec<TomlTest>,
}

#[derive(Debug, Deserialize, Default)]
struct TomlMetadata {
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TomlTest {
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    description: String,
    #[serde(default)]
    #[allow(dead_code)]
    tags: Vec<String>,
    #[serde(default)]
    compile_check: bool,
    python: TomlPython,
    assertions: TomlAssertions,
}

#[derive(Debug, Deserialize)]
struct TomlPython {
    #[serde(alias = "snippet")]
    code: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct TomlAssertions {
    contains: Vec<String>,
    not_contains: Vec<String>,
    contains_any: Vec<Vec<String>>,
}

impl Default for TomlAssertions {
    fn default() -> Self {
        Self {
            contains: Vec::new(),
            not_contains: Vec::new(),
            contains_any: Vec::new(),
        }
    }
}

fn compile_as_lib(rust_code: &str) -> Result<(), String> {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src directory");

    let cargo_toml = r#"[package]
name = "depyler_toml_test"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"

[dependencies]
lazy_static = "1"
serde_json = "1"
rand = "0.8"
bevy_reflect = "0.15"
clap = { version = "4", features = ["derive"] }
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    let hashmap_import = if rust_code.contains("use std::collections::HashMap") {
        ""
    } else {
        "use std::collections::HashMap;\n"
    };

    let full_code =
        format!("#![allow(dead_code, unused_variables, unused_mut, unused_imports)]\n{hashmap_import}{rust_code}");

    fs::write(src_dir.join("lib.rs"), &full_code).expect("Failed to write lib.rs");

    let output = Command::new("cargo")
        .arg("build")
        .arg("--quiet")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run cargo build");

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn run_toml_test(test: &TomlTest) {
    let rust_code = if test.compile_check {
        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(&test.python.code).unwrap_or_else(|e| {
            panic!(
                "[{}] Transpilation failed:\n{e}\n\nPython source:\n{}",
                test.name, test.python.code
            );
        });

        if let Err(compile_err) = compile_as_lib(&rust_code) {
            panic!(
                "[{}] Rust compilation failed:\n{}\n\nGenerated code:\n{}",
                test.name, compile_err, rust_code
            );
        }
        rust_code
    } else {
        transpile(&test.python.code)
    };

    for pattern in &test.assertions.contains {
        assert!(
            rust_code.contains(pattern),
            "[{}] Expected pattern '{}' not found.\nGenerated:\n{}",
            test.name,
            pattern,
            rust_code
        );
    }

    for pattern in &test.assertions.not_contains {
        assert!(
            !rust_code.contains(pattern),
            "[{}] Pattern '{}' should NOT appear.\nGenerated:\n{}",
            test.name,
            pattern,
            rust_code
        );
    }

    for alternatives in &test.assertions.contains_any {
        let found = alternatives.iter().any(|alt| rust_code.contains(alt));
        assert!(
            found,
            "[{}] Expected one of {:?} but none found.\nGenerated:\n{}",
            test.name, alternatives, rust_code
        );
    }
}

fn load_and_run_toml_tests(toml_path: &Path) {
    let content =
        fs::read_to_string(toml_path).unwrap_or_else(|e| panic!("Failed to read {}: {}", toml_path.display(), e));

    let test_file: TomlTestFile =
        toml::from_str(&content).unwrap_or_else(|e| panic!("Failed to parse {}: {}", toml_path.display(), e));

    for test in &test_file.test {
        run_toml_test(test);
    }
}

#[test]
fn test_basic_types_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/basic-types.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_functions_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/functions.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_classes_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/classes.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_control_flow_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/control-flow.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_string_operations_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/string-operations.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_assignment_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/assignment.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_dictionaries_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/dictionaries.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_lists_arrays_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/lists-arrays.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_slicing_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/slicing.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_comprehensions_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/comprehensions.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_generators_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/generators.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_lambdas_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/lambdas.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_iterators_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/iterators.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_operators_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/operators.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_error_handling_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/error-handling.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_type_inference_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/type-inference.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_ownership_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/ownership.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_mutability_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/mutability.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_optional_types_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/optional-types.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_result_types_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/result-types.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_math_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-math.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_json_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-json.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_os_sys_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-os-sys.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_datetime_time_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-datetime-time.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_collections_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-collections.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_itertools_functools_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-itertools-functools.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_random_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-random.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_regex_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-regex.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_ast_hir_codegen_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/ast-hir-codegen.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_cli_integration_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/cli-integration.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_io_streams_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/io-streams.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_modules_csv_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/modules-csv.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_regressions_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/regressions.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_argparse_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/argparse.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_boundary_values_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/boundary-values.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_collections_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/collections.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_crypto_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/crypto.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_custom_attributes_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/custom-attributes.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_exceptions_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/exceptions.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_file_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/file.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_lifetimes_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/lifetimes.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_loop_for_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/loop-for.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_numeric_csv_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/numeric-csv.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_pattern_matching_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/pattern-matching.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_stdlib_misc_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/stdlib-misc.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_return_statements_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/return-statements.toml");
    load_and_run_toml_tests(&toml_path);
}
