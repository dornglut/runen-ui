# Toolchain Support Policy

> **Category: Current contract**

RunenUI's current 0.x compiler support policy is **latest stable Rust only**.

- **Supported:** the latest stable Rust channel used by repository validation and CI.
- **MSRV:** none is currently declared. Workspace packages intentionally omit `rust-version`.
- **Contributor default:** `rust-toolchain.toml` selects the `stable` channel with `rustfmt` and `clippy`.

`cargo validate` invokes stable Rust explicitly for formatting, locked workspace tests, Clippy, public-consumer validation, and the repository's remaining checks. Contributors need the stable toolchain with `rustfmt` and `clippy`; CI installs the same supported channel and calls the same repository-owned validation entry point.

Older Rust versions may continue to work accidentally, but they are **unsupported and unvalidated**. No compatibility promise may be inferred from a historical milestone, a dependency's own MSRV, or a previously declared RunenUI MSRV.

Dependency reviews may still record upstream MSRVs as maintenance and upgrade evidence. A dependency must remain usable on RunenUI's supported stable toolchain, and a dependency requiring nightly, beta-only behavior, or otherwise unsupported compiler behavior is not silently accepted.

Before 1.0, the release policy requires an explicit compiler-support policy. A formal MSRV or support window may be introduced later only through an intentional policy change that updates package metadata, toolchain configuration, validation, CI expectations, documentation, and release notes together.
