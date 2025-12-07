class Inner:
    def __init__(self):
        self.values: list[int] = []

class Outer:
    def __init__(self):
        self.inner: Inner = Inner()

class State:
    def __init__(self):
        self.data: Outer = Outer()

def mutate_nested_field(state: State, value: int) -> None:
    """Test that nested field mutation properly marks the variable as mutable."""
    # This pattern should generate: let mut obj = state.data.clone();
    obj = state.data
    obj.inner.values.append(value)
