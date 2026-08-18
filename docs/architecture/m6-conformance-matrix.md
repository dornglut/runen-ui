# M6 Conformance Matrix

> **Category: Target architecture**
>
> **Status:** Accepted
>
> **Milestone:** M6
>
> **Reviewed baseline:** `8e09a61832e2077db0e1366472b628c9b2478880`
>
> **Acceptance condition:** This matrix becomes normative only when the exact
> M6A0 authority package containing it is explicitly accepted by the repository
> owner and merged. Until then accepted `main` contains no M6 implementation
> authority.

This matrix is the single M6-specific observable behavior and proof inventory.
[ADR 0007](../adr/0007-renderer-neutral-paint-hit-scene-protocol.md) owns the
renderer-neutral scene/publication architecture. The accepted M4 matrix continues
to own exact displayed-generation surface input, including `SURFACE-*`; the
accepted M5 matrix continues to own semantic independence and staged publication
atomicity. Those observations are referenced here where inherited but are not
duplicated with new IDs.

M6A0 owns this architecture/conformance authority only and implements no scene
behavior. Every M6 behavior row therefore begins `blocked`. Acceptance of M6A0
freezes the contract and implementation sequence; it does not promote any row.

```text
35 total unique rows
0 owner-accepted
0 implementation-complete
0 proof-complete
35 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

## Row contract and completion rule

Every ID is permanent. New observations append the next zero-padded number in
that family; IDs are never recycled because implementation moves. Allowed
statuses retain the repository meanings:

- `blocked`: the owning implementation slice has not been accepted;
- `implementation-complete`: public behavior exists but the complete proof
  package has not passed;
- `proof-complete`: exact-head positive/negative/diagnostic or trace proof and
  validation pass, but owner acceptance and merge remain pending;
- `owner-accepted`: public behavior, complete proof, validation, critical review,
  explicit owner acceptance, guarded merge, content identity, and required
  accepted-main validation have passed.

`Required` means the row must be `owner-accepted` before M6 closes. M6A0 has no
behavior rows. Delivery slices are fixed by ADR 0007:

- M6A — persistent retained-publication substrate (#59);
- M6B — canonical paint/hit scene kernel and displayed-hit cutover;
- M6C — transforms, clips, resources, metadata, damage, and capabilities;
- M6D — independent consumers, migration, and M6 closure.

Proof through a test-only parallel scene, forged target/generation, private
callback bridge, compatibility alias, or backend-specific alternate scene is not
valid conformance.

## M6A — persistent retained-publication substrate

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SCENE-PUB-01 | Non-structural planning starts from cheap immutable/shared retained phase products and does not deep-clone the complete surface cache before deciding dirty replacements. | Narrow-publication retained-product reuse tests and allocation/copy characterization | Whole-`SurfaceCache` clone regression proof across clean, focus-only, semantic-only, style-only, layout, and paint cases | Publication phase diagnostics/benchmark evidence | M6A | blocked | Required |
| SCENE-PUB-02 | Unchanged topology/style/layout/hit/paint/diagnostic products are reused while each dirty phase owns an explicit replacement; deterministic output order is independent of the chosen storage handles. | Per-phase handle/reuse and dirty-replacement tests | Cross-phase accidental replacement, storage-order iteration, and alias-to-mutable-state tests | Surface phase report plus retained-product identity diagnostics | M6A | blocked | Required |
| SCENE-PUB-03 | The retained-storage migration preserves accepted M5 `SEM-PUB-04`: rejected/backpressured/counter-exhausted/integrity-failed plans commit zero new cache, semantic, displayed-hit, redraw, rehit, or success-trace state. | Existing M5 atomicity corpus rerun against new substrate plus M6 narrow-plan cases | Partial-swap, rollback-copy, lost-reservation, and terminal-wrap regression proof | Inherited M5 publication trace/counter evidence | M6A | blocked | Required |
| SCENE-PUB-04 | Semantic publication remains an independently typed sibling coordinated by the same final surface commit; persistent renderer phase storage never becomes semantic identity/tree authority. | Public semantic-versus-renderer product independence proof after storage migration | Semantic facts in renderer cache/scene and renderer facts in semantic product audit | Cross-product publication diagnostics and M5 semantic trace regression | M6A | blocked | Required |
| SCENE-PUB-05 | Structural and dirty-phase publications remain deterministic while narrow clean/focus/semantic publications demonstrate no O(surface) deep copy of every retained renderer-side phase. | Structural/narrow regression suite and bounded cost characterization required by #59 | Reintroduced whole-surface copies, hidden rollback snapshots, or second mutable cache audit | #59 performance record plus exact phase-execution diagnostics | M6A | blocked | Required |

## M6B — canonical paint/hit scene kernel and displayed-hit cutover

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SCENE-OWN-01 | `PaintScene`, `HitTestScene`, layout, semantics, and diagnostics are distinct immutable authoritative products aligned by one `SurfacePublication`; no mixed scene node becomes authority for all five. | Public complete-publication product/type conformance | Cross-product field/type leakage and duplicate-authority audit | Publication alignment diagnostics | M6B | blocked | Required |
| SCENE-OWN-02 | Core owns only renderer/host-neutral paint/hit contribution vocabulary; runtime alone composes surface scenes, injects live mounted targets, retains displayed generations, and commits publication state. | Core/runtime dependency and downstream contribution proof | Runtime arena/backend/host authority in core and downstream live-target construction compile/API proof | Repository authority audit | M6B | blocked | Required |
| SCENE-OWN-03 | `SurfacePublication` exposes the exact immutable paint and hit scenes committed with its layout/semantic/diagnostic siblings; consuming one product transfers no live runtime mutation authority. | Public publication access/consumption tests | Public scene constructors/mutators and detached-live-authority proof | Publication identity/alignment diagnostics | M6B | blocked | Required |
| SCENE-OWN-04 | Paint scene data contains no semantic role/action/state tree, `WidgetTypeId`, concrete widget/control kind, mutable mounted state, backend handle, or native host type. | Genuine neutral scene consumer/API audit | Forbidden-type compile/API audit and source scan | Repository scene-vocabulary audit | M6B | blocked | Required |
| PAINT-01 | `WidgetPaintProof` is replaced by a state-aware public `PaintContribution` hook shared by built-ins and downstream widgets; contribution is renderer-neutral and action-type-independent. | Built-in plus genuine external-widget contribution proof | Registry/built-in special case, action coupling, and backend-type exclusion | Contribution validation diagnostics | M6B | blocked | Required |
| PAINT-02 | Paint contribution is owner-local and receives only the final local logical size/resolved style facts required by M6; it cannot author surface coordinates, mounted target identity, scene order identity, semantic state, or backend state. | Downstream custom-widget context inspection proof | Absolute-surface/ID/backend construction compile/API proof | Paint contribution validation diagnostics | M6B | blocked | Required |
| PAINT-03 | Basic filled and stroked logical rectangle primitives validate finite geometry/color/stroke data and compose deterministically into public `PaintScene` items. | Built-in/downstream primitive snapshot proof | Non-finite, negative stroke, malformed geometry, and silent fallback proof | Paint validation diagnostics | M6B | blocked | Required |
| HIT-01 | Widgets contribute zero or more owner-local hit regions through a distinct hit hook; default contribution is empty so layout participation alone never implies pointer targetability. | Button/custom-widget opt-in and generic container/text contrast proof | Every-layout-rect-targetable and paint-derived-hit proof | Hit contribution diagnostics | M6B | blocked | Required |
| HIT-02 | Widgets cannot author mounted hit targets; runtime injects the exact live `MountedNodeId` of the contributing owner into targetable composed regions. | Runtime composition plus downstream contribution proof | Forged/foreign/stale/arbitrary target construction proof | Mounted target injection/integrity diagnostics | M6B | blocked | Required |
| HIT-03 | The public immutable `HitTestScene` is the exact scene retained by runtime for displayed pointer resolution; no copied private rectangle snapshot or `SurfaceFrame` hit implementation remains a competing authority. | Public-scene versus runtime-resolution identity/integration proof | Deliberately divergent duplicate-scene regression test and source audit | M4 pointer/surface trace plus scene-selection diagnostics | M6B | blocked | Required |
| HIT-04 | `SurfaceInputContext::hit_test_generation` remains the sole public displayed hit-scene generation, and current/retained scene lookup preserves accepted M4 `SURFACE-01..08` behavior with no second scene generation or retargeting. | Full inherited M4 surface-context corpus against `HitTestScene` retention | Independent scene counter, generation mismatch, current-geometry retarget, and retired-scene reuse proof | Inherited M4 `SURFACE-*` trace evidence plus scene generation diagnostics | M6B | blocked | Required |
| HIT-05 | Public `WidgetInvalidation::HIT_TEST` exists, participates in `ALL`, invalidates hit contribution, and dirties hit composition; layout/structure imply required hit work while paint/semantics alone do not. | Invalidation dependency/phase-count tests including state-only hit-policy change | Missed hit update and undocumented PAINT/SEMANTICS coupling proof | Surface phase report plus hit invalidation diagnostics | M6B | blocked | Required |

## M6C — composition semantics, resources, metadata, damage, and capabilities

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| PAINT-04 | Every paint item has validated finite affine transform, explicit rect/rounded-rect clips, finite `[0,1]` opacity, and snapshot-local layer; item representation cannot contain an unbalanced transform/clip/opacity stack. | Nested transform/clip/opacity snapshot fixtures | Non-finite transform, invalid opacity, malformed clip, and stack-underflow/overflow exclusion | Paint item validation diagnostics | M6C | blocked | Required |
| PAINT-05 | Global paint order is deterministic by layer, mounted logical preorder, and contribution-local order with stable equal-key ordering, independent of map/storage/backend iteration. | Reorder/layer/equal-key deterministic scene corpus | Hash/storage iteration and backend-resort dependence proof | Scene order diagnostics/snapshot comparison | M6C | blocked | Required |
| PAINT-06 | Image and shaped-text-run primitives carry only logical resource references and logical placement; M6 performs no image decode, glyph shaping, font discovery, upload, or backend realization. | Deterministic fixture-resource consumer proof | Raw backend handle/native image/font/shaper dependency audit | Resource requirement/lookup diagnostics | M6C | blocked | Required |
| HIT-06 | Hit regions support rectangle/rounded-rectangle shapes with the accepted finite transform and clip semantics; transformed/clipped points resolve identically in public consumers and runtime input. | Transform/clip/rounded hit corpus | Untransformed fallback, clip bypass, and paint-geometry substitution proof | Pointer resolution trace plus hit evaluation diagnostics | M6C | blocked | Required |
| HIT-07 | Non-invertible hit transforms never fall back to untransformed geometry or retarget another owner; the region is non-hittable with deterministic diagnostic coverage. | Singular-transform hit fixture | Fallback/NaN/undefined inverse and lower-order corruption proof | Hit transform diagnostic plus no-route trace | M6C | blocked | Required |
| HIT-08 | `PointerPolicy::{Target, Block, PassThrough}` has exact topmost semantics: target resolves owner, block terminates with no target, pass-through continues to lower regions; visibility removes a region without consulting semantic state. | Overlap/policy/visibility downstream corpus | First/last ambiguity, semantic-inert coupling, and blocked-through-target proof | Pointer resolution/default trace plus hit policy diagnostics | M6C | blocked | Required |
| SCENE-RES-01 | Resource references have explicit neutral kind and logical key/reference identity and are distinct from mounted/semantic/surface identity, bytes, provider objects, and backend handles. | Resource identity/kind public API and fixture-provider proof | Identity substitution, raw bytes/backend handle, and kind-confusion compile/API proof | Missing/kind-mismatch resource diagnostics | M6C | blocked | Required |
| SCENE-RES-02 | Missing or kind-mismatched resources are reported deterministically by a consumer/admission boundary and never reinterpret a primitive as a widget or unrelated fallback primitive. | Missing/kind-mismatch fixture consumers | Silent fallback/widget-kind lookup/backend-specific substitution proof | Resource diagnostics | M6C | blocked | Required |
| SCENE-META-01 | Paint metadata exposes a positive finite raster scale used only for renderer realization; layout/hit/pointer geometry remains logical and no physical-pixel/DPI/native-window type leaks into the neutral scene/input contract. | Scale 1.0/2.0 consumer comparison and logical hit invariance proof | Non-finite/non-positive scale and physical-type API audit | Scene metadata diagnostics | M6C | blocked | Required |
| SCENE-META-02 | Paint damage is deterministic and sound: every changed renderer-relevant output is covered; full-surface damage is permitted when precision is unavailable; empty damage is permitted only when paint scene and renderer-relevant metadata are unchanged. | Clean/narrow/full-damage deterministic corpus | Underdamage and false-empty proof | Damage validation diagnostics | M6C | blocked | Required |
| SCENE-META-03 | Scene requirements are derived from the canonical paint scene and checked against neutral consumer capabilities; unsupported requirements are reported without backend-specific scene rewriting or semantic fallback. | Capability acceptance/rejection fixture consumers | Alternate-backend-scene, silent lowering, and widget-kind fallback proof | Unsupported-requirement diagnostics | M6C | blocked | Required |

## M6D — independent consumers, migration, and milestone closure

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SCENE-CONS-01 | Two independent deterministic consumers process the same public paint/hit scene products and agree on order, transforms/clips, hit outcomes, and resource requirements within their declared capability sets. | Runtime/reference consumer plus genuine independent consumer corpus | Shared private implementation/hidden test seam and divergent semantics proof | Cross-consumer comparison diagnostics | M6D | blocked | Required |
| SCENE-CONS-02 | At least one genuine downstream/custom renderer consumer renders or deterministically interprets `PaintScene` without importing/branching on `Button`, `WidgetTypeId`, semantic roles, mounted storage, or private runtime types. | External renderer package/reference app proof | Concrete-control match, registry, semantic renderer input, and private dependency audit | Downstream consumer diagnostics | M6D | blocked | Required |
| SCENE-CONS-03 | `runenui_testing` inspects/asserts the ordinary latest public paint/hit scenes and derives point input from the exact public context without fabricating scenes, targets, generations, or parallel hit logic. | Public `TestHarness` scene assertion and input convergence tests | Private bridge, forged identity, duplicate hit algorithm, and parallel expected-runtime proof | Harness/public runtime trace comparison | M6D | blocked | Required |
| SCENE-MIG-01 | `WidgetPaintProof`, its mounted capability cache, public re-exports, downstream uses, and compatibility aliases are removed once `PaintContribution` is live. | Removed-symbol/public API/downstream migration audit | Deprecated/type-alias/wrapper/doc-hidden compatibility scans | Repository migration audit | M6D | blocked | Required |
| SCENE-MIG-02 | `SurfaceFrame`/`SurfaceNode` no longer claim renderer paint or hit-test authority; any surviving layout/debug product has a separately truthful purpose and cannot be selected accidentally as the renderer/hit protocol. | Public API/documentation and consumer migration proof | Paint/hit methods, mixed widget-proof fields, and renderer-selection compatibility audit | Repository authority audit | M6D | blocked | Required |
| SCENE-MIG-03 | The private copied `HitTestSnapshot`/rectangle resolver and any duplicate debug/test hit algorithm are removed after canonical `HitTestScene` cutover; runtime pointer ingress and public consumers use the same scene semantics. | Source/public integration and deliberate divergence regression proof | Parallel snapshot/resolver/search audit | M4 pointer trace plus M6 hit-scene diagnostics | M6D | blocked | Required |
| SCENE-MIG-04 | Current docs/status/support/public API and downstream examples describe the accepted M6 products truthfully; no backend/native/semantic-widget support is claimed, no proof-era scene authority remains, and integrated M4/M5/M6 validation is green. | Cross-document truth audit, downstream examples, full configured matrix audit | Stale symbol/support claim, premature M7+/backend claim, duplicate matrix ID/status audit | Repository validation authority and M6 closure record | M6D | blocked | Required |

## M6 closure rule

M6 closes only when all 35 rows are `owner-accepted`, all inherited M4/M5
validation remains green, #59 is completed under the accepted M6A substrate,
proof-era paint/hit authorities assigned above are removed, two independent
scene consumers pass, and final current-contract reconciliation is accepted and
validated on `main`.
