# 005: Optional Auto-Unwrap for Unary Operations

**Priority**: Low  
**Status**: ✅ Implemented  
**Complexity**: Low

## Problem

Unary operators (`-`, `not`, `~`) on Optional fields need unwrapping before the operation can be applied.

## Current Behavior

```python
@dataclass
class Numbers:
    count: Optional[int]
    flag: Optional[bool]

def negate_count(n: Numbers) -> int:
    return -n.count

def invert_flag(n: Numbers) -> bool:
    return not n.flag
```

**Generated (Incorrect)**:
```rust
pub fn negate_count(n: &Numbers) -> i32 {
    return -n.count;  // Error: cannot apply unary `-` to Option<i32>
}
```

## Expected Behavior

```rust
pub fn negate_count(n: &Numbers) -> i32 {
    return -n.count.unwrap();
}

pub fn invert_flag(n: &Numbers) -> bool {
    return !n.flag.unwrap();
}
```

## Implementation Approach

### 1. Modify Unary Operation Handling

## Implementation Location

**File: `crates/depyler-core/src/rust_gen/expr_gen.rs`** (line 835-845)

The `convert_unary()` function checks if operand is Optional and unwraps it:

```rust
fn convert_unary(&mut self, op: &UnaryOp, operand: &HirExpr) -> Result<syn::Expr> {
    let is_optional = self.expr_is_optional(operand);
    let operand_expr = operand.to_rust_expr(self.ctx)?;

    // Unwrap Optional operands for unary operations
    let unwrapped_expr = if is_optional {
        parse_quote! { #operand_expr.unwrap() }
    } else {
        operand_expr.clone()
    };
    // ... handles UnaryOp::Not, UnaryOp::Neg, UnaryOp::Pos, UnaryOp::BitNot
}
```

## Unary Operators

| Python | Rust | Notes |
|--------|------|-------|
| `-x` | `-x` | Numeric negation |
| `not x` | `!x` | Logical NOT |
| `~x` | `!x` | Bitwise NOT (for integers) |
| `+x` | `x` | Unary plus (identity) |

## Files Modified

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Modify `convert_unary()` to check Optional operands |

## Tests to Add

```python
def test_unary_neg_optional():
    @dataclass
    class Numbers:
        value: Optional[int]
    
    def negate(n: Numbers) -> int:
        return -n.value

def test_unary_not_optional():
    @dataclass
    class Flags:
        active: Optional[bool]
    
    def invert(f: Flags) -> bool:
        return not f.active

def test_unary_invert_optional():
    @dataclass
    class Bits:
        mask: Optional[int]
    
    def flip(b: Bits) -> int:
        return ~b.mask
```

## Edge Cases

1. **Chained unary**: `--n.count` (double negation)
2. **Combined with binary**: `-n.count + 5`
3. **In expressions**: `abs(-n.value)`

## Acceptance Criteria

- [ ] Unary `-` unwraps Optional operands
- [ ] Unary `not` unwraps Optional operands
- [ ] Unary `~` unwraps Optional operands
- [ ] Works with field access and variables
- [ ] Unit tests passing
