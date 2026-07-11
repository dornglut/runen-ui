# Documentation Retention and Disposition Plan

> **Category: Current contract**

This inventory assigns one authority category and one disposition to every documentation file active at the M0 baseline. M0B consolidations/removals are now complete on the program branch after protecting legacy history. M0C adds governance and release-policy documents.

Allowed categories are **Current contract**, **Target architecture**, **ADR**, **Guide**, **Historical record**, and **Obsolete**.

## Root and canonical authority

| Current path | Category | Current accuracy | Decision | Replacement or destination | Deletion milestone | Notes |
|---|---|---|---|---|---|---|
| `README.md` | Current contract | Rewritten in M0A | Keep as entry point | Canonical status/support/roadmap/architecture links | — | Must remain truthful and use implemented examples only |
| `docs/architecture.md` | Target architecture | Rewritten in M0A | Keep as canonical overview | Accepted current/target split and linked ADRs | — | Must not present target pipeline as implemented |
| `docs/status-map.md` | Current contract | Replaced in M0A | Keep canonical | Current subsystem maturity | — | Uses absent/planned/proof/partial/usable/stable/deferred/archived |
| `docs/feature-support-matrix.md` | Current contract | Added in M0A | Keep canonical | Capability-level support record | — | Types alone never imply support |
| `docs/roadmap.md` | Current contract | Replaced in M0A | Keep canonical | M0–M12 milestones and gates | — | One primary milestone owns each capability family |
| `docs/documentation-retention-plan.md` | Current contract | Added in M0A | Keep through M0; then maintain as documentation map | This file | — | Update whenever documents change category or disposition |

## Obsolete maps, cutover, and target sketches

| Current path | Category | Current accuracy | Decision | Replacement or destination | Deletion milestone | Notes |
|---|---|---|---|---|---|---|
| `docs/crate-map.md` | Obsolete | Stale skeleton status and speculative crate list | Delete | Canonical architecture plus `docs/architecture/workspace-structure.md` | M0B | Duplicate authority |
| `docs/dependency-map.md` | Obsolete | Current small graph partly accurate; target graph stale | Consolidate valid rules, then delete | `docs/architecture/workspace-structure.md` and `AGENTS.md` | M0B | No separate duplicate map |
| `docs/cutover-plan.md` | Obsolete | Initial clean cutover is complete | Delete | `docs/history/legacy-archive.md` for historical context | M0B | Completed plan is not active architecture |
| `docs/legacy-audit.md` | Obsolete | Recommends keeping active legacy tree, contradicting M0 | Replace then delete | `docs/history/legacy-archive.md` | M0B | New note points to archival tag/history |
| `docs/target-api.md` | Obsolete | Contains nonexistent facade/runtime builder and speculative effects API | Migrate accepted effects direction, then delete | `docs/architecture/events-effects-and-scheduling.md`; current examples in README/guides | M0B | Fake API must not remain discoverable |
| `docs/vocabulary.md` | Obsolete | Mixes implemented and nonexistent terms (`on_change`, `LayoutBox`, `Primitive`) | Rewrite | Canonical current/target vocabulary document | M0B | Must identify current versus target terms |
| `docs/audits/RunenUI-deep-audit-and-remediation-backlog.md` | Historical record | Valuable 2026-07-11 audit but baseline/process and open-work facts are superseded | Migrate all open work, then remove from active tree | `docs/roadmap.md`, status/support matrices, Git history | M0B | Production audit/charter supplied for M0 remain external execution inputs; active repository keeps no duplicate backlog |

## Architecture documents

