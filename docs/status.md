# Current Status

> **Category: Current status**

This file is the single durable owner of RunenUI capability maturity. It describes accepted default-branch capability, not active implementation progress, branch state, pull-request state, or blocker state.

## Maturity vocabulary

- **absent** — no accepted implementation exists;
- **planned** — roadmap/accepted target contract exists, but implementation is absent;
- **proof** — bounded implementation demonstrates the architecture/conformance contract but is not production breadth;
- **partial** — meaningful implementation exists but required production behavior is incomplete;
- **usable** — intended profile works for real downstream use with documented limitations;
- **stable** — compatibility/support policy is deliberately committed and release-grade;
- **deferred** — intentionally outside the active production path;
- **archived** — historical only; not active authority.

No current subsystem is `stable`.

## Capability map

| Area | Maturity | Current accepted capability | Decisive limitation / durable next owner |
|---|---|---|---|
| Application model | proof | typed `UiApp` state/action/update and transient typed view authoring | broader ergonomic facade remains later production work |
| Mounted runtime | proof | persistent keyed generational mounted tree, state/lifecycle, invalidation, exact targeting | production breadth still depends on later subsystems |
| Effects/scheduling | proof | bounded FIFO/pump, tasks/timers/subscriptions/host requests, deterministic time, wake/redraw, terminal/shutdown | broader host/platform operational breadth remains later |
| Routed interaction | proof | pointer, focus scopes/navigation, keyboard, committed text, composition, automation, semantic commands, plus accepted loss-preserving native winit translation reused by the reference host and native Counter | production controls/text remain later; supported platform breadth belongs to M13 |
| Trace/replay | proof | bounded canonical trace, deterministic export, optional sink, inert offline replay | production devtools/inspection UI remains later |
| Semantics/accessibility core | proof | independent semantic identity/tree/update/action ingress with deterministic public testing and an accepted native AccessKit projection/action round-trip | broader platform/accessibility profiles belong to M13 |
| Testing | usable | public deterministic headless harness over ordinary public runtime contracts, including latest public paint/hit publication inspection, exact input-context derivation, runtime convergence, semantics, trace, and replay | concrete backend/native-host assertions remain separate platform evidence |
| Styling | proof | validated typed style/token proof and computed style/provenance | production themes/recipes/state layers/property breadth belong to M8 |
| Layout/measurement | proof | deterministic measurement/layout proof with gaps/padding/linear child layout and invalidation | production responsive layout breadth belongs to M8 |
| Renderer-neutral paint/hit scenes | proof | complete accepted M6 protocol: retained publication, canonical immutable paint/hit products, composition/resources/metadata/damage/capabilities, two independent deterministic consumers, downstream public-contract consumption, testing convergence, and proof-era authority migration closure | concrete production breadth remains later |
| Concrete renderer backend | proof | accepted reusable wgpu renderer/resource edge with real offscreen pixels, provider/cache realization, exact golden/readback evidence, renderer observations, and real native presentation through the accepted winit hosts | external-host closure remains M7; broader production/platform profiles remain later |
| Native window/event-loop host | proof | accepted reusable winit translation edge plus standalone reference host and native Counter application, each with host-owned wake/pump/redraw, displayed-frame mapping, resize/raster-scale handling, native input, real wgpu presentation, and bounded target recovery | external-host closure remains M7; supported platform breadth belongs to M13 |
| Native accessibility adapter | proof | accepted reusable AccessKit adapter over ordinary semantic publication with stable adapter-owned identity, exact delta/full-resync behavior, exact action translation, install-before-show ordering, proxy callbacks, and host-thread runtime ingress, exercised by both native hosts | broader platform profiles belong to M13 |
| Production text shaping/editing | absent | routed text/IME transport only | M8 owns shaping/layout foundations; M10 owns production editing |
| Standard control library | partial | proof-level built-in authoring widgets used by conformance/examples | production control behavior/recipes/accessibility breadth belongs to M11 |
| Multi-window/multi-surface host lifecycle | absent | one logical surface proof | M13 |
| Stable facade/release | absent | lower-level pre-1.0 crates only | M15 qualifies the first public `0.1.0`; `1.0.0` is the later compatibility/support stability gate |

## Milestone summary

M0–M5 form the accepted headless foundation: repository/tooling policy, typed core values, open widget/component architecture, mounted runtime/layout/style proofs, deterministic effects and routed interaction, trace/replay, semantic publication/action ingress, and public deterministic testing.

M6 is accepted complete at `proof` maturity: retained publication, canonical renderer-neutral paint/hit products, composition/resource/metadata/damage/capability semantics, independent-consumer proof, public testing convergence, and migration closure are accepted on the default branch. M7A, M7B, M7C, and the bounded native Counter application showcase are accepted at `proof` maturity: the reusable wgpu renderer/resource edge, reusable winit translation and AccessKit adapter edge, real offscreen/readback and golden evidence, standalone reference-host native input/presentation, and the same Counter application exercised through deterministic headless and real native execution are accepted on the default branch. M7 remains incomplete only until M7D proves the winit-free external host-owned frame loop and completes closure reconciliation. Accepted target documents do not promote capability maturity by themselves; maturity changes only when accepted default-branch implementation and required conformance evidence change.

See the [roadmap](roadmap.md) for durable sequencing and [conformance](conformance/README.md) for permanent observable/proof contracts.
