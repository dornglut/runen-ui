# Changelog

RunenUI has not published a public package release. This file summarizes user-visible changes planned for the first `0.1.0` release. Detailed development history, reviews, exact commits, and CI evidence remain in GitHub history, issues, and pull requests.

## [Unreleased]

### Added

- Host-neutral typed `UiApp`, `View`, `Element`, `Widget`, state/action/effect protocols, and standard control authoring.
- Persistent keyed mounted runtime; deterministic routed input, focus, scheduling, lifecycle, trace, and semantic publication.
- Renderer-neutral styling, Block/Flex/Grid layout, paint/hit products, deterministic motion, and international text shaping/caret maps.
- Application-owned transactional text editing with runtime-managed sessions, logical scrolling, input services, and native IME/clipboard integration.
- Standard text, button, selection, collection, scroll, popup/dialog, menu, and ComboBox controls within the accepted M11 scope.
- Reusable wgpu renderer, winit input/accessibility adapter, headless testing harness, and native/embedded reference consumers.
- Scoped event-correlated UI input arbitration; broader native/Engine conformance remains separately qualified.

### Changed

- Consolidated architecture and acceptance contracts under ADRs, documentation ownership, and permanent conformance matrices.
- Established the current public `GPL-3.0-only` distribution and separate commercial-licensing policy; see [Licensing](LICENSING.md).
- Kept all workspace packages unpublished while the feature-complete `0.1.0` release gate remains open.

For exact accepted capability maturity, consult [Current status](docs/status.md); for unfinished production outcomes, consult the [Roadmap](docs/roadmap.md). The full pre-release delivery record remains accessible in the Git history preceding this consolidation.
