## Test Commands

```bash
# Run all TOML tests
cargo run -- test -j

# Run specific test file
cargo run -- test -p tests/toml/exceptions.toml

# Filter by name
cargo run -- test -f "abc"

# With compilation verification
cargo run -- test -c
```

