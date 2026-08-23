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
| Testing | usable | public deterministic headless harness over ordinary public runtime contracts | scene/backend/native-host assertions follow those later capabilities |
| Styling | proof | validated typed style/token proof and computed style/provenance | production themes/recipes/state layers/property breadth belong to M7 |
| Layout/measurement | proof | deterministic measurement/layout proof with gaps/padding/linear child layout and invalidation | production responsive layout breadth belongs to M7 |
| Renderer-neutral paint/hit scenes | proof | accepted M6A retained-publication substrate, M6B canonical immutable paint/hit products and displayed-generation hit cutover, plus M6C composition/resources/metadata/damage/capabilities | M6D independent-consumer/migration closure remains |
| Concrete renderer backend | absent | none | M10 after renderer-neutral scene protocol is implemented |
| Native window/event-loop host | absent | none | M10 |
| Native accessibility adapter | absent | none | M10 |
| Production text shaping/editing | absent | routed text/IME transport only | M8 |
| Standard control library | partial | proof-level built-in authoring widgets used by conformance/examples | production control behavior/recipes/accessibility breadth belongs to M9 |
| Multi-window/multi-surface host lifecycle | absent | one logical surface proof | M10 |
| Stable facade/release | absent | lower-level pre-1.0 crates only | M11 |

## Milestone summary

M0–M5 form the accepted headless foundation: repository/tooling policy, typed core values, open widget/component architecture, mounted runtime/layout/style proofs, deterministic effects and routed interaction, trace/replay, semantic publication/action ingress, and public deterministic testing.

M6 has accepted M6A retained-publication, M6B canonical paint/hit scene behavior, and M6C composition/resource/metadata/damage/capability behavior, but remains incomplete until its M6D independent-consumer/migration obligations are accepted. M7 and later remain successor outcomes. Accepted target documents do not promote capability maturity by themselves; maturity changes only when accepted default-branch implementation and required conformance evidence change.

See the [roadmap](roadmap.md) for durable sequencing and [conformance](conformance/README.md) for permanent observable/proof contracts.
