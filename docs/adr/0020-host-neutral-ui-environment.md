# ADR 0020: Host-neutral UI environment and coherent surface reconciliation

> **Category:** ADR
>
> **Status:** Accepted target architecture upon owner-reviewed squash merge; **not implemented**
>
> **Milestone:** M13
>
> **Decision authority:** [Investigation #299](https://github.com/dornglut/runen-ui/issues/299); [ADR delivery #441](https://github.com/dornglut/runen-ui/issues/441)
>
> **Inherits:** [ADR 0009](0009-production-style-layout-text-foundation.md), [ADR 0010](0010-visual-composition-and-animation.md), [ADR 0018](0018-editable-text-and-interaction-services.md), [ADR 0019](0019-event-correlated-host-input-arbitration.md)

## Context and decision

Today's `UiApp::root(&State)` builds transient authoring before a caller provides `SurfaceBuildContext` during later surface publication. That later context supplies `StyleEnvironment`, root constraints and `RasterScale`. M8 inherits foreground/typography and applies explicit high-contrast/reduced-motion preferences, but lacks inherited locale, UI direction, user text/UI scale and pointing affordances. `runenui_text::TextLanguage` is only an explicit language/script/region shaping input, not application localization. M11 already retains the canonical **causal** focus-visible latch but no user always-show preference.

Adding locale solely to late surface publication would permit an old-language authored label with new-language text metrics/semantics. Scattered native getters, global environment state and paint-time RTL transforms would fragment runtime reconciliation and publication.

**Select one closed, typed host-neutral UI environment family**, with one canonical immutable resolved environment and nonwrapping revision per exact logical surface generation under `runenui_runtime`. Validated host observations are inputs, application policy chooses localizable content and theme, runtime owns ordered admission, effective inherited node snapshots, reconciliation, invalidation, trace, staged atomic publication and shutdown. Root/subtree construction sees the same effective revision before it creates language-dependent content. The renderer receives only already resolved logical products and disposable rasterization instructions.

This is **target architecture, not today's Rust API**. No source implementation, native support or executable conformance is implied.

## Fact set and authoritative owners

The *complete initial M13 neutral fact set* is:

| Fact | Neutral representation | Meaning/owner |
| --- | --- | --- |
| User languages | Ordered nonempty full BCP 47 `LocaleTag` list | Host/user observation or explicit headless injection; no invented translations |
| Selected content locale | One full `LocaleTag` and deterministic selection/fallback outcome | **Application** policy chooses supported content locale |
| UI direction | `LayoutDirection::{LeftToRight,RightToLeft}` | App-supplied surface default, optionally inherited/overridden within a subtree; distinct from Unicode bidi |
| User text scaling | `TextScale::{Linear,Curve}` validated font-size mapping | User/host fact; downstream subtree cannot weaken |
| UI metric scaling | Positive finite `UiMetricScale` for explicitly opted-in control/chrome metrics | Distinct from user text and renderer DPI; not a whole-scene transform |
| Appearance preference | `Appearance::{Light,Dark,Unspecified}` | Observed fact and app policy; not a framework-chosen theme |
| Protected preferences | `AccessibilityPreferences { high_contrast, reduced_motion, always_show_focus }` | User requirement; true cannot be negated by app/subtree |
| Pointing affordances | `InputAffordances { fine_pointer, coarse_pointer, hover }`, each `Availability::{Yes,No,Unknown}` | Per-surface coarse abilities, potentially simultaneous; not active input or OS permissions |

Deterministic headless baseline: languages/selected locale `und`, LTR, identity scales, appearance `Unspecified`, user flags false, all pointing abilities `Unknown`. These defaults MUST NOT be misrepresented as current native observations.

`runenui_core` owns validated immutable fact/authoring types; `runenui_text` owns explicit shaping projection and retained artifacts; `runenui_runtime` alone owns live environment revision, mounted ancestry, derived style/layout/semantic work and publication; native host/platform adapter owns OS queries, change notifications, window mapping and observed freshness; application owns localized strings, supported locales, theme choice and adaptive layout intent. `runenui_testing` reads ordinary public API only. No new crate, runtime translation catalog, mutable dictionary of arbitrary environment keys, ambient OS read, renderer-owned UI policy or parallel reactive/layout engine.

Explicitly **exclude** platform/device name categories, inferred current keyboard/pointer modality, connected controller state, raw gameplay deltas, native pointer-mode support/request/lease, game bindings and generic window manager. [#307](https://github.com/dornglut/runen-ui/issues/307) and [#424](https://github.com/dornglut/runen-ui/issues/424) own host-private pointer modes; [ADR 0019](0019-event-correlated-host-input-arbitration.md) and #428 own event-correlated UI input settlement; [#289](https://github.com/dornglut/runen-ui/issues/289) owns any later size/container-query conditions. Input affordance ≠ permission, modality or gameplay eligibility.

## BCP 47 preservation, selection and shaping

`LocaleTag` accepts complete **well-formed RFC 5646** tags: language, extlang/script/region, variants, extensions, grandfathered forms and private-use. For the bounded initial 0.1 representation, set an explicit maximum of **255 ASCII bytes per complete tag** and **16 preferred language tags per surface**. RFC 5646 has no intrinsic maximum tag length; these are RunenUI resource limits, **not BCP 47 syntax claims**. Validation MUST reject malformed grammar, duplicate extension singletons, empty/over-limit inputs and case-insensitive duplicate preference entries with typed errors; the host MUST NOT silently truncate, discard lower-priority entries or claim a complete preference observation after rejection. The previous coherent observation (or explicit first-mount baseline) remains in force. Representation retains every accepted subtag (not original casing), with case-insensitive matching. Stable class-based case normalization is permitted, but **registry-dependent Preferred-Value canonicalization requires a separately explicit versioned data policy** and may not silently rewrite/lossily remove tags. A well-formed tag need not denote an installed translation, an active font or supported native service.

The application receives user language priority and resolves one **supported** content locale by an explicit deterministic application policy, optionally using RFC 4647 Lookup. The application supplies its own fallback locale when no supported match exists; runtime does not choose English or invent a message catalog. Retain the original full user preferences and selected full tag independently. Application-localized labels/keys are rebuilt through normal environment-triggered view reconciliation; merely changing shaping metadata never translates a string.

Current `TextLanguage::new` intentionally permits only `language[-Script][-REGION]` via Parley and rejects trailing variants, extensions or private-use. **Do not weaken this guard or pretend the prefix is the selected locale**. Select a separate *explicit fallible* `LocaleTag -> ShapingLanguageProjection` operation: it extracts a semantically supported language/script/region prefix, retains original full-tag identity/provenance elsewhere and reports when trailing subtags are intentionally **not interpreted by the shaping backend**. If a meaningful prefix cannot be represented (e.g. private-use-only/grandfathered without recognized language), result is `Unavailable`; explicit paragraph language or documented text-engine neutral/default behavior applies, with typed diagnostics. This is not a global fallback to another localized content language. A full semantic language tag MUST be independently publishable where the neutral semantic contract requires one; no AccessKit or renderer type enters core.

Per text segment, shaping-language precedence is: explicit metric-span language > explicit paragraph language > inherited content-language override > projected selected locale > text-engine neutral default. Text cache compatibility includes final resolved shaping language, source revision/font identity, spans and metric typography. The selected full `LocaleTag` remains available for application/localization and semantic metadata even when the shaping backend ignores extensions. Mixed `ar-EG` chrome, `en-US` code subtree and Arabic/Latin spans prove locale, UI direction and bidi cannot be one flag. [#340](https://github.com/dornglut/runen-ui/issues/340) owns localized mnemonic binding/matching rather than a runtime translation catalog.

## Nonlinear text scale, UI metrics and raster

Distinguish: `RasterScale` changes only renderer/device realization, never logical layout/hit/semantic geometry; `TextScale` maps **unscaled logical font size** to a new metric size **before** production text shaping/line breaking/intrinsic measurement; `UiMetricScale` changes only explicitly scalable chrome/spacing/minimum hit target metrics, never automatically scales absolute authored lengths, viewport dimensions or text a second time.

A single scalar cannot faithfully express all supported host text policies: Windows' `UISettings.TextScaleFactor` documentation says large text is affected less than small text; Flutter/Android's `TextScaler` migration likewise documents nonlinear text sizing. **Choose a bounded deterministic curve rather than an opaque native callback**:

- `Linear(factor)` accepts one finite strictly positive factor.
- `Curve(knots)` accepts **2–16** ordered finite (original-size, effective-size) pairs. Inputs are strictly positive and strictly increasing; outputs are strictly positive and **nondecreasing**. Two knot pairs define one interpolation segment. Equal adjacent output values are valid (nondecreasing). Validate all values, ordering and resulting finite derived values before admission.
- Inside the knot interval interpolate linearly between neighbors, including exact knots. Outside the interval multiply original size by the **nearest endpoint ratio** (first_output/first_input or last_output/last_input), giving `f(0)=0` without introducing a discontinuity. The positive endpoint ratios and nonnegative interior slopes make the function globally nondecreasing. Runtime evaluation must reject unrepresentable/overflowing logical results; it must not wrap, saturate or silently downscale a user request.
- Equivalent curves MAY canonicalize only with exact semantic-equality and cache-identity proofs; otherwise preserve all validated knots and compare the exact ordered values. Construction is bounded O(16), evaluation bounded O(log 16). No mutable plugin, native function pointer or unbounded sampled list is retained.

A Windows scale *number* does **not** expose a trustworthy full native size-response curve. An adapter MAY present a clearly labeled **linear application policy** or a measured/documented bounded curve, but MUST expose `Approximate/NotNativeEquivalent` provenance when fidelity is unproved; it cannot label an arbitrary interpolation as exact Windows accessibility rendering. `Unknown/Unsupported` native mapping is distinct from user-selected identity. Headless callers may explicitly inject either form reproducibly.

Apply scaling once before `TextRequest`, shape and measure once, then retain the same logical artifact for paint, caret/preedit, selected text, intrinsic bounds and semantic geometry. Text growth may reflow/scroll/clip through accepted constraints; runtime MUST NOT secretly shrink text to fit. Scroll extents and offsets are recomputed/clamped, hit/semantic bounds follow actual geometry, and movement between monitors changing only `RasterScale` cannot alter logical shape or semantic bounds. Subtree override cannot force `TextScale::Linear(1.0)` against a greater required user scale. Authored ordinary font sizes remain application decisions under the one effective scaling law.

## UI direction, logical edges and bidi

A surface-root direction is explicit policy and inherited along mounted ancestry. For the supported horizontal writing mode, `InlineStart/InlineEnd` map to left/right under LTR and right/left under RTL; `BlockStart/BlockEnd` remain top/bottom. Existing physical `EdgeInsets.left/right` and `LayoutInsets.left/right` remain **physical**; add semantically distinct logical edge authoring and cleanly migrate confusing pre-1.0 names, never silently reinterpret physical offsets. Do not introduce vertical writing parity without a supported profile.

Flex `Row` traverses authored source order from inline-start, `RowReverse` traverses reversed source order relative to resolved inline direction, `Column` remains top-to-bottom and its inline cross-axis start/end follow direction. Wrap, alignment, margin, logical absolute insets and overlay start/end obey the same resolved axes. Grid column line 1 and automatic placement begin at logical inline-start; rows remain top-down; explicit physically authored placements preserve physical intent. Logical and physical values lower **exactly once** into the existing M8/Taffy authority. Accepted Taffy 0.14 provides a direction field, but implementation MUST prove actual Taffy RTL Flex/Grid/wrap behavior and compensate only at the **single** runtime layout lowering boundary where necessary. Do NOT manually reverse authored children **and** invoke directional Taffy reversal, or mirror the final paint scene: either would corrupt text/images/hits.

Runtime retains canonical nonnegative **inline-start-relative scroll distance**, maps it once to physical viewport positioning, recomputes/clamps on direction/content changes and keeps the same scroll owner where mounted identity is unchanged. Semantic/hit geometry is always surface-physical logical geometry. Logical start/end actions follow each standard control's contracted directional meaning; physically spatial ArrowLeft/ArrowRight focus uses actual physical layout unless text editing or another accepted control contract governs the action. **Unicode bidi, caret affinity, IME and text-run ordering remain M8 text authority**. UI direction is at most a default paragraph base direction; authored paragraph/span language/direction and bidi segmentation do not get overridden by a whole-widget scene flip.

## Inheritance, access and protected policies

For legally overridable values, resolve validated host observation -> application-selected surface policy -> effective root -> nearest inherited ancestor -> explicit current subtree override. Ordinary subtree overrides are restricted to **selected content locale/language, layout direction and appearance**; theme choice remains application policy under existing M8 style tokens/recipes. Siblings and other surface roots never share implicit ancestry. User accessibility flags, text/UI scale input and pointing observations cannot be mutated as subtree host facts. An app can author larger typography, stricter reduced-motion or stronger contrast styles, but MUST NOT downgrade a user-required high-contrast/reduced-motion/always-show-focus value or falsely report user scale/capability.

The accepted `StylePreferences` and `StyleEnvironment` remain one **derived read-only** presentation/style projection of this single effective environment, not independently mutable per-publication preference authorities. Production cutover MUST remove the duplicate public `with_preferences` input path, preserve existing property-local high-contrast mandatory priority above authored overrides and preserve M9 deterministic reduced-motion sampling/lifecycle. A new environment cannot automatically select a branded theme.

Focus-visible MUST obey exactly:

    effective_focus_visible =
        focused && (runtime_causal_focus_visible || always_show_focus)

The user preference changes *derived style facts only*. Never mutate or erase the causal focus-visible latch; turning preference off exposes the retained latch. A pointer-focused ordinary Button with causal false gains then loses the ring under false/true/false preference; keyboard-focused causal true remains visible. Style/paint invalidation is differential and never fabricates a focus transition.

Input `Unknown` is not `No`; fine and coarse pointers can both exist, with hover available concurrently. Capability never equals current modality or #428 gameplay-input ownership. Hover-only animation affordances cannot remove keyboard focus/accessibility behavior.

A preconstructed child's `environment_override(locale)` alone cannot translate its already-authored content. The long-term public model MUST provide an **environment-scoped child authoring closure/builder evaluated before constructing its descendants**, with the identical effective context retained as mounted ancestry, or an equivalent proven single-build authoring protocol. Public custom widgets MUST receive the same read-only effective value at authoring/rebuild, metric measurement and semantic publication phases without private runtime access or built-in-widget exceptions. No retained escaping closure may create another live environment owner or register mount effects outside normal generation lifetimes.

## Coherent root construction, ingestion and publication

Host provides a validated complete observation at initial mount (headless has explicit reproducible defaults), bound to the exact runtime namespace and `SurfaceId` generation. Runtime issues nonwrapping `EnvironmentRevision` per surface. Host-provided observation revision/freshness can reject stale ingress, but a host never mints runtime identity. Equal effective snapshots are idempotent; invalid, foreign, retired, stale or out-of-order inputs MUST fail with typed reasons, preserve previously coherent state and never claim an accepted revision.

A **committed ordinary application update** can also change the application-selected content locale or root layout direction even if host observations are identical. The existing application update/reconciliation transaction MUST resolve the new effective surface environment and advance its per-surface `EnvironmentRevision` when the effective values actually change; it MUST NOT wait for a fabricated host observation. An application theme/recipe change that does not change an environment field remains an ordinary **application/style revision** but MUST still be frozen with the same app-state generation and effective environment used for root construction and surface planning. Host/environment revision and app-state/style revision are distinct provenance, correlated at the one staged publication boundary. A rejected/terminal app update follows existing commit/poison rules rather than pretending to roll back durable application state.

An environment update is a host-neutral **configuration occurrence** ordered by the existing canonical runtime FIFO/lifecycle, not native input and **not a synthetic input receipt or application action**. [ADR 0019](0019-event-correlated-host-input-arbitration.md) / #428 independently owns exact input settlements, `InputOwnershipRevision`, direct synchronous mutation witnesses and terminal scope invalidation. An environment change does not create a second queue, parallel input ownership counter or fake `InputSettled`. If it affects facts exposed through #428 ownership projection, that owner MUST issue its own normal witness; otherwise no redundant ownership revision. Reconciliation, capacity preflight and any synchronous public host ingress MUST respect the same ordered mutation and fail-closed publication contract.

After accepted environment mutation, application policy chooses the selected full content locale, UI direction/theme interpretation and the runtime freezes one immutable **effective root snapshot** before rebuilding language-dependent content. The same revision is pinned across authored root/subtree construction, keyed reconciliation, style resolution, production text shaping, layout, motion sampling, paint/hit/semantic planning and atomic commit. Changes arriving during staging are admitted for a **later** revision; no candidate can combine new preference fields with old application-generated labels. `SurfaceBuildContext` continues to own logical root constraints and `RasterScale`, **not** a second mutable `StylePreferences`; its publication admission MUST correlate to the runtime's chosen effective environment revision. Stage refusal/terminal error yields no partially published product. Immutable retained logical publication has exact revision and surface generation but is **not native Present**; host still owns displayed-frame correlation.

Environment-triggered authoring refresh is not `UiApp::update`, does not mutate durable application state or invent an application action/second scheduler. Unchanged keyed mounted generations retain valid identity and effects/subscriptions; changed translations/content use ordinary reconciliation and necessary retirement. Invalidation is per affected dependency, not unconditional remount-all.

[#353](https://github.com/dornglut/runen-ui/issues/353)'s reviewed multi-surface **investigation** selects one `AppRuntime`/`State`/action/work/clock/trace authority and one mounted arena with exact generational per-surface domains. It is **not yet implemented or an accepted separate ADR**. This ADR binds the **environment interface constraint** but DOES NOT independently freeze competing `UiApp::surfaces/root` signatures: the eventual #353-owned application-root clean cutover MUST pass the resolved per-surface environment **before** root construction, while this environment ADR owns fact validity and revision coherence. One app action may rebuild two roots against distinct A/B environment snapshots; A's environment change cannot advance B's revision, focus or cache. Moving a mounted subtree across roots retires and remounts its identity. Retiring a surface invalidates pending environment ingress and snapshots; no stale same-slot generation rebinding. #428 input arbiter maintains its own scoped/runtime ownership.

## Illustrative public Rust authoring, not current exports

~~~rust
// Target neutral values; fields shown for clarity, not present Rust signatures.
pub struct HostUiFacts {
    pub preferred_languages: Vec<LocaleTag>,
    pub appearance: Appearance,
    pub text_scale: TextScale,
    pub ui_metric_scale: UiMetricScale,
    pub accessibility: AccessibilityPreferences,
    pub input: InputAffordances,
}
pub struct UiEnvironment {
    pub selected_locale: LocaleTag,
    pub preferred_languages: Vec<LocaleTag>,
    pub direction: LayoutDirection,
    pub appearance: Appearance,
    pub text_scale: TextScale,
    pub ui_metric_scale: UiMetricScale,
    pub accessibility: AccessibilityPreferences,
    pub input: InputAffordances,
}

// Root signature / surface key remain owned by #353's eventual cutover.
fn root(state: &Settings, surface: &Panel, env: &UiEnvironment)
    -> impl View<Action>
{
    let label = state.strings.for_locale(&env.selected_locale);
    column((text(label.title()), button(label.save()))).key("settings")
}

// Scoped authoring must apply before building descendants, not afterward.
environment_scope(
    EnvironmentOverrides::new().with_direction(LayoutDirection::LeftToRight),
    |child_env| code_editor(source, child_env)
);

// Host/engine injects one coherent fact update with exact scope; this is
// conceptual, and queue+revision signatures remain coordinated with #428.
host.submit_ui_facts(surface_id, previous_revision, validated_facts)?;
~~~

No current production method by these names is claimed. Exact signatures and placement must satisfy public downstream widget build/semantic parity, #353 multi-root cutover and #428 event ordering **in one implementation route**, without compatibility-only duplicate setters.

## Invalidation, diagnostics and failure

Track typed changed-field masks and **effective** field provenance (baseline/host/application/ancestor) in runtime-owned read-only inspection. **Environment consumption is a correctness dependency, not automatically discoverable from an authored-element diff.** On an effective surface environment change, the runtime MUST re-evaluate its root and relevant environment-scoped authoring builders before comparing keyed element values. Mounted custom-widget measurement/semantic/style consumers without a proven field-level read set MUST conservatively be treated as depending on **all** environment fields and receive the established phase-appropriate invalidation; they cannot silently reuse a cached artifact merely because their authored widget payload is unchanged. A future typed field-dependency mask or runtime-managed read tracking MAY narrow that work only when every reading phase and downstream custom-widget callback is covered by deterministic positive/negative proof. Reuse existing mounted dirty phases and M8/M9 dependency effect classification, not a second environment scheduler or layout engine. This permits ordinary keyed descendants with unchanged computed dependencies to remain mounted and avoids cross-surface invalidation, while never relying on guesswork about custom widget reads. Reconcile only consumers of changed facts or conservatively unknown dependencies:

- Locale/actual authored translation or shaping language: affected authored roots/text artifacts, metric measurement, semantic language/text, downstream layout/paint.
- Direction: logical layout/placement/scroll, directional navigation where contracted, hit/focus and semantic bounds; preserve unchanged keyed identity.
- Text scale: typography, shaped artifacts/caret/preedit, intrinsic measurement, layout/scroll, paint/hit/semantics.
- UI metric scale: only opted-in control metrics plus dependent layout/hit/semantics.
- Appearance: app-authorized theme/style changes and their actual property-local effect.
- High contrast: existing mandatory style preference classifier; reduced motion: M9 timeline sampling/deadline policy; always-show-focus: focused owner style fact/effect only.
- Input capability: only reading adaptive consumers; no input receipt/modality, pointer lock or gameplay mutation.

Unchanged values, unrelated siblings and other surfaces MUST not dirty entire trees. #289 independently owns conditional available-size/container-query cycles.

Canonical trace records only surface generation, from/to revision, changed field class, source provenance class, result/rejection and redacted bounded diagnostics; default trace MUST NOT dump raw preferred languages, translated sensitive text or user profile. Trace disabled/capacity zero cannot change authoritative environment, exact revisions or staged commit. Rejection kinds MUST distinguish invalid BCP 47, unrepresentable/no shaping prefix, invalid/nonmonotone/overflowing scale, unsupported/approximate host mapping, stale/foreign/retired surface, exhausted revision/capacity and publication rejection. No invalid value may be coerced to an apparently valid native locale. Native query failure reports Unknown or explicitly stale observation provenance, not a falsely fresh source. Exhaustion never wraps; errors before mutation leave prior coherent state, and errors after irreversible app update obey accepted terminal/poison behavior without pretending rollback.

## Platform boundary and safe-area decision

Windows user locale preferences, `UISettings.TextScaleFactor`, supported contrast/appearance APIs and native pointing enumeration; macOS `Locale`/effective appearance/accessibility notifications and backing-scale distinction; Linux/SteamOS toolkit/portal/compositor preferences and available pointing/hover observations are **candidates**, not proved adapters. Every selected Windows/macOS/Linux mapping must be independently executed under the supported profile before claimed native parity. Missing platform APIs/notifications are explicit Unsupported/Unknown or stale; do not infer an always-focus API from last keyboard input or a nonlinear text-size curve from one Windows scalar. Embedded/game/headless hosts inject deterministic values without native framework types. Native cursor lock and relative raw motion remain #424 host-private.

**Do not add a public safe-area/content-inset field for 0.1 now**. Existing desktop content-rectangle bounds and embedded/game host-chosen logical root constraints plus authored padding cover demonstrated initial profiles. Reopen only on reproducible evidence of an obscured region **inside** the RunenUI content rectangle that these cannot represent without incorrect hit/semantic geometry, together with a concrete consuming UI. First consider one surface-scoped validated logical four-edge inset and explicit single-use rule; do not assume mobile cutouts/foldables/nonrectangular obstructions or silently subtract insets twice.

## Alternatives, falsifying counterexamples and consequences

1. Paint-only `with_locale`: application label stays German while paint/semantics claim Arabic -> **reject**.
2. Treat `TextLanguage` prefix as full BCP 47: `de-DE-u-co-phonebk` loses a relevant extension -> **reject**; preserve full identity, mark shaping projection.
3. Runtime translation catalog or mutable global environment: duplicated app/localization or per-window state authority -> **reject**.
4. One global text multiplier claiming exact native scaling: larger system text can have a different effective factor -> **reject**; bounded explicitly approximate curve.
5. Scale final paint scene or conflate raster with user scale: caret, logical hit and semantic geometry diverge -> **reject**.
6. Flip entire scene or both child order and Taffy direction in RTL: glyphs/images mirrored, Grid/Flex double-reversed -> **reject**.
7. Permit an RTL code quote to force outer UI text bidi: wrong separation of layout/paragraph semantics -> **reject**.
8. Allow subtree `high_contrast=false`, `reduced_motion=false` or `always_show_focus=false` against required true: accessibility regression -> **reject**.
9. Rewrite focus-visible causal latch on preference toggle: strong pointer ring sticks after preference off -> **reject**.
10. Treat `hover=Yes` as active pointer, or `coarse_pointer=No` as no controller/game motion: capability is not runtime input ownership -> **reject**.
11. Add universal safe-area graphs based on mobile precedents: unsupported 0.1 scope -> **defer pending evidence**.
12. Environment change remounts every widget/reissues mount effects or mutates application state: extra lifecycle authority -> **reject**.
13. One global environment in #353's shared app runtime: RTL tool window contaminates LTR game window -> **reject**.
14. New #299 input revision/queue to bypass #428: broken native settlement lineage -> **reject**.

## Proposed conformance and delivery sequence

The **future** M13 environment conformance matrix, not this ADR, will own permanent executable rows `M13ENV-01` through `M13ENV-18`. Register them **blocked** in a separate issue before implementation; acceptance of this ADR does NOT promote any observed behavior. Proposed observation groups:

| IDs | Required positive, negative and diagnostic proof |
| --- | --- |
| `01–03` | Validated full tags/priority/fallback, shaping projection without lost full identity, explicit mixed-script authoring |
| `04–06` | LTR/RTL subtree inheritance, logical/physical Flex/Grid/overlay/scroll layout, independent bidi/caret/IME |
| `07–08` | Raster/text/UI scale independence, linear/curve interpolation and overflow rejection, text/hit/semantic/caret parity |
| `09–11` | Appearance/theme separation, mandatory contrast/motion precedence, focus preference true/false preserving causal latch |
| `12` | Simultaneous/unknown fine/coarse/hover and downstream adaptive custom-widget parity |
| `13–15` | Surface-revision FIFO admission, same-revision atomic root-to-publication time cut, differential invalidation/trace-free equality and failure |
| `16–17` | Opposite per-surface environments/retirement, public-only headless external-host desktop/tool/game use cases |
| `18` | Actually executed supported Windows/macOS/Linux/SteamOS preference changes, Unsupported/Unknown paths and truthful native fidelity |

**Dependencies:** (A) independently review and accept this docs-only ADR; (B) reconcile #353 surface-root contract and #428 ingress/observation without conflicting signature ownership; (C) separately issue-owned clean pre-1.0 cutover for neutral values, runtime environment, M8 RTL/shaping, M9 preferences, M11 controls and public custom widgets—delete obsolete alternative `StyleEnvironment::with_preferences` authoring rather than maintaining a competing path; (D) register one blocked matrix and accept actual deterministic public tests, real-wgpu and supported native-profile proofs separately. #289 adaptive queries and #340 mnemonic execution remain independently owned. M13's roadmap M7–M12 dependency gate remains. No production implementation or platform parity follows just from an ADR merge.

## Primary external evidence

- [RFC 5646 / BCP 47](https://www.rfc-editor.org/rfc/rfc5646), [RFC 4647 matching](https://www.rfc-editor.org/rfc/rfc4647), [Unicode CLDR likely subtags](https://unicode.org/reports/tr35/) distinguish complete tags, app locale selection and optional heuristic script expansion.
- [Flutter text scale migration](https://docs.flutter.dev/release/breaking-changes/deprecate-textscalefactor), [Android 14 nonlinear scaling](https://docs.flutter.dev/release/breaking-changes/android-14-nonlinear-text-scaling-migration), [Windows UISettings text factor](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.uisettings.textscalefactor) show why a linear scalar is not universally faithful.
- [Taffy 0.14 Style direction](https://docs.rs/taffy/0.14.0/taffy/style/struct.Style.html), [SwiftUI environment](https://developer.apple.com/documentation/swiftui/environmentvalues), [GTK direction](https://docs.gtk.org/gtk4/method.Widget.set_direction.html), [Qt GUI application](https://doc.qt.io/qt-6/qguiapplication.html) are design evidence, never alternative RunenUI state owners.
- [W3C focus-visible](https://www.w3.org/TR/selectors-4/#the-focus-visible-pseudo) supports an explicit always-show user preference while retaining actual focus separately.
