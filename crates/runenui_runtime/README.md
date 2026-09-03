# `runenui_runtime`

> **Category: Current contract**

`runenui_runtime` owns RunenUI's live deterministic mounted execution, scheduling, routing, style/text orchestration, tracing, layout execution, and surface publication. It consumes public `runenui_core` and `runenui_text` contracts and remains independent from `runenui_testing`, native platform implementations, and concrete renderer backends.

## Current ownership

Runtime owns:

- one persistent generational mounted tree with keyed reconciliation, widget state/lifecycle, interaction slots, focus, invalidation, and checked mounted targeting;
- one separate generational semantic arena plus exact mounted-owner/semantic-key bindings;
- one generalized sequenced FIFO and explicit bounded pump for application actions, routed commands/input, tasks, timers, subscriptions, host responses, and derived work;
- deterministic monotonic time, producer generations, cancellation/replacement, configured backpressure, wake/redraw authority, terminal/shutdown behavior, and one bounded canonical trace with deterministic export and inert replay;
- routed pointer, focus, keyboard, committed-text, composition, automation, and exact displayed-surface input behavior;
- production style-resolution orchestration over one explicit `StyleEnvironment`, including ephemeral projection of canonical hover/focus/active facts, shared staged activation for disabled state, exact environment/interaction cache compatibility, inspection, and property-effect-driven invalidation;
- the live production `TextSystem`, topology-aligned reusable `TextLayoutState`, lowering of runtime layout availability into renderer-neutral text constraints, and measurement from immutable logical text artifacts;
- exact reuse of those same logical text artifacts/resources during paint planning, with publication-retained shaped-resource leases that survive runtime destruction and renderer cache/device rebuild;
- the current proof-level general layout algorithm and one fallible staged surface-publication transaction;
- an independent semantic publication sibling with stable semantic identities, deterministic tree order, bounds, relationships, composed state/support, runtime-derived focus, revisions/updates, and typed diagnostics;
- public exact surface-scoped semantic action admission/resolution with private semantic-to-mounted owner/key resolution and convergence onto the existing command FIFO/routed/default/update/trace architecture.

## Identity and authority boundaries

Mounted and semantic IDs are distinct opaque runtime-issued identities under one runtime namespace. Mounted IDs address mounted widget lifetimes. Semantic IDs use a separate semantic arena and are retained by exact mounted-owner lifetime plus stable owner-local semantic key. Removing a semantic key revokes that semantic lifetime; replacing/removing its mounted owner revokes all owned semantic lifetimes. Later slot reuse advances generation and never retargets stale IDs.

Public semantic snapshots expose semantic identities only. They do not expose or reconstruct the private mounted owner. Semantic action target metadata is read-only origin context, not a mounted routing capability. There is no public semantic-to-mounted shortcut, bare semantic-ID surface guessing, second semantic queue/default engine, or semantic scrolling compatibility path.

Style interaction values are likewise projections, not new authorities. Pointer registry state remains hover/active authority, runtime focus remains focus authority, and the staged widget activation result remains disabled authority. Retained style projection/cache data exists only to determine compatibility and downstream work; it cannot become a second transient interaction state machine.

Text state follows the same ownership discipline. Runtime owns when text computation participates in mounted measurement/publication and which reusable state remains topology-aligned, but `runenui_text` owns font-source policy, shaping, line breaking, text-specific constraints, immutable artifacts, and logical shaped-resource bindings. Runtime neither reimplements those algorithms nor gives renderer/device state authority over logical text identity.

## Publication

`AppRuntime::publish_surface` is the sole live surface-publication authority. A successful publication contains aligned renderer-facing products plus the independent semantic publication, style inspection, and diagnostics. Renderer-facing products do not carry production semantic or style-policy authority.

Publication follows the accepted staged boundary:

```text
admit -> read-only/staged plan -> candidate-dependent final preflight -> commit
```

Recoverable refusal exposes no partial new publication and preserves the previous coherent products/identities/revisions. Checked counter, work/trace-sequence, and integrity exhaustion never wrap or saturate into false success.

Style and text planning participate in that same transaction. Disabled styling reuses the staged activation fact consumed by capability/semantic publication, and failed planning does not commit a partial new style or text-layout cache. Layout-affecting style/typography changes propagate through runtime-owned layout/hit/paint/semantic dependencies; paint-only foreground changes remain bounded when retained facts are otherwise compatible.

For text nodes, runtime measures from the immutable `TextArtifact` returned by the live text system and later projects the exact cached artifact's shaped run/resource facts into paint instead of reshaping or reminting at paint time. `PaintPublication` retains the exact shaped-resource leases referenced by its scene, allowing downstream renderer retry after runtime destruction without a new publication.

Renderer publication lineage and backend realization remain downstream authorities. Runtime-owned retained caches are derived publication-planning state and never substitute for renderer-local successful-publication/resource-cache state.

## Scheduling, routing, and tracing

The runtime has one canonical queue and work/trace lineage. Accepted semantic actions, pointer/keyboard/automation/programmatic commands, application actions, and scheduler outputs converge through existing sequencing rather than source-specific behavior engines. Submission rejection is fail-closed and returns owned input where the public API promises recovery. Accepted work is revalidated at processing boundaries and never retargets stale identities.

`AppRuntime::pump` is caller-bounded. Runtime never hides an unbounded settle loop or wall-clock sleep. Public deterministic testing composes these same APIs but owns no live runtime state or private mutation seam.

## Must not own

`runenui_runtime` must not own application-domain policy/state, testing convenience authority, native window/event-loop or accessibility adapters, concrete renderer backends, renderer/platform theme policy, text shaping/font-discovery/line-breaking algorithms, SDF/MSDF atlas/device realization, production controls/editing behavior, ECS assumptions, or compatibility aliases preserving retired prototype APIs. Host/platform code supplies explicit neutral style preference/environment inputs; runtime remains the live orchestrator rather than an ambient theme/provider or text-rendering authority.

See the [public API contract](../../docs/architecture/public-api.md), [layout/measurement architecture](../../docs/architecture/layout.md), [styling architecture](../../docs/architecture/styling.md), [`runenui_text` contract](../runenui_text/README.md), [workspace structure](../../docs/architecture/workspace-structure.md), [M5 semantic/testing charter](../../docs/conformance/m5-semantics-and-testing-charter.md), [current status](../../docs/status.md), [testing guide](../../TESTING.md), and [roadmap](../../docs/roadmap.md).
