# 012: Optional in Collection Operations

**Priority**: Low  
**Status**: Not Implemented  
**Complexity**: Medium-High

## Problem

When Optional collections are used in operations like iteration, `extend`, `update`, list comprehensions, etc., special handling is needed to unwrap the collection before the operation.

## Current Behavior

```python
@dataclass
class Container:
    items: Optional[List[int]]
    
def extend_items(c: Container, more: List[int]) -> None:
    c.items.extend(more)  # items is Optional[List[int]]
```

**Generated (Incorrect)**:
```rust
pub fn extend_items(c: &mut Container, more: Vec<i32>) {
    c.items.extend(more);  // Error: Option<Vec<i32>> has no method `extend`
}
```

## Expected Behavior

```rust
pub fn extend_items(c: &mut Container, more: Vec<i32>) {
    c.items.as_mut().unwrap().extend(more);
}
```

## Common Patterns

### 1. Iteration over Optional Collection

```python
def process_items(c: Container) -> int:
    total = 0
    for item in c.items:  # items is Optional[List[int]]
        total += item
    return total
```

**Expected**:
```rust
let mut total = 0;
for item in c.items.as_ref().unwrap().iter() {
    total += item;
}
total
```

### 2. Collection Methods on Optional

```python
def modify_container(c: Container) -> None:
    c.items.append(42)
    c.items.extend([1, 2, 3])
    c.items.clear()
```

### 3. List Comprehension with Optional Iterable

```python
def double_items(c: Container) -> List[int]:
    return [x * 2 for x in c.items]
```

**Expected**:
```rust
c.items.as_ref().unwrap().iter().map(|x| x * 2).collect()
```

### 4. Filter None from Collections

```python
def get_ages(people: List[Person]) -> List[int]:
    return [p.age for p in people if p.age is not None]
```

**Expected**:
```rust
people.iter()
    .filter_map(|p| p.age)
    .collect()
```

### 5. Dict Operations on Optional

```python
@dataclass
class Config:
    settings: Optional[Dict[str, str]]

def update_config(c: Config, updates: Dict[str, str]) -> None:
    c.settings.update(updates)
```

## Implementation Approach

### 1. Handle For Loop with Optional Iterable

**File: `crates/depyler-core/src/rust_gen/stmt_gen.rs`**

```rust
fn convert_for(&mut self, target: &str, iter: &HirExpr, body: &[HirStmt]) -> Result<syn::Stmt> {
    let iter_expr = if self.expr_is_optional(iter) {
        let base = self.convert_expr(iter)?;
        parse_quote! { #base.as_ref().unwrap().iter() }
    } else {
        self.convert_expr(iter)?
    };
    
    let body_stmts = self.convert_stmts(body)?;
    let target_ident = format_ident!("{}", target);
    
    Ok(parse_quote! {
        for #target_ident in #iter_expr {
            #(#body_stmts)*
        }
    })
}
```

### 2. Handle Mutable Collection Methods

```rust
fn convert_collection_method(&mut self, receiver: &HirExpr, method: &str, args: &[HirExpr]) -> Result<syn::Expr> {
    // Mutable methods need as_mut().unwrap()
    let mutable_methods = ["append", "extend", "clear", "pop", "insert", "remove", "update"];
    
    if self.expr_is_optional(receiver) && mutable_methods.contains(&method) {
        let receiver_expr = self.convert_expr(receiver)?;
        let args_expr = self.convert_args(args)?;
        return Ok(parse_quote! {
            #receiver_expr.as_mut().unwrap().#method(#(#args_expr),*)
        });
    }
    
    // ... normal handling
}
```

### 3. Handle Comprehensions with Optional

```rust
fn convert_list_comp(&mut self, elt: &HirExpr, generators: &[Comprehension]) -> Result<syn::Expr> {
    for gen in generators {
        if self.expr_is_optional(&gen.iter) {
            // Insert unwrap in the iterator
        }
        
        // Handle condition: if p.age is not None
        if let Some(condition) = &gen.condition {
            if self.is_none_filter_condition(condition) {
                // Use filter_map instead of filter + map
            }
        }
    }
    // ...
}
```

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/stmt_gen.rs` | Handle for loops with Optional iterables |
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Handle comprehensions, collection method calls |

## Tests to Add

```python
def test_for_optional_list():
    @dataclass
    class Container:
        items: Optional[List[int]]
    
    def sum_items(c: Container) -> int:
        total = 0
        for item in c.items:
            total += item
        return total

def test_append_optional_list():
    @dataclass
    class Container:
        items: Optional[List[int]]
    
    def add(c: Container, val: int) -> None:
        c.items.append(val)

def test_extend_optional_list():
    @dataclass
    class Container:
        items: Optional[List[int]]
    
    def add_many(c: Container, vals: List[int]) -> None:
        c.items.extend(vals)

def test_comprehension_optional():
    @dataclass
    class Container:
        items: Optional[List[int]]
    
    def double(c: Container) -> List[int]:
        return [x * 2 for x in c.items]

def test_filter_none():
    def get_values(items: List[Optional[int]]) -> List[int]:
        return [x for x in items if x is not None]
```

## Edge Cases

1. **Nested Optional**: `Optional[List[Optional[int]]]`
2. **Dict comprehension**: `{k: v for k, v in opt_dict.items()}`
3. **Set operations**: `opt_set.add(item)`
4. **Generator expressions**: `sum(x for x in opt_list)`
5. **`in` operator**: `x in opt_list`

## Related Work Items

- Depends on 003 (Method Calls) for collection method handling
- Related to 008 (Conditional Context) for `if x is not None` filtering
- Interacts with 004 (Chained Access) for nested collection access

## Acceptance Criteria

- [ ] For loops over Optional collections work
- [ ] Mutable collection methods (`append`, `extend`, etc.) work
- [ ] List comprehensions with Optional iterables work
- [ ] Filter-map pattern for `if x is not None` recognized
- [ ] Dict operations on Optional dicts work
- [ ] Unit tests passing
