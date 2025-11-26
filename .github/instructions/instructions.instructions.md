---
applyTo: '**/*.rs'
---
## Behaviour

1. Do not generate lengthy summary comments. One or two sentence max.
2. When I asked you to go through ALL of something. Don't just do the open files.

## Tests

1. Never use `#[ignore]` attribute.
2. Test assertions should:
    a. Never contain ||.
    b  Assert on complete, meaningful statements rather than fragments.
    c. Be unique and specific to the intention of the test.
    d. Avoid unnecessary complexity. Keep tests simple and focused.
3. Before creating a new test file look for existing files that can be extended.

## Code

1. Do not create markdown files unless specifically asked.
2. Do not add unnecessary comments when the code is self-explanatory. Only add comments that clarify complex logic.