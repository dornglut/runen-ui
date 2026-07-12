# API Stability Policy

> **Category: Current contract**

RunenUI is pre-1.0. Public Rust visibility currently supports workspace proofs and design iteration; it is not a stable compatibility promise.

During `0.x`:

- breaking changes are allowed when they improve correctness, ownership, extensibility, or maintainability;
- migrations must be explicit in pull requests and the changelog;
- obsolete prototype paths should be removed rather than retained as parallel compatibility layers unless a reviewed bounded migration requires one;
- public enums, traits, type erasure, and extension contracts require deliberate semver design before ecosystem use;
- a facade crate is deferred until lower-level APIs are ready to present a coherent normal-user surface.

M1 and M2 apply this policy concretely: obsolete prototype constructors, closed
widget dispatch types, and compatibility aliases were removed; evolution-prone
runtime/proof enums and `TokenFamily` are `#[non_exhaustive]`; intentionally
closed authored value enums remain exhaustive. `UiApp`, `MeasurementProvider`,
`View`, `Views`, and `Widget` are open, while identifier-input conversion is
sealed to preserve validation and safe erasure remains private. Generated
products and opaque widget state are non-forgeable. See the [public API
contract](architecture/public-api.md) and [ADR 0003](adr/0003-extensible-view-widget-component-protocol.md).

The M2 corrective pass deliberately broke prototype APIs again: lifecycle
mismatches became a truthful category enum; built-in authored views were split
from private runtime widgets; child ownership moved from an empty marker and
element constructor to `ChildLayoutWidget`, `ChildLayout`, and canonical
`Container<Action>` authoring; activation changed from hidden mutation behind
`&self` to `&mut self`; intrinsic measurement was separated from child layout;
and runtime `ButtonLabel` vocabulary was replaced outright by `ControlLabel`.
M3 is also expected to break the state-independent M2 proof capability
signatures when it introduces the reviewed state-aware mounted interface.

A type being public, documented, or tested does not make it stable. Support claims come from the feature/support matrix and behavioral milestone gates.

`1.0.0` is reserved for the M11 gate: required production profiles work, public API and semver strategy are reviewed, compatibility checks and release automation exist, supported platforms/MSRV are documented, and no unresolved P0/P1 correctness defects remain.
