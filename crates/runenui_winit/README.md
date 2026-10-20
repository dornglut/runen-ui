# `runenui_winit`

> **Category: Library reference**

`runenui_winit` is the reusable native translation edge proven by the second real M7 winit consumer. It packages only mechanics that are independent of an application's host-loop policy:

- stable host-session mapping from winit device IDs to neutral `InputDeviceId` values;
- loss-preserving native keyboard lifetime and key translation;
- native mouse lifetime/button translation into neutral pointer events;
- AccessKit projection over ordinary `SemanticPublication` plus exact semantic-action translation.

It intentionally does **not** own or hide a winit event loop, window creation, `AppRuntime::pump`, wake/redraw handling, surface publication or acknowledgement, displayed-frame authority, renderer configuration/recovery, or presentation. Those remain visibly host-owned in each application. The crate has no `runenui_render_wgpu` dependency.

The adapter state is derived edge state. RunenUI runtime, semantic, input, and publication authority remain in `runenui_runtime`; native types do not enter `runenui_core` or `runenui_runtime`.

## Checked native device reincarnation

`DeviceIdentityMap::known` reads an existing mapping without allocating. Once
the host has **independently established** retirement of that exact
window-event source and has completed all required keyboard, touch, pointer,
gameplay and runtime-settlement cleanup, it may invoke
`retire_exact(native, expected)`. A stale expected neutral identity cannot
retire a newer incarnation, and subsequent `resolve` allocates a fresh,
nonreused neutral identity. An unknown removed source never allocates one.

**Raw `DeviceEvent::Removed` is not enough to identify a window keyboard
source.** winit 0.30 notes that raw-device and window-event device identities
may differ, including virtual devices. Hosts must prove the correlation or
choose an independently justified conservative source policy; never guess
the mapping or cancel another source's pressed keys. This utility does not
perform device cancellation, establish M13 processing finality or decide
game input permission.
