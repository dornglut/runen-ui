# ADR 0021 (ACCEPTED): Fluent independent UI applications and host composition

> **Category:** Architecture decision — narrowly scoped opt-in native host
>
> **Status:** **Owner-accepted architecture decision.** Effective as durable authority upon protected merge of PR #542; it does **not** accept or ship the native implementation in #544. The protected merge and separate delivery gates remain mandatory.
>
> **Decision input:** [Investigation #541](https://github.com/dornglut/runen-ui/issues/541); this decision narrowly supersedes the earlier [#463](https://github.com/dornglut/runen-ui/issues/463) rejection of a reusable independent native application host, **not** its restriction against a universal runner or its pure-adapter ownership boundaries.
>
> **Related target decisions:** [#353 multi-surface](https://github.com/dornglut/runen-ui/issues/353#issuecomment-6090346752), [ADR 0020 host-neutral environment](0020-host-neutral-ui-environment.md), [ADR 0019 input arbitration](0019-event-correlated-host-input-arbitration.md). Their *target* APIs are not currently implemented.
>
> **Scope:** Standalone RunenUI application ergonomics; independently hosted/headless compatibility; bounded native host composition. Not a new UI runtime, multi-window implementation, Engine API, or accepted M13 capability.

## The product problem

RunenUI's accepted application contract is already typed and host-neutral: a `UiApp` defines state, actions, update, root, effects and subscriptions. `AppRuntime<App>` is the **single** live UI owner. However a developer who wants an ordinary native window has to reimplement the winit/wgpu/AccessKit host composition.

At the inspected accepted baseline, Counter's application model is 55 lines; its native host is 1,138 lines. The specialized reference host is 2,871 lines; Gallery's **unmerged** native host adds a further 1,103 lines. The latter two are not interchangeable implementations, yet all must manage startup, window and GPU targets, wake/pump, publication, pending/displayed identity, device adapters, accessibility, retry, focus and shutdown.

The earlier [#463](https://github.com/dornglut/runen-ui/issues/463#issuecomment-6120679273) deliberately authorized only pure native mapping/ingress helpers because *the evidence then considered* did not justify a generic host runner. That narrower no-runner restriction remains except for this explicitly bounded native-host exception. #541 established independently usable, easy-to-start **application** hosting as a distinct product requirement. Simply moving 1,000 lines of Counter code into another source file does not solve this.

The design goal is: **one application model; optional capability-specific hosts; only one live UI authority**. It is not “one universal loop for every environment”.

## Decision boundary for a protected ADR acceptance

The accepted ownership change is **only** permission for one opt-in, independently packaged, native **single-window application host** behind a small fluent `App` facade. That host composes the existing public runtime, native input/semantics adapter and renderer; it does not acquire mounted state, scheduler, input finality, editing, semantic, layout or renderer authority. It is not a universal headless/embedded/game loop, multi-surface implementation, Counter-specific runner or Runenwerk dependency. Expert external hosts remain free to own their own loop.

This ADR supersedes **only** #463's earlier conclusion that no *stateful reusable native application host* was justified by the then-available consumers. It **retains** #463/#464's pure-adapter boundaries and the existing specialized reference host, and does not by itself migrate Counter/Gallery, amend #353, ship `App::run()` or authorize an implementation merge. The broad generic-runner restriction remains binding.

**Separate gates:** owner approval of this narrow architecture/ownership boundary is recorded by this ADR. Correctness, public API compatibility, exact-head execution, platform support, proof and acceptance of a *specific implementation* belong to delivery #543. An unmerged implementation candidate may provide evidence for review but never becomes the accepted architecture, dependency or capability merely by compiling.

## Primary developer contract — target only

**The accepted owner preference for investigation #541 is a RunenUI-first fluent facade at the same ergonomic level as Runenwerk:**

```rust
App::new()
    .window("Counter")
    .ui(Counter::new())
    .run()?;
```

**This code is an authorized target public contract, not yet a shipped API.** An unmerged delivery draft may compile it; that is not production acceptance. The fluent `App` is a small configuration/composition facade, **not** a new runtime. Its purpose is to let the user supply UI state and optional window intent while hiding ordinary native bootstrap. The existing `AppRuntime` remains the sole live mounted/input/work/publication authority.

The straightforward type-safe approach needs **no `CounterApp` marker**: make the application state itself implement the existing `UiApp` contract with `type State = Self`. The current accepted trait supports that relationship directly:

```rust
struct Counter { count: i32 }
enum CounterAction { Increment }

impl UiApp for Counter {
    type State = Self;
    type Action = CounterAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self) -> impl View<Self::Action> {
        text(format!("Count: {}", state.count))
    }

    fn update(
        state: &mut Self,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            CounterAction::Increment => state.count += 1,
        }
    }
}
```

The following is the **candidate type structure, not shipped library code**:

```rust
pub struct App { /* window/capability intent, not runtime */ }
pub struct ConfiguredApp<A: UiApp> { /* one A::State, explicit intent */ }

impl App {
    pub fn new() -> Self { /* ... */ }
    pub fn window(self, title: impl Into<WindowTitle>) -> Self { /* ... */ }
    pub fn ui<A: UiApp<State = A>>(self, state: A) -> ConfiguredApp<A> { /* ... */ }
    pub fn ui_with_state<A: UiApp>(self, state: A::State) -> ConfiguredApp<A> { /* ... */ }
}
```

Here `ui(self, model)` infers the application type from the value itself, requiring an actual `UiApp<State = Self>` implementation. **An arbitrary state object cannot identify a distinct `UiApp` marker**, because multiple app implementations could share the same associated `State`; Rust cannot safely reverse-lookup that type. `ui_with_state::<A>(state)` is an explicit advanced route for existing marker-based or separately typed applications, not a requirement for Counter. This distinction avoids blanket trait implementations, macros, registries, type erasure or a second application framework.

A non-generic staging `App` followed by a typed `ConfiguredApp<A>` lets the facade accept `.window(...).ui(...)` or `.ui(...).window(...)` without moving a live host/runtime between modes; exact fluent method return types and capabilities must be compile-proved in the real package. Ordinary `Window::primary().title(...).size(...)` can be offered later as a typed advanced configuration; the shorthand string is a **title**, never a logical surface key or OS identity.

## Authorized execution semantics, subject to delivery qualification

1. **One canonical UI runtime.** Both fluent and expert entry paths mount `AppRuntime<A>` exactly once, retaining typed actions/effects, host requests, input finality and publication. No second queue, state or focus manager.
2. **Explicit capability selection.** `.window(...)` selects the optional native platform host *before* runtime admission. The native host owns winit events, wgpu presentation, AccessKit and native input lifetime; the low-level `runenui_winit` remains the pure adapter. Core/runtime/headless and embedded users incur **no native dependencies**.
3. **Meaning of `run()` in the authorized first slice.** A configured **native-window** application enters its native event-driven loop with typed return/error handling. A missing window is a typed unsupported-configuration error, never a silent fabricated headless or GPU run. A future windowless `.run()` or distinct headless executor requires a **separate accepted** wake/clock/exit/finite-advancement contract and is not included in this ADR's first native delivery. Headless and foreign/game hosts keep their own loops and use public `AppRuntime` directly.
4. **No hidden defaults.** Styles, host effects, fonts, resources, native initial focus and OS capability failures must be explicit or have documented valid defaults. For an ordinary native window, conveniences can choose system fonts without changing deterministic bundled-only headless behavior.
5. **Surface correctness.** Current RunenUI has one runtime surface. Future #353 requires one app with `SurfaceKey`-declared logical domains mapped to OS windows by the host; do not auto-create a second runtime or permanently equate a native window title with a `SurfaceId`. A single `.ui()` registers the application, not one independent `UiApp` per window.
6. **No premature implementation acceptance.** This architecture decision changes only the narrow opt-in native-host ownership boundary. #463's rejection of an all-purpose generic runner remains in force; delivery #543 must independently establish the user-visible API and proof gates.

## Compile evidence: what the draft probe can and cannot say

The type-only external-host test includes a **locally defined** fluent type facade and a `Counter`-like model that implements the actual public `UiApp` trait with `State = Self`. It checks that `.window("Counter").ui(Counter::new())` can infer the app type and that the resulting state executes through the real `AppRuntime` FIFO. It also checks a separate marker-based app with `ui_with_state::<A>(state)`; the current public marker route stays type-safe.

**This is not a shipped `App`, native `.run()` or production desktop capability.** A compiler pass proves the *associated-type relationship* but not actual host creation, startup-order correctness, fallible configuration, font/resource/host protocol admission, native lifecycle or shutdown. Those require a separate implementation/CI/real-native proof after architecture acceptance.

## Owner map

| Responsibility | Sole authority / implementation role | Prohibited duplication |
| --- | --- | --- |
| Product state, actions, application styles, initial focus intent, required host services | Application (`UiApp` and explicit configuration) | Native host hard-coding Counter-specific choices |
| Mounted tree(s), canonical queue/clock/time, effects/work, input routing, semantic/focus/edit authority, logical publication, trace, terminal status | `runenui_runtime` | Host-local queue, stateful focus policy, alternative scheduler, noncanonical UI node store |
| Pure winit input identities and keyboard/IME/pointer translation, semantic AccessKit projection | `runenui_winit` | Second native mapping/parser embedded in host |
| Paint realization, GPU cached resources, surface/backend status and exact renderer observation | `runenui_render_wgpu` | Host reinterpreting widgets or semantic tree |
| Window/event loop/native target attachment, native input source, wake dispatch, GPU target creation and renderer composition, accessibility adapter lifetime, confirmed displayed mapping, native retry/failure/cleanup | Optional native host if approved | UI semantic ownership or cross-window application state |
| Host loop/OS target/gameplay arbitration when external/embedded | External consumer | Library seizing that application's main loop |
| Multi-surface logical identity/domains and roots | Future #353 runtime cutover | Native WindowId used as a fabricated SurfaceId |
| Host-neutral locale/accessibility/scale facts consumed during authoring | Future ADR 0020/runtime protocol cutover; app supplies content/theme policy | Late native getter treated as independent UI truth |

## Lifecycle and error contract: a real design gate

**Native startup:** validate type-level/static configuration and required capability declarations before irreversible acquisition; acquire native event loop/proxy when needed; choose monotonic clock, wake transport and send executor at a *qualified* runtime initialization boundary; mount one `AppRuntime<App>` and inspect its status/typed mount result; validate initial application effects and host service requirements; create native window and GPU target and install accessibility before allowing displayed input. Errors must be classified as before/after mount and cleanup must be completed/observed as permitted, without reporting a successful run.

Today's runtime **mounts and initializes work before hosts call `set_monotonic_clock` and `set_wake_transport`**, and may enter `RuntimeStatus::Terminal` during construction despite returning `Self`. A future host cannot assume late clock/wake install is semantically equivalent to pre-mount configuration, or invent a `Result` signature without a reviewed runtime change. Coordinate with #353's future fallible mount proposal and moving #539 runtime work, rather than building a compatibility scheduler.

**Ordinary execution:** runtime grants work via one `pump(PumpBudget)` and returns `InputPumpBatch` / `PumpOutcome`, pause, next deadline and work-readiness data. An empty/idle loop sleeps until a legitimate wake/deadline or platform event. A bounded observation failure is retried through the existing wake protocol, not treated as a completed input receipt, arbitrary idle sleep or terminal close. Host requests and services are handled only through the existing typed public paths.

**Publication and native presentation:** describe states explicitly:

```text
Dirty (runtime-owned)
  -> Published / immutable pending frame (runtime-authoritative output)
  -> Present attempted (host/renderer; may fail recoverably)
  -> Presented (confirmed successful native Present observation)
  -> Displayed input context + exact native mapping (host-owned correlation)
```

Recovery of timeout/occlusion/outdated/lost renderer must retain and retry the **correct immutable publication** where still compatible, not invoke `App::root` to fabricate another output or advance displayed pointer authority. Resize/scale/target generation changes may invalidate the pending target mapping, requiring an explicit fresh compatible publication; no old physical coordinates are rebound to a different surface. AccessKit projection is derived from published semantics; it must not create a competing UI tree authority.

**Normative first-slice single-surface rule:** use the **accepted** `acknowledge_redraw` contract after *successful publication*, retain the same immutable pending publication across recoverable render/Present retries while its target mapping remains valid, and promote the **displayed input context only after confirmed successful Present**. Do not issue a second redraw acknowledgment on Present or fabricate a new publication to retry. This freezes the existing accepted contract for the native facade, not a claim that all loss/retry cases have been qualified. **Separate future #353 decision:** its proposed per-surface acknowledgment after successful Present differs materially and requires an explicit runtime-owner migration/receipt decision before multi-surface cutover; the native host may not silently redefine either token.

**Revocation:** native focus loss, occlusion, suspend, surface loss and destroy immediately fence input before cleanup can race queued work, while canonical runtime still owns focus/composition/pressed states and exact retirement. Native device IDs, IME generation, active key lifetimes, recorded pointer/capture receipts, window epochs, and renderer/frame identity must not cross incarnations. Host-native pointer lock/relative/gameplay policy belongs to #424/external host, never to this default UI host.

**Shutdown:** native close, normal runtime `Closed`, terminal `RuntimeTerminalReason`, native fatal `HostError`, and incomplete `shutdown() -> Result<InputShutdownBatch, InputObservationError>` are **different outcomes**. Shutdown observation may fail *before the shutdown mutation*, so do not report that runtime/native input custody was retired merely because a `Drop` method ran. The resulting fallible exit must preserve the *primary error* and any *cleanup error* for callers/operators. No plain `eprintln!(); event_loop.exit(); Ok(())` on a fatal path.

## Alternatives and reasons

| Alternative | Position | Why |
| --- | --- | --- |
| Documentation/test refactor only | Useful independent cleanup, not sufficient | Does not remove repeated lifecycle knowledge from ordinary native apps |
| Existing `runenui_winit` pure input/mapping helpers only | Retain; not enough for ordinary applications | #463 was right to avoid false sharing; a third native host shows the residual window/present/bootstrap burden |
| Optional native implementation **behind the fluent facade** | **Qualify** | Ordinary applications get simple setup; embedded/engine hosts retain existing external loop/render ownership |
| Fluent user-visible `App` facade, keeping `UiApp`/`AppRuntime` canonical | **Primary design target** | Removes marker and native bootstrap from ordinary app call sites without introducing another UI runtime |
| One *universal execution loop* behind `App::run()` | Reject | Native, headless and embedded execution retain distinct owners; one fluent spelling does not imply one runner/scheduler |
| Runenwerk providing standalone RunenUI host | Reject | Violates independent repo/product dependency direction |
| One `AppRuntime` for each window | Reject for a single multi-window application | Conflicts with #353's single state/action/work authority |

## Required acceptance proof and delivery boundaries

This accepted **architecture boundary** changes only the permission described above; it does not certify or ship the implementation, change the single-runtime or adapter boundaries, or relax the required delivery evidence. The local Rust type probe and source comparison informed the decision but do not certify a native executable.

**The following are separate delivery #543 production acceptance gates**, even if provisional implementation code already exists before its own acceptance:

1. First prove the direct-self-state `UiApp<State = Self>` and local fluent `.ui(model)` signature with real public Rust types, then separately **compiler-back an actual public `App` facade and native `.run()?`**: correct associated types, optional capabilities, fallible mount/startup, `HostProtocol`, fonts/resources and positive/negative cases. A local API-shape mock is **necessary but not sufficient**; it cannot establish that native hosting works.
2. A state-transition design review must trace exact wake/clock startup, bounded pump, publication/retry/present, AccessKit, IME/keyboard/source admission, focus/occlusion/suspension, GPU device loss and error/cleanup outcomes. Validate no double work/queue/renderer/focus authority.
3. One **ordinary independent external native app**, one second materially different consumer (Gallery only after its native draft is appropriately reconciled), a headless consumer without native dependencies, and an embedded owner of its own loop must compile/test on exact accepted APIs. Do not borrow experimental #539/#460 branches as accepted dependencies.
4. Selected native operating-system walkthrough must cover a real window, click/Tab, text/IME as supported, accessibility, resize/DPI, focus loss, surface loss and normal/fatal process exit; skip claims are explicit.
5. Cargo dependency graph/build cost and public API clean-cut migration must be measured. Any new host crate should be opt-in and must not pollute `core`, `runtime`, or `runenui_testing`.
6. Reconcile #353 single-runtime multi-surface and ADR 0020 pre-root environment requirements in **accepted** owner contracts. Keep Counter introductory, Gallery catalog, reference_winit specialized conformance. Refactor tests and layout primitives in separate bounded issue(s) with no proof deletion.
7. Independent complete-diff review, exact-head canonical `cargo validate`, public-consumer tests and real-wgpu/native evidence; protected merge only with explicit owner authorization.

**Research probe accompanying this decision:** `tests/external_host/tests/host_composition_probe.rs` checks existing public trait composition and a **local** fluent `.window(...).ui(...)` shape where the UI model implements `UiApp<State = Self>`, plus a separate explicit marker/state path. It deliberately imports **no winit/AccessKit/native host** and does **not** establish that a public `App`, `.run()?`, multiple surfaces, clock startup or fatal-exit contract exists or compiles. This is narrow compiler evidence only, not implementation acceptance.

**Disposition:** **ACCEPTED architecture:** fluent `App` as the primary UX, composed over one optional single-window native host and one canonical UI runtime. The scoped #463 supersession becomes active through protected ADR merge. This decision does **not** implement or accept #543, ship a native API, alter #353, or change an accepted conformance matrix. Delivery #543 remains blocked from production acceptance until its own independent gates pass.
