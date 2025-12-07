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
fn test_01_basic_types_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/01-basic-types.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_02_functions_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/02-functions.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_03_classes_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/03-classes.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_04_control_flow_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/04-control-flow.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_05_string_operations_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/05-string-operations.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_06_assignment_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/06-assignment.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_07_dictionaries_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/07-dictionaries.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_08_lists_arrays_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/08-lists-arrays.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_09_slicing_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/09-slicing.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_10_comprehensions_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/10-comprehensions.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_11_generators_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/11-generators.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_12_lambdas_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/12-lambdas.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_13_iterators_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/13-iterators.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_14_operators_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/14-operators.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_15_error_handling_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/15-error-handling.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_16_type_inference_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/16-type-inference.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_17_ownership_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/17-ownership.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_18_mutability_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/18-mutability.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_19_optional_types_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/19-optional-types.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_20_result_types_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/20-result-types.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_21_math_module_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/21-math-module.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_22_json_module_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/22-json-module.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_23_os_sys_modules_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/23-os-sys-modules.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_24_datetime_time_modules_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/24-datetime-time-modules.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_25_collections_module_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/25-collections-module.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_26_itertools_functools_modules_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/26-itertools-functools-modules.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_27_random_module_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/27-random-module.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_28_regex_module_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/28-regex-module.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_29_ast_hir_codegen_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/29-ast-hir-codegen.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_30_cli_integration_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/30-cli-integration.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_32_io_streams_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/32-io-streams.toml");
    load_and_run_toml_tests(&toml_path);
}

#[test]
fn test_33_csv_module_toml() {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml/33-csv-module.toml");
    load_and_run_toml_tests(&toml_path);
}
