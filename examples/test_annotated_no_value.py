#!/usr/bin/env python3
"""Test annotated assignments without initial values."""

# Basic types without values
counter: int
name: str
is_valid: bool
score: float

# Collections without values
items: list[int]
mapping: dict[str, int]
unique_values: set[str]

# Custom type without value
class FieldPosition:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

field_position: FieldPosition
new_team: str
valid: bool

# Optional type without value
from typing import Optional
maybe_value: Optional[str]

# Now assign values
counter = 10
name = "test"
is_valid = True
score = 3.14

items = [1, 2, 3]
mapping = {"a": 1, "b": 2}
unique_values = {"x", "y", "z"}

field_position = FieldPosition(5, 10)
new_team = "TeamA"
valid = True
maybe_value = "found"

# Test usage
print(f"Counter: {counter}")
print(f"Name: {name}")
print(f"Valid: {is_valid}")
print(f"Score: {score}")
print(f"Items: {items}")
print(f"Mapping: {mapping}")
print(f"Unique values: {unique_values}")
print(f"Field position: ({field_position.x}, {field_position.y})")
print(f"Team: {new_team}")
print(f"Valid flag: {valid}")
print(f"Maybe value: {maybe_value}")
