# API Stability Policy

> **Category: Current contract**

RunenUI is pre-1.0. Public Rust visibility currently supports workspace proofs and design iteration; it is not a stable compatibility promise.

During `0.x`:

- breaking changes are allowed when they improve correctness, ownership, extensibility, or maintainability;
- migrations must be explicit in pull requests and the changelog;
- obsolete prototype paths should be removed rather than retained as parallel compatibility layers unless a reviewed bounded migration requires one;
- public enums, traits, type erasure, and extension contracts require deliberate semver design before ecosystem use;
- a facade crate is deferred until lower-level APIs are ready to present a coherent normal-user surface.

M1 applies this policy concretely: obsolete prototype constructors and aliases
were removed without compatibility wrappers; evolution-prone runtime enums are
`#[non_exhaustive]`; intentionally closed authored value enums remain exhaustive;
`UiApp`, `MeasurementProvider`, and conversion traits are open; generated products
are read-only. See the [public API contract](architecture/public-api.md).

A type being public, documented, or tested does not make it stable. Support claims come from the feature/support matrix and behavioral milestone gates.

`1.0.0` is reserved for the M11 gate: required production profiles work, public API and semver strategy are reviewed, compatibility checks and release automation exist, supported platforms/MSRV are documented, and no unresolved P0/P1 correctness defects remain.
