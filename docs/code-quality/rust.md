# Rust code quality

> Load when: Rust source or a Rust-facing API is implemented or reviewed.
> Skip when: no Rust code changes.

## Types and APIs

- Model domain states with enums, newtypes, and validated constructors instead of sentinel values or
  loosely related booleans.
- Keep public APIs minimal. Document invariants, ownership, units, blocking behavior, and errors.
- Accept borrowed inputs when ownership is unnecessary; return owned data when it prevents leaked
  lifetimes or unstable internal coupling.
- Prefer exhaustive `match` for domain state. Use wildcard arms only when future variants truly have
  identical behavior.
- Derive `Debug` for public types unless exposure would reveal sensitive data.

## Ownership and allocation

- Prefer borrowing over cloning. A clone in a hot or large-data path needs a concrete ownership or
  concurrency reason.
- Avoid collecting iterators when streaming or a single pass is sufficient.
- Reuse buffers only when ownership stays clear and measurement shows allocation cost matters.
- Do not retain `Arc`, texture, frame, or project data beyond its actual lifetime merely to satisfy
  the borrow checker.

## Errors and control flow

- Use `Result` and `?` for recoverable failures. Add context at subsystem boundaries.
- Do not use `unwrap`, `expect`, indexing, or assertions on runtime-controlled input unless an
  immediately adjacent invariant proves the operation cannot fail.
- Keep panic for violated programmer invariants, not media, project, filesystem, network, or user
  input failures.
- Preserve cancellation and partial-failure semantics across background tasks and channels.

## Concurrency

- Keep the UI thread free of decode, probe, filesystem, network, and long lock operations.
- Bound worker counts, queues, caches, and retry loops. Define behavior when capacity is reached.
- Do not hold a mutex guard across blocking work, callbacks, `.await`, or FFI calls.
- Make thread-safety limitations explicit. In particular, serialize APIs documented as process-global
  or not thread-safe.
- Background results must be correlated with stable project, sequence, asset, or request identifiers
  before mutating current UI state.

## Unsafe and FFI

- Minimize the unsafe surface and place a `// SAFETY:` justification immediately above every unsafe
  block.
- State pointer validity, length, alignment, lifetime, aliasing, initialization, and ownership
  assumptions in the justification or called wrapper contract.
- Validate integer conversions, nullability, buffer sizes, and callback lifetimes at the safe wrapper.
- Keep raw handles private and expose RAII ownership where possible.
- Never let a Rust panic unwind across a C ABI boundary.

## Dependencies and maintainability

- Reuse the standard library or an existing workspace dependency before adding a crate.
- A new dependency needs a current use, compatible license, maintained release history, and a reason
  its transitive and binary cost is acceptable.
- Keep platform-specific code behind a narrow adapter and compile-time target boundary.
- Remove dead flags, temporary compatibility branches, debug output, and commented-out code before
  completion.

## Required validation

Run formatting, targeted tests, and the narrowest useful compilation check. Run workspace clippy and
the wider suite when the change affects shared APIs or crosses crates. Report commands actually run
and distinguish unavailable environmental validation from passing validation.
