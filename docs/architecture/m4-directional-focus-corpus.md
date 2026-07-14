# M4 Directional-Focus Conformance Corpus

> **Category: Target architecture**
>
> **Milestone:** M4
>
> **Status:** Proposed; normative only after ADR 0005 is accepted
>
> **Reviewed baseline:** `83e3771c34e021ac2960cab2cfd926c1128998ca`

## Purpose

This corpus freezes observable M4 focus-navigation outcomes without publishing
the runtime's private directional-scoring formula. Every implementation must
exercise every vector through the public event/command path and produce the
specified exact mounted generation or `None`.

Rectangles use RunenUI logical coordinates `[x, y, width, height]`, with positive
`x` rightward and positive `y` downward. Rectangle edges are inclusive for beam
and zero-gap eligibility. Candidate labels such as `A@7` include a private
generation only to make exact-lifetime expectations readable; public tests must
obtain these opaque values from runtime publication/inspection, never construct
them. Unless a vector says otherwise, candidates are live, visible, enabled,
focusable, and in the active scope. `R` is the root scope and `N` is a nested
scope. The listed mounted logical order is the complete relevant order.

`Restore` is the scope-entry restoration request rather than a geometric
direction. `Next` is included to freeze the root linear-wrap boundary alongside
directional scope policy. These are semantic focus commands and use the same
canonical queue as Left/Right/Up/Down.

## Normative vectors

| ID | Origin rectangle | Candidates | Mounted logical order | Active scope and policy | Direction/request | Expected | Rationale |
|---|---|---|---|---|---|---|---|
| DF-01 | `O [0,0,10,10]` | `A [20,0,10,10]` | `O, A` | `R`: default | Right | `A` | The only eligible candidate is directly right in the beam. |
| DF-02 | `O [20,0,10,10]` | `A [0,0,10,10]` | `A, O` | `R`: default | Left | `A` | The only eligible candidate is directly left in the beam. |
| DF-03 | `O [0,20,10,10]` | `A [0,0,10,10]` | `A, O` | `R`: default | Up | `A` | The only eligible candidate is directly above in the beam. |
| DF-04 | `O [0,0,10,10]` | `A [0,20,10,10]` | `O, A` | `R`: default | Down | `A` | The only eligible candidate is directly below in the beam. |
| DF-05 | `O [0,0,10,10]` | `A [30,0,10,10]`; `B [12,20,10,10]` | `O, B, A` | `R`: default | Right | `A` | An in-beam candidate wins over a nearer off-beam candidate. |
| DF-06 | `O [0,0,20,20]` | `A [20,15,10,10]`; `B [20,25,10,10]` | `O, B, A` | `R`: default | Right | `A` | Partial orthogonal projection overlap is in-beam and beats off-beam `B`. |
| DF-07 | `O [0,0,10,40]` | `A [20,10,30,5]`; `B [15,50,10,10]` | `O, B, A` | `R`: default | Right | `A` | Unequal sizes do not discard the in-beam candidate. |
| DF-08 | `O [0,0,10,10]` | `A [5,0,10,10]`; `B [20,0,10,10]` | `O, B, A` | `R`: default | Right | `A` | A candidate whose bounds overlap the origin but extend in the requested half-plane remains eligible and wins. |
| DF-09 | `O [0,10,10,10]` | `A [20,0,10,10]`; `B [20,20,10,10]` | `O, B, A` | `R`: default | Right | `B` | Geometrically tied candidates use mounted logical order as the final tie-break. |
| DF-10 | `O [0,0,10,10]` in `N` | `N1 [-20,0,10,10]` in `N`; `P [20,0,10,10]` in `R` | `N1, O, P` | `N`: default delegate | Right | `P` | With no eligible nested candidate, the default nested scope delegates to its parent. |
| DF-11 | `O [0,0,10,10]` in `N` | `N1 [-20,0,10,10]` in `N`; `P [20,0,10,10]` in `R` | `N1, O, P` | `N`: trap | Right | `None` | A trapped nested scope does not escape to the eligible parent candidate. |
| DF-12 | `O [20,0,10,10]` in `N` | `A [0,0,10,10]` in `N`; `P [40,0,10,10]` in `R` | `A, O, P` | `N`: directional wrap | Right | `A` | Explicit nested wrapping selects the opposite boundary in the same scope. |
| DF-13 | `B [40,0,10,10]` | `O [0,0,10,10]`; `A [20,0,10,10]` | `O, A, B` | `R`: default linear wrap | Next | `O` | Root linear traversal wraps from the last eligible node to the first. |
| DF-14 | `O [20,0,10,10]` | `A [0,0,10,10]` | `A, O` | `R`: default directional stop | Right | `None` | Root directional traversal stops when no candidate exists in the requested half-plane. |
| DF-15 | scope entry `O [0,0,10,10]` | remembered `A@7 [20,0,10,10]`; `B@3 [40,0,10,10]` | `O, A@7, B@3` | `N`: remember last descendant `A@7` | Restore | `A@7` | Restoration returns only the exact remembered live and eligible generation. |
| DF-16 | scope entry `O [0,0,10,10]` | stale remembered `A@7`; live replacement `A@8 [40,0,10,10]`; `B@3 [20,0,10,10]` | `O, B@3, A@8` | `N`: remember last descendant `A@7` | Restore | `B@3` | A stale generation is never retargeted to `A@8`; normal traversal selects the first eligible live fallback. |
| DF-17 | `O [0,0,10,10]` | disabled `A [12,0,10,10]`; `B [30,0,10,10]` | `O, A, B` | `R`: default | Right | `B` | Disabled candidates are excluded even when nearer and in-beam. |
| DF-18 | `O [0,0,10,10]` | hidden `A [12,0,10,10]`; `B [30,0,10,10]` | `O, A, B` | `R`: default | Right | `B` | Hidden candidates are excluded even when nearer and in-beam. |
| DF-19 | `O [20,20,10,10]` | `A [0,20,10,10]`; `B [20,0,10,10]`; `C [0,0,10,10]` | `A, B, C, O` | `R`: default | Right | `None` | Candidates wholly outside the requested half-plane are ineligible. |
| DF-20 | `O [0,0,10,10]` | `A [10,0,10,10]`; `B [20,0,10,10]` | `O, B, A` | `R`: default | Right | `A` | Edge-touching creates a valid zero primary-axis gap and beats the farther candidate. |

## Conformance rule

The M4 suite must name every vector ID individually. A single test may table-drive
the corpus, but its failure output must identify the vector, expected target, and
observed target. Tests must additionally prove that the result came through the
public semantic-command queue, used current published rectangles and exact
mounted generations, and produced the normal focus transition/trace facts.

The vector expectations are architecture authority. The score, weights, helper
types, and spatial data structure remain private implementation details and must
not be copied into public protocol documentation.
