# Standalone winit reference host

This package is the native reference application for the RunenUI production spine. It is an application boundary, not a reusable platform crate. `runenui_render_wgpu` remains winit-free and owns GPU/surface rendering state; `runenui_runtime` remains the sole UI/runtime authority.

The window opens on a visible, application-owned editable document rather than an empty service probe. Its text, revision, and accepted edit resolutions belong to this example app; the widget contributes public editable and semantic facts, while input, selection, clipboard/IME services, and publication remain on the ordinary RunenUI runtime path. The reference text is intentionally simple: this is integration evidence, not a production editor or a claim of broad platform/hardware coverage.

Because the example is a native desktop host, it explicitly permits system-font discovery; deterministic consumers should instead register controlled font bytes under the default bundled-only policy.

Run the host with `cargo run -p reference_winit`. Click or drag in the text, type committed text, use native IME composition where supported, and try copy/cut/paste with a selected range. Native winit touch contacts are normalized through the public `runenui_winit` adapter using the exact displayed-frame mapping; mapping, focus, and suspension loss cancel live contacts. Pointer identity uses disjoint mouse/touch namespaces. File drops are admitted only at the exact widget target; the example does not open or read dropped files.

## M13 host-private real-time pointer-mode proof (#424)

The example also serves as a host-owned pointer-mode probe. **F7** toggles confined absolute cursor mode; **F8** toggles locked-relative game-input mode; **Escape** releases a pending or active mode. These are example-host policy hotkeys, not RunenUI actions or new core/runtime APIs. The demonstration does not implement a camera; native relative motion samples are counted by the host and never converted into RunenUI logical pointer events.

An exact displayed RunenUI surface identity and live native window epoch are required before requesting a native mode. The host cancels existing routed mouse/touch streams before acquiring gameplay lock. While relative mode is pending or active, ordinary absolute pointer, wheel, touch, file-drop, IME and editor keyboard ingress are withheld; native device motion remains at the host boundary. The native grab can be pending while its relative-motion stream remains unproven: the cursor stays visible and raw game input remains gated until the first finite native device-motion observation, which applies the host's hidden-cursor override before activating gameplay input. A five-second pending-motion watchdog releases an unusable grab and restores a visible cursor. This timeout is native host policy, not RunenUI scheduling. This is not proof that cursor visibility was visually realized or that the operating system cannot release the grab independently.

The host owns one native cursor writer: the M10 framework cursor request sets the ordinary UI shape/visibility baseline, while gameplay lock applies a temporary native visibility override. Escape, focus loss, suspension, destruction, host failure and exit attempt native release independently of the runtime queue. Native release failure is recorded with the native error, leaves UI/game input gated, and is retried at most once per second while the event loop remains active; it is never reported as success. Native mapping changes, loss of the displayed render surface, a changed issued SurfaceId, focus loss, occlusion and suspension revoke the lease. After successful release the system cursor remains visible until a fresh translated absolute cursor position is submitted, even if the retained M10 UI cursor baseline requested invisibility. A fresh absolute cursor position is required after return to UI. No native fallback silently substitutes one mode for another.

For native proof, enable RUNENUI_REFERENCE_PROOF=1, press F7/F8 and Escape, physically move the mouse, force focus loss (alt-tab), reenter, suspend/close, and inspect the stage=host_pointer_* lines. Record OS, display server (X11/Wayland), pinned winit version, exact result, cursor visibility, raw-motion delivery and restoration. Deterministic fake-host tests validate ordering and failure semantics but cannot prove OS realization. Platforms not actually run remain unproven. Pinned winit 0.30.13 documents macOS confinement and X11 lock as unsupported; an actual NotSupported result is expected negative evidence.

Multi-window/seat handoff is excluded from this single-window reference host. #353 owns RunenUI multi-surface identity and activation; a later authorized host consumer must consume that decision. The independent host-input arbitration investigation owns any reusable UI/gameplay input-consumption contract.

