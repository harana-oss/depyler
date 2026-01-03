#!/usr/bin/env python3
"""
Format all Rust code expectations in TOML test files using rustfmt.

This script:
1. Reads all TOML files in tests/toml/
2. Extracts the 'rust' field from each [[test]] section
3. Formats the Rust code using rustfmt
4. Updates the TOML file with the formatted code
5. Preserves all other TOML structure and formatting

Usage:
    python scripts/format_toml_expectations.py [--dry-run] [--verbose]
"""

import argparse
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple
import re


def format_rust_code(rust_code: str) -> str:
    """
    Format Rust code using rustfmt.
    
    Args:
        rust_code: The Rust code to format
        
    Returns:
        Formatted Rust code, or original if rustfmt fails
    """
    # Skip formatting for certain special cases
    if rust_code.strip().startswith('// Transpilation failed'):
        # This is an error message, not code
        return rust_code
    
    try:
        result = subprocess.run(
            ['rustfmt', '--edition', '2024'],
            input=rust_code.encode('utf-8'),
            capture_output=True,
            timeout=5
        )
        
        if result.returncode == 0:
            return result.stdout.decode('utf-8')
        else:
            # rustfmt failed - might be a code fragment
            # Try wrapping in fn main() and formatting
            stderr = result.stderr.decode('utf-8')
            if 'expected item' in stderr or 'let` cannot be used for global variables' in stderr:
                wrapped = f"fn main() {{\n{rust_code}\n}}"
                result = subprocess.run(
                    ['rustfmt', '--edition', '2024'],
                    input=wrapped.encode('utf-8'),
                    capture_output=True,
                    timeout=5
                )
                if result.returncode == 0:
                    formatted = result.stdout.decode('utf-8')
                    # Extract the body from fn main() { ... }
                    # This is a bit hacky but works for simple cases
                    if formatted.startswith('fn main() {\n') and formatted.endswith('\n}\n'):
                        return formatted[12:-2]  # Remove "fn main() {\n" and "\n}\n"
            
            # Still failed, return original
            return rust_code
    except subprocess.TimeoutExpired:
        return rust_code
    except FileNotFoundError:
        print("  ⚠ rustfmt not found in PATH", file=sys.stderr)
        return rust_code
    except Exception as e:
        return rust_code


def extract_and_format_toml(content: str, verbose: bool = False) -> Tuple[str, int]:
    """
    Extract Rust code blocks from TOML, format them, and reconstruct the TOML.
    
    This function uses a simple state machine to parse TOML and identify
    rust = '''...''' or rust = \"\"\"...\"\"\" blocks.
    
    Args:
        content: The TOML file content
        verbose: Whether to print verbose output
        
    Returns:
        Tuple of (formatted content, number of formatted blocks)
    """
    lines = content.split('\n')
    result_lines = []
    i = 0
    formatted_count = 0
    
    while i < len(lines):
        line = lines[i]
        
        # Check if this line starts a rust field with triple quotes
        if re.match(r"^\s*rust\s*=\s*'''", line):
            # Multi-line string with '''
            indent = len(line) - len(line.lstrip())
            indent_str = ' ' * indent
            
            # Collect all lines until closing '''
            rust_lines = []
            i += 1
            while i < len(lines):
                if lines[i].strip() == "'''":
                    break
                rust_lines.append(lines[i])
                i += 1
            
            # Format the Rust code
            rust_code = '\n'.join(rust_lines)
            if rust_code.strip():
                formatted = format_rust_code(rust_code)
                if formatted != rust_code:
                    formatted_count += 1
                    if verbose:
                        print(f"  ✓ Formatted rust block")
                
                # Reconstruct the TOML with formatted code
                result_lines.append(f"{indent_str}rust = '''")
                result_lines.extend(formatted.rstrip('\n').split('\n'))
                result_lines.append(f"{indent_str}'''")
            else:
                # Empty rust block, keep as-is
                result_lines.append(f"{indent_str}rust = '''")
                result_lines.append(f"{indent_str}'''")
                
        elif re.match(r'^\s*rust\s*=\s*"""', line):
            # Multi-line string with """
            indent = len(line) - len(line.lstrip())
            indent_str = ' ' * indent
            
            # Collect all lines until closing """
            rust_lines = []
            i += 1
            while i < len(lines):
                if lines[i].strip() == '"""':
                    break
                rust_lines.append(lines[i])
                i += 1
            
            # Format the Rust code
            rust_code = '\n'.join(rust_lines)
            if rust_code.strip():
                formatted = format_rust_code(rust_code)
                if formatted != rust_code:
                    formatted_count += 1
                    if verbose:
                        print(f"  ✓ Formatted rust block")
                
                # Reconstruct the TOML with formatted code
                result_lines.append(f'{indent_str}rust = """')
                result_lines.extend(formatted.rstrip('\n').split('\n'))
                result_lines.append(f'{indent_str}"""')
            else:
                # Empty rust block, keep as-is
                result_lines.append(f'{indent_str}rust = """')
                result_lines.append(f'{indent_str}"""')
        else:
            # Regular line, keep as-is
            result_lines.append(line)
        
        i += 1
    
    return '\n'.join(result_lines), formatted_count


