# Depyler Test Implementation Plan
## Comprehensive Python Language Coverage for Python-to-Rust Transpilation

### Executive Summary

This plan identifies **75+ missing test categories** to achieve exhaustive Python language coverage. Tests are organized by priority based on frequency of use and translation complexity.

---

## Current Coverage Analysis

Your existing tests cover:
- Basic types, operators, control flow
- Functions, lambdas, comprehensions
- Classes (basic), exceptions
- Collections (lists, dicts, iterators, generators)
- Pattern matching, slicing
- Several stdlib modules

---

## Missing Test Categories

### Priority 1: Core Language Constructs

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `walrus-operator.toml` | Assignment expressions (`:=`) in conditions, comprehensions, loops | `let` bindings in expressions |
| `unpacking.toml` | Extended iterable unpacking (`*rest`, `a, *b, c = x`), starred expressions in calls | Pattern matching, `Vec` splitting |
| `augmented-assignment.toml` | `+=`, `-=`, `*=`, `/=`, `//=`, `%=`, `**=`, `&=`, `\|=`, `^=`, `>>=`, `<<=` | Direct operator equivalents |
| `chained-comparisons.toml` | `a < b < c`, `a == b != c` | Expanded boolean chains |
| `ternary-expressions.toml` | `x if cond else y`, nested ternaries | `if`/`else` expressions |
| `global-nonlocal.toml` | `global` and `nonlocal` statements | `Rc<RefCell<T>>`, closures |
| `del-statement.toml` | `del` for variables, list items, slices, attributes, dict keys | `drop()`, `remove()`, `Option::take()` |
| `assert-statement.toml` | `assert` with/without messages | `assert!`, `debug_assert!` |
| `binary-literals.toml` | `0b1010`, `0o777`, `0xFF`, underscores in numbers | Direct literal translation |
| `complex-numbers.toml` | Complex literals `3+4j`, arithmetic | `num::Complex` |
| `ellipsis.toml` | `...` as placeholder, in slicing, type hints | Unit type, slice markers |

### Priority 2: Advanced OOP

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `metaclasses.toml` | `type()`, custom metaclasses, `__new__`, `__prepare__` | Traits, proc macros (limited) |
| `descriptors.toml` | `__get__`, `__set__`, `__delete__`, `__set_name__` | Custom `Deref`/`DerefMut` |
| `properties.toml` | `@property`, `@x.setter`, `@x.deleter` | Methods, builder pattern |
| `class-methods.toml` | `@classmethod`, `@staticmethod` | Associated functions |
| `multiple-inheritance.toml` | MRO, diamond problem, `super()` with MI | Trait composition |
| `slots.toml` | `__slots__` memory optimization | Struct fields (default) |
| `new-vs-init.toml` | `__new__` for immutable types, singletons | `new()` constructors |
| `abc.toml` | Abstract base classes, `@abstractmethod` | Traits with required methods |
| `init-subclass.toml` | `__init_subclass__`, class registration | Trait implementations |
| `class-getitem.toml` | `__class_getitem__` for subscriptable classes | Generics |

### Priority 3: Dunder Methods (Comprehensive)

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `dunder-lifecycle.toml` | `__new__`, `__init__`, `__del__`, `__repr__`, `__str__` | `new()`, `Drop`, `Debug`, `Display` |
| `dunder-comparison.toml` | `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`, `__ge__`, `__hash__` | `PartialEq`, `Eq`, `Ord`, `Hash` |
| `dunder-arithmetic.toml` | `__add__`, `__sub__`, `__mul__`, `__truediv__`, `__floordiv__`, `__mod__`, `__pow__` | `Add`, `Sub`, `Mul`, `Div`, `Rem` |
| `dunder-reflected.toml` | `__radd__`, `__rsub__`, etc. (right-hand operations) | Trait impls with type params |
| `dunder-inplace.toml` | `__iadd__`, `__isub__`, etc. (in-place operations) | `AddAssign`, `SubAssign`, etc. |
| `dunder-unary.toml` | `__neg__`, `__pos__`, `__abs__`, `__invert__` | `Neg`, `Not`, custom traits |
| `dunder-bitwise.toml` | `__and__`, `__or__`, `__xor__`, `__lshift__`, `__rshift__` | `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr` |
| `dunder-container.toml` | `__len__`, `__getitem__`, `__setitem__`, `__delitem__`, `__contains__` | `Index`, `IndexMut`, iterator |
| `dunder-iteration.toml` | `__iter__`, `__next__`, `__reversed__` | `Iterator`, `IntoIterator` |
| `dunder-callable.toml` | `__call__` | `Fn`, `FnMut`, `FnOnce` |
| `dunder-context.toml` | `__enter__`, `__exit__` | `Drop`, RAII pattern |
| `dunder-attribute.toml` | `__getattr__`, `__setattr__`, `__delattr__`, `__getattribute__` | Custom accessor methods |
| `dunder-format.toml` | `__format__`, `__bytes__`, `__bool__` | `Display`, `Into<Vec<u8>>`, bool conversion |

