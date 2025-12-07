# Migration Task: OS and Sys Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_os_unit.rs | ~150 | ~8 | `transpile_and_check` |
| test_os_additional_unit.rs | ~100 | ~5 | `transpile_and_check` |
| test_sys_unit.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/23-os-sys-modules.toml`

## Tests to Migrate

### OS Environment

```toml
[[test]]
name = "os_environ_get"
description = "os.environ.get() for environment variable"

[test.python]
code =  '''
import os

def get_env(key: str) -> str:
    return os.environ.get(key, "")
'''

[test.assertions]
any_of = ["std::env::var", "env::var"]
```

```toml
[[test]]
name = "os_getenv"
description = "os.getenv() function"

[test.python]
code =  '''
import os

def get_home() -> str:
    return os.getenv("HOME", "/tmp")
'''

[test.assertions]
any_of = ["std::env::var", "env::var", "unwrap_or"]
```

### OS Path Operations

```toml
[[test]]
name = "os_path_join"
description = "os.path.join() for paths"

[test.python]
code =  '''
import os.path

def make_path(directory: str, filename: str) -> str:
    return os.path.join(directory, filename)
'''

[test.assertions]
any_of = ["Path::new", "PathBuf", "join("]
```

```toml
[[test]]
name = "os_path_exists"
description = "os.path.exists() check"

[test.python]
code =  '''
import os.path

def file_exists(path: str) -> bool:
    return os.path.exists(path)
'''

[test.assertions]
any_of = ["Path::new", "exists()"]
```

```toml
[[test]]
name = "os_path_isfile"
description = "os.path.isfile() check"

[test.python]
code =  '''
import os.path

def is_regular_file(path: str) -> bool:
    return os.path.isfile(path)
'''

[test.assertions]
any_of = ["is_file()", "metadata"]
```

```toml
[[test]]
name = "os_path_isdir"
description = "os.path.isdir() check"

[test.python]
code =  '''
import os.path

def is_directory(path: str) -> bool:
    return os.path.isdir(path)
'''

[test.assertions]
any_of = ["is_dir()", "metadata"]
```

```toml
[[test]]
name = "os_path_basename"
description = "os.path.basename() extraction"

[test.python]
code =  '''
import os.path

def get_filename(path: str) -> str:
    return os.path.basename(path)
'''

[test.assertions]
any_of = ["file_name()", "Path"]
```

```toml
[[test]]
name = "os_path_dirname"
description = "os.path.dirname() extraction"

[test.python]
code =  '''
import os.path

def get_directory(path: str) -> str:
    return os.path.dirname(path)
'''

[test.assertions]
any_of = ["parent()", "Path"]
```

### OS Directory Operations

```toml
[[test]]
name = "os_listdir"
description = "os.listdir() directory listing"

[test.python]
code =  '''
import os

def list_files(directory: str) -> list[str]:
    return os.listdir(directory)
'''

[test.assertions]
any_of = ["read_dir()", "fs::read_dir"]
```

```toml
[[test]]
name = "os_makedirs"
description = "os.makedirs() create directories"

[test.python]
code =  '''
import os

def ensure_directory(path: str) -> None:
    os.makedirs(path, exist_ok=True)
'''

[test.assertions]
any_of = ["create_dir_all()", "fs::create_dir_all"]
```

### OS File Operations

```toml
[[test]]
name = "os_remove"
description = "os.remove() file deletion"

[test.python]
code =  '''
import os

def delete_file(path: str) -> None:
    os.remove(path)
'''

[test.assertions]
any_of = ["remove_file()", "fs::remove_file"]
```

### Sys Module

```toml
[[test]]
name = "sys_argv"
description = "sys.argv command line arguments"

[test.python]
code =  '''
import sys

def get_first_arg() -> str:
    if len(sys.argv) > 1:
        return sys.argv[1]
    return ""
'''

[test.assertions]
any_of = ["std::env::args()", "env::args"]
```

```toml
[[test]]
name = "sys_exit"
description = "sys.exit() program termination"

[test.python]
code =  '''
import sys

def exit_with_code(code: int) -> None:
    sys.exit(code)
'''

[test.assertions]
any_of = ["std::process::exit", "process::exit"]
```

```toml
[[test]]
name = "sys_platform"
description = "sys.platform detection"

[test.python]
code =  '''
import sys

def is_linux() -> bool:
    return sys.platform.startswith("linux")
'''

[test.assertions]
any_of = ["cfg!(target_os", "std::env::consts::OS"]
```

## Notes

- Python `os` module → Rust `std::fs`, `std::path`, `std::env`
- `os.path.join()` → `Path::new().join()`
- `os.environ.get()` → `std::env::var()`
- `os.listdir()` → `std::fs::read_dir()`
- `os.makedirs()` → `std::fs::create_dir_all()`
- Python `sys` module → Rust `std::env`, `std::process`
- `sys.argv` → `std::env::args()`
- `sys.exit()` → `std::process::exit()`

## Acceptance Criteria

- [ ] OS environment tests migrated
- [ ] OS path operation tests migrated
- [ ] OS directory operation tests migrated
- [ ] OS file operation tests migrated
- [ ] Sys module tests migrated

## Estimated Effort

**Time**: 2-3 hours
**Risk**: Low-Medium
