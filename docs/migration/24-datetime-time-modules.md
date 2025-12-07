# Migration Task: Datetime and Time Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_datetime_unit.rs | ~120 | ~6 | `transpile_and_check` |
| test_time_unit.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/24-datetime-time-modules.toml`

## Tests to Migrate

### Datetime Creation

```toml
[[test]]
name = "datetime_now"
description = "datetime.now() current time"

[test.python]
code =  '''
from datetime import datetime

def get_current_time() -> datetime:
    return datetime.now()
'''

[test.assertions]
any_of = ["chrono::Local::now()", "Local::now"]
```

```toml
[[test]]
name = "datetime_utcnow"
description = "datetime.utcnow() UTC time"

[test.python]
code =  '''
from datetime import datetime

def get_utc_time() -> datetime:
    return datetime.utcnow()
'''

[test.assertions]
any_of = ["chrono::Utc::now()", "Utc::now"]
```

```toml
[[test]]
name = "datetime_constructor"
description = "datetime(year, month, day) constructor"

[test.python]
code =  '''
from datetime import datetime

def create_date(year: int, month: int, day: int) -> datetime:
    return datetime(year, month, day)
'''

[test.assertions]
any_of = ["NaiveDate::", "DateTime::", "chrono"]
```

### Date Operations

```toml
[[test]]
name = "datetime_date_extraction"
description = "datetime component extraction"

[test.python]
code =  '''
from datetime import datetime

def get_year(dt: datetime) -> int:
    return dt.year
'''

[test.assertions]
any_of = [".year()", ".year"]
```

```toml
[[test]]
name = "datetime_month_extraction"
description = "datetime month extraction"

[test.python]
code =  '''
from datetime import datetime

def get_month(dt: datetime) -> int:
    return dt.month
'''

[test.assertions]
any_of = [".month()", ".month"]
```

```toml
[[test]]
name = "datetime_day_extraction"
description = "datetime day extraction"

[test.python]
code =  '''
from datetime import datetime

def get_day(dt: datetime) -> int:
    return dt.day
'''

[test.assertions]
any_of = [".day()", ".day"]
```

### Time Operations

```toml
[[test]]
name = "time_sleep"
description = "time.sleep() delay"

[test.python]
code =  '''
import time

def wait_seconds(seconds: float) -> None:
    time.sleep(seconds)
'''

[test.assertions]
any_of = ["thread::sleep", "Duration::from_secs", "Duration::from_millis"]
```

```toml
[[test]]
name = "time_time"
description = "time.time() unix timestamp"

[test.python]
code =  '''
import time

def get_timestamp() -> float:
    return time.time()
'''

[test.assertions]
any_of = ["SystemTime::now()", "UNIX_EPOCH", "duration_since"]
```

### Datetime Formatting

```toml
[[test]]
name = "datetime_strftime"
description = "datetime.strftime() formatting"

[test.python]
code =  '''
from datetime import datetime

def format_date(dt: datetime) -> str:
    return dt.strftime("%Y-%m-%d")
'''

[test.assertions]
any_of = [".format(", "strftime"]
```

```toml
[[test]]
name = "datetime_strptime"
description = "datetime.strptime() parsing"

[test.python]
code =  '''
from datetime import datetime

def parse_date(s: str) -> datetime:
    return datetime.strptime(s, "%Y-%m-%d")
'''

[test.assertions]
any_of = ["parse_from_str", "NaiveDateTime::parse"]
```

### Timedelta

```toml
[[test]]
name = "timedelta_creation"
description = "timedelta creation"

[test.python]
code =  '''
from datetime import timedelta

def days_duration(n: int) -> timedelta:
    return timedelta(days=n)
'''

[test.assertions]
any_of = ["Duration::", "chrono::Duration"]
```

```toml
[[test]]
name = "timedelta_addition"
description = "datetime + timedelta arithmetic"

[test.python]
code =  '''
from datetime import datetime, timedelta

def add_days(dt: datetime, days: int) -> datetime:
    return dt + timedelta(days=days)
'''

[test.assertions]
any_of = ["+ Duration::", "checked_add", "chrono"]
```

## Notes

- Python `datetime` module → Rust `chrono` crate
- `datetime.now()` → `Local::now()`
- `datetime.utcnow()` → `Utc::now()`
- `timedelta` → `chrono::Duration`
- `strftime`/`strptime` → `format`/`parse_from_str`
- Python `time` module → Rust `std::thread`, `std::time`
- `time.sleep()` → `thread::sleep(Duration::from_secs())`
- `time.time()` → `SystemTime::now().duration_since(UNIX_EPOCH)`

## Acceptance Criteria

- [ ] Datetime creation tests migrated
- [ ] Date component extraction tests migrated
- [ ] Time module tests migrated
- [ ] Datetime formatting tests migrated
- [ ] Timedelta tests migrated

## Estimated Effort

**Time**: 2 hours
**Risk**: Low
