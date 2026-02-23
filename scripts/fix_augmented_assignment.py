#!/usr/bin/env python3
"""
Fix augmented assignment patterns in test expectations.
Replace 'x = x + y' with 'x += y', 'x = x - y' with 'x -= y', etc.
"""

import re
from pathlib import Path
from typing import Tuple

def fix_augmented_assignment(content: str) -> Tuple[str, int]:
    """Replace 'var = var op expr' with 'var op= expr'."""
    fixes = 0
    
    # Pattern: var = var + expr (with or without semicolon)
    # Captures: variable name, operator, expression, optional semicolon
    patterns = [
        # Match: var = var + expr; or var = var + expr\n
        (r'\b(\w+)\s*=\s*\1\s*\+\s*([^;\n]+)(;?)', r'\1 += \2\3'),
        (r'\b(\w+)\s*=\s*\1\s*-\s*([^;\n]+)(;?)', r'\1 -= \2\3'),
        (r'\b(\w+)\s*=\s*\1\s*\*\s*([^;\n]+)(;?)', r'\1 *= \2\3'),
        (r'\b(\w+)\s*=\s*\1\s*/\s*([^;\n]+)(;?)', r'\1 /= \2\3'),
    ]
    
    new_content = content
    for pattern, replacement in patterns:
        def replacer(match):
            nonlocal fixes
            fixes += 1
            return match.expand(replacement)
        
        new_content = re.sub(pattern, replacer, new_content)
    
    return new_content, fixes

def main():
    toml_dir = Path(__file__).parent.parent / "tests" / "toml"
    
    # Get all TOML files
    toml_files = sorted(toml_dir.glob("*.toml"))
    
    total_fixes = 0
    files_modified = 0
    
    for filepath in toml_files:
        content = filepath.read_text()
        new_content, fixes = fix_augmented_assignment(content)
        
        if fixes > 0:
            filepath.write_text(new_content)
            print(f"✅ {filepath.name}: Fixed {fixes} instances")
            total_fixes += fixes
            files_modified += 1
    
    print(f"\n📊 Total fixes: {total_fixes} across {files_modified} files")

if __name__ == "__main__":
    main()
