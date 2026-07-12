# External Widget Conformance Fixture

> **Category: Test fixture**

This non-publishable package is compiled as a genuine downstream consumer. It
depends only on the public `runenui_core` and `runenui_runtime` packages and
defines a stateful interactive `PulseButton` without framework registration or
private imports. It also defines `CustomColumn`, `CustomRow`, fixed/text/
unsupported intrinsic panels, and a counter-backed layout panel through
`ChildLayoutWidget` and the canonical `Container<Action>` builder.
Its stateless implementations explicitly declare `State = ()` and an empty
`create_state`; those requirements are convenient but not automatic defaults.

Its tests prove concrete and generic widget type identity, checked state and
lifecycle hooks, recursive local-to-parent action mapping, non-`Clone`
activation, built-in composition, measurement/arrangement, deterministic
paint/semantic/diagnostic facts, focusability, and public runtime inspection.
The external container proof covers heterogeneous and arbitrary child counts,
traversal and parent relationships, descendant identity diagnostics, recursive
action mapping and activation, custom layout participation, and descendant
surface publication.
Measurement conformance uses an external counter-backed container and text
descriptor to prove one capability query per node/publication, reuse of the
same axis during arrangement, a fresh single query on the next publication,
and deterministic unsupported-capability diagnostics. Alignment tests compare
index, frame, style, and layout preorder IDs/parents across built-in, external,
nested, intrinsic-minimum, and unsupported cases, then prove descendant hit
testing and runtime activation. External and nested nonzero gaps are geometric
proofs, not documentation-only claims.

The lifecycle execution and paint/semantic values are M2 conformance proofs.
They do not claim persistent mounted state, a production semantic/accessibility
tree, or a renderer-neutral paint scene. Only lifecycle can access typed state
in M2; state-dependent mounted capabilities remain a breaking M3 contract.
