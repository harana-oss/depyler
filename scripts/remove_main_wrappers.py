#!/usr/bin/env python3
"""
Script to remove fn main() wrappers from Rust code in TOML test files
where the Python code doesn't have a main function.
"""

import re
import sys
from pathlib import Path


def has_python_main(python_code: str) -> bool:
    """Check if Python code has a main function definition."""
    # Look for "def main(" pattern
    return bool(re.search(r'\bdef\s+main\s*\(', python_code))


def remove_rust_main_wrapper(rust_code: str) -> str:
    """Remove fn main() wrapper from Rust code."""
    # Pattern to match fn main() { ... } and extract the body
    pattern = r'^(.*?)fn main\(\) \{\n(.*)\n\}$'
    match = re.match(pattern, rust_code, re.DOTALL)
    
    if not match:
        return rust_code
    
    prefix = match.group(1)
    body = match.group(2)
    
    # Dedent the body (remove 4 spaces of indentation)
    lines = body.split('\n')
    dedented_lines = []
    for line in lines:
        if line.startswith('    '):
            dedented_lines.append(line[4:])
        else:
            dedented_lines.append(line)
    
    dedented_body = '\n'.join(dedented_lines)
    
    return prefix + dedented_body


def process_toml_file(file_path: Path) -> tuple[int, int]:
    """
    Process a TOML file and remove fn main() wrappers where appropriate.
    Returns (tests_processed, tests_modified).
    """
    content = file_path.read_text()
    
    # Pattern to match test blocks with python and rust sections
    test_pattern = r'\[\[test\]\](.*?)(?=\[\[test\]\]|\Z)'
    
    tests_processed = 0
    tests_modified = 0
    modified_content = content
    
    for test_match in re.finditer(test_pattern, content, re.DOTALL):
        test_block = test_match.group(0)
        tests_processed += 1
        
        # Extract python code
        python_match = re.search(r'python = """(.*?)"""', test_block, re.DOTALL)
        if not python_match:
            continue
        
        python_code = python_match.group(1)
        
        # Skip if Python has a main function
        if has_python_main(python_code):
            continue
        
        # Extract rust code
        rust_match = re.search(r'rust = """(.*?)"""', test_block, re.DOTALL)
        if not rust_match:
            continue
        
        rust_code = rust_match.group(1)
        
        # Check if rust has fn main()
        if 'fn main()' not in rust_code:
            continue
        
        # Remove the main wrapper
        new_rust_code = remove_rust_main_wrapper(rust_code)
        
        # Replace in the content
        old_rust_section = f'rust = """{rust_code}"""'
        new_rust_section = f'rust = """{new_rust_code}"""'
        
        if old_rust_section in modified_content:
            modified_content = modified_content.replace(old_rust_section, new_rust_section, 1)
            tests_modified += 1
    
    # Write back if modified
    if tests_modified > 0:
        file_path.write_text(modified_content)
    
    return tests_processed, tests_modified


def main():
    """Main entry point."""
    toml_dir = Path(__file__).parent.parent / 'tests' / 'toml'
    
    if not toml_dir.exists():
        print(f"Error: {toml_dir} does not exist", file=sys.stderr)
        sys.exit(1)
    
    # Get all .toml files except Cargo.toml
    toml_files = [f for f in toml_dir.glob('*.toml') if f.name != 'Cargo.toml']
    
    total_tests = 0
    total_modified = 0
    files_modified = 0
    
    for toml_file in sorted(toml_files):
        tests_processed, tests_modified = process_toml_file(toml_file)
        total_tests += tests_processed
        total_modified += tests_modified
        
        if tests_modified > 0:
            files_modified += 1
            print(f"✓ {toml_file.name}: {tests_modified}/{tests_processed} tests modified")
    
    print(f"\n{'='*60}")
    print(f"Summary:")
    print(f"  Files processed: {len(toml_files)}")
    print(f"  Files modified: {files_modified}")
    print(f"  Total tests processed: {total_tests}")
    print(f"  Total tests modified: {total_modified}")
    print(f"{'='*60}")


if __name__ == '__main__':
    main()
