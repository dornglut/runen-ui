# `runenui_winit`

> **Category: Library reference**

`runenui_winit` is the reusable native translation edge proven by the second real M7 winit consumer. It packages only mechanics that are independent of an application's host-loop policy:

- stable host-session mapping from winit device IDs to neutral `InputDeviceId` values;
- loss-preserving native keyboard lifetime and key translation;
- native mouse lifetime/button translation into neutral pointer events;
- AccessKit projection over ordinary `SemanticPublication` plus exact semantic-action translation.

It intentionally does **not** own or hide a winit event loop, window creation, `AppRuntime::pump`, wake/redraw handling, surface publication or acknowledgement, displayed-frame authority, renderer configuration/recovery, or presentation. Those remain visibly host-owned in each application. The crate has no `runenui_render_wgpu` dependency.

The adapter state is derived edge state. RunenUI runtime, semantic, input, and publication authority remain in `runenui_runtime`; native types do not enter `runenui_core` or `runenui_runtime`.

## Native keyboard admission lifecycle

`KeyboardInputState::key_input` translates an original physical Down into a
`KeyboardInputOutcome::Submit` containing both the neutral keyboard event
and one opaque `down_admission` token. The adapter's original device/key
entry starts **Pending**, not canonically admitted. The embedding host MUST
call `resolve_down_admission(token, KeyboardDownDecision::Queued)` only when
the canonical runtime actually admits the event to its queue; this does **not**
claim M13 settlement or UI conflict. Only an exact, definitive **pre-queue**
rejection of that same original Down (such as a proven `NoFocusedTarget`)
permits `KeyboardDownDecision::RejectedBeforeQueue`. A capacity, terminal,
ambiguous or missing-receipt failure does not qualify.

Pending original Down generations fail closed on repeats **and physical or
synthetic release**. An unresolved Up/Cancel returns the typed fatal
`KeyboardIngressFailure::UnresolvedOriginalDownAdmission` without erasing the
pending entry or fabricating a terminal runtime event. Whole-source/device
retirement still exposes unresolved Cancels for mandatory host M13 handling;
it is not a way to downgrade an unacknowledged original Down. Rejected generations
remain in the existing adapter pressed-key collection as physical tombstones.
Their repeats and authentic Up are suppressed as **UI events** (not physical
observations), and device/source cancellation produces no fictitious M13 Cancel.
Queued generations emit Up/Cancel and require normal M13 scope/finality handling.
Pending generations cannot issue ordinary Up until their exact admission decision;
if retirement occurs first, their generated device/source Cancel remains mandatory
and cannot be reclassified as never-admitted.

Admission tokens contain one checked adapter-lifetime identity and one
nonwrapping per-adapter Down generation: they cannot acknowledge
a key from a replaced or concurrent adapter, nor a retired generation. These
identifiers never carry runtime authority, an M13 work receipt or game permission.
Checked token exhaustion yields `KeyboardInputOutcome::Failed`, **not** an
ordinary suppressible diagnostic: the host must fail closed rather than
quietly resume native gameplay with an unrepresented physical Down.

The host still owns exact native source/window/device identity and any game
input policy. The RunenUI runtime alone owns M13 queue, processing, focus and
finality. Do not treat a rejected Down, suppressed repeat or derived Cancel as
proof of new gameplay eligibility. See the native example hosts and the
[M13 arbitration contract](../../docs/adr/0019-event-correlated-host-input-arbitration.md).
