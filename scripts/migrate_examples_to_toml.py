#!/usr/bin/env python3
"""
Script to migrate Python examples from examples/ directory to TOML test format in tests/toml/
"""

import os
import sys
from pathlib import Path
import re
from typing import Dict, List, Tuple

EXAMPLES_DIR = Path("/Users/naden/Developer/depyler/examples")
TOML_DIR = Path("/Users/naden/Developer/depyler/tests/toml")

# Mapping of example categories to TOML file names
CATEGORY_MAPPINGS = {
    'algorithms': 'examples-algorithms.toml',
    'arrays': 'examples-arrays.toml',
    'async': 'async-functions.toml',  # Merge with existing
    'classes': 'classes.toml',  # Merge with existing
    'cli': 'cli-integration.toml',  # Merge with existing
    'control_flow': 'control-flow.toml',  # Merge with existing
    'data_processing': 'examples-data-processing.toml',
    'data_structures': 'examples-data-structures.toml',
    'decorators': 'decorators-basic.toml',  # Merge with existing
    'dictionaries': 'dictionaries.toml',  # Merge with existing
    'file_processing': 'examples-file-processing.toml',
    'game_development': 'examples-game-development.toml',
    'lambdas': 'lambdas.toml',  # Merge with existing
    'marco_polo_cli': 'examples-marco-polo-cli.toml',
    'mathematical': 'examples-mathematical.toml',
    'networking': 'examples-networking.toml',
    'sets': 'set-operations.toml',  # Merge with existing
    'stdlib': 'examples-stdlib.toml',
    'string_processing': 'string-operations.toml',  # Merge with existing
    'tooling': 'examples-tooling.toml',
    'type_hints': 'examples-type-hints.toml',
    'validation': 'examples-validation.toml',
    'wasm': 'examples-wasm.toml',
    'web_scraping': 'examples-web-scraping.toml',
}


def sanitize_test_name(filename: str) -> str:
    """Convert filename to a valid test name."""
    name = filename.replace('.py', '').replace('.rs', '')
    name = re.sub(r'[^a-zA-Z0-9_]', '_', name)
    return name


def extract_description(content: str) -> str:
    """Extract description from Python file comments."""
    lines = content.split('\n')
    comments = []
    for line in lines[:10]:  # Look at first 10 lines
        line = line.strip()
        if line.startswith('#'):
            comment = line.lstrip('#').strip()
            if comment and not comment.startswith('@depyler'):
                comments.append(comment)
        elif line and not line.startswith('from') and not line.startswith('import'):
            break
    
    if comments:
        return ' '.join(comments)
    return "Example test"


def read_file_safe(filepath: Path) -> str:
    """Read file content safely."""
    try:
        return filepath.read_text()
    except Exception as e:
        print(f"Warning: Could not read {filepath}: {e}")
        return ""


def create_toml_test(py_file: Path, rs_file: Path = None, category: str = "") -> str:
    """Create a TOML test entry from Python (and optionally Rust) files."""
    py_content = read_file_safe(py_file)
    if not py_content:
        return ""
    
    test_name = sanitize_test_name(py_file.stem)
    description = extract_description(py_content)
    
    # Build the test entry
    test = f'''
[[test]]
name = "{test_name}"
description = "{description}"

python = \'\'\'
{py_content.strip()}
\'\'\'
'''
    
    # Add Rust content if available
    if rs_file and rs_file.exists():
        rs_content = read_file_safe(rs_file)
        if rs_content:
            test += f'''
rust = \'\'\'
{rs_content.strip()}
\'\'\'
'''
    
    return test


def process_category(category: str, category_path: Path) -> Tuple[str, List[Path]]:
    """Process all Python files in a category directory."""
    toml_filename = CATEGORY_MAPPINGS.get(category, f'examples-{category}.toml')
    toml_path = TOML_DIR / toml_filename
    
    # Get all Python files (including nested directories)
    py_files = sorted(category_path.rglob('*.py'))
    if not py_files:
        return toml_filename, []
    
    # Check if TOML file exists
    toml_exists = toml_path.exists()
    
    # Build the content
    if toml_exists:
        # We'll append to existing file
        content = ""
    else:
        # Create new file with metadata
        category_title = category.replace('_', ' ').title()
        content = f'''# {category_title} Examples
# Migrated from examples/{category}/
[metadata]
name = "{category_title} Examples"
category = "{category}"

# =============================================================================
# Examples
# =============================================================================
'''
    
    # Process each Python file
    processed_files = []
    for py_file in py_files:
        # Check for corresponding Rust file
        rs_file = py_file.with_suffix('.rs')
        
        # Create test entry
        test_entry = create_toml_test(py_file, rs_file if rs_file.exists() else None, category)
        if test_entry:
            content += test_entry
            processed_files.append(py_file)
            if rs_file.exists():
                processed_files.append(rs_file)
    
    # Write or append to TOML file
    if content:
        if toml_exists:
            # Append to existing file
            with open(toml_path, 'a') as f:
                f.write('\n')
                f.write('# =============================================================================\n')
                f.write(f'# Additional examples from examples/{category}/\n')
                f.write('# =============================================================================\n')
                f.write(content)
        else:
            # Create new file
            toml_path.write_text(content)
        
        print(f"✓ Processed {len(py_files)} files from {category} -> {toml_filename}")
    
    return toml_filename, processed_files


def process_root_examples() -> List[Path]:
    """Process Python files in the root examples directory."""
    processed_files = []
    
    # Get root-level Python files
    py_files = [f for f in EXAMPLES_DIR.glob('*.py') if f.is_file()]
    
    if not py_files:
        return processed_files
    
    toml_path = TOML_DIR / 'examples-root.toml'
    
    content = '''# Root Examples
# Miscellaneous examples from examples/ root
[metadata]
name = "Root Examples"
category = "examples"

# =============================================================================
# Examples
# =============================================================================
'''
    
    for py_file in py_files:
        rs_file = py_file.with_suffix('.rs')
        test_entry = create_toml_test(py_file, rs_file if rs_file.exists() else None, "root")
        if test_entry:
            content += test_entry
            processed_files.append(py_file)
            if rs_file.exists():
                processed_files.append(rs_file)
    
    if processed_files:
        toml_path.write_text(content)
        print(f"✓ Processed {len(py_files)} root files -> examples-root.toml")
    
    return processed_files


def main():
    """Main migration function."""
    print("Starting migration of examples to TOML tests...")
    print(f"Source: {EXAMPLES_DIR}")
    print(f"Target: {TOML_DIR}")
    print()
    
    all_processed_files = []
    
    # Process each category directory
    for category_dir in sorted(EXAMPLES_DIR.iterdir()):
        if not category_dir.is_dir():
            continue
        
        category = category_dir.name
        toml_file, processed_files = process_category(category, category_dir)
        all_processed_files.extend(processed_files)
    
    # Process root-level examples
    root_files = process_root_examples()
    all_processed_files.extend(root_files)
    
    print()
    print(f"Total files processed: {len(all_processed_files)}")
    print()
    print("Migration complete!")
    print()
    print("Files that were processed:")
    for f in sorted(all_processed_files):
        print(f"  - {f.relative_to(EXAMPLES_DIR.parent)}")
    
    return all_processed_files


if __name__ == '__main__':
    main()
