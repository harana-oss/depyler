#!/usr/bin/env python3
"""
Script to refactor stmt_gen.rs into modular files according to REFACTOR.MD plan.
This script extracts functions from the monolithic file and creates separate modules.
"""

import re
import os
from pathlib import Path
from typing import Dict, List, Tuple, Set

# Base paths
CORE_PATH = Path("/Users/naden/Developer/depyler/crates/depyler-core/src/rust_gen")
STMT_GEN_FILE = CORE_PATH / "stmt_gen.rs"
STMT_GEN_DIR = CORE_PATH / "stmt_gen"

# Function to module mapping based on REFACTOR.MD
FUNCTION_MAPPINGS: Dict[str, str] = {
    # helpers.rs
    "extract_nested_indices_tokens": "helpers",
    "extract_nested_indices_tokens_no_clone": "helpers",
    "build_expr_no_clone": "helpers",
    "is_attribute_sourced_expr": "helpers",
    "is_empty_collection_init_expr": "helpers",
    "is_enum_variant_expr": "helpers",
    "is_var_used_in_expr": "helpers",
    "is_var_used_in_stmt": "helpers",
    "is_var_used_in_assign_target": "helpers",
    "is_var_mutated_in_stmts": "helpers",
    "is_var_mutated_in_stmt": "helpers",
    "exprs_are_equivalent": "helpers",
    
    # type_helpers.rs
    "expr_returns_usize": "type_helpers",
    "needs_type_conversion": "type_helpers",
    "apply_type_conversion": "type_helpers",
    "infer_binary_expr_type": "type_helpers",
    "is_expr_float": "type_helpers",
    "expr_is_optional": "type_helpers",
    "apply_optional_truthiness": "type_helpers",
    "apply_truthiness_conversion": "type_helpers",
    "looks_like_option_expr": "type_helpers",
    "hir_type_to_tokens": "type_helpers",
    
    # assign.rs
    "codegen_assign_stmt": "assign",
    "codegen_assign_symbol": "assign",
    "codegen_assign_index": "assign",
    "codegen_assign_slice": "assign",
    "codegen_assign_attribute": "assign",
    "codegen_assign_tuple": "assign",
    "codegen_complex_tuple_unpack": "assign",
    "codegen_starred_unpack": "assign",
    "build_unpack_pattern": "assign",
    "is_dict_augassign_pattern": "assign",
    "is_optional_attr_augassign_pattern": "assign",
    "is_optional_var_augassign_pattern": "assign",
    "is_augassign_pattern": "assign",
    
    # control_flow.rs
    "codegen_if_stmt": "control_flow",
    "codegen_while_stmt": "control_flow",
    "codegen_for_stmt": "control_flow",
    "codegen_break_stmt": "control_flow",
    "codegen_continue_stmt": "control_flow",
    "extract_none_check": "control_flow",
    "codegen_if_let_some": "control_flow",
    "extract_walrus_assignments": "control_flow",
    "extract_walrus_recursive": "control_flow",
    "extract_assigned_symbols": "control_flow",
    "find_variable_type": "control_flow",
    "generate_field_access_without_clone": "control_flow",
    "is_field_access_iter": "control_flow",
    "does_loop_body_mutate_items": "control_flow",
    "is_loop_var_mutated": "control_flow",
    
    # return_stmt.rs
    "codegen_return_stmt": "return_stmt",
    "expr_creates_owned_value": "return_stmt",
    "is_block_expr": "return_stmt",
    
    # exception.rs
    "codegen_raise_stmt": "exception",
    "codegen_try_stmt": "exception",
    "extract_exception_type": "exception",
    "extract_parse_from_tokens": "exception",
    "contains_floor_div": "exception",
    "contains_div": "exception",
    "extract_divisor_from_floor_div": "exception",
    "extract_divisor_from_div": "exception",
    
    # context_mgr.rs
    "codegen_with_stmt": "context_mgr",
    "codegen_async_with_stmt": "context_mgr",
    
    # pattern_match.rs
    "codegen_match_stmt": "pattern_match",
    "codegen_match_stmt_with_mapping": "pattern_match",
    "codegen_match_arm": "pattern_match",
    "codegen_match_arm_mapping": "pattern_match",
    "codegen_pattern": "pattern_match",
    "codegen_pattern_for_option": "pattern_match",
    "expr_to_pattern_literal": "pattern_match",
    "convert_assign_target_to_pattern": "pattern_match",
    
    # functions.rs
    "codegen_nested_function_def": "functions",
    "codegen_async_nested_function_def": "functions",
    "codegen_global_stmt": "functions",
    "codegen_nonlocal_stmt": "functions",
    
    # argparse.rs
    "try_generate_subcommand_match": "argparse",
    "is_subcommand_check": "argparse",
    "to_pascal_case_subcommand": "argparse",
    "extract_string_literal": "argparse",
    "extract_kwarg_string": "argparse",
    "extract_kwarg_bool": "argparse",
    
    # simple.rs
    "codegen_pass_stmt": "simple",
    "codegen_assert_stmt": "simple",
    "codegen_expr_stmt": "simple",
    "codegen_delete_stmt": "simple",
    "codegen_import_stmt": "simple",
    "codegen_import_from_stmt": "simple",
    "codegen_async_for_stmt": "simple",
}

