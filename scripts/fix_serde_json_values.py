#!/usr/bin/env python3
"""
Script to replace serde_json::Value with proper types in TOML test files.
"""

import re
import os
from pathlib import Path

def fix_toml_file(filepath):
    """Fix a single TOML file."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    original_content = content
    
    # Pattern 1: Simple result with serde_json::Value that should be i32
    # pub const result: (serde_json::Value, serde_json::Value) = (p.x, p.y);
    content = re.sub(
        r'pub const (\w+): \(serde_json::Value, serde_json::Value\) = \((\w+\.x), (\w+\.y)\);',
        r'pub const \1: (i32, i32) = (\2, \3);',
        content
    )
    
    # Pattern 2: Three-element tuples
    content = re.sub(
        r'pub const (\w+): \(serde_json::Value, serde_json::Value, serde_json::Value\) = \((\w+\.x), (\w+\.y), (\w+\.z)\);',
        r'pub const \1: (i32, i32, i32) = (\2, \3, \4);',
        content
    )
    
    # Pattern 2b: Three-element tuples (String, int, float)
    content = re.sub(
        r'pub const (\w+): \(serde_json::Value, serde_json::Value, serde_json::Value\) =\s*\((\w+)\.name, (\w+)\.age, (\w+)\.height\);',
        r'pub const \1: (String, i32, f64) = (\2.name, \3.age, \4.height);',
        content
    )
    
    # Pattern 3: hash results
    content = re.sub(
        r'pub const (\w+): serde_json::Value = \{\s*use std::collections::hash_map::DefaultHasher;.*?hasher\.finish\(\) as i64\s*\};',
        r'pub const \1: i64 = {\n    use std::collections::hash_map::DefaultHasher;\n    use std::hash::{Hash, Hasher};\n    let mut hasher = DefaultHasher::new();\n    p.hash(&mut hasher);\n    hasher.finish() as i64\n};',
        content,
        flags=re.DOTALL
    )
    
    # Pattern 4: format!("{:?}", ...) returns String
    content = re.sub(
        r'pub const (\w+): serde_json::Value = format!\("\{:?\?\}", (\w+)\);',
        r'pub const \1: String = format!("{:?}", \2);',
        content
    )
    
    # Pattern 5: len() returns i32
    content = re.sub(
        r'pub const (\w+): serde_json::Value = (\w+)\.(?:clone\(\)\.)?len\(\) as i32;',
        r'pub const \1: i32 = \2.len() as i32;',
        content
    )
    
    # Pattern 6: Boolean comparisons
    content = re.sub(
        r'pub const (\w+): serde_json::Value = (\w+) (==|!=|<|>|<=|>=) (\w+);',
        r'pub const \1: bool = \2 \3 \4;',
        content
    )
    
    # Pattern 6b: Boolean negation
    content = re.sub(
        r'pub const (\w+): serde_json::Value = !',
        r'pub const \1: bool = !',
        content
    )
    
    # Pattern 6c: Boolean from is_subset, is_superset, is_disjoint
    content = re.sub(
        r'pub const (\w+): serde_json::Value = (\w+)\.clone\(\)\.(is_subset|is_superset|is_disjoint)\(',
        r'pub const \1: bool = \2.clone().\3(',
        content
    )
    
    # Pattern 7: HashSet results
    content = re.sub(
        r'pub const (\w+): serde_json::Value = vec!\[[\d, ]+\]\.into_iter\(\)\.collect::<HashSet<_>>\(\);',
        r'pub const \1: HashSet<i32> = vec![1, 2, 3].into_iter().collect::<HashSet<_>>();',
        content
    )
    
    # Pattern 7b: Empty HashSet
    content = re.sub(
        r'pub const (\w+): serde_json::Value = HashSet::<i32>::new\(\);',
        r'pub const \1: HashSet<i32> = HashSet::<i32>::new();',
        content
    )
    
    # Pattern 7c: HashSet from string
    content = re.sub(
        r'pub const (\w+): serde_json::Value = "(\w+)"\.into_iter\(\)\.collect::<HashSet<_>>\(\);',
        r'pub const \1: HashSet<char> = "\2".chars().collect::<HashSet<_>>();',
        content
    )
    
    # Pattern 7d: HashSet from range
    content = re.sub(
        r'pub const (\w+): serde_json::Value = \((\d+)\.\.(\d+)\)\.map\(\|(\w+)\| (\w+) \* (\d+)\)\.collect::<HashSet<_>>\(\);',
        r'pub const \1: HashSet<i32> = (\2..\3).map(|\4| \5 * \6).collect::<HashSet<_>>();',
        content
    )
    
    # Pattern 8: HashSet operations returning HashSet
    content = re.sub(
        r'pub const (\w+): serde_json::Value = (\w+)\s*\n\s*\.clone\(\)\s*\n\s*\.(union|intersection|difference|symmetric_difference)\(',
        r'pub const \1: HashSet<i32> = \2\n    .clone()\n    .\3(',
        content
    )
    
    # Pattern 9: Class instance types
    content = re.sub(
        r'pub const (\w+): serde_json::Value = (Point|Container|Rectangle|Inner|Outer|Person|Counter|Point3D|Point2D|Config)::new\(',
        r'pub const \1: \2 = \2::new(',
        content
    )
    
    # Pattern 10: TypeVar instances should remain as is (they're intentionally dynamic)
    # Skip: pub const T: serde_json::Value = TypeVar::new("T");
    
    # Pattern 11: Field access that returns primitive - area, value, count, etc.
    content = re.sub(
        r'pub const (\w+): serde_json::Value = (\w+)\.(area|value|count|total);',
        r'pub const \1: i32 = \2.\3;',
        content
    )
    
    # Pattern 12: Two-element tuple with field and len
    content = re.sub(
        r'pub const (\w+): \(serde_json::Value, serde_json::Value\) = \((\w+), (\w+)\.clone\(\)\.len\(\) as i32\);',
        r'pub const \1: (i32, i32) = (\2, \3.len() as i32);',
        content
    )
    
    # Pattern 13: Two-element tuple with two values
    content = re.sub(
        r'pub const (\w+): \(serde_json::Value, serde_json::Value\) = \((\w+)\.value, (\w+)\.value\);',
        r'pub const \1: (i32, i32) = (\2.value, \3.value);',
        content
    )
    
    # Pattern 14: FrozenSet variable
    content = re.sub(
        r'pub const (\w+): serde_json::Value =\s*\n\s*frozenset',
        r'pub const \1: HashSet<i32> =\n    frozenset',
        content
    )
    
    # Pattern 15: HashMap with serde_json::Value keys for dictionary operations
    content = re.sub(
        r'HashMap<serde_json::Value, serde_json::Value>',
        r'HashMap<String, i32>',
        content
    )
    
    # Pattern 16: Function parameters with Vec<serde_json::Value>
    content = re.sub(
        r'pub fn (\w+)\((\w+): &Vec<serde_json::Value>\)',
        r'pub fn \1(\2: &Vec<i32>)',
        content
    )
    
    # Pattern 17: Function parameters with HashSet<serde_json::Value>
    content = re.sub(
        r'pub fn (\w+)\((\w+): &HashSet<serde_json::Value>\)',
        r'pub fn \1(\2: &HashSet<i32>)',
        content
    )
    
    # Pattern 18: HashSet return types in function signatures
    content = re.sub(
        r'-> HashSet<serde_json::Value>',
        r'-> HashSet<i32>',
        content
    )
    
    # Pattern 19: Vec return types in function signatures
    content = re.sub(
        r'-> Vec<serde_json::Value>',
        r'-> Vec<i32>',
        content
    )
    
    # Pattern 20: Tuples with three serde_json::Value in return types
    content = re.sub(
        r'-> \(serde_json::Value, serde_json::Value, serde_json::Value\)',
        r'-> (i32, i32, i32)',
        content
    )
    
    # Pattern 21: Tuples with two serde_json::Value in return types
    content = re.sub(
        r'-> \(serde_json::Value, serde_json::Value\)',
        r'-> (i32, i32)',
        content
    )
    
    # Pattern 22: HashMap return types
    content = re.sub(
        r'-> HashMap<serde_json::Value, i32>',
        r'-> HashMap<String, i32>',
        content
    )
    
    # Pattern 23: Remove unnecessary use serde_json; if no other serde_json usage
    if 'serde_json::Value' not in content and content.count('serde_json::') <= content.count('use serde_json;'):
        content = re.sub(r'use serde_json;\n', '', content)
    
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        return True
    return False

def main():
    """Process all TOML files in tests/toml directory."""
    toml_dir = Path("/Users/naden/Developer/depyler/tests/toml")
    
    fixed_files = []
    for toml_file in toml_dir.glob("*.toml"):
        if fix_toml_file(toml_file):
            fixed_files.append(toml_file.name)
            print(f"Fixed: {toml_file.name}")
    
    print(f"\nTotal files fixed: {len(fixed_files)}")
    if fixed_files:
        print("Files modified:")
        for f in sorted(fixed_files):
            print(f"  - {f}")

if __name__ == "__main__":
    main()
