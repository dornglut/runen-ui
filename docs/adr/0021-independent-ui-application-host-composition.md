# ADR 0021 (PROPOSED): Independent UI application and native host composition

> **Category:** Proposed ADR — review artifact only
>
> **Status:** **Proposed, NOT ACCEPTED; NOT IMPLEMENTED.** This document is carried on a draft investigation PR. It MUST NOT be treated as active architecture or implementation authority before a separately authorized owner review and protected acceptance.
>
> **Decision input:** [Investigation #541](https://github.com/dornglut/runen-ui/issues/541); prior [#463](https://github.com/dornglut/runen-ui/issues/463) explicitly rejected a generic runner and remains binding until revised.
>
> **Related target decisions:** [#353 multi-surface](https://github.com/dornglut/runen-ui/issues/353#issuecomment-6090346752), [ADR 0020 host-neutral environment](0020-host-neutral-ui-environment.md), [ADR 0019 input arbitration](0019-event-correlated-host-input-arbitration.md). Their *target* APIs are not currently implemented.
>
> **Scope:** Standalone RunenUI application ergonomics; independently hosted/headless compatibility; bounded native host composition. Not a new UI runtime, multi-window implementation, Engine API, or accepted M13 capability.

## The product problem

RunenUI's accepted application contract is already typed and host-neutral: a `UiApp` defines state, actions, update, root, effects and subscriptions. `AppRuntime<App>` is the **single** live UI owner. However a developer who wants an ordinary native window has to reimplement the winit/wgpu/AccessKit host composition.

At the inspected accepted baseline, Counter's application model is 55 lines; its native host is 1,138 lines. The specialized reference host is 2,871 lines; Gallery's **unmerged** native host adds a further 1,103 lines. The latter two are not interchangeable implementations, yet all must manage startup, window and GPU targets, wake/pump, publication, pending/displayed identity, device adapters, accessibility, retry, focus and shutdown.

The earlier [#463](https://github.com/dornglut/runen-ui/issues/463#issuecomment-6120679273) deliberately authorized only pure native mapping/ingress helpers because *the evidence then considered* did not justify a generic host runner. That remains the accepted restriction. #541 asks whether independently usable, easy-to-start **application** hosting is now an independently justified product requirement. Simply moving 1,000 lines of Counter code into another source file does not solve this.

The design goal is: **one application model; optional capability-specific hosts; only one live UI authority**. It is not “one universal loop for every environment”.

## Proposed decision, subject to the gates below

1. **Keep `UiApp` canonical.** Do not add a second application state, queue, reconciliation, effects engine, focus manager or renderer-neutral tree. The current static associated-type model is valid; avoid making users define their app twice.
2. **Qualify an optional native application-host package** consuming the *public* `runenui_core`, `runenui_runtime`, `runenui_winit` and `runenui_render_wgpu` contracts. If a second real consumer and Cargo/dependency analysis justify the boundary, a candidate name is `runenui_host_winit`. It alone owns its own winit event loop, native target orchestration, renderer lifecycle and presentation observation. This would be an **explicit change** to the previous “application-owned loops” choice, approved only when this ADR is accepted.
3. **Do not move the native host into `runenui_winit`.** That crate's accepted purpose is native input/AccessKit translation, with no renderer dependency and no host loop. Do not put winit, wgpu or AccessKit dependencies into `core` or `runtime`.
4. **Do not require a new host-neutral `App` wrapper.** First qualify a native builder taking `App::State` directly plus `RuntimeConfig`, application-provided styles, resources, fonts, capability requirements and optional host-effect implementation. A shared immutable prepared-app definition is justified *only* if more than one real profile benefits from a single valid pre-mount configuration boundary.
5. **Keep production headless and embedded use independent.** `AppRuntime::<App>::mount(...)` and explicit `pump`, input, publication and shutdown remain first-class public integration for headless, external renderers, game engines and tools that own their event loop. `runenui_testing` is a test harness, not the general production headless runner.
6. **Do not claim multi-window support.** The first native host may implement one explicitly documented native window for the current single-surface runtime. The ownership structure must remain compatible with #353's future **one runtime, application-declared `SurfaceKey`, generational surface domains and host-owned `WindowId ↔ SurfaceId` association**, without introducing a permanent “first window = entire app” semantic shortcut.

**This is a conditional recommendation**, not owner acceptance to implement the new package. Its viability depends on the compiler and lifecycle gates at the end of this document.

## Current public facts versus target API

The existing accepted `UiApp` trait has associated `State`, `Action` and `HostProtocol`, with static `root(state)` and `update(state, action)`. `AppRuntime::<App>::mount_with_config(state, config)` returns `AppRuntime<App>`, not a fallible builder or `App` instance. Its `pump`, native-neutral input and publication APIs are public. `StyleEnvironment` is presently supplied through `SurfaceBuildContext` during publication.

The following example is **an unimplemented design candidate**, NOT compiling against accepted RunenUI and NOT approved syntax:

```rust
use runenui_host_winit::{NativeUiHost, NativeWindowOptions};

// CounterApp is a UiApp whose associated State is Counter.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    NativeUiHost::<CounterApp>::builder(Counter::new())
        .window(NativeWindowOptions::new().title("Counter"))
        .styles(ui::style_environment())
        .run()?;
    Ok(())
}
```

The same exact application type remains usable today in a host-neutral program:

```rust
let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
let batch = runtime.pump(PumpBudget::new(64, 64, 64, 64))?;
```

A game or external renderer keeps calling the public runtime itself; it must not be made to enter a second library-controlled winit loop. A headless user controls quiescence, clock, budgets and logical publication explicitly. Only a *native-host* method owns `run()`. Adding `.run_headless()` solely to make a chain look symmetrical would be a misleading abstraction.

### Public API questions that the compiler must settle

- Generic construction: `NativeUiHost::<App>::builder(state: App::State)` and `RuntimeConfig` ownership; ensure Rust can infer the correct associated app, preserve the initial state on *pre-mount* configuration error, and avoid duplicate state.
- Typed `HostProtocol`: a no-host-command default may be available for `NoHostProtocol` only. For a real command/response protocol, an explicit host executor/handler must provide exact typed completion and cancellation or the host must refuse the capability before mount. No effect is silently discarded.
- External images and fonts: preserve caller's complete `ResourceRef`-keyed `ResourceProvider` and runtime-owned font source policy; do not replace missing resource or platform API with fictitious success. A native system-font default and deterministic headless default are distinct, explicitly documented profiles.
- Native `WindowOptions` are native-layer types, not `UiApp` state or neutral surface semantics. Screen geometry/scaling must not be conflated with `RasterScale`, locale, accessibility facts or theme.
- Typed `Result` and ordinary `main() -> Result` propagate native fatal errors. An explicit process exit bypassing required teardown is not the success path.

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

**Open unresolved contract:** accepted present code acknowledges redraw after *successful publication*; #353 target says acknowledge per-surface redraw **after successful Present**. These are materially different contracts. The correct runtime owner must decide if future publication-complete and presentation-complete tokens need distinct semantics or if the target contract should change. Do **not** silently change current `acknowledge_redraw` or accept the proposed host on an unproven assumption.

**Revocation:** native focus loss, occlusion, suspend, surface loss and destroy immediately fence input before cleanup can race queued work, while canonical runtime still owns focus/composition/pressed states and exact retirement. Native device IDs, IME generation, active key lifetimes, recorded pointer/capture receipts, window epochs, and renderer/frame identity must not cross incarnations. Host-native pointer lock/relative/gameplay policy belongs to #424/external host, never to this default UI host.

**Shutdown:** native close, normal runtime `Closed`, terminal `RuntimeTerminalReason`, native fatal `HostError`, and incomplete `shutdown() -> Result<InputShutdownBatch, InputObservationError>` are **different outcomes**. Shutdown observation may fail *before the shutdown mutation*, so do not report that runtime/native input custody was retired merely because a `Drop` method ran. The resulting fallible exit must preserve the *primary error* and any *cleanup error* for callers/operators. No plain `eprintln!(); event_loop.exit(); Ok(())` on a fatal path.

## Alternatives and reasons

| Alternative | Position | Why |
| --- | --- | --- |
| Documentation/test refactor only | Useful independent cleanup, not sufficient | Does not remove repeated lifecycle knowledge from ordinary native apps |
| Existing `runenui_winit` pure input/mapping helpers only | Retain; not enough for ordinary applications | #463 was right to avoid false sharing; a third native host shows the residual window/present/bootstrap burden |
| An **optional** native composition layer over `AppRuntime` | **Investigate and qualify** | Encapsulates actual reusable native orchestration without forcing an event loop on embedding hosts |
| A host-neutral `PreparedApp<App>` descriptor | Defer unless source/compile proof shows real shared pre-mount burden | Extra wrapper increases conceptual/public complexity; `UiApp` is already the model |
| One all-platform universal `App::run()`/runner | Reject | Would confuse explicitly bounded headless execution with native loop ownership and violate embedded engine lifecycle |
| Runenwerk providing standalone RunenUI host | Reject | Violates independent repo/product dependency direction |
| One `AppRuntime` for each window | Reject for a single multi-window application | Conflicts with #353's single state/action/work authority |

## Required acceptance proof and delivery boundaries

This **draft** is not itself acceptance and does not alter #463. Before an owner accepts the new optional native host boundary:

1. A **compiler-backed external-consumer prototype** must validate real associated `UiApp::State` / `HostProtocol` signatures, fallible host startup, the selected native API, type-safe resource and font configuration and both positive and negative capability cases. No dummy native runner that merely holds an `AppRuntime` qualifies.
2. A state-transition design review must trace exact wake/clock startup, bounded pump, publication/retry/present, AccessKit, IME/keyboard/source admission, focus/occlusion/suspension, GPU device loss and error/cleanup outcomes. Validate no double work/queue/renderer/focus authority.
3. One **ordinary independent external native app**, one second materially different consumer (Gallery only after its native draft is appropriately reconciled), a headless consumer without native dependencies, and an embedded owner of its own loop must compile/test on exact accepted APIs. Do not borrow experimental #539/#460 branches as accepted dependencies.
4. Selected native operating-system walkthrough must cover a real window, click/Tab, text/IME as supported, accessibility, resize/DPI, focus loss, surface loss and normal/fatal process exit; skip claims are explicit.
5. Cargo dependency graph/build cost and public API clean-cut migration must be measured. Any new host crate should be opt-in and must not pollute `core`, `runtime`, or `runenui_testing`.
6. Reconcile #353 single-runtime multi-surface and ADR 0020 pre-root environment requirements in **accepted** owner contracts. Keep Counter introductory, Gallery catalog, reference_winit specialized conformance. Refactor tests and layout primitives in separate bounded issue(s) with no proof deletion.
7. Independent complete-diff review, exact-head canonical `cargo validate`, public-consumer tests and real-wgpu/native evidence; protected merge only with explicit owner authorization.

**Research probe accompanying this draft:** `tests/external_host/tests/host_composition_probe.rs` compiles **existing** `UiApp`-associated state/config composition in a downstream crate and verifies two distinct apps can mount and drive the ordinary runtime. It deliberately imports **no winit/AccessKit/native host** and does **not** establish that the proposed native builder, multiple surfaces, clock startup or fatal-exit contract exists or compiles. This is useful narrow evidence only, not implementation acceptance.

**Disposition:** Proposed conditional choice of opt-in native composition, subject to the above gates and a deliberate owner-approved change to #463. No accepted production source, existing native host or conformance matrix is changed by this investigation.
