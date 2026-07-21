# ADR 0002: Keep Layout in `runenui_runtime`

> **Category: ADR**
>
> **Status:** Accepted
>
> **Decision date:** 2026-07-11
>
> **Reviewed baseline:** `633ad932cb2478bbe1c54bf136c86f5b022d2da9`

## Context

Explicit constraints, a measurement provider, one measured result, computed padding, and overflow diagnostics are real contracts. Layout still consumes runtime-owned identity and prepared element facts, directly arranges runtime-owned `SurfaceFrame`, has one small row/column algorithm, and has no independent consumer or conformance harness outside runtime.

## Decision

Keep constraints, measurement orchestration, measured results, arrangement, layout diagnostics, geometry, hit-test bounds, and surface publication in `runenui_runtime`. Do not extract `runenui_layout` based on a count of vocabulary types or module size.

## Consequences

The current dependency graph remains simple and no premature identity genericization or nominal crate is introduced. Extraction waits until layout input/output ownership is independent of surface internals, geometry ownership is explicit, runtime identity is deliberately neutral/generic or absent, real typography/resource inputs exist, and conformance can run without the full application publication path.

At least one hard pressure must also exist: an independent consumer, a Cargo-enforced dependency/optionality boundary, or multiple substantial algorithms needing independent ownership. Revisit after the renderer-neutral scene protocol, production text/layout expansion, a public testing consumer, or an external host/backend creates that pressure.
