# Migration Task: JSON Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_json_unit.rs | ~80 | ~4 | `transpile_and_check` |
| test_json_basics_unit.rs | ~100 | ~5 | `transpile_and_check` |

## Target File

`tests/toml/22-json-module.toml`

## Tests to Migrate

### JSON Serialization

```toml
[[test]]
name = "json_dumps"
description = "json.dumps() serialization"

[test.python]
code =  '''
import json

def serialize_data(data: dict) -> str:
    return json.dumps(data)
'''

[test.assertions]
any_of = ["serde_json", "to_string"]
```

```toml
[[test]]
name = "json_dumps_indent"
description = "json.dumps() with indentation"

[test.python]
code =  '''
import json

def pretty_json(data: dict) -> str:
    return json.dumps(data, indent=2)
'''

[test.assertions]
any_of = ["serde_json::to_string_pretty", "to_string_pretty"]
```

### JSON Deserialization

```toml
[[test]]
name = "json_loads"
description = "json.loads() deserialization"

[test.python]
code =  '''
import json

def deserialize_data(s: str) -> dict:
    return json.loads(s)
'''

[test.assertions]
any_of = ["serde_json", "from_str"]
```

### JSON File I/O

```toml
[[test]]
name = "json_dump"
description = "json.dump() to file"

[test.python]
code =  '''
import json

def write_json(data: dict, filename: str) -> None:
    with open(filename, 'w') as f:
        json.dump(data, f)
'''

[test.assertions]
any_of = ["serde_json", "to_writer"]
```

```toml
[[test]]
name = "json_load"
description = "json.load() from file"

[test.python]
code =  '''
import json

def read_json(filename: str) -> dict:
    with open(filename, 'r') as f:
        return json.load(f)
'''

[test.assertions]
any_of = ["serde_json", "from_reader"]
```

### JSON with Custom Types

```toml
[[test]]
name = "json_list"
description = "JSON with list data"

[test.python]
code =  '''
import json

def parse_array(s: str) -> list:
    return json.loads(s)
'''

[test.assertions]
any_of = ["Vec<", "serde_json::Value"]
```

```toml
[[test]]
name = "json_nested"
description = "JSON with nested structure"

[test.python]
code =  '''
import json

def get_nested_value(s: str, key1: str, key2: str):
    data = json.loads(s)
    return data[key1][key2]
'''

[test.assertions]
any_of = ["serde_json::Value", "HashMap", "get"]
```

## Notes

- Python `json` module → Rust `serde_json` crate
- `json.dumps(data)` → `serde_json::to_string(&data)`
- `json.loads(s)` → `serde_json::from_str(&s)`
- `json.dump(data, f)` → `serde_json::to_writer(f, &data)`
- `json.load(f)` → `serde_json::from_reader(f)`
- Pretty printing: `to_string_pretty`

## Acceptance Criteria

- [ ] JSON serialization tests migrated
- [ ] JSON deserialization tests migrated
- [ ] JSON file I/O tests migrated
- [ ] JSON complex type tests migrated

## Estimated Effort

**Time**: 1-2 hours
**Risk**: Low
