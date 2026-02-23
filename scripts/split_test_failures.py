#!/usr/bin/env python3
"""
Script to parse test failures from temp file and create separate files
in the failures directory for each test failure.
"""

import re
import os
from pathlib import Path


def parse_test_failures(temp_file: str, output_dir: str):
    """
    Parse the temp file and create separate files for each test failure.
    
    Args:
        temp_file: Path to the temp file containing test failures
        output_dir: Directory where failure files will be created
    """
    # Create output directory if it doesn't exist
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    # Read the temp file
    with open(temp_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Pattern to match test failure sections
    # Format: "  filename.toml :: test_name"
    pattern = r'^  ([a-z_\-]+\.toml) :: ([a-z_\-0-9]+)$'
    
    # Split content into lines for processing
    lines = content.split('\n')
    
    current_test = None
    current_file = None
    current_content = []
    test_count = 0
    
    separator = '─' * 60
    
    for i, line in enumerate(lines):
        # Check if this line matches a test header
        match = re.match(pattern, line)
        
        if match:
            # Save previous test if exists
            if current_test and current_file and current_content:
                save_failure(output_path, current_file, current_test, current_content)
                test_count += 1
            
            # Start new test
            current_file = match.group(1)
            current_test = match.group(2)
            current_content = [line]
            
        elif current_test is not None:
            # We're inside a test failure section
            current_content.append(line)
            
            # Check if we've reached the end of this test
            # (next test starts or we hit the separator followed by whitespace)
            if i + 1 < len(lines):
                next_line = lines[i + 1]
                # If next line is a test header, we're done with current test
                if re.match(pattern, next_line):
                    save_failure(output_path, current_file, current_test, current_content)
                    test_count += 1
                    current_test = None
                    current_file = None
                    current_content = []
    
    # Save last test if exists
    if current_test and current_file and current_content:
        save_failure(output_path, current_file, current_test, current_content)
        test_count += 1
    
    print(f"✓ Processed {test_count} test failures")
    print(f"✓ Output directory: {output_path.absolute()}")


def save_failure(output_path: Path, toml_file: str, test_name: str, content: list):
    """
    Save a single test failure to a file.
    
    Args:
        output_path: Directory to save the file
        toml_file: Name of the TOML file (e.g., "abc.toml")
        test_name: Name of the test (e.g., "abc_basic")
        content: List of lines containing the failure content
    """
    # Create filename: toml_file__test_name.txt
    # e.g., "abc__abc_basic.txt"
    toml_name = toml_file.replace('.toml', '')
    filename = f"{toml_name}__{test_name}.txt"
    filepath = output_path / filename
    
    # Join content and clean up
    failure_content = '\n'.join(content)
    
    # Write to file
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(failure_content)


def main():
    """Main entry point."""
    # Get the project root (assuming script is in scripts/)
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    
    temp_file = project_root / 'temp'
    failures_dir = project_root / 'failures'
    
    # Check if temp file exists
    if not temp_file.exists():
        print(f"Error: temp file not found at {temp_file}")
        return 1
    
    print(f"Processing test failures from: {temp_file}")
    parse_test_failures(str(temp_file), str(failures_dir))
    
    return 0


if __name__ == '__main__':
    exit(main())
