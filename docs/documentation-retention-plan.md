# Documentation Retention and Disposition Plan

> **Category: Current contract**

This record distinguishes retained active documents from paths removed during M0B. Active inventory tables use **Document path**; the removed inventory uses **M0 baseline path** so deleted paths cannot be mistaken for current files. Removed content remains recoverable through Git history; legacy content also has the annotated archival tag documented below.

Allowed categories are **Current contract**, **Target architecture**, **ADR**, **Guide**, **Historical record**, and **Obsolete**.

## Retained active documents

| Document path | Category | Current disposition | Purpose |
|---|---|---|---|
| `README.md` | Current contract | Retained active | Truthful entry point, current proof, production profiles, canonical links, validation, and licensing |
| `docs/architecture.md` | Target architecture | Retained active | Canonical current-versus-target pipeline and ownership overview |
| `docs/status-map.md` | Current contract | Retained active | Subsystem maturity using the canonical status taxonomy |
| `docs/feature-support-matrix.md` | Current contract | Retained active | Capability-level support, limitations, and milestone ownership |
| `docs/roadmap.md` | Current contract | Retained active | M0–M12 dependency gates and completion criteria |
| `docs/documentation-retention-plan.md` | Current contract | Retained active | Active/removed document disposition and recovery record |
| `docs/architecture/styling.md` | Target architecture | Retained active | Consolidated current and target styling contract |
| `docs/architecture/layout.md` | Target architecture | Retained active | Consolidated current and target layout/measurement contract |
| `docs/architecture/events-effects-and-scheduling.md` | Target architecture | Retained active | Event, command, effect, scheduling, trace, and controller-navigation direction |
| `docs/architecture/workspace-structure.md` | Target architecture | Retained active | Current workspace and evidence-based extraction rules |
| `docs/adr/0001-typed-token-authoring.md` | ADR | Retained active | Accepted typed-expression token authoring decision |
| `docs/adr/0002-keep-layout-in-runtime.md` | ADR | Retained active | Accepted layout ownership and extraction-gate decision |
| `docs/history/legacy-archive.md` | Historical record | Retained active | Legacy tag, removal, recovery, and bounded salvage guidance |
| `docs/vocabulary.md` | Current contract | Retained active | Explicit current and target terms |
| `docs/influences.md` | Guide | Retained active | Non-authoritative upstream lessons and candidates |
| `docs/tooling/validation.md` | Guide | Retained active | Locked root/nested validation, stable formatting, and link-checker scope |
| `crates/runenui_core/README.md` | Current contract | Retained active | Actual crate ownership and limitations |
| `crates/runenui_runtime/README.md` | Current contract | Retained active | Actual runtime proof ownership and limitations |
| `examples/counter/README.md` | Guide | Retained active | Current headless Counter proof and non-goals |
| `tools/context/README.md` | Guide | Retained active | Context profiles, coverage, budgets, and exclusions |
| `AGENTS.md` | Current contract | Retained active | Repository authority, architecture, workflow, and validation rules |
| `CONTRIBUTING.md` | Guide | Retained active | Contributor workflow and quality expectations |
| `SECURITY.md` | Current contract | Retained active | Private vulnerability reporting and current support scope |
| `CODE_OF_CONDUCT.md` | Current contract | Retained active | Conduct scope, enforcement, private reporting, and non-retaliation |
| `CHANGELOG.md` | Current contract | Retained active | Pre-release change record |
| `docs/release-policy.md` | Current contract | Retained active | Publication gate, release checklist, and `1.0.0` criteria |
| `docs/api-stability.md` | Current contract | Retained active | 0.x compatibility and public stability guidance |
| `docs/toolchain-policy.md` | Current contract | Retained active | Stable formatter, MSRV, and toolchain-update policy |
| `docs/dependency-policy.md` | Current contract | Retained active | Dependency, license, MSRV, and adoption review baseline |
| `docs/unsafe-code-policy.md` | Current contract | Retained active | Safe-Rust default and future unsafe-boundary review |
| `LICENSE-MIT` | Current contract | Retained active | Owner-approved MIT license with RunenUI copyright notice |
| `LICENSE-APACHE` | Current contract | Retained active | Unmodified Apache License 2.0 text |
| `xtask/README.md` | Guide | Retained active | Shared validation task ownership and usage |

## Removed or consolidated M0 baseline documents

The paths below are historical M0 baseline paths and no longer exist at `HEAD`.

| M0 baseline path | Baseline category | Completed disposition | Replacement or recovery |
|---|---|---|---|
| `docs/crate-map.md` | Obsolete | Removed in M0B | `docs/architecture/workspace-structure.md`; Git history |
| `docs/dependency-map.md` | Obsolete | Consolidated and removed in M0B | `docs/architecture/workspace-structure.md`, `AGENTS.md`; Git history |
| `docs/cutover-plan.md` | Obsolete | Removed in M0B | `docs/history/legacy-archive.md`; Git history |
| `docs/legacy-audit.md` | Obsolete | Replaced and removed in M0B | `docs/history/legacy-archive.md`; Git history and archival tag |
| `docs/target-api.md` | Obsolete | Accepted direction migrated; fake API removed in M0B | `docs/architecture/events-effects-and-scheduling.md`; Git history |
| `docs/audits/RunenUI-deep-audit-and-remediation-backlog.md` | Historical record | Open work migrated and backlog removed in M0B | Status/support/roadmap authorities; Git history |
| `docs/architecture/computed-style-model.md` | Current contract | Consolidated and removed in M0B | `docs/architecture/styling.md`; Git history |
| `docs/architecture/computed-style-runtime-integration.md` | Obsolete | Consolidated and removed in M0B | `docs/architecture/styling.md`; Git history |
| `docs/architecture/styling-target.md` | Target architecture | Consolidated and removed in M0B | `docs/architecture/styling.md`; Git history |
| `docs/architecture/token-authoring-ergonomics.md` | ADR | Converted and removed in M0B | `docs/adr/0001-typed-token-authoring.md`; Git history |
| `docs/architecture/token-reference-target.md` | Target architecture | Consolidated and removed in M0B | `docs/architecture/styling.md`; Git history |
| `docs/architecture/layout-constraints-measurement-contract.md` | Current contract | Consolidated and removed in M0B | `docs/architecture/layout.md`; Git history |
| `docs/architecture/layout-boundary-review.md` | ADR | Converted and removed in M0B | `docs/adr/0002-keep-layout-in-runtime.md`; Git history |

## Historical recovery

All removed documents remain recoverable through ordinary Git history. The removed 671-file `legacy/` tree is additionally recoverable at annotated tag `legacy-runenwerk-ui-archive-2026-07-11`, which points to audited baseline `141f005289e1ac2fe5d66c6cc35622f5db8d3406`. Removed legacy crates are not active authority and must not be restored to the active workspace; individual contracts, fixtures, or test ideas may be reimplemented only when an active design requires them.
