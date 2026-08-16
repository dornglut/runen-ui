# RunenUI Testing and Validation

The canonical merge-readiness command is:

```text
cargo validate
```

It is repository-owned and read-only. The baseline includes stable formatting checks, locked workspace tests, Clippy with denied warnings, Rust 1.93.0 MSRV tests, repository metadata and authority audits, public API checks, unsafe-code checks, documentation consistency, and workspace-root Markdown link validation.

For intentional Rust edits, format first with:

```text
cargo +stable fmt --all
```

Also run the focused tests and conformance proofs owned by the active issue, followed by `git diff --check`. Exact-head GitHub Actions evidence must refer to the current reviewed head; an earlier successful run is not reusable after the head moves.

## Public deterministic application testing

M5D adds the public downstream `runenui_testing` crate for deterministic headless application tests. It depends only on public `runenui_core` and `runenui_runtime` contracts and does not enable `internal-test-seams`.

Its main public testing surface is `TestHarness<App>` plus explicit supporting values for deterministic surface configuration, bounded settling, and snapshot-scoped semantic queries/targets. The harness composes ordinary runtime APIs rather than owning a second runtime model:

- mount an ordinary `UiApp` with deterministic `ManualClock` authority;
- publish a deterministic non-zero fixed surface or inject an explicit public measurement/build context;
- submit pointer, keyboard, committed-text, composition, automation, application-action, exact-mounted command, and exact surface-scoped semantic-action ingress through existing public runtime methods;
- query exact committed semantic snapshots without first/last ambiguity fallback;
- retain `SurfaceId + SemanticNodeId` scope for semantic action helpers without reconstructing `MountedNodeId`;
- pump explicit `PumpBudget` values or call `run_until_idle` only with a finite `SettleBudget`;
- advance logical time without sleeping;
- inspect public application state, focus, reconciliation, publication/frame/layout/hit/paint/semantic products, scheduler observations, canonical trace, export, and inert replay.

`run_until_idle` reports idle only after a complete zero-progress quiescent pump. Dormant future timers, publication dirtiness, and externally pending host work do not cause hidden waiting; self-requeue remains bounded by the caller's explicit iteration and pump limits.

The testing crate is convenience authority only. It must not use doc-hidden runtime bridges, mutate mounted state, fabricate IDs or generations/sequences, seed runtime counters, replace publication snapshots, invoke callbacks directly, use wall-clock sleeps, provide a bare semantic-ID helper that guesses surface scope, expose a semantic-to-mounted routing shortcut, or recreate semantic `LogicalScroll` compatibility vocabulary.

The accepted M5D feature implementation was reviewed at exact head `471d2acf402a0f7d3f89a1de2a1b908fe23ff619`, passed exact-head CI #1230 / `31962536977`, and was guarded-squash-merged in PR #64 as `72d2405211a3fd6d11e0d17680b7769df90b5ffe`. Reviewed head and squash share exact tree `bdbf19f5c2197490d6b922fb792791b205f40370`; accepted-main push CI #1231 / `31967898198` passed at that exact squash. The separate post-M5D current-contract reconciliation remains the current execution gate until independently accepted and merged.

The maintained command inventory, CI relationship, audit details, and infrastructure-only waiver policy are documented in:

- [Validation details](docs/tooling/validation.md)
- [Work-tracking and evidence rules](docs/work-tracking.md)
- [M4 conformance matrix](docs/architecture/m4-conformance-matrix.md)
- [M5 conformance matrix](docs/architecture/m5-conformance-matrix.md)
- [`runenui_testing` crate README](crates/runenui_testing/README.md)

CI may invoke `cargo validate` through shared orchestration, but shared workflows do not own or recreate RunenUI validation semantics.
