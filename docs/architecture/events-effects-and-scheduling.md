# Events, Effects, and Scheduling

> **Category: Target architecture**

This document records the accepted M4 direction. It is not an implemented API sketch.

## Current proof

The runtime currently performs synchronous direct dispatch, rebuilds the full root, and clears focus. It has typed pointer/keyboard vocabulary, traversal focus, rectangle targeting, press-based button activation, an overlapping input-intent path, and a coarse unbounded trace. There is no event propagation, pointer identity/capture, action queue, effect executor, timer, subscription, cancellation, wake scheduling, text/IME event stream, or deterministic clock.

## Canonical target event path

```text
Host event
  -> normalization
  -> capture phase
  -> target phase
  -> bubble phase
  -> semantic default behavior
  -> application action
  -> update
  -> effects and invalidation
```

Pointer, keyboard, normalized controller/navigation, accessibility, automation, and programmatic activation converge on semantic control commands. Device-independent commands include next/previous and directional focus movement plus activate, cancel/back, menu, and context actions. The runtime tracks the active input modality without making device-specific vocabulary the semantic control contract. Default pointer button activation presses, captures the pointer, updates mounted pressed state, then activates on a still-valid release. Cancellation or release outside prevents activation.

Pointer input needs stable pointer identity and device kind, capture ownership, cancellation, movement, buttons, wheel/scroll, coordinates, and only device facts required by real consumers. Keyboard commands are distinct from text commit and IME composition events. Focus includes scopes, linear and directional/spatial navigation, and deterministic transition reasons.

Platform hosts own controller connection/disconnection, device identity, raw button/axis events, dead-zone and normalization policy, and mapping into the normalized command vocabulary. Embedded hosts provide the equivalent mapping. RunenUI core/runtime do not own platform gamepad APIs.

## Application update and effects

The simple application contract remains available:

```rust
fn update(state: &mut State, action: Action)
```

A scalable form returns or collects `Effects<Action>`. The exact API requires an ADR; examples here are conceptual, not implemented signatures.

Effects describe requested work. They never execute inside `update` and never belong to the renderer. The runtime/host owns the action queue, deterministic ordering, batching/reentrancy policy, tasks, timers, subscriptions, host commands, cancellation, wake/redraw scheduling, completion actions, errors, shutdown, and lifecycle ownership.

Completions re-enter application logic as normal typed actions. Mounted lifecycle removal cancels node-owned work. A deterministic executor and clock allow tests to advance and inspect work without wall time or platform services.

## Trace

Trace v2 is one canonical bounded record sequence with monotonic sequence numbers, tree/frame generation, target identity, and structured records for input, focus/capture, commands, actions, updates, effects, reconciliation, invalidation, layout, semantics, hit/paint publication, and shutdown. Sinks/export and replay must define text-input privacy/redaction.

## Required design decisions

M4 implementation requires reviewed ADRs for the concrete event API and effects/task/subscription API, including cancellation identity, Send/Sync requirements, single-threaded and embedded-host behavior, error propagation, ordering, and shutdown. This document fixes ownership and sequencing, not concrete Rust surface syntax.