### Priority 4: Modern Python Features (3.8+)

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `positional-only-args.toml` | `/` syntax for positional-only parameters | N/A (all positional by default) |
| `keyword-only-args.toml` | `*` syntax for keyword-only parameters | Builder pattern |
| `type-parameter-lists.toml` | Python 3.12+ `def f[T](x: T) -> T` | Generic functions |
| `type-alias-statement.toml` | `type` soft keyword (3.12+) | `type` aliases |
| `exception-groups.toml` | `ExceptionGroup`, `except*` (3.11+) | `Result` composition |
| `match-guards.toml` | `case x if x > 0:` | Match guards |
| `match-class-patterns.toml` | `case Point(x=0, y=y):` | Struct patterns |
| `match-mapping-patterns.toml` | `case {"key": value, **rest}:` | HashMap destructuring |
| `match-or-patterns.toml` | `case 1 \| 2 \| 3:` | `\|` in patterns |
| `match-as-patterns.toml` | `case [x, y] as point:` | `@` binding |

### Priority 5: Async/Await

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `async-functions.toml` | `async def`, basic coroutines | `async fn` |
| `await-expressions.toml` | `await` semantics, awaitable protocol | `.await` |
| `async-for.toml` | Async iteration, `__aiter__`, `__anext__` | `Stream` trait |
| `async-with.toml` | Async context managers | `async` RAII |
| `async-generators.toml` | `async def` with `yield` | `async_stream` crate |
| `async-comprehensions.toml` | `[x async for x in ...]` | `stream.map().collect()` |
| `asyncio-tasks.toml` | `asyncio.create_task`, `gather`, `wait` | `tokio::spawn`, `join!` |

### Priority 6: Context Managers

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `context-managers.toml` | `__enter__`, `__exit__`, exception handling | RAII, `Drop` |
| `contextlib-decorator.toml` | `@contextmanager` generator-based | Custom wrapper |
| `contextlib-utilities.toml` | `closing`, `suppress`, `redirect_stdout`, `nullcontext` | Stdlib equivalents |
| `nested-context-managers.toml` | Multiple `with` targets | Nested scopes |
| `parenthesized-context.toml` | Multi-line `with` (3.10+) | Multiple `let` bindings |

### Priority 7: Decorators & Closures

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `decorators-basic.toml` | Function decorators, stacking order | Wrapper functions, macros |
| `decorators-with-args.toml` | Parameterized decorators | Macro with params |
| `decorators-class.toml` | Class decorators | Derive macros |
| `functools-wraps.toml` | Metadata preservation | Proc macro attributes |
| `functools-cache.toml` | `@cache`, `@lru_cache` | `cached` crate |
| `functools-partial.toml` | `partial()`, `partialmethod()` | Closures capturing args |
| `closures.toml` | Free variables, capture semantics | Move/borrow closures |
| `closures-mutable.toml` | Mutable closures with `nonlocal` | `FnMut`, `RefCell` |
| `closures-late-binding.toml` | Late binding gotchas | Explicit capture |

