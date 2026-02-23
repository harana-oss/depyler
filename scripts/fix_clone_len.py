#!/usr/bin/env python3
"""
Fix unnecessary .clone().len() calls in test expectations.
These should just be .len() without the clone.
"""

import re
from pathlib import Path

def fix_clone_len(content: str) -> tuple[str, int]:
    """Remove unnecessary .clone() before .len() calls."""
    fixes = 0
    
    # Pattern: .clone().len()
    # Replace with: .len()
    pattern = r'\.clone\(\)\.len\(\)'
    
    def replacer(match):
        nonlocal fixes
        fixes += 1
        return '.len()'
    
    new_content = re.sub(pattern, replacer, content)
    
    return new_content, fixes

def main():
    toml_dir = Path(__file__).parent.parent / "tests" / "toml"
    
    # Files with .clone().len() issues based on grep results
    files_to_fix = [
        "set-operations.toml",
        "lifetimes.toml",
        "loop-for.toml",
        "optional-types.toml",
        "functions.toml",
        "iterators.toml",
    ]
    
    total_fixes = 0
    
    for filename in files_to_fix:
        filepath = toml_dir / filename
        if not filepath.exists():
            print(f"⚠️  File not found: {filepath}")
            continue
        
        content = filepath.read_text()
        new_content, fixes = fix_clone_len(content)
        
        if fixes > 0:
            filepath.write_text(new_content)
            print(f"✅ {filename}: Fixed {fixes} instances")
            total_fixes += fixes
        else:
            print(f"✓  {filename}: No changes needed")
    
    print(f"\n📊 Total fixes: {total_fixes}")

if __name__ == "__main__":
    main()
