# Migration Task: Regex Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_re_unit.rs | ~180 | ~10 | `transpile_and_check` |
| test_fnmatch_unit.rs | ~60 | ~3 | `transpile_and_check` |
| test_glob_unit.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/28-regex-module.toml`

## Tests to Migrate

### Basic Pattern Matching

```toml
[[test]]
name = "re_match"
description = "re.match() pattern at start"

[test.python]
code =  '''
import re

def starts_with_hello(s: str) -> bool:
    return re.match(r"^hello", s) is not None
'''

[test.assertions]
any_of = ["Regex::new", "is_match(", "starts_with"]
```

```toml
[[test]]
name = "re_search"
description = "re.search() pattern anywhere"

[test.python]
code =  '''
import re

def contains_number(s: str) -> bool:
    return re.search(r"\d+", s) is not None
'''

[test.assertions]
any_of = ["Regex::new", "is_match(", "find("]
```

```toml
[[test]]
name = "re_fullmatch"
description = "re.fullmatch() complete string match"

[test.python]
code =  '''
import re

def is_valid_email(s: str) -> bool:
    pattern = r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$"
    return re.fullmatch(pattern, s) is not None
'''

[test.assertions]
any_of = ["Regex::new", "is_match(", "^", "$"]
```

### Pattern Finding

```toml
[[test]]
name = "re_findall"
description = "re.findall() all matches"

[test.python]
code =  '''
import re

def find_all_numbers(s: str) -> list[str]:
    return re.findall(r"\d+", s)
'''

[test.assertions]
any_of = ["find_iter(", "captures_iter(", "collect()"]
```

```toml
[[test]]
name = "re_finditer"
description = "re.finditer() match iterator"

[test.python]
code =  '''
import re

def find_word_positions(s: str, word: str) -> list[int]:
    return [m.start() for m in re.finditer(word, s)]
'''

[test.assertions]
any_of = ["find_iter(", "start()", "collect()"]
```

### Capture Groups

```toml
[[test]]
name = "re_groups"
description = "regex capture groups extraction"

[test.python]
code =  '''
import re

def extract_date_parts(s: str) -> tuple[str, str, str]:
    m = re.match(r"(\d{4})-(\d{2})-(\d{2})", s)
    if m:
        return (m.group(1), m.group(2), m.group(3))
    return ("", "", "")
'''

[test.assertions]
any_of = ["captures(", "get(", ".as_str()"]
```

```toml
[[test]]
name = "re_named_groups"
description = "regex named capture groups"

[test.python]
code =  '''
import re

def extract_name_value(s: str) -> dict[str, str]:
    m = re.match(r"(?P<key>\w+)=(?P<value>\w+)", s)
    if m:
        return {"key": m.group("key"), "value": m.group("value")}
    return {}
'''

[test.assertions]
any_of = ["captures(", "name(", "?P<"]
```

### Pattern Substitution

```toml
[[test]]
name = "re_sub"
description = "re.sub() pattern replacement"

[test.python]
code =  '''
import re

def redact_numbers(s: str) -> str:
    return re.sub(r"\d+", "XXX", s)
'''

[test.assertions]
any_of = ["replace_all(", "replace("]
```

```toml
[[test]]
name = "re_subn"
description = "re.subn() with count"

[test.python]
code =  '''
import re

def replace_first_number(s: str) -> str:
    result, count = re.subn(r"\d+", "N", s, count=1)
    return result
'''

[test.assertions]
any_of = ["replace(", "replacen("]
```

### Pattern Splitting

```toml
[[test]]
name = "re_split"
description = "re.split() by pattern"

[test.python]
code =  '''
import re

def split_on_whitespace(s: str) -> list[str]:
    return re.split(r"\s+", s)
'''

[test.assertions]
any_of = ["split(", ".split_whitespace()"]
```

### Compiled Patterns

```toml
[[test]]
name = "re_compile"
description = "re.compile() pattern caching"

[test.python]
code =  '''
import re

pattern = re.compile(r"\d+")

def find_numbers(s: str) -> list[str]:
    return pattern.findall(s)
'''

[test.assertions]
any_of = ["Regex::new", "lazy_static!", "once_cell"]
```

### Glob Patterns

```toml
[[test]]
name = "glob_match"
description = "glob pattern matching"

[test.python]
code =  '''
import glob

def find_python_files(directory: str) -> list[str]:
    return glob.glob(f"{directory}/*.py")
'''

[test.assertions]
any_of = ["glob::", "Pattern", "read_dir"]
```

```toml
[[test]]
name = "glob_recursive"
description = "glob ** recursive matching"

[test.python]
code =  '''
import glob

def find_all_python_files(directory: str) -> list[str]:
    return glob.glob(f"{directory}/**/*.py", recursive=True)
'''

[test.assertions]
any_of = ["glob::", "WalkDir", "recursive"]
```

### Fnmatch Patterns

```toml
[[test]]
name = "fnmatch_match"
description = "fnmatch filename matching"

[test.python]
code =  '''
import fnmatch

def matches_pattern(filename: str, pattern: str) -> bool:
    return fnmatch.fnmatch(filename, pattern)
'''

[test.assertions]
any_of = ["Pattern::new", "matches("]
```

```toml
[[test]]
name = "fnmatch_filter"
description = "fnmatch.filter() list filtering"

[test.python]
code =  '''
import fnmatch

def filter_py_files(files: list[str]) -> list[str]:
    return fnmatch.filter(files, "*.py")
'''

[test.assertions]
any_of = ["filter(", "Pattern", "ends_with"]
```

## Notes

- Python `re` module → Rust `regex` crate
- `re.match()` → `Regex::new().find()` with `^` anchor
- `re.search()` → `Regex::new().is_match()` or `.find()`
- `re.findall()` → `regex.find_iter().collect()`
- `re.sub()` → `regex.replace_all()`
- `re.split()` → `regex.split()`
- `re.compile()` → `Regex::new()` with `lazy_static!` or `once_cell` for caching
- Python `glob` module → Rust `glob` crate
- Python `fnmatch` module → Rust `glob::Pattern`

## Acceptance Criteria

- [ ] Basic pattern matching tests migrated
- [ ] Pattern finding tests migrated
- [ ] Capture group tests migrated
- [ ] Pattern substitution tests migrated
- [ ] Pattern splitting tests migrated
- [ ] Compiled pattern tests migrated
- [ ] Glob pattern tests migrated
- [ ] Fnmatch pattern tests migrated

## Estimated Effort

**Time**: 2-3 hours
**Risk**: Low
