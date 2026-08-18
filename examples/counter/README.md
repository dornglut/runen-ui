# Counter Headless Proof

> **Category: Guide**

Counter is the repository's ordinary application-level proof consumer. It owns only typed product state/actions/update logic and transient view authoring; mounted identity, semantic identity, routing, scheduling, publication, tracing, and testing authority remain framework-owned.

## What it proves

The current Counter exercises accepted public behavior across the mounted, routed, semantic, testing, and replay stack:

- stable authored keys and compatible mounted lifetime retention;
- independent mounted and semantic lifetimes, with stale identities after removal/replacement;
- routed pointer release-inside activation, raw Enter and matched Space activation, authored-ID automation, programmatic command activation, and exact semantic activation converging on the canonical command/default/application-action path;
- public `runenui_testing::TestHarness` mounting, bounded pumping/settling, deterministic fixed-surface publication, semantic snapshot/query/target use, logical-time control, and read-only state/publication/trace inspection;
- semantic publication revision/update observation after Counter state changes without recovering a mounted owner from a semantic target;
- deterministic canonical trace export and inert M4D3 replay correlation for accepted interaction/update/publication behavior.

The M5E cross-origin closure proof specifically compares five activation origins—semantic action, pointer, keyboard, authored-ID automation, and programmatic command—and requires exactly one Counter increment through the same accepted action/update architecture for each origin. Semantic targets remain exact `SurfaceId + SemanticNodeId` values from committed public snapshots.

## Boundaries

Counter is a deterministic headless proof, not a desktop application or native host example. It does not provide native keyboard/pointer translation, editable text, native accessibility, production paint/hit scenes, a renderer backend, production controls, or multi-surface lifecycle. It contains no private runtime test seam, fabricated IDs/sequences, direct semantic-to-mounted routing shortcut, compatibility activation path, or parallel expected runtime model.

Run it with:

```powershell
cargo run --package counter
```

Repository-level conformance runs through `cargo validate` and the active slice-specific tests. See the [feature/support matrix](../../docs/feature-support-matrix.md), [M5 conformance matrix](../../docs/architecture/m5-conformance-matrix.md), [testing guide](../../TESTING.md), and [roadmap](../../docs/roadmap.md) for exact limits and acceptance state.
