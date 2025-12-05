# 006: Optional in Augmented Assignment

**Priority**: Low  
**Status**: ✅ Implemented  
**Complexity**: Medium

## Problem

Augmented assignments (`+=`, `-=`, `*=`, etc.) with Optional fields require special handling. The field must be unwrapped for the read operation, the arithmetic performed, and then potentially re-wrapped for the write.

## Current Behavior

```python
@dataclass
class Counter:
    value: Optional[int]

def increment(c: Counter) -> None:
    c.value += 1  # c.value is Optional[int]
```

**Generated (Incorrect)**:
```rust
pub fn increment(c: &mut Counter) {
    c.value += 1;  // Error: cannot add i32 to Option<i32>
}
```

## Expected Behavior

### Option 1: Unwrap-Modify-Rewrap

```rust
pub fn increment(c: &mut Counter) {
    c.value = Some(c.value.unwrap() + 1);
}
```

### Option 2: Use Option Methods

```rust
pub fn increment(c: &mut Counter) {
    c.value = c.value.map(|v| v + 1);
}
```

**Note**: Option 1 maintains Python semantics (panic on None), Option 2 is idiomatic Rust but silently does nothing on None.

## Implementation Location

**File: `crates/depyler-core/src/rust_gen/stmt_gen.rs`**

Two helper functions detect augmented assignment patterns on Optional types:
- `is_optional_attr_augassign_pattern()` (line 2028-2055) - for field access like `obj.field += 1`
- `is_optional_var_augassign_pattern()` (line 2058-2075) - for variables like `x += 1`

These are used in `codegen_assign_stmt()` (line 2269-2335) to generate:
```rust
target = Some(target.unwrap() <op> value);
```

Additionally, the re-wrapping logic in `codegen_assign_stmt` (line 2686-2705) handles the case where CSE transforms `c.value += 1` into:
```rust
let _cse_temp = c.value.unwrap() + 1;
c.value = Some(_cse_temp);
```

## Handled Augmented Operators

| Python | Rust (Normal) | Rust (with Optional) |
|--------|---------------|---------------------|
| `+=` | `+=` | `= Some(_.unwrap() + _)` |
| `-=` | `-=` | `= Some(_.unwrap() - _)` |
| `*=` | `*=` | `= Some(_.unwrap() * _)` |
| `/=` | `/=` | `= Some(_.unwrap() / _)` |
| `//=` | N/A | Integer division handling |
| `%=` | `%=` | `= Some(_.unwrap() % _)` |
| `**=` | N/A | Power operation handling |
| `&=` | `&=` | `= Some(_.unwrap() & _)` |
| `\|=` | `\|=` | `= Some(_.unwrap() \| _)` |

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/stmt_gen.rs` | Modify `convert_aug_assign()` for Optional targets |

## Tests to Add

```python
def test_augment_add_optional():
    @dataclass
    class Counter:
        value: Optional[int]
    
    def increment(c: Counter) -> None:
        c.value += 1

def test_augment_mul_optional():
    @dataclass
    class Numbers:
        factor: Optional[int]
    
    def double(n: Numbers) -> None:
        n.factor *= 2

def test_augment_string_optional():
    @dataclass
    class Builder:
        text: Optional[str]
    
    def append(b: Builder, s: str) -> None:
        b.text += s
```

## Edge Cases

1. **String concatenation**: `str += str` needs different handling
2. **List append**: `list += [item]` is extend operation
3. **Float division**: `/=` may change int to float
4. **Power operator**: `**=` needs `pow()` function call

## Semantic Consideration

Python behavior on `None`:
```python
counter.value += 1  # Raises TypeError: unsupported operand type(s) for +=: 'NoneType' and 'int'
```

Generated Rust with `.unwrap()`:
```rust
c.value = Some(c.value.unwrap() + 1);  // Panics if None
```

This maintains semantic equivalence - both fail at runtime on None.

## Acceptance Criteria

- [ ] Augmented assignments on Optional fields generate correct code
- [ ] All arithmetic aug-ops handled (`+=`, `-=`, `*=`, `/=`, `%=`)
- [ ] Bitwise aug-ops handled (`&=`, `|=`, `^=`)
- [ ] String concat aug-op handled
- [ ] Result re-wrapped in `Some()` after operation
- [ ] Unit tests passing