| Current path | Category | Current accuracy | Decision | Replacement or destination | Deletion milestone | Notes |
|---|---|---|---|---|---|---|
| `docs/architecture/computed-style-model.md` | Current contract | Accurate for implemented narrow style model | Consolidate | `docs/architecture/styling.md` | M0B | Preserve concrete resolver/provenance facts |
| `docs/architecture/computed-style-runtime-integration.md` | Obsolete | Implemented record mixed with completed slice plan | Consolidate current contract, then delete | `docs/architecture/styling.md`; Git history for slice plan | M0B | Completed implementation plan is not durable architecture |
| `docs/architecture/styling-target.md` | Target architecture | Mostly aligned; resolution order differs from accepted charter | Consolidate and correct | `docs/architecture/styling.md` | M0B | Canonical doc separates current proof from target |
| `docs/architecture/token-authoring-ergonomics.md` | ADR | Decision remains useful | Convert to numbered ADR, then delete source | `docs/adr/0001-typed-token-authoring.md` | M0B | Preserve decision and consequences without incremental-plan text |
| `docs/architecture/token-reference-target.md` | Target architecture | Accepted token model is implemented; contains stale “next slice” and unused `LengthToken` direction | Consolidate | `docs/architecture/styling.md`; M1 owns dead vocabulary | M0B | Do not present completed sequencing as current |
| `docs/architecture/layout-constraints-measurement-contract.md` | Current contract | Mostly accurate; ends with stale “boundary review next” | Consolidate | `docs/architecture/layout.md` | M0B | Preserve constraints/provider/one-measurement/overflow contract |
| `docs/architecture/layout-boundary-review.md` | ADR | Accurate formal decision not to extract layout | Convert to numbered ADR, then delete source | `docs/adr/0002-keep-layout-in-runtime.md` | M0B | Preserve reviewed baseline, decision, triggers, and consequences |
| `docs/architecture/workspace-structure.md` | Target architecture | Current workspace accurate; long-term graph and near-term PR list are stale | Rewrite/keep | Same path | — | Remove completed PR sequence; align with M0–M12 and extraction gates |

## Guides, crate, example, and tooling documents

| Current path | Category | Current accuracy | Decision | Replacement or destination | Deletion milestone | Notes |
|---|---|---|---|---|---|---|
| `docs/influences.md` | Guide | Links useful; vocabulary implies unsupported output | Keep short and non-authoritative; correct wording | Same path | — | Cite lessons, not equivalence or dependency decisions |
| `docs/tooling/validation.md` | Guide | Describes divergent local baseline and omits MSRV/link checks | Rewrite | Same path | M0C update | Must describe one local/CI implementation |
| `crates/runenui_core/README.md` | Current contract | Future-tense responsibilities and stale dependency/status links | Rewrite | Same path | M0B/M0C update | State actual closed proof and limitations; package README metadata points here |
| `crates/runenui_runtime/README.md` | Current contract | Mostly current but overclaims capture/accessibility/primitive extraction | Rewrite | Same path | M0B/M0C update | Package README metadata points here |
| `examples/counter/README.md` | Guide | Describes implemented behavior as future target | Rewrite | Same path | M0B/M0C update | Clearly label it a headless proof, not renderer/backend proof |
| `tools/context/README.md` | Guide | Accurate operation; full-audit profile still includes legacy and default types omit audit-relevant files | Update | Same path | M0B | Document comprehensive audit coverage and historical exclusion |

## M0B retained documents

| Destination | Category | Purpose |
|---|---|---|
| `docs/architecture/styling.md` | Target architecture | Current/target styling contract replacing five incremental documents |
| `docs/architecture/layout.md` | Target architecture | Current/target layout contract replacing the incremental constraint plan |
| `docs/architecture/events-effects-and-scheduling.md` | Target architecture | Accepted event/effect direction migrated from fake target API and charter |
| `docs/adr/0001-typed-token-authoring.md` | ADR | Preserves typed-expression token authoring decision |
| `docs/adr/0002-keep-layout-in-runtime.md` | ADR | Preserves reviewed no-extraction decision and revisit triggers |
| `docs/history/legacy-archive.md` | Historical record | Records archival tag, removal, salvage rules, and recovery instructions |

## M0C documents to add

| Destination | Category | Purpose |
|---|---|---|
| `AGENTS.md` | Current contract | Repository authority, scope, preflight, validation, architecture and workflow rules |
| `CONTRIBUTING.md` | Guide | Supported contributor workflow, changes, tests, commits, and review |
| `SECURITY.md` | Current contract | Private vulnerability reporting and support scope |
| `CODE_OF_CONDUCT.md` | Current contract | Contributor conduct and enforcement baseline |
| `CHANGELOG.md` | Current contract | Pre-release change record and Keep-a-Changelog policy |
| `docs/release-policy.md` | Current contract | Pre-1.0 versioning, publication gate, release checklist, and `1.0.0` criteria |
| `docs/api-stability.md` | Current contract | 0.x compatibility and public API stability guidance |
| `docs/toolchain-policy.md` | Current contract | Stable/MSRV definitions, update policy, and local/CI requirements |
| `LICENSE-*` or `LICENSE` | Current contract | Owner-selected legal license text and Cargo metadata |

## M0B deletion gate

M0B may remove a document only after its still-valid current facts, target constraints, ADR decision, or open work appear in the destination listed above. Historical implementation sequencing remains recoverable through Git history. The `legacy/` tree may be deleted only after the annotated archival tag exists and the history note records recovery.
