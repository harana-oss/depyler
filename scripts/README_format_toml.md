# Format TOML Test Expectations

This script formats all Rust code blocks in TOML test files using `rustfmt`.

## Purpose

The Depyler test suite compares transpiled Rust output against expected Rust code stored in TOML files. To ensure consistent formatting and avoid false test failures due to whitespace or formatting differences, all expected Rust code should be normalized through `rustfmt`.

## Features

- ✅ Formats all `rust = '''...'''` blocks in TOML files
- ✅ Preserves TOML structure and other fields
- ✅ Handles code fragments by wrapping in `fn main()` if needed
- ✅ Skips error messages (lines starting with `// Transpilation failed`)
- ✅ Dry-run mode to preview changes
- ✅ Verbose output for debugging

## Usage

### Preview Changes (Recommended First)

```bash
python3 scripts/format_toml_expectations.py --dry-run --verbose
```

### Apply Formatting

```bash
python3 scripts/format_toml_expectations.py
```

### Format Specific Directory

```bash
python3 scripts/format_toml_expectations.py --path tests/toml/subdirectory
```

### Help

```bash
python3 scripts/format_toml_expectations.py --help
```

## How It Works

1. **Find TOML Files**: Recursively finds all `.toml` files in `tests/toml/`
2. **Extract Rust Code**: Identifies `rust = '''...'''` or `rust = """..."""` blocks
3. **Format with rustfmt**: Runs each Rust code block through `rustfmt --edition 2024`
4. **Handle Fragments**: If rustfmt fails (e.g., for code fragments like `let x = 5;`), wraps the code in `fn main() { ... }`, formats it, then extracts the body
5. **Preserve Structure**: Maintains all TOML formatting, comments, and structure
6. **Update Files**: Writes the formatted code back to the TOML file

## Requirements

- Python 3.6+
- `rustfmt` installed and available in PATH
- Write access to the test files (when not using `--dry-run`)

## Example

### Before

```toml
[[test]]
name = "simple_function"
python = "def add(a, b): return a + b"
rust = '''
pub fn add(a:i32,b:i32)->i32{
    return a+b;
}
'''
```

### After

```toml
[[test]]
name = "simple_function"
python = "def add(a, b): return a + b"
rust = '''
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
'''
```

## Integration with Test Suite

The test runner (`cargo run -- test`) automatically formats both expected and actual code through `rustfmt` before comparison, so formatting differences are normalized. However, keeping the expectations pre-formatted makes diffs easier to read and maintain.

## Notes

- The script is idempotent - running it multiple times produces the same result
- Failed rustfmt operations (e.g., on invalid Rust code) leave the original code unchanged
- Comments and other TOML fields are preserved exactly as-is
