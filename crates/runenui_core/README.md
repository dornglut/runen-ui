# `runenui_core`

> **Category: Current contract**

`runenui_core` owns RunenUI's host-neutral authored values and public protocol vocabulary. It contains no live mounted runtime, native host, renderer backend, or platform accessibility adapter.

## Current ownership

Core owns:

- `UiApp`, `HostProtocol`, `NoHostProtocol`, ordered effects, subscriptions, task/timer/host-request descriptions, and validated `WorkKey` values;
- validated logical geometry, authored IDs/keys, typed token/style values, transient `View`/`Element` authoring, typed built-in views, and the open state-aware `Widget` / `ChildLayoutWidget` contracts;
- lifecycle, event, activation, invalidation, focusability, pointer/input, semantic-command, and application-work protocol values consumed by runtime;
- opaque runtime-issued protocol identity types including `MountedNodeId`, `SemanticNodeId`, `SurfaceId`, `SurfaceInputContext`, `MonotonicInstant`, and `WorkSequence`; ordinary application code cannot construct a live identity or extract its private namespace/slot/generation authority;
- canonical M5 semantic authoring vocabulary: `SemanticKey`, `SemanticRole`, values/text/state, `SemanticAction`, relationships, bounds, `SemanticItem`, `SemanticNodeContribution`, `SemanticContribution`, validation/error/context, and read-only `SemanticActionTarget` metadata.

`SemanticNodeId` is a distinct opaque semantic identity type. It shares the runtime namespace needed for foreign-runtime rejection but is allocated by a separate runtime-owned semantic arena; it is not derived from or coupled to a mounted arena slot/generation. One mounted owner may own zero, one, or many semantic lifetimes.

Widgets contribute action-type-independent owner-local semantic forests keyed by `SemanticKey`. They do not author live semantic IDs, mounted IDs, runtime focus, absolute surface coordinates, adapter objects, or platform vocabulary. M5 semantic actions are `Activate`, `RequestFocus`, `OpenMenu`, and `OpenContextMenu`; routed `SemanticCommand::LogicalScroll` remains M4 command behavior and is not a semantic-node action or compatibility alias.

Built-in and downstream widgets use the same checked erased protocol. Every widget declares persistent `State`; stateless widgets use `State = ()`. Recursive `map_action` changes only application-action plumbing and preserves widget/state identity and semantic contribution. `Button::on_activate` is a repeatable action factory; retired `on_press` and M2 `WidgetSemanticProof` authority are absent.

## Core/runtime bridge

The doc-hidden `runenui_core::__runtime` module exists only because core and runtime are separate crates. It safely consumes transient elements into checked erased widget/state plumbing needed by `runenui_runtime`. It is outside the prelude, unstable, unsupported for application use, and provides no live runtime namespace, mounted/semantic arena, queue, sequence, publication, or mutation authority.

Both core and runtime remain safe-Rust by repository policy. Payload/type mismatches are diagnosed and fail closed rather than granting unchecked typed access.

## Must not own

`runenui_core` must not own persistent mounted or semantic storage, semantic publication/revisions, live scheduling, queue/work/trace state, application state, native windows/accessibility adapters, concrete renderer backends, ECS integration, or legacy compatibility paths. Runtime alone owns live identity allocation, reconciliation, routing, scheduling, publication, and semantic action resolution.

M0–M5 are complete. M5 semantics/testing ownership is closed without moving live authority into core; M6 is the next renderer-neutral scene milestone and does not change this core/runtime boundary unless an accepted M6 protocol decision requires a public host-neutral scene value.

See the [public API contract](../../docs/architecture/public-api.md), [workspace structure](../../docs/architecture/workspace-structure.md), [M5 charter](../../docs/architecture/m5-semantics-and-testing-charter.md), [status map](../../docs/status-map.md), and [roadmap](../../docs/roadmap.md).
