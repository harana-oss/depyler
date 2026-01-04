//! TOML-based test runner for Depyler transpilation tests

use anyhow::{Context, Result};
use colored::Colorize;
use depyler_core::DepylerPipeline;
use rayon::prelude::*;
use serde::Deserialize;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tempfile::TempDir;

/// Test file structure from TOML
#[derive(Debug, Deserialize)]
pub struct TomlTestFile {
    pub metadata: Option<TestMetadata>,
    #[serde(default)]
    pub test: Vec<TomlTest>,
}

#[derive(Debug, Deserialize)]
pub struct TestMetadata {
    pub name: Option<String>,
    pub category: Option<String>,
}

/// Skip value can be boolean or array of strings
#[derive(Debug, Clone, Default)]
pub struct SkipValue(pub bool);

impl<'de> Deserialize<'de> for SkipValue {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct SkipVisitor;

        impl<'de> Visitor<'de> for SkipVisitor {
            type Value = SkipValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("boolean or array of strings")
            }

            fn visit_bool<E>(self, value: bool) -> std::result::Result<SkipValue, E>
            where
                E: de::Error,
            {
                Ok(SkipValue(value))
            }

            fn visit_seq<A>(self, mut _seq: A) -> std::result::Result<SkipValue, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Ok(SkipValue(true))
            }
        }

        deserializer.deserialize_any(SkipVisitor)
    }
}

/// Individual test case
#[derive(Debug, Clone, Deserialize)]
pub struct TomlTest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub python: String,
    pub rust: Option<String>,
    pub expected_output: Option<String>,
    #[serde(default)]
    pub skip: SkipValue,
    pub skip_reason: Option<String>,
    #[serde(default)]
    pub assertions: TestAssertions,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TestAssertions {
    #[serde(default)]
    pub contains: Vec<String>,
    #[serde(default)]
    pub contains_any: Vec<Vec<String>>,
    #[serde(default)]
    pub not_contains: Vec<String>,
}

/// Test result
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub file: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration_ms: u128,
    pub diff: Option<String>,
}

/// Generate a colored diff between expected and actual output
fn generate_colored_diff(expected: &str, actual: &str) -> String {
    let diff = TextDiff::from_lines(expected, actual);
    let mut output = String::new();

    output.push_str(&format!("{}\n", "─".repeat(60).dimmed()));

    for change in diff.iter_all_changes() {
        let line = change.value();
        let line_content = line.strip_suffix('\n').unwrap_or(line);
        match change.tag() {
            ChangeTag::Delete => {
                output.push_str(&format!("{}\n", format!("− {}", line_content).red()));
            }
            ChangeTag::Insert => {
                output.push_str(&format!("{}\n", format!("+ {}", line_content).green()));
            }
            ChangeTag::Equal => {
                output.push_str(&format!("  {}\n", line_content.dimmed()));
            }
        }
    }

    output.push_str(&format!("{}", "─".repeat(60).dimmed()));
    output
}

/// Format Rust code using rustfmt
///
/// Returns the formatted code if rustfmt succeeds, otherwise returns the original code
fn format_with_rustfmt(code: &str) -> String {
    let mut child = match Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return code.to_string(), // rustfmt not available, return as-is
    };

    // Write to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(code.as_bytes());
        // stdin is dropped here
    }

    match child.wait_with_output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).to_string()
        }
        _ => code.to_string(), // rustfmt failed, return as-is
    }
}

/// Parsed test file with its path
#[allow(dead_code)]
struct ParsedTestFile {
    path: PathBuf,
    file_name: String,
    category: String,
    tests: Vec<TomlTest>,
}