def extract_function_with_body(content: str, func_name: str) -> Tuple[str, int, int]:
    """
    Extract a function and its body from the content.
    Returns (function_text, start_pos, end_pos)
    """
    # Find function definition
    pattern = rf'((?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+{re.escape(func_name)}\s*[<(])'
    match = re.search(pattern, content)
    
    if not match:
        return "", -1, -1
    
    start_pos = match.start()
    
    # Find the end of the function by counting braces
    brace_count = 0
    in_function = False
    i = start_pos
    
    while i < len(content):
        char = content[i]
        
        if char == '{':
            brace_count += 1
            in_function = True
        elif char == '}':
            brace_count -= 1
            if in_function and brace_count == 0:
                # Found the end of the function
                end_pos = i + 1
                return content[start_pos:end_pos], start_pos, end_pos
        
        i += 1
    
    return "", -1, -1

def main():
    print("🔄 Starting stmt_gen.rs refactoring...")
    
    # Read the original file
    with open(STMT_GEN_FILE, 'r') as f:
        content = f.read()
    
    # Create stmt_gen directory if it doesn't exist
    STMT_GEN_DIR.mkdir(exist_ok=True)
    
    # Group functions by module
    modules: Dict[str, List[Tuple[str, str]]] = {}
    for func_name, module_name in FUNCTION_MAPPINGS.items():
        if module_name not in modules:
            modules[module_name] = []
        
        func_text, start, end = extract_function_with_body(content, func_name)
        if func_text:
            modules[module_name].append((func_name, func_text))
            print(f"  ✓ Extracted {func_name} → {module_name}.rs")
        else:
            print(f"  ✗ Could not find function: {func_name}")
    
    # Common imports that most modules will need
    common_imports = """use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};
"""
    
    # Create each module file
    for module_name, functions in modules.items():
        module_path = STMT_GEN_DIR / f"{module_name}.rs"
        
        # Write module file
        with open(module_path, 'w') as f:
            f.write(f"//! {module_name.replace('_', ' ').title()} code generation\n\n")
            f.write(common_imports)
            f.write("\n")
            
            for func_name, func_text in functions:
                # Make functions pub(crate) if they're not already pub
                if not func_text.strip().startswith('pub'):
                    func_text = 'pub(crate) ' + func_text
                
                f.write(func_text)
                f.write("\n\n")
        
        print(f"📝 Created {module_name}.rs with {len(functions)} functions")
    
    print("\n✅ Refactoring script completed!")
    print(f"📁 Created {len(modules)} module files in {STMT_GEN_DIR}")
    print("\n⚠️  Note: You still need to:")
    print("   1. Create mod.rs with the trait impl and module declarations")
    print("   2. Adjust imports in each module as needed")
    print("   3. Remove extracted functions from original stmt_gen.rs")
    print("   4. Run cargo check to identify missing dependencies")

if __name__ == "__main__":
    main()
