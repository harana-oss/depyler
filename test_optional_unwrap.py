from typing import Optional

def test_optional_unwrap(value: Optional[int]) -> int:
    # This pattern should unwrap the optional
    return value if value is not None else 0
