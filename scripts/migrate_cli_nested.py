#!/usr/bin/env python3
"""
Script to migrate nested CLI examples to TOML format
"""

import sys
from pathlib import Path

# Add parent to path to import the migration functions
sys.path.insert(0, str(Path(__file__).parent))

from migrate_examples_to_toml import (
    create_toml_test, sanitize_test_name, TOML_DIR
)

CLI_DIR = Path("/Users/naden/Developer/depyler/examples/cli")
TOML_PATH = TOML_DIR / "cli-integration.toml"

def main():
    """Process nested CLI examples."""
    print(f"Processing CLI examples from: {CLI_DIR}")
    
    # Get all Python files recursively
    py_files = sorted(CLI_DIR.rglob('*.py'))
    
    if not py_files:
        print("No Python files found in CLI directory")
        return
    
    print(f"Found {len(py_files)} Python files")
    
    # Build content to append
    content = '''
# =============================================================================
# Additional CLI examples from nested directories
# =============================================================================
'''
    
    processed = []
    for py_file in py_files:
        # Check for corresponding Rust file
        rs_file = py_file.with_suffix('.rs')
        
        # Create test entry
        test_entry = create_toml_test(py_file, rs_file if rs_file.exists() else None, "cli")
        if test_entry:
            content += test_entry
            processed.append(py_file)
            if rs_file.exists():
                processed.append(rs_file)
    
    # Append to existing TOML file
    if processed:
        with open(TOML_PATH, 'a') as f:
            f.write('\n')
            f.write(content)
        
        print(f"✓ Appended {len(py_files)} CLI files to cli-integration.toml")
        print("\nProcessed files:")
        for f in sorted(processed):
            print(f"  - {f.relative_to(CLI_DIR.parent)}")
    
    return processed

if __name__ == '__main__':
    main()
