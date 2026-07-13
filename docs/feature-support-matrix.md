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

## 1. Authoring and composition

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Transient UI descriptions | `supported` | Open owned `Element<Action>` trees consumed by reconciliation | Descriptions never own or expose mounted state/identity | M3 complete |
| Builder authoring | `supported` | Separate typed built-in views; downstream leaves use `Element::new`; all child-layout widgets use `Container<Action>` | Built-ins remain proof-level controls | M9 |
| `element!` authoring | `supported` | One ordinary builder/view expression lowered through `View` | Thin convenience only; no property DSL | M2 complete |
| Composite function components | `supported` | Ordinary Rust functions return typed views/elements | Components are not mounted state owners | M2 complete |
| Component action mapping | `supported` | Recursive `Element::map_action(ChildAction -> ParentAction)` | Stored mapping closure is operation-local `'static`; no string/`Any` action conversion | M2 complete |
| External custom widgets | `supported` | State-aware public widgets, unstable checked bridge, mounted downstream conformance | Production event/semantic/paint/layout contracts remain later | M3 complete; M4–M8 |
| Child-layout authoring | `supported` | Canonical `Container<Action>`/`container`, `ChildLayout::Linear`, arbitrary children, container-only gaps | M2 proof policy only; M7 owns production custom layout | M2 complete; M7 |
| Typed control-specific builders | `supported` | Kind-specific builders; shared identity/style only where behavior is shared | Broader control vocabulary waits for M2/M9 | M2, M9 |
| Arbitrary child counts | `supported` | Iterator/collection `Views` plus arity-free heterogeneous `children!` | None within the current transient protocol | M2 complete |

## 2. Application model

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Application-owned state | `supported` | `UiApp::State`, Counter, mounted reconciliation | Application dispatch still rebuilds a transient description; no queue/effects | M4 |
| Typed application actions | `supported` | `UiApp::Action`; typed widget actions; recursive component mapping; non-`Clone` activation/direct-dispatch proof | Current activation consumes the current transient action slot, then rebuilds the transient root and reconciles it into the mounted tree | M4 |
| Explicit update | `supported` | `UiApp::update(&mut State, Action)` | Synchronous only; no effect result | M4 |
| Conditional root composition | `supported` | Counter/win root replacement with deterministic unmount/remount | One mounted root | M3 complete |
| Batched/reentrant action processing | `unsupported` | None | No queue or ordering contract | M4 |
| Fine-grained signals as primary model | `deferred` | None by design | Signals may only be future adapters | Post-M3 |

## 3. Runtime identity and lifecycle

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Mounted runtime indexing | `supported` | `MountedNodeId`/`SemanticNodeId`, logical-preorder mounted index | Runtime-local, non-serialized identity | M3 complete |
| Authored element IDs | `supported` | Validated lookup/diagnostic metadata; changes preserve mounted lifetime | Not mounted identity | M3 complete |
| Stored element keys | `supported` | Unique sibling keys reconcile; duplicates preserve no state | Keys are sibling-local | M3 complete |
| Persistent generational IDs | `supported` | Safe private arena, deterministic reuse, retirement at overflow | Not serialized or cross-runtime | M3 complete |
| Keyed reconciliation | `supported` | Transactional compatible update, reorder retention, unkeyed ordinal matching, cross-parent remount, structured duplicate diagnostics | Stable reorderable collections require keys | M3 complete |
| Mount/update/unmount lifecycle | `supported` | Deterministic preorder/postorder, arena-live hooks, state drop after removal, idempotent shutdown | Callbacks must not panic | M3 complete |
| Runtime-local widget state | `supported` | Integrity-aware checked capabilities; persistent state and private interaction slots | Broader control state waits for later milestones | M3 complete |
| Focus retention | `supported` | Compatible/keyed updates retain focus; removal/replacement/disable and state-only loss clear it immediately | One focus domain; no scopes/directional navigation | M3 complete; M4 queued |
| Granular invalidation | `supported` | Explicit phase functions, exact context key, topology-only whole-surface cache, current mounted common-field reads, independently verified `SurfacePhaseReport` | Whole-surface structural rebuilds remain conservative; production incremental layout is later work | M3 complete; M7/M11 |