/// Run all TOML tests in a directory or single file
pub fn run_toml_tests(
    path: Option<PathBuf>,
    filter: Option<String>,
    verbose: bool,
    parallel: bool,
    compile: bool,
) -> Result<()> {
    let test_path = path.unwrap_or_else(|| PathBuf::from("tests/toml"));

    if !test_path.exists() {
        anyhow::bail!("Test path not found: {}", test_path.display());
    }

    let start = Instant::now();

    let toml_files = if test_path.is_file() {
        vec![test_path]
    } else {
        collect_toml_files(&test_path)?
    };

    let mode = if parallel { "parallel" } else { "sequential" };
    let compile_mode = if compile { ", with compilation" } else { "" };
    println!(
        "{}",
        format!(
            "Running tests from {} TOML file(s) ({} mode{})...\n",
            toml_files.len(),
            mode,
            compile_mode
        )
        .cyan()
    );

    let (results, parse_errors) = if parallel {
        run_tests_parallel(&toml_files, filter.as_deref(), compile)
    } else {
        run_tests_sequential(&toml_files, filter.as_deref(), verbose, compile)
    };

    // Process results
    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;
    let mut skipped_tests = 0;
    let mut failed_details: Vec<(String, String, String, Option<String>)> = Vec::new();

    for result in &results {
        total_tests += 1;
        if result.passed {
            passed_tests += 1;
            if verbose {
                println!(
                    "  {} {} ({:.2}ms)",
                    "✓".green(),
                    result.name,
                    result.duration_ms
                );
            }
        } else if result
            .error
            .as_ref()
            .map_or(false, |e| e.contains("skipped"))
        {
            skipped_tests += 1;
            if verbose {
                println!("  {} {} (skipped)", "○".yellow(), result.name);
            }
        } else {
            failed_tests += 1;
            let err = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            println!(
                "  {} {} ({:.2}ms)",
                "✗".red(),
                result.name.red(),
                result.duration_ms
            );
            if verbose {
                println!("    {}", err.dimmed());
            }
            failed_details.push((
                result.file.clone(),
                result.name.clone(),
                err,
                result.diff.clone(),
            ));
        }
    }

    let duration = start.elapsed();
    println!();
    println!("{}", "═".repeat(60));
    println!(
        "Tests: {} total, {} passed, {} failed, {} skipped",
        total_tests,
        passed_tests.to_string().green(),
        if failed_tests > 0 {
            failed_tests.to_string().red()
        } else {
            "0".normal()
        },
        skipped_tests.to_string().yellow()
    );
    if !parse_errors.is_empty() {
        println!(
            "Files: {} parse error(s)",
            parse_errors.len().to_string().yellow()
        );
    }
    println!("Time:  {:.2}s", duration.as_secs_f64());
    println!("{}", "═".repeat(60));

    if !parse_errors.is_empty() {
        println!("\n{}", "Parse Errors:".yellow().bold());
        for (file, err) in &parse_errors {
            println!("\n  {}", file.yellow());
            for line in err.lines().take(5) {
                println!("    {}", line.dimmed());
            }
        }
    }

    if !failed_details.is_empty() {
        println!("\n{}", "Failed Tests:".red().bold());
        for (file, name, err, diff) in &failed_details {
            println!("\n  {} :: {}", file.yellow(), name.red());
            for line in err.lines().take(10) {
                println!("    {}", line.dimmed());
            }
            if let Some(diff_output) = diff {
                println!();
                print!("{}", diff_output);
            }
        }
        std::process::exit(1);
    }

    Ok(())
}

/// Run tests in parallel using Rayon
fn run_tests_parallel(
    toml_files: &[PathBuf],
    filter: Option<&str>,
    compile: bool,
) -> (Vec<TestResult>, Vec<(String, String)>) {
    let passed = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let total = AtomicUsize::new(0);

    // First, parse all files and collect tests
    let parsed_files: Vec<_> = toml_files
        .iter()
        .filter_map(|path| {
            let content = fs::read_to_string(path).ok()?;
            let test_file: TomlTestFile = toml::from_str(&content).ok()?;
            let file_name = path.file_name()?.to_string_lossy().to_string();
            let category = test_file
                .metadata
                .as_ref()
                .and_then(|m| m.name.clone())
                .unwrap_or_else(|| file_name.clone());
            Some(ParsedTestFile {
                path: path.clone(),
                file_name,
                category,
                tests: test_file.test,
            })
        })
        .collect();

    // Collect parse errors
    let parse_errors: Vec<_> = toml_files
        .iter()
        .filter_map(|path| {
            let file_name = path.file_name()?.to_string_lossy().to_string();
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => return Some((file_name, e.to_string())),
            };
            match toml::from_str::<TomlTestFile>(&content) {
                Ok(_) => None,
                Err(e) => Some((file_name, e.to_string())),
            }
        })
        .collect();

    // Flatten all tests with their file info
    let all_tests: Vec<_> = parsed_files
        .iter()
        .flat_map(|pf| {
            pf.tests.iter().filter_map(move |test| {
                if test.skip.0 || test.skip_reason.is_some() {
                    return None;
                }
                // Apply filter
                if let Some(filter_str) = filter {
                    if !test.name.contains(filter_str) {
                        return None;
                    }
                }
                Some((pf.file_name.clone(), test.clone()))
            })
        })
        .collect();

    // Run tests in parallel
    let results: Vec<TestResult> = all_tests
        .into_par_iter()
        .map(|(file_name, test)| {
            let pipeline = DepylerPipeline::new();
            let result = run_single_test(&pipeline, &test, &file_name, compile);

            total.fetch_add(1, Ordering::Relaxed);
            if result.passed {
                passed.fetch_add(1, Ordering::Relaxed);
            } else {
                failed.fetch_add(1, Ordering::Relaxed);
            }

            // Print progress indicator (dots for passed, X for failed)
            if result.passed {
                eprint!(".");
            } else {
                eprint!("x");
            }

            result
        })
        .collect();

    eprintln!(); // New line after progress dots

    (results, parse_errors)
}

