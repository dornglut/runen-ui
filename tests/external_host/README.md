# External Host Conformance

`runenui_external_host_conformance` is the M7D downstream embedding proof. It is a non-publishable workspace fixture, not a production host framework.

The fixture consumes only ordinary public `runenui_core`, `runenui_runtime`, and `runenui_render_wgpu` contracts. It deliberately has no winit, `runenui_winit`, AccessKit, testing-convenience, direct wgpu, or private-runtime dependency.

Its focused proof keeps frame ownership visible in caller code:

```text
submit -> pump -> take redraw -> publish -> acknowledge -> render -> present
```

A resource-provider failure is injected only after successful publication and acknowledgement. The caller retains that exact immutable publication and retries renderer work directly; it does not pump or republish unchanged runtime state. A later semantic action is constructed from the published semantic snapshot, submitted through ordinary runtime ingress, explicitly pumped, and rendered as a second frame.

The offscreen renderer path uses the same `runenui_render_wgpu::Renderer` and complete opaque `ResourceRef` provider contract as the accepted native hosts, while leaving event-loop and presentation policy entirely outside the framework.
