# Migration Task: String Operations Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_strings.rs | 529 | ~35 | `transpile_and_check` |
| test_string_interoperability.rs | ~200 | ~12 | `transpile_and_check` |
| test_string_literals_os_module.rs | ~80 | ~4 | `transpile_and_check` |
| test_string_optimizations.rs | ~100 | ~5 | `transpile_and_check` |
| test_string_unit.rs | ~150 | ~8 | `transpile_and_check` |
| test_string_constants_unit.rs | ~100 | ~5 | `transpile_and_check` |
| test_str_additional_unit.rs | ~200 | ~12 | `transpile_and_check` |
| test_str_more_unit.rs | ~150 | ~8 | `transpile_and_check` |
| test_formatting.rs | ~120 | ~6 | `transpile_and_check` |

## Target File

`tests/toml/05-string-operations.toml`

## Tests to Migrate

### String Method Mappings

```toml
[[test]]
name = "string_lstrip"
description = "Python lstrip() → Rust trim_start()"

[test.python]
code =  '''
def strip_leading(s: str) -> str:
    return s.lstrip()
'''

[test.assertions]
contains = ["trim_start()"]
not_contains = ["lstrip()"]
```

```toml
[[test]]
name = "string_rstrip"
description = "Python rstrip() → Rust trim_end()"

[test.python]
code =  '''
def strip_trailing(s: str) -> str:
    return s.rstrip()
'''

[test.assertions]
contains = ["trim_end()"]
not_contains = ["rstrip()"]
```

```toml
[[test]]
name = "string_strip"
description = "Python strip() → Rust trim()"

[test.python]
code =  '''
def strip_both(s: str) -> str:
    return s.strip()
'''

[test.assertions]
contains = ["trim()"]
not_contains = ["strip()"]
```

```toml
[[test]]
name = "string_upper"
description = "Python upper() → Rust to_uppercase()"

[test.python]
code =  '''
def to_upper(s: str) -> str:
    return s.upper()
'''

[test.assertions]
contains = ["to_uppercase()"]
not_contains = ["upper()"]
```

```toml
[[test]]
name = "string_lower"
description = "Python lower() → Rust to_lowercase()"

[test.python]
code =  '''
def to_lower(s: str) -> str:
    return s.lower()
'''

[test.assertions]
contains = ["to_lowercase()"]
not_contains = ["lower()"]
```

```toml
[[test]]
name = "string_isalnum"
description = "Python isalnum() → Rust chars().all(is_alphanumeric)"

[test.python]
code =  '''
def is_alphanumeric(s: str) -> bool:
    return s.isalnum()
'''

[test.assertions]
contains = ["chars()", "is_alphanumeric()"]
not_contains = ["isalnum()"]
```

```toml
[[test]]
name = "string_isdigit"
description = "Python isdigit() → Rust chars().all(is_numeric)"

[test.python]
code =  '''
def is_numeric(s: str) -> bool:
    return s.isdigit()
'''

[test.assertions]
contains = ["chars()", "is_numeric"]
not_contains = ["isdigit()"]
```

```toml
[[test]]
name = "string_startswith"
description = "Python startswith() → Rust starts_with()"

[test.python]
code =  '''
def check_prefix(s: str, prefix: str) -> bool:
    return s.startswith(prefix)
'''

[test.assertions]
contains = ["starts_with"]
not_contains = ["startswith"]
```

```toml
[[test]]
name = "string_endswith"
description = "Python endswith() → Rust ends_with()"

[test.python]
code =  '''
def check_suffix(s: str, suffix: str) -> bool:
    return s.endswith(suffix)
'''

[test.assertions]
contains = ["ends_with"]
not_contains = ["endswith"]
```

```toml
[[test]]
name = "string_count"
description = "Python count() → Rust matches().count()"

[test.python]
code =  '''
def count_occurrences(s: str, substring: str) -> int:
    return s.count(substring)
'''

[test.assertions]
contains = ["matches", ".count()"]
```

```toml
[[test]]
name = "string_replace"
description = "Python replace() → Rust replace()"

[test.python]
code =  '''
def replace_text(s: str, old: str, new: str) -> str:
    return s.replace(old, new)
'''

[test.assertions]
contains = ["replace("]
```

```toml
[[test]]
name = "string_split"
description = "Python split() → Rust split()"

[test.python]
code =  '''
def split_words(s: str) -> list[str]:
    return s.split()
'''

[test.assertions]
contains = ["split_whitespace", "collect"]
any_of = ["split_whitespace()", "split(' ')"]
```

```toml
[[test]]
name = "string_split_delimiter"
description = "Python split(delimiter) → Rust split(delimiter)"

[test.python]
code =  '''
def split_by_comma(s: str) -> list[str]:
    return s.split(",")
'''

[test.assertions]
contains = ["split", "collect"]
```

