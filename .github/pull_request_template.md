## Repository state

- Branch:
- Base SHA:
- Merge base:
- Previous reviewed head:
- Current head:
- Remote head:
- Owning issue:

## Scope

### Included


### Explicit non-goals


## Changes by responsibility

For each responsibility, describe:

- previous problem;
- implemented correction;
- authority owner;
- tests;
- matrix rows;
- deferred scope.

## Public API

### Added

- None.

### Changed

- None.

### Removed

- None.

## Structure and ownership

- Changed modules:
- Responsibility boundaries:
- Remaining god-file risks:
- Remaining architecture debt:
- New queue/store/runtime authority: none.

## Conformance matrix

```text
total rows:
owner-accepted:
proof-complete:
implementation-complete:
blocked:
duplicates:
invalid statuses:
invalid schemas:
```

## Validation

- [ ] `cargo metadata --no-deps --format-version 1`
- [ ] `cargo +stable fmt --all --check`
- [ ] `cargo test --workspace --all-features --locked`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo +1.93.0 test --workspace --all-features --locked`
- [ ] `cargo test --doc --workspace --all-features --locked`
- [ ] `cargo validate`
- [ ] `git diff --check`
- [ ] matrix uniqueness/status/schema/count audit
- [ ] public API audit
- [ ] removed-symbol audit
- [ ] unsafe-code audit
- [ ] cross-document truth audit
- [ ] exact base/head/remote verification
- [ ] clean-worktree verification

Do not reuse a result from an earlier head.

## Exact-head CI

- Workflow:
- Run ID:
- Job:
- Head SHA:
- Status:
- Conclusion:

## Review checklist

- [ ] Actual diff and every changed source/test file reviewed.
- [ ] ADR, charter, matrix, and roadmap compared against behavior.
- [ ] Positive, negative, and trace proofs are complete for the claimed scope.
- [ ] No duplicate authority, hidden compatibility layer, or premature later-slice API.
- [ ] Documentation reports accepted truth and keeps volatile state in issues/PRs.
- [ ] The next slice does not begin from this branch before merge.

## Final status

State the exact truthful milestone/slice status. Do not claim owner acceptance or merge before they occur.