The public consumer boundary remains unchanged: `ReferenceHost::toggle_host_pointer` handles example-host F7/F8 intent and calls its private `PointerModes::request` on the live `winit::Window`; `ReferenceHost::device_event` accepts native `MouseMotion` for the host only; ordinary UI `CursorMoved` and button transitions continue through the existing `runenui_winit`-translated displayed-frame `SurfaceInputContext` and `AppRuntime` submit/pump/publication flow. `NativeFrameworkServices` continues executing the ordinary M10 cursor request, with its native writer delegated to the same host-private controller. On Escape, focus/activation loss or terminal conditions, the host releases the native grab before any runtime cleanup. These demo hotkeys are not an implementation of the separately accepted M13 event-correlated input-arbitration ADR; there is **no** generalized UI-modal/gameplay ownership claim here. An embedding engine MUST supply its own authorized game/menu interaction policy before consuming raw motion.

For reproducible large-document interaction checks, the same binary can initialize the ordinary application-owned editor state with deterministic generated text:

```text
cargo run -p reference_winit --release -- --large-document
cargo run -p reference_winit --release -- --stress-document
```

`--large-document` generates 4,000 lines at roughly 230 KB; `--stress-document` generates 16,000 lines at roughly 930 KB. Both fixtures include stable line/section markers, varied line lengths and wrapping, whitespace, combining graphemes, emoji/ZWJ sequences, and a small mixed-direction sample. They use the same application/runtime/text/render path as the default editor and are intended for repeatable manual scrolling, caret/selection, editing, clipboard, undo/redo, resize/reflow, and IME checks—not as a separate editor implementation or Unicode correctness corpus.

The selected native profile is this winit 0.30.13 desktop reference host, exercised on macOS/Metal in the local proof run. That verifies the host mechanisms and event translations, not attached-device touch, all IME implementations, or other platform backends; broader native hardware/platform coverage remains M13.

## Native wheel normalization

RunenUI's neutral wheel payload is a `LogicalDelta`, so native wheel units are normalized at this application edge rather than becoming core/runtime protocol. Winit `PixelDelta` values are physical pixels and are divided by the exact scale factor of the successfully displayed frame. Winit `LineDelta` values are abstract lines/rows; this standalone reference host deliberately maps one native line to **60 RunenUI logical units**. That line step is reference-host UX policy, not a framework constant, and another host may choose a different line metric while still emitting the same neutral logical-coordinate protocol.

Winit also reports native wheel gesture phase separately from displacement. The current accepted RunenUI wheel protocol does not carry native gesture phase, so phase-only zero-delta notifications are not submitted as wheel events and do not fabricate logical-scroll commands.

## Native proof logging

From a real desktop session with a working GPU/adapter, run the reference host and retain the complete stderr capture:

```text
RUNENUI_REFERENCE_PROOF=1 cargo run -p reference_winit --release 2>reference-proof.log
rg '^(RUNENUI_PROOF|RUNENUI_TRACE) ' reference-proof.log
```

Proof mode emits two correlated evidence streams:

- `RUNENUI_PROOF` records host-edge stages such as adapter/backend selection, native mapping and surface changes, publication/presentation, and neutral input translation.
- `RUNENUI_TRACE` prefixes the versioned canonical runtime JSON records delivered through a bounded subordinate trace sink. The runtime's ordinary retained canonical trace remains authoritative; the sink is an evidence export, not a second trace authority.

To extract the canonical records as plain JSONL:

```text
sed -n 's/^RUNENUI_TRACE //p' reference-proof.log >runtime-trace.jsonl
```

Before treating the exported runtime stream as evidence, verify that its `runenui.trace.record` `sequence` values start at `1` and remain contiguous through a clean close that includes `kind.name == "runtime_shutdown"`. Any sequence gap or missing clean-shutdown record means the bounded sink export is incomplete; discard that export and repeat the native run.

Use the host-stage records and canonical runtime records together for the `HOST-*` observations that require correlation, especially redraw acknowledgement (`HOST-02`), displayed-surface input authority (`HOST-04`), and keyboard/text/composition processing (`HOST-05`). Keep the complete stderr capture as well because withheld/suppressed host diagnostics are intentionally not duplicated into every structured proof record.

These logs are evidence aids, not proof by themselves; record the observed native window, GPU, presentation, resize, pointer, keyboard, text, and IME behavior alongside them. Committed-text and IME-preedit contents remain redacted in the canonical trace by default, and the host-stage proof records retain only their lengths/ranges. Logical keyboard key identities are intentionally preserved, including `LogicalKey::Character` values, because HOST-05 requires evidence of loss-preserving logical-key translation. Treat the captured proof log as potentially sensitive and review it before sharing.
