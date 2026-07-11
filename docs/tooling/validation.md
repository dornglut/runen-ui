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

The Cargo alias executes `cargo run --locked --package xtask -- validate`. The locked outer invocation and every nested Cargo check use `--locked`; validation must not update `Cargo.lock`, manifests, formatting, or source. The task is the single implementation used locally and by CI.

`xtask` derives the RunenUI workspace root from its compile-time `CARGO_MANIFEST_DIR`, verifies the root `Cargo.toml`, runs Cargo subprocesses from that root, and scans repository documentation from that root. Calling `cargo validate` within a workspace package therefore cannot reduce validation to that package subtree.

The baseline runs, in order:

```powershell
cargo +stable fmt --all --check
cargo +stable test --workspace --locked
cargo +stable clippy --workspace --all-targets --locked -- -D warnings
cargo +1.93.0 test --workspace --locked
# repository-relative Markdown links from the resolved workspace root
# MIT ownership, workspace license expression, and publish=false metadata
```

The Markdown checker deliberately validates inline Markdown links to repository files. Targets resolve relative to the document containing the link. It does not fetch external URLs or validate same-document anchors, reference-style links, URL-encoded paths, or unusual Markdown constructs that are not covered by tests. It is not a complete Markdown specification parser.

Install both Rust channels through `rustup`. The pinned `rust-toolchain.toml` supplies Rust 1.93.0 for reproducible defaults; latest stable with rustfmt and Clippy is also required. See the [toolchain policy](../toolchain-policy.md).

Check only documentation links with the locked alias:

```powershell
cargo xtask check-links
```

To verify read-only behavior after committing a slice, run `git status --short`, `cargo validate`, then `git status --short` again. Both status outputs must be empty. Also run `git diff --check` and any slice-specific context, metadata, platform, benchmark, or release checks required by the roadmap.
