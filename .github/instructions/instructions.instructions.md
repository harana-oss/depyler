---
applyTo: '**/*.rs'
---
## Tests

1. Never use `#[ignore]` attribute.
2. Test assertions should:
    a. Never contain ||. Be specific about what the test should check.
    b. Be unique and specific to the intention of the test.
    c. Avoid unnecessary complexity. Keep tests simple and focused.
3. Before creating a new test file look for existing files that can be extended.

## Code

1. Do not create markdown files unless specifically asked.
2. Do not add unnecessary comments. Only add comments that clarify complex logic.