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
| Immutable UI descriptions | `supported` | `Element<Action>` trees derived from application state | Current descriptor model is closed and transient | M2–M3 |
| Builder authoring | `supported` | Typed `Text`, `Button<Action>`, and `Container<Action>` builders with `IntoElement` erasure | Built-in element vocabulary remains closed | M2 |
| `element!` authoring | `proof` | One ordinary builder expression lowered through `IntoElement` | No general component expression | M2 |
| Composite function components | `partial` | Rust functions can return `Element<Action>` | Child action type must already be the parent action type | M2 |
| Component action mapping | `unsupported` | None | No `map`-equivalent contract | M2 |
| External custom widgets | `unsupported` | None | `ElementKind` is closed to core-defined variants | M2 |
| Typed control-specific builders | `supported` | Kind-specific builders; shared identity/style only where behavior is shared | Broader control vocabulary waits for M2/M9 | M2, M9 |
| Arbitrary child counts | `supported` | Iterator/collection `IntoElements` plus arity-free heterogeneous `children!` | None within the current built-in proof vocabulary | M2 |

## 2. Application model

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Application-owned state | `supported` | `UiApp::State` and Counter | No granular runtime invalidation | M3–M4 |
| Typed application actions | `supported` | `UiApp::Action`; typed button actions; non-`Clone` mount/direct-dispatch proof | Activation alone clones the action retained by the immutable authored tree | M2–M4 |
| Explicit update | `supported` | `UiApp::update(&mut State, Action)` | Synchronous only; no effect result | M4 |
| Conditional root composition | `supported` | Counter/win screen switch | Full transient rebuild clears focus | M3 |
| Batched/reentrant action processing | `unsupported` | None | No queue or ordering contract | M4 |
| Fine-grained signals as primary model | `deferred` | None by design | Signals may only be future adapters | Post-M3 |

## 3. Runtime identity and lifecycle

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Per-build runtime indexing | `proof` | Preorder `RuntimeNodeId`/`RuntimeTreeIndex` | ID can identify a different node after rebuild | M3 |
| Authored element IDs | `partial` | Unicode-validated textual IDs, invalid-authoring diagnostics, true-preorder duplicate paths, ambiguity-safe activation | IDs remain transient handles rather than persistent identity | M3 |
| Stored element keys | `proof` | Unicode-validated textual keys with true-preorder sibling-duplicate diagnostics | Keys do not participate in reconciliation | M3 |
| Persistent generational IDs | `unsupported` | None | No mounted arena or generation validation | M3 |
| Keyed reconciliation | `unsupported` | None | Full root replacement after dispatch | M3 |
| Mount/update/unmount lifecycle | `unsupported` | None | No mounted widget protocol | M2–M3 |
| Runtime-local widget state | `unsupported` | None | Hover, pressed, scroll, edit, and animation state have no owner | M3 |
| Focus retention | `unsupported` | None | Dispatch clears focus | M3 |
| Granular invalidation | `unsupported` | None | Publication recomputes transient products | M3–M4 |

## 4. Events and interaction

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Typed pointer vocabulary | `partial` | Position, phase, button, modifiers, optional target | No pointer/device ID, pressure, tilt, hover transitions, wheel, or capture | M4 |
| Typed keyboard vocabulary | `partial` | Key, phase, modifiers, Tab traversal | Logical/physical keys, repeat, location, commands, and shortcuts incomplete | M4 |
| Pointer hit targeting | `proof` | Frame rectangle targeting | Transient IDs and simplistic hit order | M4, M6 |
| Pointer button activation | `proof` | Primary press dispatches button action | Incorrect production default: no capture/release-inside/cancellation | M4 |
| Keyboard button activation | `proof` | Focused Enter/Space behavior | No shared semantic command pipeline | M4–M5 |
| Focus traversal | `proof` | First/last/next/previous enabled buttons | Hardcoded built-in policy, no scopes or persistence | M3–M4 |
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
| Synchronous dispatch | `supported` | `AppRuntime::dispatch` | Immediate update and full rebuild only | M4 |
| Action queue and ordering | `unsupported` | None | No batching/reentrancy rules | M4 |
| Effects | `unsupported` | Target direction only | No executable contract | M4 |
| Async tasks | `unsupported` | None | No executor, completion mapping, or lifecycle ownership | M4 |
| Timers and subscriptions | `unsupported` | None | No deterministic time or cancellation | M4 |
| Host commands | `unsupported` | None | No host/effect boundary | M4, M10 |
| Wake/redraw scheduling | `unsupported` | None | Explicit publication only | M3–M4 |
| Deterministic scheduler testing | `unsupported` | None | No clock/task executor | M4–M5 |

