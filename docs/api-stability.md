# API Stability Policy

> **Category: Current contract**

RunenUI is pre-1.0. Public Rust visibility currently supports framework use, downstream conformance, and design iteration; it is not a stable compatibility promise.

## Before the first public 0.1 release

Before `0.1.0`, package publication remains disabled under the [release policy](release-policy.md). Breaking changes are expected when they improve correctness, ownership, extensibility, maintainability, performance, or remove a superseded prototype contract. Prefer clean cutovers over compatibility aliases or parallel authorities unless a reviewed migration has a real external compatibility requirement and explicit removal condition.

The first public `0.1.0` is reserved for feature-complete supported product profiles as defined by the [roadmap](roadmap.md). Feature completeness does not make every public API stable; it means no foundational framework capability required by those declared profiles is intentionally deferred.

## 0.x policy after 0.1

During public `0.x` evolution:

- breaking changes remain allowed when they materially improve correctness, ownership, extensibility, ergonomics, performance, or long-term API quality;
- public enums, traits, type erasure, identity, extension contracts, renderer/host/provider seams, and control APIs require deliberate evolution rather than accidental compatibility promises;
- generated runtime products and live runtime identities remain non-forgeable unless a reviewed contract explicitly requires construction authority;
- migrations that affect downstream users must be explicit in the owning pull request and release notes;
- compatibility aliases or parallel authorities require a real downstream migration need and an explicit removal condition;
- a public type being documented, tested, or shipped in 0.x does not by itself make it stable.

Exact current Rust signatures, bounds, visibility, and documentation are authoritative in source/Rustdoc. Conceptual ownership and invariants are summarized in the [public API contract](architecture/public-api.md). Current accepted maturity is summarized in [status](status.md); permanent behavioral obligations are recorded in [conformance](conformance/README.md).

## Stability boundary

A compatibility guarantee is deliberate product policy, not an accidental consequence of `pub` or of feature completeness at `0.1.0`. Before stable release, RunenUI may replace prototype or immature APIs rather than retain aliases that create a second path or weaken ownership boundaries.

The doc-hidden `runenui_core::__runtime` bridge is technically public only where Rust crate boundaries require it. It is outside normal application API, unsupported for downstream application use, and semver-exempt before 1.0.

## Stable-release gate

`1.0.0` is the deliberate compatibility and support boundary over an already feature-complete product. Stable release requires successful real-world 0.x use, reviewed public API/semver and deprecation strategy, compatibility and release automation, documented platform/MSRV/support policy, security/dependency/license enforcement, sustained performance budgets, migration policy, representative downstream applications, and no unresolved stable-release correctness or compatibility defects.

`1.0.0` therefore does not serve as the roadmap milestone where missing foundational layout, text, controls, animation, virtualization, accessibility, renderer, host, or engine-integration capability first arrives; those belong to the feature-complete `0.1.0` gate.
