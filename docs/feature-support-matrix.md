# RunenUI Feature and Support Matrix

> **Category: Current contract**

This matrix distinguishes implemented behavior from accepted targets. A public type or design document alone is not support.

Support labels:

| Label | Meaning |
|---|---|
| `supported` | Deliberately usable within the stated current contract and limitations. |
| `partial` | Real implementation exists but major required behavior is missing. |
| `proof` | A narrow deterministic case is implemented and tested. |
| `planned` | Accepted target with no implementation. |
| `deferred` | Accepted later target outside the first foundation or release. |
| `unsupported` | Not available and not safe to infer from current APIs. |

M4 is complete and owner-accepted through M4D3. M5 is active. M5A semantic
contribution/independent identity, M5B semantic publication/incremental updates,
and M5C semantic action ingress/accessibility resolution are owner-accepted.
M5C exact reviewed head `504899b79059eb94ad4474d67bba1e27eb30b374`
passed exact-head CI #1170 / `31889342640` and was guarded-squash-merged in
[PR #62](https://github.com/dornglut/runen-ui/pull/62) as
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`; reviewed and squash trees are
identical at `dfa7cb71166a3f333b560508a7e82fbeb45df000`, and accepted-main push CI
#1171 / `31903354382` passed at that exact squash. M5C adds exact public semantic
action admission/resolution through the existing canonical FIFO/routed/default/
trace authority without exposing mounted routing identity or adding a second
dispatcher. It does not add semantic LogicalScroll, the M5D testing harness, or
native accessibility.

This post-merge reconciliation records M5 truth as `53 total / 38
owner-accepted / 15 blocked`, M4 truth as `237 total / 237 owner-accepted`, and
aggregate configured truth as `290 total / 275 owner-accepted / 15 blocked`.
M5C #49 remains open for this reconciliation only; M5D #50 remains blocked until
the reconciliation itself is accepted, merged, and accepted-main verified.
Exact branch, head, blocker, validation, and next-action state belongs in the
[work-tracking system](work-tracking.md), GitHub issues, and pull requests.
Historical acceptance evidence remains in the
[public repository migration history](history/public-repository-migration.md).

## 1. Authoring and composition

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Transient UI descriptions | `supported` | Open owned `Element<Action>` trees consumed by reconciliation | Descriptions never own or expose mounted state/identity | M3 complete |
| Builder authoring | `supported` | Separate typed built-in views; downstream leaves use `Element::new`; all child-layout widgets use `Container<Action>` | Built-ins remain proof-level controls | M9 |
| `element!` authoring | `supported` | One ordinary builder/view expression lowered through `View` | Thin convenience only; no property DSL | M2 complete |
| Composite function components | `supported` | Ordinary Rust functions return typed views/elements | Components are not mounted state owners | M2 complete |
| Component action mapping | `supported` | Recursive `Element::map_action(ChildAction -> ParentAction)` including semantic-contribution neutrality | Stored mapping closure is operation-local `'static`; no string/`Any` action conversion | M2 complete; M5A complete |
| External custom widgets | `supported` | State-aware public widgets, checked routed bridge, non-`Clone` mapping, pointer/focus C/T/B, keyboard/text/composition conformance, canonical semantic contribution/owner-local bounds authoring, accepted semantic publication consumption, and downstream M5C semantic-action/readiness conformance | Production paint/layout, editable text, M5D convenience harness, and native accessibility adapters remain later | M3/M4/M5A–M5C complete |
| Child-layout authoring | `supported` | Canonical `Container<Action>`/`container`, `ChildLayout::Linear`, arbitrary children, container-only gaps | M2 proof policy only; M7 owns production custom layout | M2 complete; M7 |
| Typed control-specific builders | `supported` | Kind-specific builders; shared identity/style only where behavior is shared | Broader control vocabulary waits for M2/M9 | M2, M9 |
| Arbitrary child counts | `supported` | Iterator/collection `Views` plus arity-free heterogeneous `children!` | None within the current transient protocol | M2 complete |

## 2. Application model

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Application-owned state | `supported` | Core-owned `UiApp::State`, Counter, queued update/reconciliation | One mounted application root | M4 complete |
| Typed application actions | `supported` | `UiApp::Action`; typed widget actions; recursive mapping including routed event/work output and action-independent semantic contribution; non-`Clone`/non-`Send` proofs; accepted semantic actions converge through canonical routed/default output | Native host integration remains later; semantic ingress is accepted M5C | M4/M5A–M5C complete |
| Explicit update | `supported` | One private processor is the sole `UiApp::update(&mut State, Action)` caller; ordered `IntoEffects` result | Synchronous by design | M4 complete |
| Conditional root composition | `supported` | Counter/win root replacement with deterministic unmount/remount | One mounted root | M3 complete |
| Batched/reentrant action processing | `proof` | Multiple action/command submissions queue before a bounded iterative pump; delegated commands and routed actions append later and never recurse; every action reconciles before the next update | Recursive execution is intentionally unsupported | M4 complete |
| Initial/update effects | `proof` | Core `Effects`/`IntoEffects`; one atomic initial plan; routed event/default exact-owner work reuses the planner | Mounted host requests remain intentionally unavailable | M4 complete |
| Fine-grained signals as primary model | `deferred` | None by design | Signals may only be future adapters | Post-M3 |

## 3. Runtime identity and lifecycle

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Mounted runtime indexing | `supported` | Core-owned shared namespace, opaque `MountedNodeId` plus independently allocated `SemanticNodeId`, `SurfaceId`/`SurfaceInputContext`, checked public slot conversion, logical-preorder mounted index, foreign/stale/missing validation, and private M5C semantic-to-mounted resolution | Runtime-local, process-local, non-serialized; public semantics deliberately expose no mounted-owner shortcut; currently one logical surface | M3/M4/M5A–M5C complete |
| Independent semantic identity | `proof` | Separate runtime-owned generational semantic arena and exact mounted-owner + `SemanticKey` bindings; compatible update/reorder retains IDs; key/owner removal revokes and later reuse advances generation; capacity/index failures are fail-closed; snapshots/updates expose exact IDs read-only and M5C resolves actions privately | Process-local and surface-scoped; no public semantic-to-mounted routing surface by design | M5A–M5C complete |
| Authored element IDs | `supported` | Validated lookup/diagnostic metadata; changes preserve mounted lifetime | Not mounted or semantic identity | M3 complete |
| Stored element keys | `supported` | Unique sibling keys reconcile; duplicates preserve no state | Keys are sibling-local mounted reconciliation identity, not `SemanticKey` | M3 complete |
| Persistent generational IDs | `supported` | Safe private arenas, deterministic reuse, retirement at overflow | Not serialized or cross-runtime | M3/M5A complete |
| Keyed reconciliation | `supported` | Transactional compatible update, reorder retention, unkeyed ordinal matching, cross-parent remount, structured duplicate diagnostics | Stable reorderable collections require keys | M3 complete |
| Mount/update/unmount lifecycle | `supported` | Deterministic preorder/postorder, arena-live hooks, semantic-owner revocation, state drop after removal, idempotent shutdown | Callbacks must not panic | M3/M5A complete |
| Runtime-local widget state | `supported` | Integrity-aware checked capabilities; persistent state and private interaction slots | Broader control state waits for later milestones | M3 complete |
| Focus retention | `supported` | One exact authority retains focused lifetime, committed focus-within route, exact-generation scope memory, reason, and modality across compatible updates; cleanup is explicit; semantic publication projects only the visible PRIMARY and M5C `RequestFocus` uses accepted M4 Focusable/Automatic eligibility | One logical focus domain; no cross-surface transfer | M3/M4/M5B–M5C complete |
| Granular invalidation | `supported` | Explicit phase functions, exact context key, topology-only whole-surface cache, current mounted common-field reads, independently verified `SurfacePhaseReport`, semantic contribution caching, semantic product-only focus dirtiness, layout-driven semantic-bound refresh without callback re-entry, and M5C fail-closed semantic-dirty admission | Whole-surface structural/layout work remains conservative; retained `SurfaceCache` deep cloning is tracked by #59; production node-granular layout remains later | M3/M5B–M5C complete; M6/M7/M11 |

## 4. Events and interaction

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Typed pointer vocabulary | `proof` | Checked `PointerId`/optional `InputDeviceId`, neutral device kind, finite logical position/movement/scroll, normalized buttons/modifiers, down/move/up/cancel/wheel, and opaque surface context; no unchecked target | No pressure, tilt, twist, click count, or native host-event type | M4 complete |
| Typed keyboard vocabulary | `proof` | Host-neutral `KeyboardEvent` carries phase, physical/logical identity, modifiers, repeat, location, composition state, and optional device identity | No native host translation or broad shortcut policy | M4 complete |
| Pointer hit targeting | `proof` | Current/retained frame rectangle targeting produces generation-safe physical paths kept separate from routed/captured owners | No explicit hit scene, stacking, clips, transforms, visibility, or M6 pointer policy | M4 complete; M6 |
| Pointer activation | `proof` | Canonical down/move/up/cancel lifecycle; eligible primary release inside the exact live pressed owner derives one routed `Activate`; Counter proves public physical convergence | No native host translation or broader production control policy | M4 complete; M9/M10 later |
| Keyboard activation | `proof` | Exact focused keyboard ingress routes C/T/B; non-repeated Enter down and matched Space down/up append canonical `Activate` | No native host translation or production control policy | M4 complete |
| Focus traversal | `proof` | Root/nested scopes; current-order next/previous; published-geometry direction; explicit boundary policy; exact restoration; resulting focus changes publish through the semantic sibling without semantic callback re-entry; semantic `RequestFocus` converges through the same default authority | Cross-surface transfer remains M10 | M4/M5B–M5C complete; M10 later |
| Event capture/target/bubble | `proof` | Immutable exact-mounted C/T/B semantic-command, pointer, focus, keyboard, committed-text, and composition routes; checked bridges; independent propagation/default control; accepted M5C semantic requests join the same routed path after private admission | No native event translation | M4/M5C complete |
| Pointer capture | `proof` | One exact live capture owner per active pointer; ordered staged capture/release/transfer; loss before gain; deterministic lifecycle cleanup and stale-owner suppression | Proof is host-neutral runtime behavior, not drag/control policy | M4 complete |
| Touch/pen behavior | `unsupported` | Checked device identity and neutral touch/pen categories share the pointer stream protocol | No contact, pressure, tilt, twist, eraser, or host translation contract | M4 protocol; later host/control work |
| Text input and IME events | `proof` | Nonempty committed Unicode text and opaque generation-scoped composition start/update/end/cancel route to exact focused opt-in capability | No editable text, native IME object, selection, or text-layout contract | M4 complete; M8–M10 later |
| Accessibility/programmatic activation | `proof` | Exact-mounted programmatic/accessibility-stub/controller origins converge through `submit_command`; automation resolves exactly one authored ID; M5C resolves exact current `SurfaceId + SemanticNodeId + SemanticAction` through private bindings into the same canonical command/default path | No native accessibility adapter or M5D convenience harness | M4/M5C complete; M5D/M10 later |
| Controller/gamepad input vocabulary | `unsupported` | None; keyboard vocabulary does not imply controller support | No normalized controller-facing command or device event model | M10 |
| Abstract UI navigation commands | `partial` | Existing commands plus `FocusNext`/`Previous`/four directions, exact request/restore, canonical `LogicalFocusScroll`, and exact M5 semantic actions with consistent origins | Logical scroll remains route-only; semantic scrolling and raw source mapping are later | M4/M5C complete; M7/M10 |
| Directional/spatial focus navigation | `proof` | Current publication geometry and mounted-order final tie-break satisfy DF-01–DF-20 through public submission | Private score is intentionally not API | M4 complete |
| Controller activation/cancel/menu commands | `proof` | Normalized controller source submits the same exact-target semantic commands without raw device vocabulary | Raw device mapping/identity/axes remain host/M10 scope | M4 complete; M10 |
| Input modality tracking | `proof` | Last accepted pointer/keyboard/controller/accessibility/automation/programmatic source is retained and traced without widget-event delivery; M5C semantic requests use accessibility source while preserving canonical command semantics | Raw controller/native accessibility translation remain later | M4/M5C complete; M10 later |
| Displayed-generation input targeting | `proof` | Runtime-issued `SurfaceInputContext`, exact current/historical logical/resolved command ingress, and pointer validation against retained generation/revision with no current-geometry fallback | One logical surface; no multi-window lifecycle | M4 complete; M10 later |
| Terminal pointer context cleanup | `proof` | Retired/missing active `Up` performs no ordinary route/re-hit/activation but clears pressed/path/capture, notifies a live capture owner, and closes; cancel treats unavailable geometry as diagnosis only | Foreign runtime/surface cannot mutate local streams | M4 complete |
| Route-only command defaults | `proof` | Cancel/menu/context each route once; eligible wheel derives exactly one logical-scroll command with no production offset mutation | Production scrolling mutation is later work | M4 complete |
| Directional focus corpus | `proof` | Every DF-01–DF-20 vector is individually identified in executable public-command proof using runtime-issued IDs and current retained publication rectangles | Private scoring remains deliberately non-public; multi-surface transfer remains absent | M4 complete |

## 5. Effects and scheduling

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Synchronous direct dispatch | `unsupported` | `AppRuntime::dispatch` and private dispatch authorities were removed | Callers must submit and explicitly pump | M4 complete |
| Application action and command submission | `proof` | `submit_action` returns exact action recovery; `submit_command` returns `CommandSubmission` or exact owned target/command/origin recovery; `submit_semantic_action` returns a runtime-issued submission or exact owned semantic request with typed surface/identity/support/state/readiness/capacity rejection | M5D convenience helpers and native adapters remain later | M4/M5C complete; M5D/M10 |
| Queue saturation | `proof` | Waiting-envelope, transaction output, live-family, completion, subscription-diagnostic, and trace limits; initial plans reserve aggregate allowance, while routed/semantic plans fail closed before mutable callbacks and preserve exact recovery/sequence/wake atomicity | Exact callback-declared capacity is not an M4/M5C API; the trace sink is subordinate and independently bounded | M4/M5C complete |
| Action queue and ordering | `proof` | One generalized FIFO sequences actions, commands, semantic actions after admission, pointer/input bundles, focus notifications, reconciliation, work, timers, and mapped results; notification/initiating/default output order is explicit | Native host translation remains absent | M4/M5C complete |
| Effects | `proof` | Opaque ordered application descriptions plus mounted lifecycle/activation/event contexts; routed event/default output commits invalidation, actions/commands, and exact-owner work atomically | Mounted host requests remain intentionally unavailable | M4 complete |
| Processed-envelope pump budget | `proof` | Four-argument `PumpBudget`; exact report and outcome | None in implemented work scope | M4 complete |
| Other readiness budgets | `proof` | Exact completion-import, local-poll, and timer-promotion limits/counters/exhaustion flags | None in implemented work scope | M4 complete |
| Owner-local keyed cancellation | `proof` | Validated `WorkKey`, private generations, commit-bound same-batch semantics, and stale-completion rejection for application and exact mounted owners | None in implemented work scope | M4 complete |
| Async tasks | `proof` | Wake-aware local futures; one-attempt send executor; bounded completion ingress; UI mapper validation | Runtime supplies adapters, not an executor implementation | M4 complete |
| Timers and subscriptions | `proof` | Manual/host monotonic time; one-shot/repeating timers; current-state complete-set diffs; wake-aware local sources; one-attempt send producers with explicit start/sink outcomes | No animation system | M4 complete |
| Mounted subscription declarations | `proof` | Public widget capability; queued newest-state evaluation without declaration caches; routed-event/activation/update invalidation, coalescing, stale-owner suppression, duplicate, and lifecycle proofs | None in implemented declaration scope | M4 complete |
| Send-executor start outcomes | `proof` | Started/unavailable/full/closed/rejected, one attempt, optional failure action | Retry requires new effect | M4 complete |
| Send-subscription outcomes | `proof` | Explicit `Starting -> Running`; synchronous startup sends return exact `NotStarted`; full/closed/stale rejection returns exact item; refusal reclaims the generation | Retry requires a new declaration revision | M4 complete |
| Host commands | `proof` | Closed application protocol, opaque token, response-kind validation, and live-only lock-protected direct/detached/cancellation response authority without terminal tombstones | Platform services remain M10 | M4 complete; M10 |
| Wake/redraw scheduling | `proof` | One state mutex owns coalesced wake request, transport, delivery claims, and callback-in-flight state; serialized callbacks run outside all framework synchronization guards and are claimed at most once per outstanding request; revisioned redraw remains independent | Native event-loop adapter absent | M4 complete |
| Deterministic scheduler testing | `proof` | Manual clock, injectable executor/clock/wake transport, exact pump reports | Unified M5D harness absent | M4 complete; M5D |

## 6. Styling

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Literal color/padding/radius | `supported` | `StyleIntent` and `ComputedStyle` | Very small property surface | M7 |
| Typed token references | `supported` | Unicode-validated text identity, color/spacing/radius families, mixed static/dynamic lookup, and non-overwriting definitions | Theme loading/fallback remain absent | M7 |
| Token resolution | `supported` | `StyleTokens`, diagnostic revision, exact-content context compatibility, mounted-current authored reference changes, and pure resolver | In-memory values only; no fallback or theme loading | M7 |
| Provenance and missing-token diagnostics | `supported` | `StyleResolution`/`SurfaceStyleReport` | Limited to current fields and one publication | M7 |
| Computed padding geometry | `proof` | Padding affects measurement, placement, and hit bounds | Incomplete box model | M7 |
| Theme tokens and selection | `planned` | Accepted resolution order | No theme object or platform/user preferences | M7 |
| Control recipes and variants | `planned` | Accepted architecture | Must wait for mounted controls | M7 |
| Interaction-state styling | `planned` | Accepted architecture | Must wait for mounted hover/pressed/focus/disabled state | M7 |
| Typography, borders, shadows, opacity, transforms | `unsupported` | None | Values and behavior absent | M7–M8 |
| CSS selector/cascade system | `deferred` | Explicitly not the initial model | May never be required | Post-M7 |

## 7. Layout and measurement

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Finite/unbounded constraints | `supported` | Core-owned validated `LogicalLength`, canonical `LogicalSize`/`LogicalRect`, finite points, normalized `LayoutConstraints`, checked baselines | Broader sizing vocabulary remains absent | M1/M5A complete; M7 |
| Renderer-neutral measurement seam | `supported` | Borrowed `MeasurementProvider` with cache identity/revision | Text-only synchronous contract; no resource or typography input | M8 |
| Deterministic headless measurement | `proof` | Unicode-scalar count with fixed metrics | Not production text geometry | M8 |
| One measurement capability snapshot per node/publication | `proof` | Counter-backed downstream tests prove one query reused by measurement and arrangement | Capability facts are retained, but a dirty Layout phase remains whole-surface rather than node-granular production incremental layout | M7, M11 |
| One child-layout snapshot per child-bearing node/publication | `proof` | Counter-backed external alternating-axis proof | Only linear M2 policy exists | M7 |
| Unsupported measurement handling | `proof` | Explicit unsupported and cross-version-unrecognized layout diagnostics | Zero fallback geometry is proof-level only | M7 |
| Publication alignment | `proof` | Warmed structural/common-field tests plus context-bearing publication prove aligned mounted IDs, metadata, style, layout, order, node counts, and fresh displayed-generation identity; M5B separately publishes semantic IDs/bounds/focus through the semantic sibling | Semantic products deliberately do not collapse into renderer-product alignment | M4/M5B complete; M6–M7 |
| Row/column layout | `proof` | Intrinsic main axis; constrained cross axis; gaps/padding | No stretch, flex, alignment, wrapping, or remaining-space distribution | M7 |
| Overflow diagnostics | `proof` | Runtime-node-aligned flags/report | No clipping or scrolling behavior | M7 |
| Width/height/min/max/fill/shrink | `unsupported` | None | Authored sizing model absent | M7 |
| Flex/grid | `unsupported` | None | Adopt-versus-build ADR required | M7 |
| Stack/absolute/overlay | `unsupported` | None | No overlay layout or stacking contract | M7 |
| Baseline layout | `unsupported` | Measurement response can carry baseline values | Layout does not consume them | M7–M8 |
| Clipping and scrolling | `unsupported` | None | No clips, extents, scroll state, input, or semantic scroll product | M7–M9 |
| Incremental layout | `unsupported` | None | Clean and non-layout phases can reuse cached publication facts, but a dirty Layout phase still recomputes the whole surface; no node-granular incremental layout or damage propagation | M7, M11 |
| Virtualization | `deferred` | None | Requires mounted identity, scrolling, and advanced controls | M12 |

## 8. Semantics and accessibility

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Focusability facts | `proof` | Open widget activation/focusability facts drive runtime indexing; M5B projects current mounted focus to the visible semantic PRIMARY only and diagnoses a focused owner with no visible PRIMARY; M5C semantic `RequestFocus` uses current M4 Focusable/Automatic eligibility | Native accessibility translation and cross-surface focus remain later | M4/M5B–M5C complete; M10 |
| Semantic contribution | `supported` | `Widget::semantics(state, SemanticContributionContext) -> SemanticContribution`; 0..N owner-local nodes; `SemanticKey::PRIMARY`/named keys; strict mounted-child marker/local-reference validation; platform-neutral role/name/description/value/state/action/relationship/text vocabulary; `SemanticBounds::{Owner, OwnerLocal}`; downstream action-map/geometry conformance | Contribution is input authority; absolute coordinates, runtime focus, resolved relationships, and action routing are runtime-derived | M5A–M5C complete |
| Independent semantic identity | `proof` | Opaque `SemanticNodeId` issued from a separate runtime semantic arena and reconciled by exact mounted owner + `SemanticKey`; reorder retention, stale-safe removal/reuse, foreign/missing/capacity/index integrity proofs; exact IDs are exposed read-only through snapshots/updates and resolved privately by M5C action ingress | No public semantic-to-mounted routing surface by design | M5A–M5C complete |
| Semantic tree and snapshot | `proof` | Independent renderer-neutral `SemanticPublication` exposes exact `SurfaceId`, revisioned deterministic forest/preorder, roots, exact-ID lookup, resolved relationships, absolute bounds, composed state/support, and runtime PRIMARY focus | One logical surface; no native adapter | M5B complete; M10 later |
| Incremental semantic updates | `proof` | Checked non-wrapping revisions; first snapshot at 1; unchanged product no bump; deterministic added/changed/removed/root/focus deltas; wrong surface or wrong/skipped prior revision returns full resync; diagnostics-only change does not advance revision | Update consumption remains read-only; action execution uses the separate exact-current M5C ingress rather than mutating snapshots | M5B/M5C complete |
| Semantic diagnostics | `proof` | Surface-scoped typed diagnostics cover owner withdrawal, missing/ambiguous relationship targets, missing bindings/owners, and focused-owner-without-visible-PRIMARY without leaking public mounted routing identity | No stable severity policy or native platform diagnostic mapping | M5B complete |
| Semantic supported actions/state | `proof` | Support is distinct from current availability; composed disabled state includes owner-wide disabled; M5 vocabulary is exactly `Activate`, `RequestFocus`, `OpenMenu`, `OpenContextMenu`; no semantic LogicalScroll alias; M5C admission rechecks exact support/state/readiness | Semantic scrolling remains deferred to M7 | M5B–M5C complete; M7 |
| Semantic actions | `proof` | Public `SemanticActionRequest { surface, target, action }` via `AppRuntime::submit_semantic_action`; exact current surface/identity/publication/support/state/readiness/freshness/capacity admission; no callback at submission; canonical FIFO/`WorkSequence`/route/default/trace convergence; queue-front and post-callback revalidation; exact owned-request recovery on rejection | No semantic LogicalScroll, public mounted-owner shortcut, native adapter, or M5D convenience harness | M5C complete; M5D/M7/M10 later |
| Accessibility queries/tests | `partial` | Public semantic snapshot supports deterministic direct inspection and exact-ID lookup; downstream direct and adapter-shaped consumers plus M5C semantic-action conformance prove the public product/ingress boundary | No M5D query DSL/unique-query result/action helper or unified public harness | M5D |
| AccessKit adapter | `planned` | Accepted adapter-only direction; M5B vocabulary plus M5C exact semantic action ingress provide platform-neutral tree/focus/state/actions/relationships/bounds/update/action facts without AccessKit/native types | M5E source-grounded mapping review remains required; no native bridge in M5 | M5E review; M10 bridge |
| Native accessibility bridge | `planned` | Required desktop profile | Depends on host/platform integration | M10 |
| Accessible text ranges | `planned` | Required production text contract | Depends on editable text and semantic mapping | M8 |

## 9. Surface, hit testing, and rendering

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Unified surface publication | `proof` | Fallible `SurfacePublication` is one staged admit/plan/final-preflight/commit transaction and carries renderer products plus mandatory independent semantic publication and semantic diagnostics; complete versus renderer-only equality/extraction is explicit | One logical surface; no neutral paint scene; whole-`SurfaceCache` clone debt is tracked by #59 | M5B complete; M6/M10 later |
| Publication failure/backpressure | `proof` | Recoverable stationary-rehit queue `Full` performs zero publication/cache/semantic/snapshot/trace/redraw/rehit commit and leaves redraw pending; redraw/hit-test/coordinate/semantic counter exhaustion is typed terminal authority with no wrap/saturation | Proof-level retained cache architecture is not yet M6's persistent scene design | M5B complete; M6 readiness #59 |
| Logical bounds inspection | `proof` | Renderer `SurfaceNode` rectangles plus independent semantic absolute logical bounds | Bounds remain proof-level rectangles rather than a production layout/hit/paint scene | M5B complete; M6–M7 |
| Rectangle hit testing | `proof` | Reverse frame order | No hit scene, stacking, clips, transforms, visibility, or pointer policy | M6 |
| Renderer-neutral paint scene | `unsupported` | M2 deterministic per-widget paint/debug proof facts | Facts are not paint primitives, resources, clips, transforms, order, or damage | M6 |
| Paint primitives/resources | `unsupported` | None | No shapes, strokes, glyph/image handles, clips, layers, or damage | M6 |
| Surface/frame generation | `proof` | Fresh runtime-issued coordinate revision and displayed hit-test generation on every public publication; semantic revision is separately surface-scoped and advances only on adapter-visible semantic change | One logical surface; not a paint-scene generation or multi-window lifecycle | M4/M5B complete; M6/M10 later |
| Retained surface-input snapshots | `proof` | Configurable nonzero bounded immutable hit-test snapshots, exact historical targeting, oldest-first retirement, and retired/missing/foreign/revision outcomes | Retains hit-test facts only, not production layout/paint scenes | M4 complete; M6 later |
| Multi-surface publication | `unsupported` | None | No independent surface lifecycle or scale | M10 |
| Debug/renderer semantic separation | `proof` | Renderer-facing `SurfaceFrame`, `SurfaceNode`, and debug output do not carry production semantic contribution; semantics are consumed from the sibling `SemanticPublication`, while M5C action resolution remains private runtime authority | No native accessibility adapter, paint-scene consumer, or renderer backend | M5B–M5C complete; M6/M10 later |
| Deterministic paint-scene consumer | `planned` | None | Needs accepted paint/hit protocols | M6 |
| Conventional renderer backend | `unsupported` | None | Protocol must stabilize first | M10 |
| Embedded/SDF renderer consumer | `deferred` | None | Follows neutral protocol and conventional proof | M10 or M12 |

## 10. Text

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Static text descriptors | `proof` | Text/button widgets use the open protocol and canonical semantic contribution | No production text shaping or complete control contract | M8–M9 |
| Headless deterministic metrics | `proof` | Fixed scalar width and line height | Not shaping, grapheme measurement, or font metrics | M8 |
| Font discovery/fallback | `unsupported` | None | No font/resource provider | M8 |
| Shaping and multilingual scripts | `unsupported` | None | No production text stack | M8 |
| Bidi/RTL | `unsupported` | None | No paragraph model | M8 |
| Line breaking/wrapping | `unsupported` | None | Layout does not negotiate paragraphs | M8 |
| Emoji/combining marks | `unsupported` | Scalar count only | Geometry is not grapheme/glyph aware | M8 |
| Baselines | `partial` | Optional measurement fields | Not consumed or published in layout | M8 |
| Editable text | `unsupported` | Host-neutral committed-text/composition events only | No editing state, selection, caret, clipboard, or text-layout behavior | M8–M9 |
| Selection/caret/clipboard/native IME | `unsupported` | None | Required production contracts absent | M8–M10 |

## 11. Controls

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Text/label | `proof` | Static text element plus canonical Text contribution published through the semantic sibling | No production text or complete native accessibility contract | M8–M9 |
| Button | `proof` | Label, enabled/actionable state, repeatable `on_activate` action factory, persistent local activation state and interaction slots; pointer, Enter, Space, programmatic, authored automation, and M5C semantic PRIMARY activation converge through routed `Activate`; built-in Button authors canonical Button semantics and appears in accepted semantic publication | No native accessibility adapter, recipes, or production control breadth | M4/M5A–M5C complete; M9/M10 |
| Checkbox/radio/toggle | `unsupported` | None | Standard control foundation absent | M9 |
| Slider/progress | `unsupported` | None | Events, semantics, and layout prerequisites incomplete | M9 |
| Text field | `unsupported` | None | Production text/editing prerequisites absent | M8–M9 |
| Scroll container/list | `unsupported` | None | Mounted scrolling and production layout absent | M7–M9 |
| Menu/popover/tooltip/dialog | `unsupported` | None | Overlay, focus-scope, event, and scene prerequisites absent | M7–M9 |
| Tabs | `unsupported` | None | Standard control contract absent | M9 |
| Tree/data grid/editor controls | `deferred` | None | Advanced application systems | M12 |

## 12. Host and platform integration

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Host-neutral core/runtime | `supported` | No native window, GPU, ECS, AccessKit, or legacy dependencies | Neutrality alone is not an embedding contract | M10 |
| Platform host contract | `unsupported` | Application-defined typed request/response protocol and runtime wake transport are isolated seams | No platform lifecycle, service capability discovery, windows, event-loop adapter, or resource contract | M10 |
| Closed application host protocol | `proof` | Core command/response/response-kind contract; opaque runtime-local request generations; exact kind validation, single-winner response state machine, and UI-thread mapper path | Platform service families remain M10 | M4 complete; M10 |
| Headless host profile | `partial` | Direct deterministic mounted runtime use with manual clock, injectable send executor, wake transport, typed host requests, accepted synthetic input ingress, serialized offline replay, semantic contribution/identity/publication/update/diagnostics, and accepted exact semantic action ingress | No unified M5D harness or native host adapter | M4/M5A–M5C complete; M5D next |
| Desktop event loop/window | `unsupported` | None | No Winit or equivalent adapter | M10 |
| Windows/macOS/Linux support | `unsupported` | Platform-independent Rust tests only | No native application proof | M10–M11 |
| DPI and resize | `unsupported` | Logical geometry only | No scale/surface lifecycle | M10 |
| Clipboard/cursor/drag-drop/file dialogs | `unsupported` | None | Host services absent | M10 |
| IME | `unsupported` | M4 supplies only owner-accepted host-neutral committed-text/composition event ingress | No native platform IME object, host adapter, editable text, selection, clipboard, or text layout | M8–M10 |
| Multi-window | `unsupported` | None | Runtime surface ownership absent | M10 |
| Embedded external-host proof | `unsupported` | None | No host and scene protocols | M10 |
| Controller connection/disconnection | `unsupported` | None | No host device lifecycle or stable controller identity | M10 |
| Axis normalization and dead-zone policy | `unsupported` | None | No host-owned raw axis translation or reviewed normalization policy | M10 |
| Embedded-host controller mapping | `unsupported` | None | No contract mapping host devices to normalized UI commands | M10 |

## 13. Testing and diagnostics

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Workspace unit/integration tests | `supported` | Substantial deterministic proof suite plus public-only downstream custom-widget package including M5A authoring, M5B publication, and M5C semantic-action/readiness conformance | No unified M5D harness and Ubuntu-only CI | M5D; M11 |
| Strict formatting and linting | `supported` | Shared `cargo validate` runs stable rustfmt, locked tests, Clippy `-D warnings`, MSRV tests, and link checks locally and in CI | Current CI is Ubuntu-only; the production platform matrix remains later work | M0 |
| Style/layout/semantic diagnostics | `supported` | Mounted-aligned style/layout reports, runtime mismatch diagnostics, contribution/semantic identity integrity handling, and M5B surface-scoped typed semantic publication diagnostics | No stable severity/strict mode or native diagnostic mapping | M5B complete; M7 later |
| Runtime trace | `partial` | One accepted bounded canonical M4D1-normalized graph, M4D2 deterministic JSONL v1 export/redaction/action-label/sink surface, M4D3 inert offline replay, and M5C semantic binding/processing-rejection/default-invalidation outcomes in the same schema | Export/sink/replay remain headless proof infrastructure rather than a production observability service or M5D semantic expectation engine | M4/M5C complete; M5D later |
| Bounded canonical trace retention | `proof` | Configured capacity including zero, oldest-first eviction, non-wrapping `TraceSequence`, borrowed iteration, exclusive dropped-before watermark, normalized-schema proof, deterministic v1 projection, accepted offline causal replay consumption, and M5C semantic causal records | Replay validates the serialized retained causal projection; it does not create live runtime authority | M4/M5C complete |
| Bounded external trace sink | `proof` | One-time public receiver; lazy atomic logical capacity; immutable canonical-record handoff; consumer-side JSON encoding; structured `Delivered`/`Full`/first `Closed` outcomes; shutdown closure; four-state isolation proof | Subordinate headless diagnostic transport only; no arbitrary callback/work capability or replay authority | M4 complete |
| Public headless test harness | `planned` | Current tests, accepted replay foundation, semantic publication/identity, and M5C exact semantic ingress prove prerequisites | No `runenui_testing` public boundary | M5D |
| Semantic/layout/hit/paint assertions | `partial` | Public semantic snapshots support direct deterministic inspection/exact-ID lookup; M5C adds exact public action ingress; current layout/frame/hit/paint proof surfaces are inspectable | No unified M5D query/assertion/action-helper layer or production M6 scene assertions | M5D; M6 |
| Deterministic time/tasks | `proof` | Manual monotonic clock, wake-aware local tasks, injectable send executor | Unified M5D harness absent | M4 complete; M5D |
| Snapshot/golden/replay tests | `partial` | Accepted deterministic JSONL snapshots are byte-stable; M4D3 round-trips real exported JSONL and reconstructs Counter causality; M5B adds deterministic semantic snapshots/update chains; M5C proves semantic trace export/replay remains inert | The unified public M5D harness and semantic query/action expectation layer are absent | M4/M5B–M5C complete; M5D |
| Property/fuzz testing | `unsupported` | None | Production hardening work | M11 |
| Benchmarks and budgets | `unsupported` | None | No performance gates; #59 owns retained-publication clone investigation | M6 readiness; M11 |
| Cross-platform CI | `unsupported` | Ubuntu-only CI | Windows/macOS jobs absent | M11 |
| Controller-only application operation | `unsupported` | None | No raw controller/device translation, applicable control conformance, or game-oriented reference proof | M11 |

## 14. Source formats and devtools

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Typed Rust as source authority | `supported` | Builders, public `View`/`Widget`, ordinary components, canonical semantic contribution, and thin `element!`/`children!` | External source formats remain deferred | M2/M5A complete |
| External UI source format | `deferred` | None | Requires stable semantic authoring and diagnostics | M12 |
| Inspector/devtools | `deferred` | Debug render/report functions plus read-only semantic publication exist, but no inspector product | No integrated source mapping/live inspection model | M12 |
| Hot reload/live preview | `deferred` | None | Requires stable identity, invalidation, source mapping, and host integration | M12 |
| Visual authoring | `deferred` | None | Depends on source and devtools foundations | M12 |

## 15. Advanced application systems

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Virtualized collections | `deferred` | None | Requires scrolling, mounted identity, and production layout | M12 |
| Animation/time model | `deferred` | None | ADR and scheduler required | M12 |
| Overlays and advanced multi-surface | `deferred` | None | Requires scenes, focus scopes, layout, and host lifecycle | M12 |
| Docking/workspaces | `deferred` | None | Requires drag/capture, overlays, persistence, and multi-surface behavior | M12 |
| Advanced editor/game controls | `deferred` | None | Builds on the production control and host foundations | M12 |
| Mobile/web profiles | `deferred` | None | Outside the first production release | Post-v1 |
