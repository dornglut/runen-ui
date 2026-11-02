# RunenUI Testing and Validation

The canonical merge-readiness command is:

```text
cargo validate
```

It is repository-owned, deterministic, read-only, and remains the complete local merge-readiness baseline. The same validation plan also exposes bounded hosted-execution partitions to the thin GitHub Actions caller; those partitions are execution projections of `cargo validate`, not separate validation authorities. The baseline covers stable formatting checks, locked stable workspace tests, Clippy with warnings denied, repository metadata and authority invariants, public-consumer isolation proofs, and repository-relative Markdown links. The current 0.x support policy is stable-only; no separate MSRV lane or older-compiler guarantee is part of merge readiness.

The checked-in `validation-partitions.txt` file is only the shared workflow's scheduling projection. `xtask` owns the authoritative ordered partition registry and validates the complete manifest before either complete validation or any `--partition` body executes. Hosted CI currently projects the plan as `workspace-tests`, `public-contract`, and `repository-quality`; changing that manifest without changing the repository-owned plan fails closed.

For intentional Rust edits, format first with:

```text
cargo +stable fmt --all
```

During implementation, also run the focused tests and conformance proofs owned by the active issue. Before handoff, run:

```text
cargo validate
git diff --check
```

Exact-head CI must validate the frozen reviewed feature head. A successful run from an earlier head is stale after the head moves. Source inspection establishes structure; it is not executed test or runtime evidence.

## Public deterministic application testing

`runenui_testing` is a downstream public crate for deterministic headless tests. It composes ordinary `runenui_core` and `runenui_runtime` contracts and owns no live runtime state or private mutation seam.

`TestHarness<App>` supports deterministic surface publication, synthetic public interaction, bounded pumping and settling, explicit logical time, semantic queries and exact semantic targets, and read-only observation of accepted runtime products. It must not fabricate runtime identities or sequences, mutate mounted state directly, invoke private callbacks, guess surface scope from a bare semantic ID, or maintain a parallel expected runtime model.

The current testing surface reflects implemented framework behavior only. Accepted target architecture does not create test assertions or runtime products before implementation.

## Opt-in producer-ledger performance qualification

The U475 admission and keyed-generation retirement diagnostics are intentionally ignored
by the normal test baseline. They measure cost; they do not set an acceptance limit or
create a second CI/validation authority. Run them on an exact checked-out revision,
with a fixed Rust toolchain, runner/hardware identity, and feature set:

```text
cargo +stable test --release --locked --all-features -p runenui_runtime --test m10_semantic_integrity bench_u475_accumulated_noop_root_admission_scaling -- --ignored --nocapture
cargo +stable test --release --locked --all-features -p runenui_runtime --lib bench_u475_keyed_generation_release_scaling -- --ignored --nocapture
cargo +stable test --release --locked --all-features -p runenui_runtime --lib bench_u475_cross_root_cancellation_preflight_scaling -- --ignored --nocapture
```

Each diagnostic uses 30 independent repetitions at root sizes 32, 256, 1024,
and 2048. Admission covers trace disabled/enabled (0/512), while retirement
compares sequential and batched keyed lease release. Cross-root cancellation
preflight also covers mixed host, framework-service, local task, send task,
and repeating timer generations at the same root sizes. Each `U475_METRIC` line reports
the phase, trace mode, roots, repetitions, p50/p95/maximum total batch time in
microseconds, and the largest observed process VmRSS in KiB when available.
VmRSS is a process-wide snapshot, not a per-root allocation or an isolated
peak-memory measurement.

Preserve the exact commit SHA, CPU/platform, Rust compiler version, feature
selection, raw observations, and any elapsed-time distributions in the owning
issue/PR. No workload-independent numeric latency, throughput, or memory limit
has been adopted by this testing procedure; interpret results with the current
conformance and accepted owner decision. These diagnostics do not qualify
physical native input-to-present evidence.

## Evidence ownership

- executable tests are evidence for observed implementation behavior;
- accepted ADR, architecture/design, and conformance contracts define the behavior and proofs the implementation must satisfy;
- `cargo validate` owns the repository baseline;
- pull requests and exact-head CI own delivery evidence;
- a code/contract mismatch is a defect, not an implicit contract change.

Detailed procedures are in [validation](docs/tooling/validation.md) and the [repository audit](docs/tooling/repository-audit.md). Permanent behavior/proof contracts are indexed under [conformance](docs/conformance/README.md).

Shared CI orchestration may invoke complete `cargo validate` or one repository-owned `cargo validate --partition <id>` projection, but it does not recreate RunenUI validation semantics or mutate repository contents. The aggregate hosted result succeeds only when the complete scheduled partition set succeeds.
