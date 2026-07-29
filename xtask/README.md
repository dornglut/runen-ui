# `xtask`

> **Category: Guide**

`xtask` implements the repository validation baseline used by contributors and CI. It is private workspace tooling and is never published.

Run the complete baseline from the repository root:

```powershell
cargo +stable fmt --all
cargo validate
```

The first command intentionally formats with the same stable rustfmt that validation and CI enforce. `cargo validate` launches `xtask` with `--locked`, resolves the workspace root from `CARGO_MANIFEST_DIR`, and remains repository-wide when invoked from a nested workspace directory.

CI invokes the same `cargo validate` implementation after explicitly checking out and verifying the pull-request head SHA. The workflow does not use a synthetic merge ref as exact-head evidence, does not mutate repository contents, and does not create or maintain pull-request comments. Successful output is compact; failures retain a complete short-retention diagnostic artifact outside the checkout.

Inspect repository authority and architecture concentration separately:

```powershell
cargo xtask audit-repository
cargo xtask audit-repository --format json
```

The fatal subset is part of `cargo validate`; diagnostics remain informational.
See [Validation](../docs/tooling/validation.md), the
[repository-audit contract](../docs/tooling/repository-audit.md), and the
[toolchain policy](../docs/toolchain-policy.md).
