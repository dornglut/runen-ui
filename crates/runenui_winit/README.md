# `runenui_winit`

> **Category: Library reference**

`runenui_winit` is the reusable native translation edge proven by the second real M7 winit consumer. It packages only mechanics that are independent of an application's host-loop policy:

- stable host-session mapping from winit device IDs to neutral `InputDeviceId` values;
- loss-preserving native keyboard lifetime and key translation;
- native mouse lifetime/button translation into neutral pointer events;
- AccessKit projection over ordinary `SemanticPublication` plus exact semantic-action translation.

It intentionally does **not** own or hide a winit event loop, window creation, `AppRuntime::pump`, wake/redraw handling, surface publication or acknowledgement, displayed-frame authority, renderer configuration/recovery, or presentation. Those remain visibly host-owned in each application. The crate has no `runenui_render_wgpu` dependency.

The adapter state is derived edge state. RunenUI runtime, semantic, input, and publication authority remain in `runenui_runtime`; native types do not enter `runenui_core` or `runenui_runtime`.
