# RunenUI Testing and Validation

The canonical merge-readiness command is:

```text
cargo validate
```

It is repository-owned, deterministic, read-only, and used by the repository's thin GitHub Actions caller. The baseline covers stable formatting checks, locked workspace tests, Clippy with warnings denied, Rust 1.93.0 MSRV tests, repository metadata and authority invariants, and repository-relative Markdown links.

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

Shared CI orchestration may invoke `cargo validate`, but it does not recreate RunenUI validation semantics or mutate repository contents.
