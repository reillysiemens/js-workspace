# Agent instructions

This repository is a Rust translation, not a transliteration, of the TypeScript
[`workspace-tools`] package. Use the original package as a behavioral reference,
but prefer idiomatic Rust abstractions, strong types, explicit errors, and small
APIs over mechanically translating TypeScript patterns.

## Development

- Use `cargo nextest run` for Rust tests when available. Fall back to `cargo
test` only if nextest is unavailable.
- Run `cargo fmt --check` before considering Rust changes complete.
- Keep tests focused on observable behavior. Existing tests use bare
  `// Arrange`, `// Act`, and `// Assert` section comments.

## Design preferences

- Avoid global mutable state and implicit caches. If caching becomes necessary,
  prefer explicit or caller-owned cache state.
- Treat filesystem and environment access as boundary concerns. Parse external
  values into typed Rust values early.

## Environment variables

- Do not mutate process environment in tests. In Rust 2024, environment mutation
  is unsafe and process-global. Test parsing behavior directly instead; use
  subprocess-based tests only if end-to-end environment behavior becomes
  important.

## Related notes

See the `notes/` directory for further design considerations (may include
incomplete designs).

[`workspace-tools`]: https://github.com/microsoft/workspace-tools
