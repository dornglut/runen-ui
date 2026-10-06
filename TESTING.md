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

## Evidence ownership

- executable tests are evidence for observed implementation behavior;
- accepted ADR, architecture/design, and conformance contracts define the behavior and proofs the implementation must satisfy;
- `cargo validate` owns the repository baseline;
- pull requests and exact-head CI own delivery evidence;
- a code/contract mismatch is a defect, not an implicit contract change.

Detailed procedures are in [validation](docs/tooling/validation.md) and the [repository audit](docs/tooling/repository-audit.md). Permanent behavior/proof contracts are indexed under [conformance](docs/conformance/README.md).

Shared CI orchestration may invoke complete `cargo validate` or one repository-owned `cargo validate --partition <id>` projection, but it does not recreate RunenUI validation semantics or mutate repository contents. The aggregate hosted result succeeds only when the complete scheduled partition set succeeds.
