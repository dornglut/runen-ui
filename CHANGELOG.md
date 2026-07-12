# Changelog

## Unreleased

### Changed

- Completed M1 public API and vocabulary repair with validated logical values,
  IDs/keys/token IDs, deterministic duplicate diagnostics, typed element builders,
  arity-free children, reduced preludes, and read-only generated products.
- Replaced the nested `element!` property grammar with ordinary builder
  expressions and canonical `on_press`; removed prototype compatibility APIs.
- Restricted `Action: Clone` to activation paths and documented public enum/trait
  evolution policy.
- Corrected M1 identity to compare, order, and hash by Unicode-validated text
  independent of static/owned storage; literal and dynamic validation now share
  one grammar, and token families are explicitly non-exhaustive.
- Corrected derived geometry to saturate at finite boundaries and identity
  diagnostics to use true numeric preorder with deterministic same-node ordering.

> **Category: Current contract**

All notable changes to RunenUI are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses Semantic Versioning as qualified by the [API stability policy](docs/api-stability.md).

## [Unreleased]

### Changed

- Reframed RunenUI truthfully as a pre-1.0 headless architecture proof with required headless, desktop, and embedded production profiles.
- Added canonical status, support, architecture, documentation-retention, and M0–M12 roadmap authorities.
- Archived the historical Runenwerk UI tree at `legacy-runenwerk-ui-archive-2026-07-11` and removed it from active content.
- Consolidated incremental architecture documents into durable styling, layout, event/effect, ADR, and history records.
- Reset workspace packages from `1.0.0` to `0.1.0` and disabled publication.
- Established dual MIT/Apache-2.0 licensing, governance, toolchain, stability, release, and validation policies.

No stable release has been published.
