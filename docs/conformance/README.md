# Conformance

Conformance documents define permanent observable requirements and proof obligations. They are not a second implementation tracker.

Configured matrices:

- [M4 conformance matrix](m4-conformance-matrix.md)
- [M5 conformance matrix](m5-conformance-matrix.md)
- [M6 conformance matrix](m6-conformance-matrix.md)
- [M7 conformance matrix](m7-conformance-matrix.md)
- [M8 conformance matrix](m8-conformance-matrix.md)

Supporting accepted contract material:

- [M4C delivery and routed-transaction charter](m4c-delivery-and-routed-transaction-charter.md)
- [M4 directional-focus corpus](m4-directional-focus-corpus.md)
- [M5 semantics and deterministic testing charter](m5-semantics-and-testing-charter.md)

A matrix row's status describes **accepted default-branch conformance state**. In-flight implementation, proof execution, review, blockers, feature heads, and exact-head CI evidence belong in the owning GitHub issue and pull request until acceptance changes repository state.

Permanent IDs are never recycled merely because implementation moves. Accepted provenance may be retained when it is necessary to explain a frozen contract or proof boundary, but conformance documents must not become mutable current-head/branch/pickup ledgers.

Target conformance does not imply current API availability. Code/tests show what is implemented; accepted ADR/design/conformance contracts define what that implementation must satisfy. A mismatch is a defect or requires explicit contract revision.
