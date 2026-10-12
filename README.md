# RunenUI

RunenUI targets host-neutral, renderer-neutral Rust UI for headless, standalone, and embedded application profiles. The repository is pre-1.0 and currently provides a deterministic headless framework foundation plus a proof-level real host/renderer/accessibility production spine; it is not yet a production-complete desktop UI stack.

## Framework overview

RunenUI is a host-neutral, renderer-neutral Rust UI framework. Applications author
typed state, actions, transient `View`/`Element` trees and public standard
controls; the mounted runtime owns reconciliation, focus, input routing, text
editing sessions, layout, semantics, deterministic work and atomic publication.

- `runenui_core` defines the public authoring, interaction, layout/style,
  semantic, visual and motion contracts.
- `runenui_text` shapes text into retained renderer-neutral artifacts, shared
  with editing, layout and presentation.
- `runenui_runtime` implements mounted identity and the single live UI
  authority, including deterministic scheduling and renderer-facing publication.
- `runenui_testing` provides downstream deterministic testing ergonomics
  through ordinary public core/runtime contracts.
- `runenui_render_wgpu` and `runenui_winit` provide reusable downstream
  GPU, input and accessibility edges without becoming widget-behavior owners.
- `examples/counter` is the small runnable application; `examples/reference_winit`
  is the specialized native-host integration reference. Neither is a
  comprehensive control gallery.

For **current accepted capabilities and maturity**, see the canonical
[current status](docs/status.md) and permanent [conformance contracts](docs/conformance/README.md).
For **planned capability and dependencies**, see the [roadmap](docs/roadmap.md).
This overview intentionally does not duplicate milestone acceptance snapshots.

## Workspace

```text
runenui_text        -> runenui_core
runenui_runtime     -> runenui_core + runenui_text
runenui_render_wgpu -> runenui_core + runenui_runtime + runenui_text
runenui_winit       -> runenui_core + runenui_runtime
runenui_testing     -> runenui_core + runenui_runtime
```

- `runenui_core` owns host-neutral public values and protocols, including normalized authored layout, bounded widget-measurement vocabulary, renderer-neutral visual/composition values, authored deterministic motion descriptions, and application-owned transactional editing values.
- `runenui_text` owns renderer-neutral production font/shaping/line-breaking/logical-text resources and immutable retained-layout caret/preedit mapping behind RunenUI contracts.
- `runenui_runtime` owns live framework authority and orchestrates style, interaction, bounded owner-local editing sessions and edit-chain reconciliation, text measurement, production layout, live deterministic motion, correlated visual/hit/focus/semantic geometry, visual/composition publication, and aligned surface publication.
- `runenui_render_wgpu` is the reusable concrete renderer/resource edge over ordinary public paint publication; external images use the caller provider, retained logical shaped text is realized directly as renderer-owned SDF/MSDF state, and accepted M9 geometry/brush/clip/group/shadow behavior plus already-sampled motion products are realized only as disposable renderer state, including retained-publication retry and cache/device reconstruction.
- `runenui_winit` owns reusable winit input and AccessKit semantic/editable translation/projection, not a host loop, renderer, document, or editing session.
- `runenui_testing` is a downstream public testing convenience layer over ordinary runtime contracts, including deterministic manual-time integration rather than a private expected-motion engine.
- `tests/external_host` is an unpublished winit-free downstream host conformance consumer over public core/runtime/renderer/resource contracts.
- `tests/external_renderer` is an unpublished downstream renderer-neutral conformance consumer over ordinary public core/runtime scene contracts.
- `xtask` owns repository validation tooling and has no framework dependency.

See [workspace structure](docs/architecture/workspace-structure.md) for the enforced ownership/dependency contract.

## Examples

- [Counter](examples/counter/README.md) is the compact application-level showcase. Run the native application with `cargo run --package counter`, or the deterministic headless walkthrough with `cargo run --package counter --bin counter`.
- [`reference_winit`](examples/reference_winit/README.md) is the specialized standalone host/integration reference. Run it with `cargo run -p reference_winit --release`; its package guide documents the optional structured native proof logging mode.

These examples consume ordinary public framework contracts. They are demonstrations and integration references, not separate runtime, renderer, or conformance authorities.

## Validation

```text
cargo validate
```

For intentional Rust edits, format first with:

```text
cargo +stable fmt --all
```

Focused tests and conformance proofs remain required for the active change. See [TESTING.md](TESTING.md) and [validation details](docs/tooling/validation.md).

## Documentation

- [Architecture](ARCHITECTURE.md)
- [Documentation index](docs/README.md)
- [Current status](docs/status.md)
- [Roadmap](docs/roadmap.md)
- [Public API contract](docs/architecture/public-api.md)
- [ADRs](docs/adr/)
- [Conformance](docs/conformance/README.md)
- [API stability](docs/api-stability.md)
- [Release policy](docs/release-policy.md)

Live work, blockers, pull-request state, exact heads, and CI evidence belong in GitHub rather than durable documentation.

## License

RunenUI is currently publicly licensed under [`GPL-3.0-only`](LICENSE). A separate commercial-license model may be available from copyright holder(s) with sufficient rights; see [`LICENSING.md`](LICENSING.md). Until reviewed inbound terms preserve commercial relicensing authority, external PRs contributing tracked repository content are not accepted. Issue reports, design discussion, reviews, and reproducible cases may still be accepted.
