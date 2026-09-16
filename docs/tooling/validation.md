# Validation

> **Category: Guide**

Format intentional Rust changes with the formatter enforced by CI:

```powershell
cargo +stable fmt --all
```

Run the complete repository baseline with:

```powershell
cargo validate
```

The Cargo alias executes `cargo run --locked --package xtask -- validate`. The locked outer invocation and every checked-in workspace Cargo check use `--locked`; validation must not update the repository's `Cargo.lock`, manifests, formatting, or source. The task is the single implementation used locally and by CI.

`xtask` derives the RunenUI workspace root from its compile-time `CARGO_MANIFEST_DIR`, verifies the root `Cargo.toml`, runs Cargo subprocesses from that root, and scans repository documentation from that root. Calling `cargo validate` within a workspace package therefore cannot reduce validation to that package subtree.

The baseline runs, in order:

```powershell
cargo +stable fmt --all --check
cargo +stable test --workspace --all-features --locked
cargo +stable clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.93.0 test --workspace --all-features --locked
cargo +stable test --locked --package runenui_testing --package runenui_external_widget_conformance --package runenui_external_renderer_conformance --package runenui_external_host_conformance
# isolated default-feature and feature-enabled private-seam compile probes
# repository-relative Markdown links from the resolved workspace root
# deterministic fatal repository structure and authority audit
```

The package-selected public-consumer lane deliberately omits `--workspace` and `--all-features`. It tests the four designated downstream crates with ordinary dependency features rather than inheriting the internal test seams enabled by the Counter example's dev-dependency in the all-features lane. A separate, disposable standalone Cargo workspace under ignored `target/` compiles a known runtime `__..._for_test` method successfully with its private feature explicitly enabled, then requires that same source to fail with the expected missing-method diagnostic under default features. The probe copies the lockfile into its temporary directory, resolves that copy offline, and runs both compiler checks offline and locked; it does not change tracked repository files. The positive control prevents unrelated compilation failures from masquerading as proof of isolation. Existing stable/MSRV all-feature lanes remain mandatory.

The fatal repository audit reuses the checked-in matrix, workspace, authority,
license, and canonical-runtime ownership contracts. It is network-free and
read-only. Its source-concentration findings are diagnostics and do not fail
validation. See [Repository audit](repository-audit.md).

The Markdown checker deliberately validates inline Markdown links to repository files. Targets resolve relative to the document containing the link. It does not fetch external URLs or validate same-document anchors, reference-style links, URL-encoded paths, or unusual Markdown constructs that are not covered by tests. It is not a complete Markdown specification parser.

Install both Rust channels through `rustup`. The pinned `rust-toolchain.toml` supplies Rust 1.93.0 for reproducible defaults; latest stable with rustfmt and Clippy is also required. See the [toolchain policy](../toolchain-policy.md).

## Exact-head CI contract

Pull-request CI explicitly checks out `github.event.pull_request.head.sha` and
verifies that `git rev-parse HEAD` equals that SHA before validation. GitHub's
default synthetic pull-request merge ref does **not** qualify as exact-head
evidence. A successful run becomes stale as soon as the feature head moves.
Final review still verifies the accepted base, mergeability, scope, and unresolved
findings. Record the reviewed feature head and accepted squash merge separately,
then inspect accepted-main push validation at the exact squash commit when required.

The shared CI workflow is read-only and requires no repository write permission.
Successful validation prints a compact evidence summary rather than the complete
command output. Failed validation preserves the canonical command's real exit
status, prints a bounded excerpt and tail, and uploads the complete failed-command
log from runner-temporary storage outside the checkout with short retention.
Temporary diagnostics are removed. Successful runs create no diagnostic artifact
and CI does not create, update, or remove pull-request comments. The Actions log
remains useful evidence; the failure-only artifact retains the complete failed
command log.

Do not add branch-mutating formatter, fixer, or self-commit workflows as a
substitute for ordinary reviewed repository edits. Automated contributors should
apply changes through the repository connector or normal Git commits and let the
shared CI baseline validate them. Ask the repository owner to run local commands
only when a required operation is genuinely unavailable through the connected
repository and CI surfaces.

## Focused commands

Inspect the full fatal and diagnostic repository report with:

```powershell
cargo xtask audit-repository
cargo xtask audit-repository --format json
```

Check only documentation links with the locked alias:

```powershell
cargo xtask check-links
```

To verify read-only behavior after committing a slice, run `git status --short`, `cargo validate`, then `git status --short` again. Both status outputs must be empty. Also run `git diff --check` and any slice-specific context, metadata, platform, benchmark, or release checks required by the roadmap.

Local validation is useful preflight but does not replace successful exact-head CI. Conversely, connector-driven work should not be transferred to the repository owner merely to reproduce checks that GitHub Actions already runs authoritatively.