### Priority 8: Data Structures

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `bytes-bytearray.toml` | Bytes literals, methods, encoding | `Vec<u8>`, `&[u8]` |
| `memoryview.toml` | Buffer protocol, zero-copy | Slices |
| `frozenset.toml` | Immutable sets | `HashSet` (wrapped) |
| `sets-advanced.toml` | Set operations, frozen members | `HashSet` methods |
| `namedtuple.toml` | `collections.namedtuple` | Structs |
| `typing-namedtuple.toml` | `typing.NamedTuple` | Structs with derives |
| `dataclasses.toml` | `@dataclass` decorator | Struct + derives |
| `dataclasses-field.toml` | `field()`, `default_factory` | `Default` trait |
| `dataclasses-frozen.toml` | Immutable dataclasses | No `mut` |
| `dataclasses-slots.toml` | `slots=True` | Default struct layout |
| `dataclasses-post-init.toml` | `__post_init__` | Builder pattern |
| `enums.toml` | `Enum`, member access | `enum` |
| `enums-int.toml` | `IntEnum`, `IntFlag` | `enum` with repr |
| `enums-flag.toml` | `Flag`, bitwise operations | Bitflags crate |
| `enums-auto.toml` | `auto()`, custom values | Explicit discriminants |
| `deque.toml` | `collections.deque` | `VecDeque` |
| `counter.toml` | `collections.Counter` | `HashMap` with counts |
| `defaultdict.toml` | `collections.defaultdict` | `HashMap::entry()` |
| `ordereddict.toml` | `collections.OrderedDict` | `IndexMap` crate |
| `chainmap.toml` | `collections.ChainMap` | Custom implementation |

### Priority 9: String Features

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `fstrings.toml` | F-string expressions, format specs | `format!` macro |
| `fstrings-debug.toml` | `f"{x=}"` debug syntax | `format!("{x:?}")` |
| `fstrings-nested.toml` | Nested f-strings, nested quotes | Nested `format!` |
| `raw-strings.toml` | `r""` strings | `r""` raw strings |
| `bytes-strings.toml` | `b""` literals | `b""` byte literals |
| `unicode-strings.toml` | Unicode escapes, identifiers | UTF-8 strings |
| `string-methods-all.toml` | Complete str method coverage | String methods |
| `format-spec.toml` | Format specification mini-language | Format traits |
| `template-strings.toml` | `string.Template` | Custom formatter |

### Priority 10: Import System

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `imports-basic.toml` | `import x`, `import x as y` | `use` statements |
| `imports-from.toml` | `from x import y`, `from x import *` | `use x::y` |
| `imports-relative.toml` | `.module`, `..package` | `super::`, `crate::` |
| `imports-dynamic.toml` | `__import__()`, `importlib` | Runtime modules (limited) |
| `packages-init.toml` | `__init__.py` semantics | `mod.rs` |
| `packages-all.toml` | `__all__` for `import *` | `pub use` |
| `namespace-packages.toml` | PEP 420 namespace packages | Workspace crates |

### Priority 11: Type System

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `type-annotations.toml` | Variable and function annotations | Type annotations |
| `typing-basic.toml` | `List`, `Dict`, `Tuple`, `Set` | `Vec`, `HashMap`, tuple, `HashSet` |
| `typing-optional.toml` | `Optional[X]`, `X \| None` | `Option<X>` |
| `typing-union.toml` | `Union[X, Y]`, `X \| Y` | `enum` |
| `typing-literal.toml` | `Literal["a", "b"]` | Const generics (limited) |
| `typing-final.toml` | `Final`, `@final` | No override |
| `typing-typevar.toml` | `TypeVar`, bounds, constraints | Generics with bounds |
| `typing-generic.toml` | `Generic[T]`, inheritance | Generic structs |
| `typing-protocol.toml` | `Protocol` for structural typing | Traits |
| `typing-callable.toml` | `Callable[[Args], Return]` | `Fn` traits |
| `typing-typeddict.toml` | `TypedDict` | Structs |
| `typing-newtype.toml` | `NewType` | Newtype pattern |
| `typing-annotated.toml` | `Annotated[X, metadata]` | Attributes |
| `typing-self.toml` | `Self` type (3.11+) | `Self` |
| `typing-never.toml` | `Never`, `NoReturn` | `!` never type |

