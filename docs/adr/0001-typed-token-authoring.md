# ADR 0001: Use Typed Rust Expressions for Token Authoring

> **Category: ADR**
>
> **Status:** Accepted
>
> **Decision date:** 2026-07-11 (migrated from the earlier token-authoring checkpoint)

## Context

`StyleIntent` accepts literal values and typed token references. Builder calls and `element!` expression attributes already carry arbitrary typed Rust expressions. Separate `background_token="..."`-style syntax would expand the macro grammar before theme loading, fallback, external source syntax, or sustained authoring pressure exists.

## Decision

Use typed token constructors through builders and macro expression attributes. Keep builder/descriptors authoritative and keep `element!` as sugar over them. Do not add token-specific string shorthand yet.

## Consequences

Token families remain visible to the Rust type system; literals and tokens share one value-union path; the macro does not become an independent styling language. Constructors are somewhat verbose. Shorthand may be reconsidered only when repeated use, external source syntax, or diagnostic/source-location needs demonstrate value without weakening typed provenance.