```toml
[[test]]
name = "string_join"
description = "Python join() → Rust join()"

[test.python]
code =  '''
def join_words(words: list[str]) -> str:
    return " ".join(words)
'''

[test.assertions]
contains = ["join"]
```

### String Slicing

```toml
[[test]]
name = "string_slice_last_char"
description = "s[-1] for last character"

[test.python]
code =  '''
def get_last_char(s: str) -> str:
    return s[-1]
'''

[test.assertions]
contains = ["chars()"]
not_contains = [".to_vec()"]
```

```toml
[[test]]
name = "string_slice_last_n"
description = "s[-n:] for last n characters"

[test.python]
code =  '''
def get_last_n_chars(s: str, n: int) -> str:
    return s[-n:]
'''

[test.assertions]
contains = [".chars()", "collect::<String>()"]
not_contains = [".to_vec()", "Vec::new()"]
```

```toml
[[test]]
name = "string_slice_all_but_last"
description = "s[:-n] for all but last n characters"

[test.python]
code =  '''
def get_all_but_last_n(s: str, n: int) -> str:
    return s[:-n]
'''

[test.assertions]
contains = [".chars()", ".take("]
not_contains = [".to_vec()"]
```

```toml
[[test]]
name = "string_reverse"
description = "s[::-1] for string reversal"

[test.python]
code =  '''
def reverse_string(s: str) -> str:
    return s[::-1]
'''

[test.assertions]
contains = [".chars()", ".rev()", "collect::<String>()"]
```

```toml
[[test]]
name = "string_substring"
description = "s[start:stop] for substring"

[test.python]
code =  '''
def substring(s: str, start: int, stop: int) -> str:
    return s[start:stop]
'''

[test.assertions]
contains = [".chars()"]
any_of = [".skip(", ".take("]
```

```toml
[[test]]
name = "string_step_slice"
description = "s[::n] for every nth character"

[test.python]
code =  '''
def every_nth_char(s: str, n: int) -> str:
    return s[::n]
'''

[test.assertions]
contains = [".chars()"]
any_of = [".step_by(", "step"]
```

### F-String Formatting

```toml
[[test]]
name = "fstring_simple"
description = "Simple f-string interpolation"

[test.python]
code =  '''
def greet(name: str) -> str:
    return f"Hello, {name}!"
'''

[test.assertions]
contains = ["format!"]
any_of = ["format!(\"Hello, {}!\"", "format!(\"Hello, {name}!\""]
```

```toml
[[test]]
name = "fstring_expression"
description = "F-string with expression"

[test.python]
code =  '''
def show_sum(a: int, b: int) -> str:
    return f"Sum: {a + b}"
'''

[test.assertions]
contains = ["format!"]
```

```toml
[[test]]
name = "fstring_multiple"
description = "F-string with multiple variables"

[test.python]
code =  '''
def point_str(x: int, y: int) -> str:
    return f"Point({x}, {y})"
'''

[test.assertions]
contains = ["format!"]
```

### String Methods Combination

```toml
[[test]]
name = "string_methods_chain"
description = "Multiple string methods chained"

[test.python]
code =  '''
def process_string(text: str) -> tuple[str, str, bool, int]:
    leading = text.lstrip()
    trailing = text.rstrip()
    is_alnum = text.isalnum()
    count_a = text.count("a")
    return (leading, trailing, is_alnum, count_a)
'''

[test.assertions]
contains = ["trim_start()", "trim_end()", "is_alphanumeric()", "matches"]
```

### String Find/Index

```toml
[[test]]
name = "string_find"
description = "Python find() → Rust find()"

[test.python]
code =  '''
def find_substring(s: str, sub: str) -> int:
    return s.find(sub)
'''

[test.assertions]
contains = ["find("]
any_of = ["Option", "unwrap_or(-1)"]
```

```toml
[[test]]
name = "string_in_operator"
description = "Python 'in' operator for strings"

[test.python]
code =  '''
def contains_substring(s: str, sub: str) -> bool:
    return sub in s
'''

[test.assertions]
contains = ["contains("]
not_contains = [" in "]
```

## Notes

- Python strings are UTF-8, Rust `String` is also UTF-8
- Character indexing in Python may not map 1:1 (grapheme clusters)
- `str.split()` without args splits on whitespace (use `split_whitespace`)
- F-strings become `format!()` macro
- String slicing uses `.chars()` iterator

## Acceptance Criteria

- [ ] All string method tests migrated
- [ ] String slicing tests migrated
- [ ] F-string formatting tests migrated
- [ ] String search/find tests migrated
- [ ] Chained method tests migrated

## Estimated Effort

**Time**: 4-5 hours
**Risk**: Low-Medium (string handling is well-established)