### Priority 12: Built-in Functions

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `builtins-introspection.toml` | `dir()`, `vars()`, `type()`, `id()` | Limited runtime info |
| `builtins-attributes.toml` | `getattr()`, `setattr()`, `hasattr()`, `delattr()` | Method calls |
| `builtins-isinstance.toml` | `isinstance()`, `issubclass()` | Trait bounds |
| `builtins-callable.toml` | `callable()` | `Fn` trait check |
| `builtins-eval-exec.toml` | `eval()`, `exec()`, `compile()` | Not supported |
| `builtins-iter.toml` | `iter()`, `next()`, sentinels | `Iterator` |
| `builtins-reversed.toml` | `reversed()` | `.rev()` |
| `builtins-enumerate.toml` | `enumerate()` | `.enumerate()` |
| `builtins-zip.toml` | `zip()`, `zip_longest` | `.zip()`, `itertools` |
| `builtins-map.toml` | `map()` | `.map()` |
| `builtins-filter.toml` | `filter()` | `.filter()` |
| `builtins-reduce.toml` | `functools.reduce()` | `.fold()` |
| `builtins-sorted.toml` | `sorted()`, key functions | `.sorted()`, `sort_by_key()` |
| `builtins-min-max.toml` | `min()`, `max()`, key/default | `.min()`, `.max()` |
| `builtins-sum.toml` | `sum()`, start value | `.sum()` |
| `builtins-all-any.toml` | `all()`, `any()` | `.all()`, `.any()` |
| `builtins-abs.toml` | `abs()` | `.abs()` |
| `builtins-divmod.toml` | `divmod()` | `(a / b, a % b)` |
| `builtins-pow.toml` | `pow()`, three-arg modular | `.pow()`, modular |
| `builtins-round.toml` | `round()`, banker's rounding | `.round()` |
| `builtins-format.toml` | `format()` | `format!` |
| `builtins-repr.toml` | `repr()`, `ascii()` | `Debug` |
| `builtins-hash.toml` | `hash()` | `Hash` trait |
| `builtins-len.toml` | `len()` | `.len()` |
| `builtins-range.toml` | `range()`, step, negative | `Range`, `(start..end).step_by()` |
| `builtins-slice.toml` | `slice()` objects | Range types |
| `builtins-open.toml` | `open()`, modes, encoding | `File::open()`, `BufReader` |
| `builtins-input.toml` | `input()` | `std::io::stdin()` |
| `builtins-print.toml` | `print()`, sep, end, file, flush | `print!`, `println!` |

### Priority 13: Edge Cases & Semantics

| File | Description | Rust Mapping |
|------|-------------|--------------|
| `name-mangling.toml` | `__private` → `_Class__private` | Private fields |
| `variable-shadowing.toml` | Built-in shadowing, nested scopes | Shadowing (allowed) |
| `late-binding.toml` | Closure late binding gotchas | Explicit capture |
| `identity-equality.toml` | `is` vs `==`, interning | `ptr::eq` vs `==` |
| `short-circuit.toml` | `and`/`or` evaluation, return values | `&&`, `\|\|` |
| `truthiness.toml` | `__bool__`, `__len__`, falsy values | `Into<bool>` |
| `else-on-loops.toml` | `for...else`, `while...else` | Flag variables |
| `exception-chaining.toml` | `raise from`, `__cause__`, `__context__` | Error chaining |
| `finally-semantics.toml` | `finally` with `return`, `break`, `continue` | `Drop` |
| `attribute-resolution.toml` | MRO for attribute lookup | Trait resolution |
| `mutable-default-args.toml` | Mutable default argument gotcha | Clone defaults |
| `reference-semantics.toml` | Object identity, aliasing | References vs values |
| `copy-deepcopy.toml` | `copy.copy()`, `copy.deepcopy()` | `Clone` |
| `weak-references.toml` | `weakref` module | `Weak<T>` |
| `garbage-collection.toml` | Cyclic references, `gc` module | Not applicable |

---

## Implementation Schedule

### Phase 1: Foundation (Weeks 1-2)
**Focus: Most commonly used features**

1. `walrus-operator.toml`
2. `unpacking.toml`
3. `fstrings.toml`
4. `ternary-expressions.toml`
5. `augmented-assignment.toml`
6. `properties.toml`
7. `decorators-basic.toml`
8. `closures.toml`
9. `context-managers.toml`
10. `dataclasses.toml`