/// Run tests sequentially (original behavior)
fn run_tests_sequential(
    toml_files: &[PathBuf],
    filter: Option<&str>,
    verbose: bool,
    compile: bool,
) -> (Vec<TestResult>, Vec<(String, String)>) {
    let mut results = Vec::new();
    let mut parse_errors = Vec::new();

    for toml_path in toml_files {
        match run_tests_from_file(toml_path, filter, verbose, compile) {
            Ok(file_results) => {
                results.extend(file_results);
            }
            Err(e) => {
                let file_name = toml_path.file_name().unwrap().to_string_lossy().to_string();
                println!("  {} {} (parse error)", "⚠".yellow(), file_name.yellow());
                parse_errors.push((file_name, e.to_string()));
            }
        }
    }

    (results, parse_errors)
}

/// Collect all TOML files from directory
fn collect_toml_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir).context("Failed to read test directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "toml") {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

/// Run tests from a single TOML file
fn run_tests_from_file(
    path: &Path,
    filter: Option<&str>,
    verbose: bool,
    compile: bool,
) -> Result<Vec<TestResult>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let test_file: TomlTestFile = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))?;

    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    if verbose {
        let category = test_file
            .metadata
            .as_ref()
            .and_then(|m| m.name.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(&file_stem);
        println!("{}", format!("📁 {}", category).blue().bold());
    }

    let mut results = Vec::new();
    let pipeline = DepylerPipeline::new();

    for test in test_file.test {
        if test.skip.0 || test.skip_reason.is_some() {
            results.push(TestResult {
                name: test.name.clone(),
                file: file_name.clone(),
                passed: true,
                error: Some("skipped".to_string()),
                duration_ms: 0,
                diff: None,
            });
            continue;
        }

        // Apply filter if provided
        if let Some(filter_str) = filter {
            if !test.name.contains(filter_str) {
                continue;
            }
        }

        let result = run_single_test(&pipeline, &test, &file_name, compile);
        results.push(result);
    }

    Ok(results)
}

/// Compile Rust code to verify it's valid
fn compile_rust_code(rust_code: &str) -> Result<()> {
    let temp_dir = TempDir::new().context("Failed to create temp directory")?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

    // Write Cargo.toml
    let cargo_toml = r#"[package]
name = "depyler_test"
version = "0.1.0"
edition = "2024"

[dependencies]
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)
        .context("Failed to write Cargo.toml")?;

    // Wrap code in main function if it doesn't have one
    let full_code = if rust_code.contains("fn main()") {
        rust_code.to_string()
    } else {
        format!("#![allow(unused)]\n{}\n\nfn main() {{}}", rust_code)
    };

    fs::write(src_dir.join("main.rs"), &full_code).context("Failed to write main.rs")?;

    // Run cargo build
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .current_dir(temp_dir.path())
        .output()
        .context("Failed to run cargo build")?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Compilation failed:\n{}", stderr)
    }
}

