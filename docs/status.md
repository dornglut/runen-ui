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
| Effects/scheduling | proof | bounded FIFO/pump, tasks/timers/subscriptions/host requests, deterministic time, wake/redraw, terminal/shutdown | host/platform integration and production operational breadth remain later |
| Routed interaction | proof | pointer, focus scopes/navigation, keyboard, committed text, composition, automation, semantic commands | native translation and production controls/text remain later |
| Trace/replay | proof | bounded canonical trace, deterministic export, optional sink, inert offline replay | production devtools/inspection UI remains later |
| Semantics/accessibility core | proof | independent semantic identity/tree/update/action ingress with deterministic public testing | native accessibility adapter remains later |
| Testing | usable | public deterministic headless harness over ordinary public runtime contracts, including latest public paint/hit publication inspection, exact input-context derivation, runtime convergence, semantics, trace, and replay | concrete backend/native-host assertions follow those later capabilities |
| Styling | proof | validated typed style/token proof and computed style/provenance | production themes/recipes/state layers/property breadth belong to M8 |
| Layout/measurement | proof | deterministic measurement/layout proof with gaps/padding/linear child layout and invalidation | production responsive layout breadth belongs to M8 |
| Renderer-neutral paint/hit scenes | proof | complete accepted M6 protocol: retained publication, canonical immutable paint/hit products, composition/resources/metadata/damage/capabilities, two independent deterministic consumers, downstream public-contract consumption, testing convergence, and proof-era authority migration closure | M7 first proves a real renderer/host/resource/accessibility production spine; concrete production breadth remains absent |
| Concrete renderer backend | proof | accepted reusable wgpu renderer/resource edge with real offscreen pixels, provider/cache realization, exact golden/readback evidence, and renderer observations | native window/present lifecycle, native accessibility, and broader production profiles remain later M7/M13 work |
| Native window/event-loop host | absent | none | M7 first proves one real host integration; M13 completes supported platform profiles |
| Native accessibility adapter | absent | none | M7 first proves one real adapter path; M13 completes supported platform profiles |
| Production text shaping/editing | absent | routed text/IME transport only | M8 owns shaping/layout foundations; M10 owns production editing |
| Standard control library | partial | proof-level built-in authoring widgets used by conformance/examples | production control behavior/recipes/accessibility breadth belongs to M11 |
| Multi-window/multi-surface host lifecycle | absent | one logical surface proof | M13 |
| Stable facade/release | absent | lower-level pre-1.0 crates only | M15 qualifies the first public `0.1.0`; `1.0.0` is the later compatibility/support stability gate |

## Milestone summary

M0–M5 form the accepted headless foundation: repository/tooling policy, typed core values, open widget/component architecture, mounted runtime/layout/style proofs, deterministic effects and routed interaction, trace/replay, semantic publication/action ingress, and public deterministic testing.

M6 is accepted complete at `proof` maturity: retained publication, canonical renderer-neutral paint/hit products, composition/resource/metadata/damage/capability semantics, independent-consumer proof, public testing convergence, and migration closure are accepted on the default branch. M7A is accepted at `proof` maturity: the reusable wgpu renderer/resource edge, real offscreen/readback and golden evidence, provider/cache realization, and renderer observations are accepted on the default branch. M7B/C/D remain blocked and own the native host/input, accessibility, and external-host closure work. Accepted target documents do not promote capability maturity by themselves; maturity changes only when accepted default-branch implementation and required conformance evidence change.

See the [roadmap](roadmap.md) for durable sequencing and [conformance](conformance/README.md) for permanent observable/proof contracts.
