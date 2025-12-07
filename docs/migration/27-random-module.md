# Migration Task: Random Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_random_unit.rs | ~150 | ~8 | `transpile_and_check` |
| test_secrets_unit.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/27-random-module.toml`

## Tests to Migrate

### Random Number Generation

```toml
[[test]]
name = "random_random"
description = "random.random() float [0, 1)"

[test.python]
code =  '''
import random

def get_random_float() -> float:
    return random.random()
'''

[test.assertions]
any_of = ["rand::random", "Rng", "gen()"]
```

```toml
[[test]]
name = "random_randint"
description = "random.randint(a, b) inclusive range"

[test.python]
code =  '''
import random

def roll_dice() -> int:
    return random.randint(1, 6)
'''

[test.assertions]
any_of = ["gen_range(", "Rng", "1..=6"]
```

```toml
[[test]]
name = "random_randrange"
description = "random.randrange(start, stop, step)"

[test.python]
code =  '''
import random

def random_even(max_val: int) -> int:
    return random.randrange(0, max_val, 2)
'''

[test.assertions]
any_of = ["gen_range(", "step_by", "filter"]
```

```toml
[[test]]
name = "random_uniform"
description = "random.uniform(a, b) float range"

[test.python]
code =  '''
import random

def random_in_range(low: float, high: float) -> float:
    return random.uniform(low, high)
'''

[test.assertions]
any_of = ["gen_range(", "Uniform"]
```

### Random Selection

```toml
[[test]]
name = "random_choice"
description = "random.choice() single selection"

[test.python]
code =  '''
import random

def pick_one(items: list[str]) -> str:
    return random.choice(items)
'''

[test.assertions]
any_of = ["choose(", "gen_range"]
```

```toml
[[test]]
name = "random_choices"
description = "random.choices() with replacement"

[test.python]
code =  '''
import random

def pick_many(items: list[str], k: int) -> list[str]:
    return random.choices(items, k=k)
'''

[test.assertions]
any_of = ["choose(", "sample", "repeat_with"]
```

```toml
[[test]]
name = "random_sample"
description = "random.sample() without replacement"

[test.python]
code =  '''
import random

def pick_unique(items: list[str], k: int) -> list[str]:
    return random.sample(items, k)
'''

[test.assertions]
any_of = ["choose_multiple(", "sample", "shuffle"]
```

### Random Shuffling

```toml
[[test]]
name = "random_shuffle"
description = "random.shuffle() in-place shuffle"

[test.python]
code =  '''
import random

def mix_up(items: list[int]) -> None:
    random.shuffle(items)
'''

[test.assertions]
any_of = ["shuffle(", "SliceRandom"]
```

### Random Seed

```toml
[[test]]
name = "random_seed"
description = "random.seed() deterministic seeding"

[test.python]
code =  '''
import random

def set_seed(seed: int) -> None:
    random.seed(seed)
'''

[test.assertions]
any_of = ["SeedableRng", "seed_from_u64", "StdRng::seed"]
```

### Secrets Module (Cryptographic)

```toml
[[test]]
name = "secrets_token_bytes"
description = "secrets.token_bytes() secure random"

[test.python]
code =  '''
import secrets

def random_bytes(n: int) -> bytes:
    return secrets.token_bytes(n)
'''

[test.assertions]
any_of = ["OsRng", "getrandom", "thread_rng"]
```

```toml
[[test]]
name = "secrets_token_hex"
description = "secrets.token_hex() hex string"

[test.python]
code =  '''
import secrets

def random_hex(n: int) -> str:
    return secrets.token_hex(n)
'''

[test.assertions]
any_of = ["OsRng", "hex::encode", "format!"]
```

```toml
[[test]]
name = "secrets_choice"
description = "secrets.choice() secure selection"

[test.python]
code =  '''
import secrets

def secure_pick(items: list[str]) -> str:
    return secrets.choice(items)
'''

[test.assertions]
any_of = ["OsRng", "choose("]
```

```toml
[[test]]
name = "secrets_randbelow"
description = "secrets.randbelow() secure int"

[test.python]
code =  '''
import secrets

def secure_int(exclusive_max: int) -> int:
    return secrets.randbelow(exclusive_max)
'''

[test.assertions]
any_of = ["OsRng", "gen_range(", "0.."]
```

### Statistical Distributions

```toml
[[test]]
name = "random_gauss"
description = "random.gauss() normal distribution"

[test.python]
code =  '''
import random

def normal_random(mu: float, sigma: float) -> float:
    return random.gauss(mu, sigma)
'''

[test.assertions]
any_of = ["Normal::", "sample(", "StandardNormal"]
```

```toml
[[test]]
name = "random_expovariate"
description = "random.expovariate() exponential distribution"

[test.python]
code =  '''
import random

def exponential_random(rate: float) -> float:
    return random.expovariate(rate)
'''

[test.assertions]
any_of = ["Exp::", "Exponential", "sample("]
```

## Notes

- Python `random` module → Rust `rand` crate
- `random.random()` → `rand::random()` or `rng.gen()`
- `random.randint(a, b)` → `rng.gen_range(a..=b)`
- `random.choice(seq)` → `seq.choose(&mut rng)`
- `random.shuffle(seq)` → `seq.shuffle(&mut rng)` (requires `SliceRandom` trait)
- `random.seed(n)` → `StdRng::seed_from_u64(n)`
- Python `secrets` module → Rust `rand` with `OsRng` for cryptographic security
- For distributions, use `rand_distr` crate

## Acceptance Criteria

- [ ] Random number generation tests migrated
- [ ] Random selection tests migrated
- [ ] Random shuffling tests migrated
- [ ] Random seeding tests migrated
- [ ] Secrets module tests migrated
- [ ] Statistical distribution tests migrated

## Estimated Effort

**Time**: 2 hours
**Risk**: Low