## 6. Styling

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Literal color/padding/radius | `supported` | `StyleIntent` and `ComputedStyle` | Very small property surface | M7 |
| Typed token references | `supported` | Unicode-validated text identity, color/spacing/radius families, mixed static/dynamic lookup, and non-overwriting definitions | Theme loading/fallback remain absent | M7 |
| Token resolution | `supported` | `StyleTokens` and pure resolver | In-memory values only; no fallback or theme loading | M7 |
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
| Renderer-neutral measurement seam | `supported` | Borrowed `MeasurementProvider` | Text-only synchronous contract; no resource or typography input | M8 |
| Deterministic headless measurement | `proof` | Unicode-scalar count with fixed metrics | Not production text geometry | M8 |
| One measurement per node/publication | `proof` | Publication-local measured result and tests | No retained cache or invalidation | M7, M11 |
| Row/column layout | `proof` | Intrinsic main axis; constrained cross axis; gaps/padding | No stretch, flex, alignment, wrapping, or remaining-space distribution | M7 |
| Overflow diagnostics | `proof` | Runtime-node-aligned flags/report | No clipping or scrolling behavior | M7 |
| Width/height/min/max/fill/shrink | `unsupported` | None | Authored sizing model absent | M7 |
| Flex/grid | `unsupported` | None | Adopt-versus-build ADR required | M7 |
| Stack/absolute/overlay | `unsupported` | None | No overlay layout or stacking contract | M7 |
| Baseline layout | `unsupported` | Measurement response can carry baseline values | Layout does not consume them | M7–M8 |
| Clipping and scrolling | `unsupported` | None | No clips, extents, scroll state, input, or semantics | M7–M9 |
| Incremental layout | `unsupported` | None | Entire publication is recomputed | M7, M11 |
| Virtualization | `deferred` | None | Requires mounted identity, scrolling, and advanced controls | M12 |

## 8. Semantics and accessibility

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Focusability facts | `proof` | Enabled built-in buttons are matched in runtime index | Not a semantic model and not extensible | M5 |
| Semantic tree | `unsupported` | None | No roles, names, state, values, relationships, or stable semantic IDs | M5 |
| Semantic actions | `unsupported` | None | No shared activation/action path | M5 |
| Accessibility queries/tests | `unsupported` | None | No public semantic test surface | M5 |
| AccessKit adapter | `planned` | Accepted desktop direction | Depends on semantic tree and mounted IDs | M5 |
| Native accessibility bridge | `planned` | Required desktop profile | Depends on host/platform integration | M10 |
| Accessible text ranges | `planned` | Required production text contract | Depends on editable text and semantic mapping | M8 |

## 9. Surface, hit testing, and rendering

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Unified surface publication | `proof` | Read-only frame/style/layout products from one preparation pass | Transient identity; no generation or neutral paint scene | M3, M6 |
| Logical bounds inspection | `proof` | `SurfaceNode` rectangles and debug renderer | Bounds are not a standalone layout result | M6–M7 |
| Rectangle hit testing | `proof` | Reverse frame order | No hit scene, stacking, clips, transforms, visibility, or pointer policy | M6 |
| Renderer-neutral paint scene | `unsupported` | None | Semantic `Container`/`Text`/`Button` are not paint primitives | M6 |
| Paint primitives/resources | `unsupported` | None | No shapes, strokes, glyph/image handles, clips, layers, or damage | M6 |
| Surface/frame generation | `unsupported` | None | Stale targets cannot be validated | M3, M6 |
| Multi-surface publication | `unsupported` | None | No independent surface lifecycle or scale | M10 |
| Debug semantic-frame consumer | `proof` | `DebugSurfaceRenderer` deterministically formats current semantic frame nodes | It is not a paint-scene consumer or renderer backend | M6 |
| Deterministic paint-scene consumer | `planned` | None | Needs accepted paint/hit protocols | M6 |
| Conventional renderer backend | `unsupported` | None | Protocol must stabilize first | M10 |
| Embedded/SDF renderer consumer | `deferred` | None | Follows neutral protocol and conventional proof | M10 or M12 |

## 10. Text

| Capability | Current support | Current proof or API | Known limitation | Target milestone |
|---|---|---|---|---|
| Static text descriptors | `proof` | Text elements and button labels | Closed primitive model | M2, M9 |
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
| Button | `proof` | Label, enabled state, typed press action | No mounted pressed state, release activation, semantics, recipes, or accessibility | M3–M9 |
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
| Headless host profile | `partial` | Direct deterministic runtime use | No mounted runtime, synthetic semantic harness, clock, or tasks | M3–M5 |
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
| Workspace unit/integration tests | `supported` | Substantial deterministic proof suite | Mostly private/proof APIs and Ubuntu CI | M5, M11 |
| Strict formatting and linting | `supported` | Shared `cargo validate` runs stable rustfmt, locked tests, Clippy `-D warnings`, MSRV tests, and link checks locally and in CI | Current CI is Ubuntu-only; the production platform matrix remains later work | M0 |
| Style/layout diagnostics | `supported` | Aligned reports and debug output | No stable codes, severity, strict mode, or generation | M3–M5 |
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
| Typed Rust as source authority | `supported` | Builders and `element!` | Extensibility and macro scaling incomplete | M1–M2 |
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