## 4. Events and interaction

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Typed pointer vocabulary | `partial` | Position, phase, button, modifiers, optional target | No pointer/device ID, pressure, tilt, hover transitions, wheel, or capture | M4 |
| Typed keyboard vocabulary | `partial` | Key, phase, modifiers, Tab traversal | Logical/physical keys, repeat, location, commands, and shortcuts incomplete | M4 |
| Pointer hit targeting | `proof` | Frame rectangle targeting returns generation-safe `MountedNodeId` values | No explicit hit scene, stacking, clips, transforms, visibility, or pointer policy | M4, M6 |
| Pointer activation | `proof` | Primary press dispatches the targeted actionable widget action | Incorrect production default: no capture/release-inside/cancellation | M4 |
| Keyboard activation | `proof` | Focused actionable widget responds to Enter/Space | No shared semantic command pipeline | M4–M5 |
| Focus traversal | `proof` | Persistent mounted first/last/next/previous traversal | No scopes, directional navigation, modality, or semantic focus model | M4–M5 |
| Event capture/target/bubble | `unsupported` | None | No propagation or default-action control | M4 |
| Pointer capture | `unsupported` | None | Cannot implement correct buttons, drag, sliders, or scrolling | M4 |
| Touch/pen behavior | `unsupported` | Generic pointer vocabulary only | No device identity or device-specific facts | M4 |
| Text input and IME events | `unsupported` | Character key variant is not text input | No composition/commit/range stream | M4, M8 |
| Accessibility/programmatic activation | `unsupported` | Direct test activation exists only as a runtime helper | No semantic action convergence | M5 |
| Controller/gamepad input vocabulary | `unsupported` | None; keyboard vocabulary does not imply controller support | No normalized controller-facing command or device event model | M4, M10 |
| Abstract UI navigation commands | `unsupported` | None | No device-independent next/previous/directional/activate/cancel/menu/context commands | M4 |
| Directional/spatial focus navigation | `unsupported` | None | Current traversal supports only linear next/previous keyboard focus | M4 |
| Controller activation/cancel/menu commands | `unsupported` | None | Controller input cannot converge on semantic control commands | M4 |
| Input modality tracking | `unsupported` | None | Runtime does not track pointer, keyboard, controller, accessibility, or automation modality | M4 |

## 5. Effects and scheduling

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Synchronous dispatch | `supported` | `AppRuntime::dispatch` | Immediate update, transient-root rebuild, and mounted reconciliation only; no queue or effect result | M4 |
| Action queue and ordering | `unsupported` | None | No batching/reentrancy rules | M4 |
| Effects | `unsupported` | Target direction only | No executable contract | M4 |
| Async tasks | `unsupported` | None | No executor, completion mapping, or lifecycle ownership | M4 |
| Timers and subscriptions | `unsupported` | None | No deterministic time or cancellation | M4 |
| Host commands | `unsupported` | None | No host/effect boundary | M4, M10 |
| Wake/redraw scheduling | `unsupported` | None | Explicit publication only | M4 |
| Deterministic scheduler testing | `unsupported` | None | No clock/task executor | M4–M5 |

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
| Finite/unbounded constraints | `supported` | Validated `LogicalLength`, `LogicalSize`, finite points, normalized `LayoutConstraints`, checked baselines | Broader sizing vocabulary remains absent | M7 |
| Renderer-neutral measurement seam | `supported` | Borrowed `MeasurementProvider` with cache identity/revision | Text-only synchronous contract; no resource or typography input | M8 |
| Deterministic headless measurement | `proof` | Unicode-scalar count with fixed metrics | Not production text geometry | M8 |
| One measurement capability snapshot per node/publication | `proof` | Counter-backed downstream tests prove one query reused by measurement and arrangement | Capability facts are retained, but a dirty Layout phase remains whole-surface rather than node-granular production incremental layout | M7, M11 |
| One child-layout snapshot per child-bearing node/publication | `proof` | Counter-backed external alternating-axis proof | Only linear M2 policy exists | M7 |
| Unsupported measurement handling | `proof` | Explicit unsupported and cross-version-unrecognized layout diagnostics | Zero fallback geometry is proof-level only | M7 |
| Publication alignment | `proof` | Warmed structural and compatible common-field tests prove aligned current IDs, metadata, style, layout, order, and node counts | No per-surface publication generation or production retained layout | M6–M7 |
| Row/column layout | `proof` | Intrinsic main axis; constrained cross axis; gaps/padding | No stretch, flex, alignment, wrapping, or remaining-space distribution | M7 |
| Overflow diagnostics | `proof` | Runtime-node-aligned flags/report | No clipping or scrolling behavior | M7 |
| Width/height/min/max/fill/shrink | `unsupported` | None | Authored sizing model absent | M7 |
| Flex/grid | `unsupported` | None | Adopt-versus-build ADR required | M7 |
| Stack/absolute/overlay | `unsupported` | None | No overlay layout or stacking contract | M7 |
| Baseline layout | `unsupported` | Measurement response can carry baseline values | Layout does not consume them | M7–M8 |
| Clipping and scrolling | `unsupported` | None | No clips, extents, scroll state, input, or semantics | M7–M9 |
| Incremental layout | `unsupported` | None | Clean and non-layout phases can reuse cached publication facts, but a dirty Layout phase still recomputes the whole surface; no node-granular incremental layout or damage propagation | M7, M11 |
| Virtualization | `deferred` | None | Requires mounted identity, scrolling, and advanced controls | M12 |

