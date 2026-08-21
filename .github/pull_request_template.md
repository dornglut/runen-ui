## Authority

- Owning issue or accepted decision:
- Accepted implementation base, when relevant:
- Reviewed exact head:

## Outcome

Describe the completed outcome and why it belongs in RunenUI.

## Scope

### Included

-

### Excluded

-

## Conformance

List permanent conformance IDs affected by this change, or `N/A`. Describe any accepted-state changes without copying live issue/PR state into durable conformance documents.

## Validation

```text
command:
revision:
result:
```

Add slice-specific behavioral/proof evidence required by the owning issue. Do not restate the internal subcommands already owned by `cargo validate`.

## Impact

- Public API or compatibility:
- Architecture/ownership:
- Migration or release:
- Security/permissions:
- Documentation:

## Review checklist

- [ ] The diff has one coherent purpose and matches the owning authority.
- [ ] Code/tests remain the source of current behavior; durable docs were changed only where their owned concern changed.
- [ ] Required conformance positive/negative/diagnostic or trace proofs are satisfied.
- [ ] No duplicate authority, hidden compatibility path, or premature later-milestone API was introduced.
- [ ] `cargo validate` and required slice-specific checks passed on the reviewed head.
- [ ] Exact-head CI refers to the reviewed feature head; moved heads invalidate earlier evidence.
- [ ] Remaining work is owned elsewhere and is not hidden in this pull request.
