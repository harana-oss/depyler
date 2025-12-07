# Migration Task: Collections Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_collections_unit.rs | ~150 | ~8 | `transpile_and_check` |
| test_counter_unit.rs | ~120 | ~6 | `transpile_and_check` |
| test_defaultdict_unit.rs | ~80 | ~4 | `transpile_and_check` |
| test_deque_unit.rs | ~100 | ~5 | `transpile_and_check` |

## Target File

`tests/toml/25-collections-module.toml`

## Tests to Migrate

### Counter

```toml
[[test]]
name = "counter_from_list"
description = "Counter from list initialization"

[test.python]
code =  '''
from collections import Counter

def count_items(items: list[str]) -> dict[str, int]:
    return dict(Counter(items))
'''

[test.assertions]
any_of = ["HashMap", "count", "entry"]
```

```toml
[[test]]
name = "counter_most_common"
description = "Counter.most_common() method"

[test.python]
code =  '''
from collections import Counter

def top_items(items: list[str], n: int) -> list[tuple[str, int]]:
    return Counter(items).most_common(n)
'''

[test.assertions]
any_of = ["sort", "HashMap", "Vec"]
```

```toml
[[test]]
name = "counter_elements"
description = "Counter.elements() iteration"

[test.python]
code =  '''
from collections import Counter

def expand_counts(counts: dict[str, int]) -> list[str]:
    c = Counter(counts)
    return list(c.elements())
'''

[test.assertions]
any_of = ["flat_map", "repeat", "iter"]
```

### DefaultDict

```toml
[[test]]
name = "defaultdict_int"
description = "defaultdict with int factory"

[test.python]
code =  '''
from collections import defaultdict

def count_words(words: list[str]) -> dict[str, int]:
    counts = defaultdict(int)
    for word in words:
        counts[word] += 1
    return dict(counts)
'''

[test.assertions]
any_of = ["entry", "or_insert", "HashMap"]
```

```toml
[[test]]
name = "defaultdict_list"
description = "defaultdict with list factory"

[test.python]
code =  '''
from collections import defaultdict

def group_by_first_char(words: list[str]) -> dict[str, list[str]]:
    groups = defaultdict(list)
    for word in words:
        groups[word[0]].append(word)
    return dict(groups)
'''

[test.assertions]
any_of = ["entry", "or_insert_with", "HashMap", "Vec"]
```

### Deque

```toml
[[test]]
name = "deque_creation"
description = "deque creation and initialization"

[test.python]
code =  '''
from collections import deque

def make_deque(items: list[int]) -> deque:
    return deque(items)
'''

[test.assertions]
any_of = ["VecDeque", "from"]
```

```toml
[[test]]
name = "deque_append"
description = "deque append operations"

[test.python]
code =  '''
from collections import deque

def add_items(d: deque, item: int) -> None:
    d.append(item)
'''

[test.assertions]
any_of = ["push_back", "VecDeque"]
```

```toml
[[test]]
name = "deque_appendleft"
description = "deque appendleft operation"

[test.python]
code =  '''
from collections import deque

def add_front(d: deque, item: int) -> None:
    d.appendleft(item)
'''

[test.assertions]
any_of = ["push_front", "VecDeque"]
```

```toml
[[test]]
name = "deque_pop"
description = "deque pop operations"

[test.python]
code =  '''
from collections import deque

def remove_last(d: deque) -> int:
    return d.pop()
'''

[test.assertions]
any_of = ["pop_back", "VecDeque"]
```

```toml
[[test]]
name = "deque_popleft"
description = "deque popleft operation"

[test.python]
code =  '''
from collections import deque

def remove_first(d: deque) -> int:
    return d.popleft()
'''

[test.assertions]
any_of = ["pop_front", "VecDeque"]
```

```toml
[[test]]
name = "deque_rotate"
description = "deque rotate operation"

[test.python]
code =  '''
from collections import deque

def rotate_right(d: deque, n: int) -> None:
    d.rotate(n)
'''

[test.assertions]
any_of = ["rotate", "VecDeque"]
```

### Named Tuple

```toml
[[test]]
name = "namedtuple_creation"
description = "namedtuple definition and creation"

[test.python]
code =  '''
from collections import namedtuple

Point = namedtuple('Point', ['x', 'y'])

def make_point(x: int, y: int) -> Point:
    return Point(x, y)
'''

[test.assertions]
any_of = ["struct Point", "struct"]
```

```toml
[[test]]
name = "namedtuple_access"
description = "namedtuple field access"

[test.python]
code =  '''
from collections import namedtuple

Point = namedtuple('Point', ['x', 'y'])

def get_x(p: Point) -> int:
    return p.x
'''

[test.assertions]
any_of = [".x", "struct"]
```

### OrderedDict

```toml
[[test]]
name = "ordereddict_creation"
description = "OrderedDict creation"

[test.python]
code =  '''
from collections import OrderedDict

def make_ordered() -> OrderedDict:
    return OrderedDict([("a", 1), ("b", 2)])
'''

[test.assertions]
any_of = ["IndexMap", "BTreeMap", "LinkedHashMap"]
```

## Notes

- Python `Counter` → Rust `HashMap` with counting logic
- Python `defaultdict` → Rust `HashMap` with `.entry().or_insert()`
- Python `deque` → Rust `VecDeque`
- Python `namedtuple` → Rust `struct`
- Python `OrderedDict` → Rust `IndexMap` or `BTreeMap`

## Acceptance Criteria

- [ ] Counter tests migrated
- [ ] DefaultDict tests migrated
- [ ] Deque tests migrated
- [ ] Named tuple tests migrated
- [ ] OrderedDict tests migrated

## Estimated Effort

**Time**: 2-3 hours
**Risk**: Low