## 8. Semantics and accessibility

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Focusability facts | `proof` | Open widget activation facts drive runtime indexing for built-in and external controls | Not the M5 semantic focus model | M5 |
| Semantic tree | `unsupported` | Per-widget role/name/enabled/action-intent proof facts plus mounted-lifetime `SemanticNodeId` values | No production semantic tree, relationships, values, semantic actions, or accessibility contract | M5 |
| Semantic actions | `unsupported` | None | No shared activation/action path | M5 |
| Accessibility queries/tests | `unsupported` | None | No public semantic test surface | M5 |
| AccessKit adapter | `planned` | Accepted desktop direction | Depends on semantic tree and mounted IDs | M5 |
| Native accessibility bridge | `planned` | Required desktop profile | Depends on host/platform integration | M10 |
| Accessible text ranges | `planned` | Required production text contract | Depends on editable text and semantic mapping | M8 |

## 9. Surface, hit testing, and rendering

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Unified surface publication | `proof` | Mounted-authoritative read-only frame/style/layout products | No per-surface generation or neutral paint scene | M6 |
| Logical bounds inspection | `proof` | `SurfaceNode` rectangles and debug renderer | Bounds are not a standalone layout result | M6–M7 |
| Rectangle hit testing | `proof` | Reverse frame order | No hit scene, stacking, clips, transforms, visibility, or pointer policy | M6 |
| Renderer-neutral paint scene | `unsupported` | M2 deterministic per-widget paint/debug proof facts | Facts are not paint primitives, resources, clips, transforms, order, or damage | M6 |
| Paint primitives/resources | `unsupported` | None | No shapes, strokes, glyph/image handles, clips, layers, or damage | M6 |
| Surface/frame generation | `unsupported` | Reconciliation generation and mounted stale-target validation exist | No independent surface/scene publication generation | M6 |
| Multi-surface publication | `unsupported` | None | No independent surface lifecycle or scale | M10 |
| Debug semantic-frame consumer | `proof` | `DebugSurfaceRenderer` deterministically formats open paint/semantic/diagnostic widget facts | It is not a paint-scene consumer, accessibility product, or renderer backend | M5–M6 |
| Deterministic paint-scene consumer | `planned` | None | Needs accepted paint/hit protocols | M6 |
| Conventional renderer backend | `unsupported` | None | Protocol must stabilize first | M10 |
| Embedded/SDF renderer consumer | `deferred` | None | Follows neutral protocol and conventional proof | M10 or M12 |

## 10. Text

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Static text descriptors | `proof` | Text/button widgets use the open protocol | No production text shaping or control contract | M8–M9 |
| Headless deterministic metrics | `proof` | Fixed scalar width and line height | Not shaping, grapheme measurement, or font metrics | M8 |
| Font discovery/fallback | `unsupported` | None | No font/resource provider | M8 |
| Shaping and multilingual scripts | `unsupported` | None | No production text stack | M8 |
| Bidi/RTL | `unsupported` | None | No paragraph model | M8 |
| Line breaking/wrapping | `unsupported` | None | Layout does not negotiate paragraphs | M8 |
| Emoji/combining marks | `unsupported` | Scalar count only | Geometry is not grapheme/glyph aware | M8 |
| Baselines | `partial` | Optional measurement fields | Not consumed or published in layout | M8 |
| Editable text | `unsupported` | None | No editing state, event, semantics, or text layout | M8–M9 |
| Selection/caret/clipboard/IME | `unsupported` | None | Required contracts absent | M8–M10 |