/// Run a single test case
fn run_single_test(
    pipeline: &DepylerPipeline,
    test: &TomlTest,
    file_name: &str,
    compile: bool,
) -> TestResult {
    let start = Instant::now();
    let test_name = test.name.clone();
    let python_code = test.python.clone();
    let assertions = test.assertions.clone();
    let expected_rust = test.rust.clone();
    let file = file_name.to_string();
    let expects_failure = expected_rust
        .as_ref()
        .map(|r| r.trim().starts_with("// Transpilation failed"))
        .unwrap_or(false);

    // Catch panics during transpilation
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pipeline.transpile(&python_code)
    }));
    let duration_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(rust_code)) => {
            // If test expects failure but succeeded, that's a failure
            if expects_failure {
                return TestResult {
                    name: test_name,
                    file,
                    passed: false,
                    error: Some(format!(
                        "Expected transpilation to fail, but it succeeded:\n{}",
                        rust_code
                    )),
                    duration_ms,
                    diff: None,
                };
            }

            // Compare with expected rust output if provided
            if let Some(expected) = &expected_rust {
                // Format both expected and actual code with rustfmt for fair comparison
                let expected_formatted = format_with_rustfmt(expected.trim());
                let actual_formatted = format_with_rustfmt(rust_code.trim());

                if expected_formatted.trim() != actual_formatted.trim() {
                    let diff = generate_colored_diff(&expected_formatted, &actual_formatted);
                    return TestResult {
                        name: test_name,
                        file,
                        passed: false,
                        error: Some("Output mismatch".to_string()),
                        duration_ms,
                        diff: Some(diff),
                    };
                }
            }

            // Check assertions
            if let Err(e) = check_assertions(&rust_code, &assertions) {
                return TestResult {
                    name: test_name,
                    file,
                    passed: false,
                    error: Some(format!(
                        "Assertion failed: {}\n\nGenerated:\n{}",
                        e, rust_code
                    )),
                    duration_ms,
                    diff: None,
                };
            }

            // Optionally compile the generated code
            if compile {
                if let Err(e) = compile_rust_code(&rust_code) {
                    return TestResult {
                        name: test_name,
                        file,
                        passed: false,
                        error: Some(format!(
                            "Compilation failed: {}\n\nGenerated:\n{}",
                            e, rust_code
                        )),
                        duration_ms,
                        diff: None,
                    };
                }
            }

            TestResult {
                name: test_name,
                file,
                passed: true,
                error: None,
                duration_ms,
                diff: None,
            }
        }
        Ok(Err(e)) => {
            // If test expects failure and it failed, that's a pass
            if expects_failure {
                return TestResult {
                    name: test_name,
                    file,
                    passed: true,
                    error: None,
                    duration_ms,
                    diff: None,
                };
            }
            TestResult {
                name: test_name,
                file,
                passed: false,
                error: Some(format!("Transpilation failed: {}", e)),
                duration_ms,
                diff: None,
            }
        }
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };
            // Panics are still failures even for expected failure tests
            TestResult {
                name: test_name,
                file,
                passed: false,
                error: Some(format!("PANIC: {}", panic_msg)),
                duration_ms,
                diff: None,
            }
        }
    }
}

/// Check test assertions against generated Rust code
fn check_assertions(rust_code: &str, assertions: &TestAssertions) -> Result<()> {
    // Check contains
    for pattern in &assertions.contains {
        if !rust_code.contains(pattern) {
            anyhow::bail!("Expected to contain: '{}'", pattern);
        }
    }

    // Check contains_any (at least one from each group must match)
    for group in &assertions.contains_any {
        let found = group.iter().any(|pattern| rust_code.contains(pattern));
        if !found {
            anyhow::bail!("Expected to contain one of: {:?}", group);
        }
    }

    // Check not_contains
    for pattern in &assertions.not_contains {
        if rust_code.contains(pattern) {
            anyhow::bail!("Expected NOT to contain: '{}'", pattern);
        }
    }

    Ok(())
}

/// Arguments for test command
pub struct TestArgs {
    pub path: Option<PathBuf>,
    pub filter: Option<String>,
    pub verbose: bool,
    pub parallel: bool,
    pub compile: bool,
}

/// Handle the test command
pub fn handle_test_command(args: TestArgs) -> Result<()> {
    run_toml_tests(
        args.path,
        args.filter,
        args.verbose,
        args.parallel,
        args.compile,
    )
}
