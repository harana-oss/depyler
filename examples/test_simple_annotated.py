#!/usr/bin/env python3
"""Simple test for annotated assignments without values."""

def test_annotated_assignments():
    # Basic types without values
    counter: int
    name: str
    is_valid: bool
    score: float
    
    # Now assign values
    counter = 10
    name = "test"
    is_valid = True
    score = 3.14
    
    # Use them
    print(counter)
    print(name)
    print(is_valid)
    print(score)

if __name__ == "__main__":
    test_annotated_assignments()
