# `xtask`

> **Category: Guide**

`xtask` implements the repository validation baseline used by contributors and CI. It is private workspace tooling and is never published.

Run the complete baseline from the repository root:

```powershell
cargo +stable fmt --all
cargo validate
```

The first command intentionally formats with the same stable rustfmt that validation and CI enforce. `cargo validate` launches `xtask` with `--locked`, resolves the workspace root from `CARGO_MANIFEST_DIR`, and remains repository-wide when invoked from a nested workspace directory.

See [Validation](../docs/tooling/validation.md) and the [toolchain policy](../docs/toolchain-policy.md).