def process_toml_file(file_path: Path, dry_run: bool = False, verbose: bool = False) -> Tuple[bool, int]:
    """
    Process a single TOML file.
    
    Args:
        file_path: Path to the TOML file
        dry_run: If True, don't write changes
        verbose: Whether to print verbose output
        
    Returns:
        Tuple of (success, number of blocks formatted)
    """
    try:
        content = file_path.read_text(encoding='utf-8')
        formatted_content, formatted_count = extract_and_format_toml(content, verbose)
        
        if formatted_count > 0:
            if dry_run:
                print(f"  Would format {formatted_count} blocks")
            else:
                file_path.write_text(formatted_content, encoding='utf-8')
                print(f"  ✓ Formatted {formatted_count} blocks")
        elif verbose:
            print(f"  No changes needed")
        
        return True, formatted_count
    except Exception as e:
        print(f"  ✗ Error: {e}", file=sys.stderr)
        return False, 0


def main():
    parser = argparse.ArgumentParser(
        description='Format Rust code expectations in TOML test files'
    )
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Show what would be changed without modifying files'
    )
    parser.add_argument(
        '--verbose', '-v',
        action='store_true',
        help='Print verbose output'
    )
    parser.add_argument(
        '--path',
        type=Path,
        default=Path('tests/toml'),
        help='Path to TOML test directory (default: tests/toml)'
    )
    
    args = parser.parse_args()
    
    # Find project root (directory containing this script's parent)
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    test_dir = project_root / args.path
    
    if not test_dir.exists():
        print(f"Error: Test directory not found: {test_dir}", file=sys.stderr)
        sys.exit(1)
    
    # Find all TOML files
    toml_files = sorted(test_dir.glob('*.toml'))
    
    if not toml_files:
        print(f"No TOML files found in {test_dir}", file=sys.stderr)
        sys.exit(1)
    
    print(f"{'DRY RUN: ' if args.dry_run else ''}Processing {len(toml_files)} TOML files...\n")
    
    total_formatted = 0
    success_count = 0
    
    for toml_file in toml_files:
        print(f"📄 {toml_file.name}")
        success, formatted = process_toml_file(toml_file, args.dry_run, args.verbose)
        if success:
            success_count += 1
            total_formatted += formatted
    
    print(f"\n{'Would format' if args.dry_run else 'Formatted'} {total_formatted} Rust code blocks in {success_count}/{len(toml_files)} files")
    
    if args.dry_run:
        print("\nRe-run without --dry-run to apply changes")


if __name__ == '__main__':
    main()
