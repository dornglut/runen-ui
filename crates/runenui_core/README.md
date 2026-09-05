# `runenui_core`

> **Category: Current contract**

`runenui_core` owns RunenUI's host-neutral authored values and public protocol vocabulary. It contains no live mounted runtime, native host, renderer backend, or platform accessibility adapter.

## Current ownership

Core owns:

- `UiApp`, host-neutral effects/work/subscription protocols, and validated work keys;
- validated logical geometry, authored IDs/keys, typed token/style values, normalized production layout values, transient `View`/`Element` authoring, typed built-in views, and the open state-aware widget/child-bearing/measurement contracts;
- lifecycle, event, activation, invalidation, focusability, pointer/input, semantic-command, and application-work protocol values consumed by runtime;
- opaque runtime-issued protocol value types for mounted, semantic, surface, time, and work identity without live allocation authority;
- platform-neutral semantic authoring and action vocabulary, including stable owner-local semantic keys, roles/content/state/actions/relationships/bounds, contribution validation, and read-only semantic-action target metadata.

Mounted and semantic identities are distinct runtime lifetimes. One mounted owner may own zero, one, or many semantic lifetimes. Widgets author stable local semantic keys, not live semantic IDs, mounted IDs, runtime focus, absolute surface placement, adapter objects, or platform vocabulary.

Built-in and downstream widgets use the same checked erased protocol. Every widget declares persistent state; stateless widgets use `State = ()`. Recursive action mapping changes only application-action plumbing and preserves widget/state identity and semantic contribution.

## Core/runtime bridge

The doc-hidden `runenui_core::__runtime` module exists only because core and runtime are separate crates. It safely consumes transient elements into checked erased widget/state plumbing needed by `runenui_runtime`. It is outside the normal prelude, unstable, unsupported for application use, and provides no live runtime namespace, arena, queue, sequence, publication, or mutation authority.

Both core and runtime remain safe-Rust by repository policy. Payload/type mismatches are diagnosed and fail closed rather than granting unchecked typed access.

## Must not own

`runenui_core` must not own persistent mounted or semantic storage, semantic publication/revisions, live layout topology or algorithm state, live scheduling, queue/work/trace state, application product state, native windows/accessibility adapters, concrete renderer backends, ECS integration, or legacy compatibility paths. Runtime alone owns live identity allocation, reconciliation, routing, scheduling, layout orchestration, publication, and semantic action resolution.

See the [public API contract](../../docs/architecture/public-api.md), [workspace structure](../../docs/architecture/workspace-structure.md), [M5 semantic/testing charter](../../docs/conformance/m5-semantics-and-testing-charter.md), [current status](../../docs/status.md), and [roadmap](../../docs/roadmap.md).
