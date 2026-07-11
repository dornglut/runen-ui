# Validation

> **Category: Guide**

Run the complete repository baseline with one command:

```powershell
cargo validate
```

This Cargo alias executes `cargo run --package xtask -- validate`. The task is the single implementation used locally and by CI. It runs, in order:

```powershell
cargo +stable fmt --all --check
cargo +stable test --workspace --locked
cargo +stable clippy --workspace --all-targets --locked -- -D warnings
cargo +1.93.0 test --workspace --locked
# repository-local relative Markdown link check
```

Validation stops on the first failure and never rewrites source. CI installs both required toolchains and calls `cargo validate`; it does not duplicate or mutate the implementation.

Install the channels locally through `rustup`. The pinned `rust-toolchain.toml` supplies Rust 1.93.0 with rustfmt/Clippy for reproducible default commands; `cargo validate` also requires latest stable with those components. See the [toolchain policy](../toolchain-policy.md).

Format intentionally before validation with:

```powershell
cargo format
```

Check only documentation links when editing docs:

```powershell
cargo xtask check-links
```

Also run `git diff --check` and any slice-specific conformance, documentation, context-export, metadata, platform, benchmark, or release checks required by the roadmap. The M11 production matrix will expand beyond the current Ubuntu proof baseline; it must continue to reuse this entry point or a reviewed successor rather than duplicate commands.