## 11. Controls

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Text/label | `proof` | Static text element | No production text, semantics, or control contract | M8–M9 |
| Button | `proof` | Label, enabled/actionable state, persistent local activation state and interaction slots | No routed press/capture/release behavior, production semantics, recipes, or accessibility | M4–M9 |
| Checkbox/radio/toggle | `unsupported` | None | Standard control foundation absent | M9 |
| Slider/progress | `unsupported` | None | Events, semantics, and layout prerequisites absent | M9 |
| Text field | `unsupported` | None | Production text/editing prerequisites absent | M8–M9 |
| Scroll container/list | `unsupported` | None | Mounted scrolling and production layout absent | M7–M9 |
| Menu/popover/tooltip/dialog | `unsupported` | None | Overlay, focus-scope, event, and scene prerequisites absent | M7–M9 |
| Tabs | `unsupported` | None | Standard control contract absent | M9 |
| Tree/data grid/editor controls | `deferred` | None | Advanced application systems | M12 |

## 12. Host and platform integration

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Host-neutral core/runtime | `supported` | No native window, GPU, ECS, or legacy dependencies | Neutrality alone is not an embedding contract | M10 |
| Host contract | `unsupported` | Input/measurement types are isolated vocabulary | No lifecycle, services, capability, wakeup, or resource contract | M10 |
| Headless host profile | `partial` | Direct deterministic mounted runtime use | No public semantic harness, deterministic clock/tasks, or host contract | M4–M5 |
| Desktop event loop/window | `unsupported` | None | No Winit or equivalent adapter | M10 |
| Windows/macOS/Linux support | `unsupported` | Platform-independent Rust tests only | No native application proof | M10–M11 |
| DPI and resize | `unsupported` | Logical geometry only | No scale/surface lifecycle | M10 |
| Clipboard/cursor/drag-drop/file dialogs | `unsupported` | None | Host services absent | M10 |
| IME | `unsupported` | None | Event/text/platform contracts absent | M8–M10 |
| Multi-window | `unsupported` | None | Runtime surface ownership absent | M10 |
| Embedded external-host proof | `unsupported` | None | Host and scene protocols absent | M10 |
| Controller connection/disconnection | `unsupported` | None | No host device lifecycle or stable controller identity | M10 |
| Axis normalization and dead-zone policy | `unsupported` | None | No host-owned raw axis translation or reviewed normalization policy | M10 |
| Embedded-host controller mapping | `unsupported` | None | No contract mapping host devices to normalized UI commands | M10 |

## 13. Testing and diagnostics

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Workspace unit/integration tests | `supported` | Substantial deterministic proof suite plus a public-only downstream custom-widget package | No unified M5 harness and Ubuntu-only CI | M5, M11 |
| Strict formatting and linting | `supported` | Shared `cargo validate` runs stable rustfmt, locked tests, Clippy `-D warnings`, MSRV tests, and link checks locally and in CI | Current CI is Ubuntu-only; the production platform matrix remains later work | M0 |
| Style/layout diagnostics | `supported` | Mounted-aligned reports, runtime mismatch diagnostics, and debug output | No stable severity/strict mode or per-surface generation | M5–M7 |
| Runtime trace | `partial` | Coarse event records | Duplicate unbounded storage; no structured export/replay | M4–M5 |
| Public headless test harness | `planned` | Current tests prove demand | No `runenui_testing` public boundary | M5 |
| Semantic/layout/hit/paint assertions | `planned` | Layout/frame internals are inspectable | No unified public assertions | M5–M6 |
| Deterministic time/tasks | `unsupported` | None | Scheduler absent | M4–M5 |
| Snapshot/golden/replay tests | `unsupported` | Debug string assertions only | Scene and replay protocols absent | M5–M6 |
| Property/fuzz testing | `unsupported` | None | Production hardening work | M11 |
| Benchmarks and budgets | `unsupported` | None | No performance gates | M11 |
| Cross-platform CI | `unsupported` | Ubuntu-only CI | Windows/macOS jobs absent | M11 |
| Controller-only application operation | `unsupported` | None | No normalized commands, applicable control conformance, or game-oriented reference proof | M11 |

## 14. Source formats and devtools

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Typed Rust as source authority | `supported` | Builders, public `View`/`Widget`, ordinary components, and thin `element!`/`children!` | External source formats remain deferred | M2 complete |
| External UI source format | `deferred` | None | Requires stable semantic authoring and diagnostics | M12 |
| Inspector/devtools | `deferred` | Debug render/report functions only | No public mounted/semantic/scene observation model | M12 |
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

## 16. Repository authority and history

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Legacy archival and recovery | `supported` | Annotated tag `legacy-runenwerk-ui-archive-2026-07-11` preserves audited baseline `141f005`; `legacy/` is absent from active content and context profiles | Historical material is opt-in reference only and cannot be treated as current implementation | M0 |
