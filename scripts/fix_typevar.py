#!/usr/bin/env python3
"""
Remove unnecessary TypeVar::new() runtime declarations from test expectations.
TypeVars should only appear as generic type parameters, not runtime constants.

This fixes issue C4 from PLAN.md.
"""

import re
from pathlib import Path
from typing import Tuple

def fix_typevar_declarations(content: str) -> Tuple[str, int]:
    """Remove TypeVar::new() constant declarations."""
    fixes = 0
    
    # Pattern: pub const T: serde_json::Value = TypeVar::new("T");
    # Also matches U, V, etc.
    # Should be completely removed - the generic parameter on the function is sufficient
    pattern = r'pub const [A-Z]: serde_json::Value = TypeVar::new\("[A-Z]"\);\n'
    
    def replacer(match):
        nonlocal fixes
        fixes += 1
        return ''  # Remove the line entirely
    
    new_content = re.sub(pattern, replacer, content)
    
    return new_content, fixes

def main():
    toml_dir = Path(__file__).parent.parent / "tests" / "toml"
    
    # Files with TypeVar issues based on grep results
    files_to_fix = [
        "functions.toml",
        "classes.toml",
        "type-inference.toml",
    ]
    
    total_fixes = 0
    
    for filename in files_to_fix:
        filepath = toml_dir / filename
        if not filepath.exists():
            print(f"⚠️  File not found: {filepath}")
            continue
        
        content = filepath.read_text()
        new_content, fixes = fix_typevar_declarations(content)
        
        if fixes > 0:
            filepath.write_text(new_content)
            print(f"✅ {filename}: Removed {fixes} TypeVar declarations")
            total_fixes += fixes
        else:
            print(f"✓  {filename}: No changes needed")
    
    print(f"\n📊 Total TypeVar declarations removed: {total_fixes}")

if __name__ == "__main__":
    main()
