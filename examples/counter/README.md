# Counter Headless Proof

> **Category: Guide**

Counter owns only typed product state, actions, update logic, and transient view
authoring. Stable keys identify the Counter and Win roots, the control row, and
each interactive control; no reconciliation or lifecycle plumbing enters the
example.

The mounted-runtime tests prove that Increment, Decrement, and Reset receive
independent mounted and semantic IDs; Increment focus, IDs, and widget-local
activation state survive compatible rebuilds; the transition to Win unmounts
the Counter controls and makes their IDs stale; removed focus clears; and
returning to Counter creates new mounted and semantic lifetimes. Mounted index,
frame, style, and layout publication remain aligned after transitions; root
replacement rebuilds every node-aligned cached product from the new lifetime. A
test-only generation seam proves that terminal exhaustion before action update
preserves Counter state, mounted/semantic identity, focus, report, and cached
publication while cancelling the failed and remaining queued actions. Multiple
activations before one pump also prove repeatable `on_activate` factories and
FIFO action processing.

Run it with:

```powershell
cargo run --package counter
```

This is a deterministic headless proof, not a desktop application, production
control/accessibility/text example, paint scene, native host, or renderer. Input
is still proof-level press activation routed only into the current action queue;
later M4 work owns routed events, capture, release-inside behavior, effects,
tasks, timers, subscriptions, and complete scheduling.

See the [feature/support matrix](../../docs/feature-support-matrix.md) and
[roadmap](../../docs/roadmap.md) for exact limits.
