# API Stability Policy

> **Category: Current contract**

RunenUI is pre-1.0. Public Rust visibility currently supports framework use, downstream conformance, and design iteration; it is not a stable compatibility promise.

## Pre-1.0 policy

During `0.x`:

- breaking changes are allowed when they improve correctness, ownership, extensibility, maintainability, or remove a superseded prototype contract;
- prefer clean cutovers over compatibility aliases or parallel authorities unless a reviewed migration has a real external compatibility requirement and explicit removal condition;
- public enums, traits, type erasure, identity, and extension contracts require deliberate evolution policy before ecosystem stabilization;
- generated runtime products and live runtime identities remain non-forgeable unless a reviewed contract explicitly requires construction authority;
- migrations that affect downstream users must be explicit in the owning pull request and changelog/release notes where appropriate;
- a public type being documented or tested does not make it stable.

Exact current Rust signatures, bounds, visibility, and documentation are authoritative in source/Rustdoc. Conceptual ownership and invariants are summarized in the [public API contract](architecture/public-api.md). Current accepted maturity is summarized in [status](status.md); permanent behavioral obligations are recorded in [conformance](conformance/README.md).

## Stability boundary

A compatibility guarantee is deliberate product policy, not an accidental consequence of `pub`. Before stable release, RunenUI may replace prototype APIs rather than retain aliases that create a second path or weaken ownership boundaries.

The doc-hidden `runenui_core::__runtime` bridge is technically public only where Rust crate boundaries require it. It is outside normal application API, unsupported for downstream application use, and semver-exempt before 1.0.

## Stable-release gate

`1.0.0` is reserved for the production-hardening gate in the [roadmap](roadmap.md). Stable release requires supported production profiles, reviewed public API/semver strategy, compatibility and release automation, documented platform/MSRV policy, security/dependency/license enforcement, performance budgets, real application release candidates, and no unresolved release-blocking correctness defects.
