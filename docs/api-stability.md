# API Stability Policy

> **Category: Current contract**

RunenUI is pre-1.0. Public Rust visibility currently supports workspace proofs and design iteration; it is not a stable compatibility promise.

During `0.x`:

- breaking changes are allowed when they improve correctness, ownership, extensibility, or maintainability;
- migrations must be explicit in pull requests and the changelog;
- obsolete prototype paths should be removed rather than retained as parallel compatibility layers unless a reviewed bounded migration requires one;
- public enums, traits, type erasure, and extension contracts require deliberate semver design before ecosystem use;
- a facade crate is deferred until lower-level APIs are ready to present a coherent normal-user surface.

M1 through M3 apply this policy concretely: obsolete prototype constructors, closed
widget dispatch types, and compatibility aliases were removed; evolution-prone
runtime/proof enums and `TokenFamily` are `#[non_exhaustive]`; intentionally
closed authored value enums remain exhaustive. `UiApp`, `MeasurementProvider`,
`View`, `Views`, and `Widget` are open, while identifier-input conversion is
sealed to preserve validation and safe erasure remains private. Generated
products and mounted identity/state are non-forgeable. See the [public API
contract](architecture/public-api.md), [ADR 0003](adr/0003-extensible-view-widget-component-protocol.md),
and [ADR 0004](adr/0004-mounted-runtime-reconciliation.md).

The M2 corrective pass deliberately broke prototype APIs again: lifecycle
mismatches became a truthful category enum; built-in authored views were split
from private runtime widgets; child ownership moved from an empty marker and
element constructor to `ChildLayoutWidget`, `ChildLayout`, and canonical
`Container<Action>` authoring; activation changed from hidden mutation behind
`&self` to `&mut self`; intrinsic measurement was separated from child layout;
and runtime `ButtonLabel` vocabulary was replaced outright by `ControlLabel`.
M3 deliberately removed that provisional seam, transient preorder identity, and
free element publication. It introduced the reviewed state-aware mounted
interface, runtime-local generational/semantic IDs, checked lifecycle contexts,
and selective invalidation without compatibility aliases. These remain pre-1.0
APIs, not a stability promise.

`MeasurementProvider` implementations supply a stable cache identity and a
behavior revision and must change one whenever measurement behavior changes.
`StyleTokens` exposes a monotonic diagnostic revision, but runtime cache
compatibility is based on exact token content rather than that revision. The doc-hidden
`runenui_core::__runtime` bridge is technically public solely to cross the Rust
crate boundary. It is outside the prelude, unstable, unsupported for application
use, and semver-exempt before 1.0.

A type being public, documented, or tested does not make it stable. Support claims come from the feature/support matrix and behavioral milestone gates.

`1.0.0` is reserved for the M11 gate: required production profiles work, public API and semver strategy are reviewed, compatibility checks and release automation exist, supported platforms/MSRV are documented, and no unresolved P0/P1 correctness defects remain.
