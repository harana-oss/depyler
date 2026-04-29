use depyler_core::{hir::HirModule, DepylerPipeline};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Result from transpiling and compiling Python code to Rust.
#[derive(Debug)]
pub struct TranspileCompileResult {
    pub rust_code: String,
    pub compilation_success: bool,
    pub compilation_stderr: String,
}

/// Transpiles Python source code to Rust. Panics on failure.
pub fn transpile(python_source: &str) -> String {
    let pipeline = DepylerPipeline::new();
    pipeline.transpile(python_source).unwrap_or_else(|e| {
        panic!("Transpilation failed:\n{e}\n\nPython source:\n{python_source}");
    })
}

/// Parses Python source to HIR. Panics on failure.
pub fn parse_to_hir(python_source: &str) -> HirModule {
    let pipeline = DepylerPipeline::new();
    pipeline.parse_to_hir(python_source).unwrap_or_else(|e| {
        panic!("Failed to parse to HIR:\n{e}\n\nPython source:\n{python_source}");
    })
}

/// Transpiles Python source code and verifies expected Rust patterns are present,
/// then compiles the output with rustc.
 fn transpile_and_compile(
    python_source: &str,
    expected_patterns: &[&str],
) -> TranspileCompileResult {
    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python_source);

    let rust_code = result.unwrap_or_else(|e| {
        panic!("Transpilation failed:\n{e}\n\nPython source:\n{python_source}");
    });

    for pattern in expected_patterns {
        assert!(
            rust_code.contains(pattern),
            "Expected pattern not found in generated Rust code.\nPattern: {pattern}\n\nGenerated code:\n{rust_code}"
        );
    }

    let compile_result = compile_rust_code(&rust_code);

    assert!(
        compile_result.compilation_success,
        "Rust compilation failed:\n{}\n\nGenerated code:\n{rust_code}",
        compile_result.compilation_stderr
    );

    compile_result
}

/// Transpiles Python source, verifies expected patterns, and compiles the output.
pub fn transpile_and_check(python_source: &str, expected_patterns: &[&str]) -> String {
    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python_source);

    let rust_code = result.unwrap_or_else(|e| {
        panic!("Transpilation failed:\n{e}\n\nPython source:\n{python_source}");
    });

    for pattern in expected_patterns {
        assert!(
            rust_code.contains(pattern),
            "Expected pattern not found in generated Rust code.\nPattern: {pattern}\n\nGenerated code:\n{rust_code}"
        );
    }

    let compile_result = compile_rust_code(&rust_code);
    assert!(
        compile_result.compilation_success,
        "Rust compilation failed:\n{}\n\nGenerated code:\n{rust_code}",
        compile_result.compilation_stderr
    );

    rust_code
}

/// Transpiles Python source, verifies patterns are absent, and compiles the output.
pub fn transpile_check_absent(python_source: &str, absent_patterns: &[&str]) -> String {
    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python_source);

    let rust_code = result.unwrap_or_else(|e| {
        panic!("Transpilation failed:\n{e}\n\nPython source:\n{python_source}");
    });

    for pattern in absent_patterns {
        assert!(
            !rust_code.contains(pattern),
            "Pattern should NOT appear in generated Rust code.\nPattern: {pattern}\n\nGenerated code:\n{rust_code}"
        );
    }

    let compile_result = compile_rust_code(&rust_code);
    assert!(
        compile_result.compilation_success,
        "Rust compilation failed:\n{}\n\nGenerated code:\n{rust_code}",
        compile_result.compilation_stderr
    );

    rust_code
}

/// Compiles Rust code using cargo and returns the result.
pub fn compile_rust_code(rust_code: &str) -> TranspileCompileResult {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src directory");

    let cargo_toml = r#"[package]
name = "depyler_test"
version = "0.1.0"
edition = "2024"

[dependencies]
lazy_static = "1"
serde_json = "1"
rand = "0.8"
bevy_reflect = "0.15"
clap = { version = "4", features = ["derive"] }
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Only add HashMap import if not already present
    let hashmap_import = if rust_code.contains("use std::collections::HashMap") {
        ""
    } else {
        "use std::collections::HashMap;\n"
    };

    let full_code = format!(
        r#"#![allow(dead_code, unused_variables, unused_mut)]
{hashmap_import}
{rust_code}

fn main() {{}}
"#
    );

    fs::write(src_dir.join("main.rs"), &full_code).expect("Failed to write main.rs");

    let output = Command::new("cargo")
        .arg("build")
        .arg("--quiet")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run cargo build");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    TranspileCompileResult {
        rust_code: rust_code.to_string(),
        compilation_success: output.status.success(),
        compilation_stderr: stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_and_compile_basic() {
        let python = r#"
def add(x: int, y: int) -> int:
    return x + y
"#;
        let result = transpile_and_compile(python, &["fn add", "-> i32"]);
        assert!(result.compilation_success);
    }

    #[test]
    fn test_transpile_and_check_patterns() {
        let python = r#"
def greet(name: str) -> str:
    return "Hello, " + name
"#;
        let rust_code = transpile_and_check(python, &["fn greet", "String"]);
        assert!(rust_code.contains("fn greet"));
    }

    #[test]
    fn test_mutable_vars_no_leak_between_functions() {
        let python = r#"
from dataclasses import dataclass

@dataclass
class Player:
    jersey_number: int
    score: int

def mutate_player(player: Player) -> None:
    player.score = player.score + 10

def read_player(player: Player) -> int:
    return player.jersey_number
"#;
        let rust_code = transpile(python);

        assert!(
            rust_code.contains("fn mutate_player(player: &mut Player)"),
            "mutate_player should take &mut Player\n{rust_code}"
        );
        assert!(
            rust_code.contains("fn read_player(player: &Player) -> i32"),
            "read_player should take &Player (not &mut)\n{rust_code}"
        );
    }

    #[test]
    fn test_read_only_param_field_access_not_mut() {
        let python = r#"
from dataclasses import dataclass

@dataclass
class PlayerStatistics:
    total_score: int

@dataclass
class Player:
    jersey_number: int
    player_index: int

def _create_player_statistics(player: Player) -> PlayerStatistics:
    return PlayerStatistics(total_score=player.jersey_number + player.player_index)
"#;
        let rust_code = transpile(python);

        assert!(
            rust_code.contains("fn _create_player_statistics(player: &Player)"),
            "Read-only function should take &Player\n{rust_code}"
        );
    }

    #[test]
    fn test_interleaved_mut_and_immut_same_param_name() {
        let python = r#"
from dataclasses import dataclass

@dataclass
class Stats:
    value: int

@dataclass
class Player:
    score: int
    name: str

def reset_score(player: Player) -> None:
    player.score = 0

def get_name(player: Player) -> str:
    return player.name

def bump_score(player: Player) -> Stats:
    player.score = player.score + 1
    return Stats(value=player.score)
"#;
        let rust_code = transpile(python);

        assert!(
            rust_code.contains("fn reset_score(player: &mut Player)"),
            "reset_score should take &mut Player\n{rust_code}"
        );
        assert!(
            rust_code.contains("fn get_name(player: &Player) -> String"),
            "get_name should take &Player (read-only)\n{rust_code}"
        );
        assert!(
            rust_code.contains("fn bump_score(player: &mut Player) -> Stats"),
            "bump_score should take &mut Player\n{rust_code}"
        );
    }
}
