# Validation

Run the repository baseline with one command:

```powershell
cargo validate
```

`cargo validate` is a Cargo alias for:

```powershell
cargo run --package xtask -- validate
```

The validate task runs these checks in order and stops on the first failure:

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Validation is read-only. It does not apply formatting changes.

## Formatting

Format the workspace with:

```powershell
cargo format
```

`cargo format` is a Cargo alias for:

```powershell
cargo fmt --all
```

Run formatting first when `cargo validate` fails at the fmt-check step, then rerun validation.

## Debugging

Use the explicit xtask form when debugging the task runner itself:

```powershell
cargo xtask validate
```
