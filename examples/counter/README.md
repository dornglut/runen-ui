# Counter Headless Proof

> **Category: Guide**

Counter is the repository's ordinary application-level proof consumer. It owns typed product state/actions/update logic and transient view authoring only; mounted identity, semantic identity, routing, scheduling, publication, tracing, and testing authority remain framework-owned.

## What it proves

The current Counter exercises accepted public behavior across the mounted, routed, semantic, testing, and replay stack:

- stable authored keys and compatible mounted lifetime retention;
- independent mounted and semantic lifetimes, including stale identities after removal or replacement;
- pointer, keyboard, authored-ID automation, programmatic command, and exact semantic activation converging on the canonical command/default/application-action path;
- public deterministic test-harness mounting, bounded pumping/settling, fixed-surface publication, semantic snapshot/query/target use, logical-time control, and read-only state/publication/trace inspection;
- semantic publication revision/update observation after application state changes without recovering a mounted owner from a semantic target;
- deterministic canonical trace export and inert replay correlation for accepted interaction/update/publication behavior.

The integrated cross-origin proof compares semantic action, pointer, keyboard, authored-ID automation, and programmatic command activation and requires one application increment through the same accepted action/update architecture for each origin. Semantic targets remain exact surface plus semantic-node identities from committed public snapshots.

## Boundaries

Counter is a deterministic headless proof, not a desktop application or native-host example. It does not provide native keyboard/pointer translation, editable text, native accessibility, production paint/hit scenes, a renderer backend, production controls, or multi-surface lifecycle. It contains no private runtime test seam, fabricated IDs/sequences, semantic-to-mounted routing shortcut, compatibility activation path, or parallel expected runtime model.

Run it with:

```text
cargo run --package counter
```

Repository-level conformance runs through `cargo validate` and focused tests owned by the active change. See [current status](../../docs/status.md), the [M5 conformance matrix](../../docs/conformance/m5-conformance-matrix.md), [testing](../../TESTING.md), and the [roadmap](../../docs/roadmap.md).
