# Toolchain and MSRV Policy

> **Category: Current contract**

RunenUI validates two Rust channels:

- **MSRV:** Rust 1.93.0, declared by `rust-version` and pinned in `rust-toolchain.toml` for reproducible contributor commands.
- **Stable:** the latest stable Rust channel installed through `rustup`, used for formatting, normal workspace tests, and Clippy.

`cargo validate` invokes both channels explicitly. Contributors need the stable toolchain with `rustfmt` and `clippy` plus the minimal 1.93.0 toolchain. CI installs the same channels and calls the same validation entry point.

The MSRV may increase during 0.x only in an intentional pull request that explains the need, updates `rust-version`, `rust-toolchain.toml`, CI, this policy, release notes, and validation, and proves all workspace packages on the new version. A dependency that requires a newer compiler is not silently accepted.

Before 1.0, the release policy will define an MSRV support window. Until then, the repository guarantees only that the current revision passes the declared MSRV and latest stable checks.
