# Unsafe Code Policy

> **Category: Current contract**

Active framework crates and `xtask` currently use `#![forbid(unsafe_code)]`. M0 preserves that baseline.

Core authored data, mounted runtime logic, layout/style/semantics protocols, deterministic testing, and ordinary controls should remain safe Rust. Unsafe code must not be introduced merely for speculative performance.

Future platform or renderer crates may require unsafe FFI or graphics boundaries. Each such boundary requires an ADR and review covering why safe alternatives are insufficient, the smallest owning module/crate, invariants, lifetime/threading rules, error and teardown behavior, tests/Miri/sanitizer applicability, dependency interaction, and audit responsibility. Prefer audited upstream wrappers where they preserve required ownership.

Do not weaken an existing crate-level `forbid` as a side effect of unrelated work. Any approved unsafe code belongs in a narrowly scoped crate/module with documented safety invariants and dedicated validation.