### Phase 2: OOP Deep Dive (Weeks 3-4)
**Focus: Advanced class features**

1. All `dunder-*.toml` files
2. `descriptors.toml`
3. `metaclasses.toml`
4. `abc.toml`
5. `multiple-inheritance.toml`
6. `class-methods.toml`
7. `init-subclass.toml`

### Phase 3: Modern Features (Weeks 5-6)
**Focus: Python 3.8+ features**

1. All `match-*.toml` files
2. `positional-only-args.toml`
3. `keyword-only-args.toml`
4. All `async-*.toml` files
5. `exception-groups.toml`

### Phase 4: Type System (Week 7)
**Focus: Type annotations**

1. All `typing-*.toml` files
2. `type-annotations.toml`

### Phase 5: Completeness (Weeks 8-9)
**Focus: Edge cases and builtins**

1. All `builtins-*.toml` files
2. All edge case files
3. Remaining data structures

### Phase 6: Polish (Week 10)
**Focus: Integration and refinement**

1. Cross-file interaction tests
2. Large program tests
3. Performance regression tests

---

## Validation Criteria

Each test file should verify:

1. **Syntactic correctness**: Generated Rust compiles
2. **Semantic equivalence**: Same behavior as Python
3. **Type safety**: Proper Rust types inferred
4. **Idiomatic output**: Generated code follows Rust conventions
5. **Edge cases**: Boundary conditions handled

Have meaningful assertions. No checking for single keywords.

---

## Notes on Untranslatable Features

Some Python features cannot be directly transpiled:

| Feature | Reason | Recommendation |
|---------|--------|----------------|
| `eval()`, `exec()` | Runtime code execution | Error with suggestion |
| Full metaclasses | Requires runtime type system | Partial support via macros |
| Dynamic attribute access | No runtime reflection | Require type hints |
| `__getattr__` fallback | Runtime dispatch | Trait-based alternative |
| Monkey patching | Open classes | Not supported |
| `gc` module | Different memory model | Ignore |
| `weakref` | Complex semantics | `Weak<T>` for simple cases |

Mark these with `skip = true` and document the limitation.

---

## Success Metrics

- [ ] 100% coverage of Python grammar productions
- [ ] 90%+ of stdlib type annotations testable
- [ ] All Python 3.10 features covered
- [ ] Key Python 3.11/3.12 features covered
- [ ] < 5% of tests marked as unsupported
- [ ] All tests pass on CI

---

## Appendix: Python Grammar Coverage Checklist

### Statements
- [x] `if` / `elif` / `else`
- [x] `while`
- [x] `for`
- [x] `try` / `except` / `finally`
- [x] `with`
- [x] `match` / `case`
- [x] `def`
- [x] `class`
- [ ] `async def`
- [ ] `async for`
- [ ] `async with`
- [x] `return`
- [ ] `yield`
- [ ] `yield from`
- [x] `raise`
- [x] `break`
- [x] `continue`
- [x] `import`
- [x] `from ... import`
- [ ] `global`
- [ ] `nonlocal`
- [x] `pass`
- [ ] `del`
- [ ] `assert`
- [ ] `type` (3.12+)

### Expressions
- [x] Literals (int, float, string, bytes)
- [x] Formatted strings (f-strings)
- [x] Lists, dicts, sets, tuples
- [x] Comprehensions
- [ ] Generator expressions (partial)
- [x] Lambda
- [x] Binary operators
- [x] Unary operators
- [x] Comparison operators
- [ ] Chained comparisons
- [x] Boolean operators
- [ ] Assignment expressions (`:=`)
- [x] Conditional expressions
- [x] Subscript
- [x] Slicing
- [x] Attribute access
- [x] Call
- [ ] Starred expressions
- [ ] `await`

### Patterns (match)
- [x] Literal patterns
- [x] Capture patterns
- [x] Wildcard pattern
- [ ] Class patterns
- [ ] Sequence patterns
- [ ] Mapping patterns
- [ ] OR patterns
- [ ] AS patterns
- [ ] Guards