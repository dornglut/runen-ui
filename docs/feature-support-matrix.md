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

M4 is complete and owner-accepted through M4D3. M5 is active, and M5A semantic
contribution and independent identity is owner-accepted. The reviewed M5A
feature head `8377ced53c08d7b5be3020368ceddd3ee81294a5` passed exact-head CI
run `31497457992` / #889 and was guarded-squash-merged in
[PR #53](https://github.com/dornglut/runen-ui/pull/53) as
`e3c304600ec1777cd17a1973946a43c765df1c31`; all 38 changed-file blob
identities are byte-identical between reviewed head and accepted squash. M5A
adds the canonical platform-neutral semantic contribution vocabulary, strict
owner-local contribution validation, core-owned logical geometry, and a
separate runtime-owned semantic generational identity store. It does not yet
publish the independent semantic tree/update product, resolve semantic-node
actions/accessibility, provide the public testing harness, or add AccessKit/
native accessibility. Exact branch, head, blocker, validation, and next-action
state belongs in the [work-tracking system](work-tracking.md), GitHub issues,
and pull requests. Historical acceptance evidence remains in the
[public repository migration history](history/public-repository-migration.md).

## 1. Authoring and composition

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Transient UI descriptions | `supported` | Open owned `Element<Action>` trees consumed by reconciliation | Descriptions never own or expose mounted state/identity | M3 complete |
| Builder authoring | `supported` | Separate typed built-in views; downstream leaves use `Element::new`; all child-layout widgets use `Container<Action>` | Built-ins remain proof-level controls | M9 |
| `element!` authoring | `supported` | One ordinary builder/view expression lowered through `View` | Thin convenience only; no property DSL | M2 complete |
| Composite function components | `supported` | Ordinary Rust functions return typed views/elements | Components are not mounted state owners | M2 complete |
| Component action mapping | `supported` | Recursive `Element::map_action(ChildAction -> ParentAction)` including M5A semantic-contribution neutrality | Stored mapping closure is operation-local `'static`; no string/`Any` action conversion | M2 complete; M5A complete |
| External custom widgets | `supported` | State-aware public widgets, checked routed bridge, non-`Clone` mapping, pointer/focus C/T/B, owner-accepted keyboard/text/composition conformance, and owner-accepted canonical M5A semantic contribution/owner-local bounds authoring | Independent semantic publication/action resolution, production paint/layout, editable text, and accessibility adapters remain later | M3/M4/M5A complete |
| Child-layout authoring | `supported` | Canonical `Container<Action>`/`container`, `ChildLayout::Linear`, arbitrary children, container-only gaps | M2 proof policy only; M7 owns production custom layout | M2 complete; M7 |
| Typed control-specific builders | `supported` | Kind-specific builders; shared identity/style only where behavior is shared | Broader control vocabulary waits for M2/M9 | M2, M9 |
| Arbitrary child counts | `supported` | Iterator/collection `Views` plus arity-free heterogeneous `children!` | None within the current transient protocol | M2 complete |

## 2. Application model

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Application-owned state | `supported` | Core-owned `UiApp::State`, Counter, queued update/reconciliation | One mounted application root | M4 complete |
| Typed application actions | `supported` | `UiApp::Action`; typed widget actions; recursive mapping including routed event/work output and action-independent semantic contribution; non-`Clone`/non-`Send` proofs | Native host and later semantic action/publication families remain later work | M4/M5A complete |
| Explicit update | `supported` | One private processor is the sole `UiApp::update(&mut State, Action)` caller; ordered `IntoEffects` result | Synchronous by design | M4 complete |
| Conditional root composition | `supported` | Counter/win root replacement with deterministic unmount/remount | One mounted root | M3 complete |
| Batched/reentrant action processing | `proof` | Multiple action/command submissions queue before a bounded iterative pump; delegated commands and routed actions append later and never recurse; every action reconciles before the next update | Recursive execution is intentionally unsupported | M4 complete |
| Initial/update effects | `proof` | Core `Effects`/`IntoEffects`; one atomic initial plan; routed event/default exact-owner work reuses the planner | Mounted host requests remain intentionally unavailable | M4 complete |
| Fine-grained signals as primary model | `deferred` | None by design | Signals may only be future adapters | Post-M3 |

## 3. Runtime identity and lifecycle

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Mounted runtime indexing | `supported` | Core-owned shared namespace, opaque `MountedNodeId` plus independently allocated `SemanticNodeId`, `SurfaceId`/`SurfaceInputContext`, checked public slot conversion, logical-preorder mounted index, foreign/stale/missing validation | Runtime-local, process-local, non-serialized; semantic IDs are not a singular mounted-index projection; currently one logical surface | M3/M4 complete; M5A complete |
| Independent semantic identity | `proof` | Separate runtime-owned generational semantic arena and exact mounted-owner + `SemanticKey` bindings; compatible update/reorder retains IDs; key/owner removal revokes and later reuse advances generation; capacity/index failures are fail-closed | M5B has not yet published semantic IDs through the independent semantic snapshot/update product | M5A complete; M5B next |
| Authored element IDs | `supported` | Validated lookup/diagnostic metadata; changes preserve mounted lifetime | Not mounted or semantic identity | M3 complete |
| Stored element keys | `supported` | Unique sibling keys reconcile; duplicates preserve no state | Keys are sibling-local mounted reconciliation identity, not `SemanticKey` | M3 complete |
| Persistent generational IDs | `supported` | Safe private arenas, deterministic reuse, retirement at overflow | Not serialized or cross-runtime | M3/M5A complete |
| Keyed reconciliation | `supported` | Transactional compatible update, reorder retention, unkeyed ordinal matching, cross-parent remount, structured duplicate diagnostics | Stable reorderable collections require keys | M3 complete |
| Mount/update/unmount lifecycle | `supported` | Deterministic preorder/postorder, arena-live hooks, semantic-owner revocation, state drop after removal, idempotent shutdown | Callbacks must not panic | M3/M5A complete |
| Runtime-local widget state | `supported` | Integrity-aware checked capabilities; persistent state and private interaction slots | Broader control state waits for later milestones | M3 complete |
| Focus retention | `supported` | One exact authority retains focused lifetime, committed focus-within route, exact-generation scope memory, reason, and modality across compatible updates; cleanup is explicit | One logical focus domain; no cross-surface transfer; semantic focus projection waits for M5B | M3/M4 complete; M5B next |
| Granular invalidation | `supported` | Explicit phase functions, exact context key, topology-only whole-surface cache, current mounted common-field reads, independently verified `SurfacePhaseReport`, semantic contribution caching with direct-child structural invalidation | Whole-surface structural rebuilds remain conservative; semantic product updates and production incremental layout are later work | M3/M5A complete; M5B/M7/M11 |

## 4. Events and interaction

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Typed pointer vocabulary | `proof` | Checked `PointerId`/optional `InputDeviceId`, neutral device kind, finite logical position/movement/scroll, normalized buttons/modifiers, down/move/up/cancel/wheel, and opaque surface context; no unchecked target | No pressure, tilt, twist, click count, or native host-event type | M4 complete |
| Typed keyboard vocabulary | `proof` | Host-neutral `KeyboardEvent` carries phase, physical/logical identity, modifiers, repeat, location, composition state, and optional device identity | No native host translation or broad shortcut policy | M4 complete |
| Pointer hit targeting | `proof` | Current/retained frame rectangle targeting produces generation-safe physical paths kept separate from routed/captured owners | No explicit hit scene, stacking, clips, transforms, visibility, or M6 pointer policy | M4 complete; M6 |
| Pointer activation | `proof` | Canonical down/move/up/cancel lifecycle; eligible primary release inside the exact live pressed owner derives one routed `Activate`; Counter proves public physical convergence | No native host translation or broader production control policy | M4 complete; M9/M10 later |
| Keyboard activation | `proof` | Exact focused keyboard ingress routes C/T/B; non-repeated Enter down and matched Space down/up append canonical `Activate` | No native host translation or production control policy | M4 complete |
| Focus traversal | `proof` | Root/nested scopes; current-order next/previous; published-geometry direction; explicit boundary policy; exact restoration | Cross-surface transfer and semantic focus projection/action resolution remain M5B/M5C | M4 complete; M5B/M5C |
| Event capture/target/bubble | `proof` | Immutable exact-mounted C/T/B semantic-command, pointer, focus, keyboard, committed-text, and composition routes; checked bridges; independent propagation/default control | No native event translation | M4 complete |
| Pointer capture | `proof` | One exact live capture owner per active pointer; ordered staged capture/release/transfer; loss before gain; deterministic lifecycle cleanup and stale-owner suppression | Proof is host-neutral runtime behavior, not drag/control policy | M4 complete |
| Touch/pen behavior | `unsupported` | Checked device identity and neutral touch/pen categories share the pointer stream protocol | No contact, pressure, tilt, twist, eraser, or host translation contract | M4 protocol; later host/control work |
| Text input and IME events | `proof` | Nonempty committed Unicode text and opaque generation-scoped composition start/update/end/cancel route to exact focused opt-in capability | No editable text, native IME object, selection, or text-layout contract | M4 complete; M8–M10 later |
| Accessibility/programmatic activation | `proof` | Exact-mounted programmatic/accessibility-stub/controller origins converge through `submit_command`; automation resolves exactly one authored ID before the same path | Exact semantic-node accessibility resolution/action ingress waits for M5C | M4 complete; M5C |
| Controller/gamepad input vocabulary | `unsupported` | None; keyboard vocabulary does not imply controller support | No normalized controller-facing command or device event model | M10 |
| Abstract UI navigation commands | `partial` | Existing commands plus `FocusNext`/`Previous`/four directions, exact request/restore, and canonical `LogicalFocusScroll`, with consistent origins | Logical scroll remains route-only; raw source mapping is later | M4 complete |
| Directional/spatial focus navigation | `proof` | Current publication geometry and mounted-order final tie-break satisfy DF-01–DF-20 through public submission | Private score is intentionally not API | M4 complete |
| Controller activation/cancel/menu commands | `proof` | Normalized controller source submits the same exact-target semantic commands without raw device vocabulary | Raw device mapping/identity/axes remain host/M10 scope | M4 complete; M10 |
| Input modality tracking | `proof` | Last accepted pointer/keyboard/controller/accessibility/automation/programmatic source is retained and traced without widget-event delivery | Raw controller translation and semantic accessibility resolution remain later | M4 complete; M5C/M10 later |
| Displayed-generation input targeting | `proof` | Runtime-issued `SurfaceInputContext`, exact current/historical logical/resolved command ingress, and pointer validation against retained generation/revision with no current-geometry fallback | One logical surface; no multi-window lifecycle | M4 complete; M10 later |
| Terminal pointer context cleanup | `proof` | Retired/missing active `Up` performs no ordinary route/re-hit/activation but clears pressed/path/capture, notifies a live capture owner, and closes; cancel treats unavailable geometry as diagnosis only | Foreign runtime/surface cannot mutate local streams | M4 complete |
| Route-only command defaults | `proof` | Cancel/menu/context each route once; eligible wheel derives exactly one logical-scroll command with no production offset mutation | Production scrolling mutation is later work | M4 complete |
| Directional focus corpus | `proof` | Every DF-01–DF-20 vector is individually identified in executable public-command proof using runtime-issued IDs and current retained publication rectangles | Private scoring remains deliberately non-public; multi-surface transfer remains absent | M4 complete |

## 5. Effects and scheduling

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Synchronous direct dispatch | `unsupported` | `AppRuntime::dispatch` and private dispatch authorities were removed | Callers must submit and explicitly pump | M4 complete |
| Application action and command submission | `proof` | `submit_action` returns exact action recovery; `submit_command` returns `CommandSubmission` or exact owned target/command/origin recovery with distinct foreign/stale/missing/terminal/capacity outcomes; authored-ID automation resolves uniquely before the same command path | Semantic-node action ingress/resolution remains M5C | M4 complete; M5C |
| Queue saturation | `proof` | Waiting-envelope, transaction output, live-family, completion, subscription-diagnostic, and trace limits; initial plans reserve aggregate allowance, while routed plans conservatively reserve the configured maximum-safe callback/output boundary before mutation | Exact callback-declared capacity is not an M4 API; the trace sink is subordinate and independently bounded | M4 complete |
| Action queue and ordering | `proof` | One generalized FIFO sequences actions, commands, pointer/input bundles, focus notifications, reconciliation, work, timers, and mapped results; notification/initiating/default output order is explicit | Native host translation remains absent | M4 complete |
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
| Publication alignment | `proof` | Warmed structural/common-field tests plus context-bearing publication prove aligned mounted IDs, metadata, style, layout, order, node counts, and fresh displayed-generation identity | Semantic IDs are deliberately no longer a singular renderer-product alignment field; M5B publishes them separately | M4/M5A complete; M5B/M6–M7 |
| Row/column layout | `proof` | Intrinsic main axis; constrained cross axis; gaps/padding | No stretch, flex, alignment, wrapping, or remaining-space distribution | M7 |
| Overflow diagnostics | `proof` | Runtime-node-aligned flags/report | No clipping or scrolling behavior | M7 |
| Width/height/min/max/fill/shrink | `unsupported` | None | Authored sizing model absent | M7 |
| Flex/grid | `unsupported` | None | Adopt-versus-build ADR required | M7 |
| Stack/absolute/overlay | `unsupported` | None | No overlay layout or stacking contract | M7 |
| Baseline layout | `unsupported` | Measurement response can carry baseline values | Layout does not consume them | M7–M8 |
| Clipping and scrolling | `unsupported` | None | No clips, extents, scroll state, input, or semantic product | M7–M9 |
| Incremental layout | `unsupported` | None | Clean and non-layout phases can reuse cached publication facts, but a dirty Layout phase still recomputes the whole surface; no node-granular incremental layout or damage propagation | M7, M11 |
| Virtualization | `deferred` | None | Requires mounted identity, scrolling, and advanced controls | M12 |

## 8. Semantics and accessibility

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Focusability facts | `proof` | Open widget activation facts drive runtime indexing for built-in and external controls | Runtime focus is not yet projected into an independent semantic product | M4 complete; M5B next |
| Semantic contribution | `supported` | `Widget::semantics(state, SemanticContributionContext) -> SemanticContribution`; 0..N owner-local nodes; `SemanticKey::PRIMARY`/named keys; strict mounted-child marker/local-reference validation; platform-neutral role/name/description/value/state/action/relationship/text vocabulary; `SemanticBounds::{Owner, OwnerLocal}`; downstream action-map/geometry conformance | Contribution is authoring/runtime input, not an independently published semantic tree; absolute coordinates and runtime focus are not widget authority | M5A complete; M5B next |
| Independent semantic identity | `proof` | Opaque `SemanticNodeId` issued from a separate runtime semantic arena and reconciled by exact mounted owner + `SemanticKey`; reorder retention, stale-safe removal/reuse, foreign/missing/capacity/index integrity proofs | IDs remain private runtime product state until M5B publishes the semantic snapshot/update API | M5A complete; M5B next |
| Semantic tree | `unsupported` | Accepted M5A contributions and private exact semantic lifetimes are prerequisites | No independent tree/forest publication, transparent-owner splice composition, absolute bounds/focus projection, relationship resolution, revisions, or incremental updates | M5B |
| Semantic actions | `unsupported` | Platform-neutral `SemanticAction` vocabulary exists in contribution | No public exact-`SemanticNodeId` semantic-action ingress/resolution into canonical commands yet | M5C |
| Accessibility queries/tests | `unsupported` | Genuine downstream M5A authoring conformance exists | No public semantic snapshot/query/action testing surface | M5D |
| AccessKit adapter | `planned` | Accepted adapter-only direction; M5A vocabulary has no AccessKit/native types | Depends on accepted M5B semantic product and M5C action resolution; no native bridge in M5 | M5B/M5E review; M10 bridge |
| Native accessibility bridge | `planned` | Required desktop profile | Depends on host/platform integration | M10 |
| Accessible text ranges | `planned` | Required production text contract | Depends on editable text and semantic mapping | M8 |

## 9. Surface, hit testing, and rendering

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Unified surface publication | `proof` | Context-bearing publication with mounted-authoritative frame/style/layout products and explicit renderer-product equality; `SurfaceNode::semantics()` temporarily carries canonical M5A contribution only | One logical surface; no neutral paint scene; no independent semantic product | M4/M5A complete; M5B/M6 |
| Logical bounds inspection | `proof` | `SurfaceNode` rectangles and debug renderer | Bounds are not a standalone layout result or M5B absolute semantic bounds product | M5B/M6–M7 |
| Rectangle hit testing | `proof` | Reverse frame order | No hit scene, stacking, clips, transforms, visibility, or pointer policy | M6 |
| Renderer-neutral paint scene | `unsupported` | M2 deterministic per-widget paint/debug proof facts | Facts are not paint primitives, resources, clips, transforms, order, or damage | M6 |
| Paint primitives/resources | `unsupported` | None | No shapes, strokes, glyph/image handles, clips, layers, or damage | M6 |
| Surface/frame generation | `proof` | Fresh runtime-issued coordinate revision and displayed hit-test generation on every public publication | One logical surface; not a paint/semantic-scene generation or multi-window lifecycle | M4 complete; M5B/M6/M10 later |
| Retained surface-input snapshots | `proof` | Configurable nonzero bounded immutable hit-test snapshots, exact historical targeting, oldest-first retirement, and retired/missing/foreign/revision outcomes | Retains hit-test facts only, not production layout/semantic/paint scenes | M4 complete; M5B/M6 later |
| Multi-surface publication | `unsupported` | None | No independent surface lifecycle or scale | M10 |
| Debug semantic-frame consumer | `proof` | `DebugSurfaceRenderer` deterministically formats temporary canonical M5A semantic contribution alongside paint/diagnostic widget facts | It is not the independent M5B semantic product, accessibility adapter, paint-scene consumer, or renderer backend | M5B–M6 |
| Deterministic paint-scene consumer | `planned` | None | Needs accepted paint/hit protocols | M6 |
| Conventional renderer backend | `unsupported` | None | Protocol must stabilize first | M10 |
| Embedded/SDF renderer consumer | `deferred` | None | Follows neutral protocol and conventional proof | M10 or M12 |

## 10. Text

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Static text descriptors | `proof` | Text/button widgets use the open protocol and canonical M5A semantic contribution | No production text shaping or complete control contract | M8–M9 |
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
| Text/label | `proof` | Static text element plus canonical M5A Text contribution | No production text or complete control/accessibility contract | M8–M9 |
| Button | `proof` | Label, enabled/actionable state, repeatable `on_activate` action factory, persistent local activation state and interaction slots; pointer, Enter, Space, programmatic, and authored automation paths converge through routed `Activate`; built-in Button authors canonical M5A Button role/name/disabled/action contribution | No independent semantic publication/action-resolution/accessibility adapter, recipes, or production control breadth | M4/M5A complete; M5B/M5C/M9 |
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
| Headless host profile | `partial` | Direct deterministic mounted runtime use with manual clock, injectable send executor, wake transport, typed host requests, accepted synthetic input ingress, accepted serialized offline replay, and canonical semantic contribution authoring | No independent semantic/public testing product or native host adapter | M4/M5A complete; M5B–M5D sequential |
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
| Workspace unit/integration tests | `supported` | Substantial deterministic proof suite plus a public-only downstream custom-widget package including M5A semantic authoring/mapping/geometry conformance | No unified M5D harness and Ubuntu-only CI | M5D; M11 |
| Strict formatting and linting | `supported` | Shared `cargo validate` runs stable rustfmt, locked tests, Clippy `-D warnings`, MSRV tests, and link checks locally and in CI | Current CI is Ubuntu-only; the production platform matrix remains later work | M0 |
| Style/layout/semantic diagnostics | `supported` | Mounted-aligned style/layout reports, runtime mismatch diagnostics, M5A contribution validation and semantic identity integrity handling, fresh surface generation/revision context, and debug output | No stable severity/strict mode or public semantic product diagnostics yet | M5B/M7 |
| Runtime trace | `partial` | One accepted bounded canonical M4D1-normalized graph, M4D2 deterministic JSONL v1 export/redaction/action-label/sink surface, and M4D3 inert offline replay foundation with replay-only identities and explicit dropped-prefix incompleteness | Export/sink/replay remain headless proof infrastructure rather than a production observability service or M5 semantic expectation engine | M4 complete; M5D later |
| Bounded canonical trace retention | `proof` | Configured capacity including zero, oldest-first eviction, non-wrapping `TraceSequence`, borrowed iteration, exclusive dropped-before watermark, normalized-schema proof, deterministic v1 projection, and accepted offline causal replay consumption | Replay validates the serialized retained causal projection; it does not create live runtime authority | M4 complete |
| Bounded external trace sink | `proof` | One-time public receiver; lazy atomic logical capacity; immutable canonical-record handoff; consumer-side JSON encoding; structured `Delivered`/`Full`/first `Closed` outcomes; shutdown closure; four-state isolation proof | Subordinate headless diagnostic transport only; no arbitrary callback/work capability or replay authority | M4 complete |
| Public headless test harness | `planned` | Current tests, accepted replay foundation, and M5A semantic contribution/identity prove prerequisites | No `runenui_testing` public boundary | M5D |
| Semantic/layout/hit/paint assertions | `planned` | Layout/frame internals and M5A contribution are inspectable through current proof surfaces | No independent public semantic snapshot/query/update assertion layer or unified public assertions | M5D; M6 |
| Deterministic time/tasks | `proof` | Manual monotonic clock, wake-aware local tasks, injectable send executor | Unified M5D harness absent | M4 complete; M5D |
| Snapshot/golden/replay tests | `partial` | Accepted deterministic JSONL snapshots are byte-stable; M4D3 additionally round-trips real exported JSONL, diagnoses dropped-prefix incompleteness, and reconstructs Counter causality after the live runtime is gone | The unified public M5D testing harness and semantic expectation layer are absent | M4 complete; M5D |
| Property/fuzz testing | `unsupported` | None | Production hardening work | M11 |
| Benchmarks and budgets | `unsupported` | None | No performance gates | M11 |
| Cross-platform CI | `unsupported` | Ubuntu-only CI | Windows/macOS jobs absent | M11 |
| Controller-only application operation | `unsupported` | None | No normalized commands, applicable control conformance, or game-oriented reference proof | M11 |

## 14. Source formats and devtools

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Typed Rust as source authority | `supported` | Builders, public `View`/`Widget`, ordinary components, canonical semantic contribution, and thin `element!`/`children!` | External source formats remain deferred | M2/M5A complete |
| External UI source format | `deferred` | None | Requires stable semantic authoring and diagnostics | M12 |
| Inspector/devtools | `deferred` | Debug render/report functions only | No public semantic/scene observation model | M12 |
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
