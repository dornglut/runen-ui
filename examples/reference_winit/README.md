# Standalone winit reference host

This package is the M7 reference application for native window/event-loop/input integration. It is intentionally an application boundary, not a reusable platform crate. `runenui_render_wgpu` remains winit-free and owns GPU/surface rendering state; `runenui_runtime` remains the sole UI/runtime authority.

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
