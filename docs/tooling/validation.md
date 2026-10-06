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

The Cargo alias executes `cargo run --locked --package xtask -- validate`. The locked outer invocation and every checked-in workspace Cargo check use `--locked`; validation must not update the repository's `Cargo.lock`, manifests, formatting, or source. `xtask` is the single semantic implementation: local merge readiness uses the complete command, while hosted CI may execute repository-owned `--partition <id>` projections of that same plan in parallel.

`xtask` derives the RunenUI workspace root from its compile-time `CARGO_MANIFEST_DIR`, verifies the root `Cargo.toml`, runs Cargo subprocesses from that root, and scans repository documentation from that root. Calling `cargo validate` within a workspace package therefore cannot reduce validation to that package subtree.

The baseline runs, in order:

```powershell
cargo +stable metadata --locked --no-deps
cargo +stable fmt --all --check
cargo +stable test --workspace --all-features --locked
cargo +stable clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +stable test --locked --package runenui_testing --package runenui_external_widget_conformance --package runenui_external_renderer_conformance --package runenui_external_host_conformance
cargo +stable fetch --locked
# locked, offline Cargo feature-graph inspection for the four public-consumer packages
# isolated default-feature and feature-enabled private-seam compile probes
# repository-relative Markdown links from the resolved workspace root
# deterministic fatal repository structure and authority audit
```

For hosted execution, the repository-owned plan exposes exactly three partitions:

- `workspace-tests`: the unchanged stable workspace all-feature test command;
- `public-contract`: the complete existing public-consumer isolation proof;
- `repository-quality`: metadata, formatting, Clippy, licensing/publish policy, documentation links, and fatal repository audit.

The authoritative ordered registry lives in `xtask`. Root `validation-partitions.txt` is only a scheduling projection for the shared workflow. Both complete `cargo validate` and every `cargo validate --partition <id>` invocation exact-check that file against the registry before any partition-specific work. Missing, added, renamed, reordered, or unknown partitions therefore fail closed rather than silently reducing validation.

The package-selected public-consumer lane deliberately omits `--workspace` and `--all-features`. It derives `runenui_testing` plus every workspace member under `tests/` from the canonical workspace inventory and tests them with ordinary dependency features rather than inheriting internal test seams enabled elsewhere in the all-features lane. Before the intentionally offline Cargo proofs, validation runs locked `cargo fetch` without a target override so the local cache contains the complete locked dependency graph, including target-specific packages that a host-only test build need not download. The same inventory derives every declared `internal-*` feature from workspace member manifests; a matching locked, offline `cargo tree --edges features` inspection rejects any of them in the public consumers' resolved feature graph. Adding a conformance fixture or private feature therefore expands the proof automatically rather than relying on a second hand-maintained list. A separate, disposable standalone Cargo workspace under ignored `target/` compiles a known runtime `__..._for_test` method successfully with `internal-test-seams` explicitly enabled, then requires that same method to be unavailable under default features. The probe copies the lockfile into its temporary directory, resolves that copy offline, and runs both compiler checks offline and locked; it does not change tracked repository files. The positive control prevents unrelated compilation failures from masquerading as proof of isolation. The stable all-feature lane remains mandatory.

The fatal repository audit reuses the checked-in matrix, workspace, authority,
license, and canonical-runtime ownership contracts. It is network-free and
read-only. Its source-concentration findings are diagnostics and do not fail
validation. See [Repository audit](repository-audit.md).

The Markdown checker deliberately validates inline Markdown links to repository files. Targets resolve relative to the document containing the link. It does not fetch external URLs or validate same-document anchors, reference-style links, URL-encoded paths, or unusual Markdown constructs that are not covered by tests. It is not a complete Markdown specification parser.

Install stable Rust through `rustup`. The checked-in `rust-toolchain.toml` selects the stable channel with rustfmt and Clippy for reproducible contributor defaults. RunenUI currently declares no MSRV and does not require a second compiler channel. See the [toolchain policy](../toolchain-policy.md).

## Exact-head CI contract

Pull-request CI explicitly checks out `github.event.pull_request.head.sha` and
verifies that `git rev-parse HEAD` equals that SHA before validation. GitHub's
default synthetic pull-request merge ref does **not** qualify as exact-head
evidence. A successful run becomes stale as soon as the feature head moves.
Final review still verifies the accepted base, mergeability, scope, and unresolved
findings. Record the reviewed feature head and accepted squash merge separately,
then inspect accepted-main push validation at the exact squash commit when required.

The shared CI workflow is read-only and requires no repository write permission.
Shared CI owns checkout, toolchain, cache, bounded partition scheduling, aggregation,
and bounded-diagnostics orchestration; RunenUI owns validation semantics. The planner
selects the exact caller revision and each partition runner independently checks out
and proves that same revision. The checked-out exact source and complete `cargo validate`
remain authoritative, and a restored caller workspace `target/` tree must never
substitute for them. Successful validation prints a compact evidence summary rather
than the complete command output. Failed validation preserves the canonical
command's real exit status, prints a bounded excerpt and tail, and uploads the
complete failed-command log from runner-temporary storage outside the checkout
with short retention. Temporary diagnostics are removed. Successful runs create
no diagnostic artifact and CI does not create, update, or remove pull-request
comments. The Actions log remains useful evidence; the failure-only artifact
retains the complete failed command log.

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
