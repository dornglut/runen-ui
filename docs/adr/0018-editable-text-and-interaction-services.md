# ADR 0018: Editable text and interaction services

> **Category:** ADR
>
> **Status:** Proposed target architecture; not owner-accepted
>
> **Decision date:** 2026-09-17
>
> **Milestone:** M10
>
> **Reviewed baseline:** `36b779f700eb3c0ca4eafad0bf4a582339cec661`
>
> **Acceptance:** owner acceptance of this ADR establishes target architecture
> but is not independently sufficient to complete M10A0. Before merge, the
> status and acceptance provenance must record that owner decision; a proposed
> ADR must not land as accepted default-branch authority. M10 implementation
> remains prohibited until the corresponding permanent conformance inventory,
> fail-closed audit registration and accepted-main validation are also accepted.

## Context and inherited authority

M10 completes the [roadmap's editing and interaction-services outcome](../roadmap.md)
without replacing the accepted M4–M9 foundations:

- [ADR 0005](0005-canonical-event-routing-and-commands.md) already defines one
  routed, sequenced input/default authority, surface-generation admission,
  keyboard versus committed text, composition lifetimes, pointer IDs, capture,
  logical wheel input and semantic commands. The existing
  [core input protocol](../../crates/runenui_core/src/input.rs) validates
  composition byte ranges and carries opaque runtime-issued generations.
- [ADR 0006](0006-effects-scheduling-and-trace-v2.md) already owns one work queue,
  deterministic clocks, host-request completion and cancellation. It explicitly
  reserves *framework-owned* clipboard/cursor/IME/drag-and-drop services for M10
  rather than routing them through an application's arbitrary `HostProtocol`.
- [ADR 0009](0009-production-style-layout-text-foundation.md) owns one text
  shaping/reflow/measurement path. The
  [text artifact](../../crates/runenui_text/src/artifact.rs) already records
  UTF-8 source ranges, visual run order, directions, cluster flags and the
  exact shaped-resource identity. Its
  [reusable layout state](../../crates/runenui_text/src/layout_state.rs)
  privately retains a Parley `Layout` and immutable correlated artifact.
- [ADR 0010](0010-visual-composition-and-animation.md) and its accepted
  clarifications own visual composition and deterministic motion. Presentation
  transforms must keep paint/hit/focus/semantic geometry correlated.
- [The M5 semantic contract](../../crates/runenui_core/src/semantic.rs) presently
  exposes plain text and generic/text/button roles, but not editable roles,
  selection or range actions. The
  [AccessKit adapter](../../crates/runenui_winit/src/accessibility.rs) already
  projects ordinary semantic publication and routes exact semantic actions.
- [`runenui_winit`](../../crates/runenui_winit/src/lib.rs) translates native
  keyboard/mouse/AccessKit facts; its caller owns the window, event loop and
  presentation. Its accepted winit 0.30.13 dependency already has IME enablement,
  caret-area, cursor and file-drop primitives, but those OS facilities are not
  yet complete RunenUI framework services.

These facts describe existing foundations, **not** completed production editing,
scrolling, clipboard, controller support or platform coverage. The current
[status](../status.md) remains authoritative for accepted maturity; this ADR
must not promote it.

## Decision: ownership and transactional editing

### One application document, owner-local mounted sessions

Application/product state owns the authoritative document value and revision,
validation, persistence, access policy and any undo/redo journal whose history
must survive widget removal or application-controlled replacement. Reusable
framework editing operations may produce typed edits, inverse information,
selection intentions and grouping hints, but may not silently mutate a second
persisted document. The application accepts or rejects document changes through
its ordinary typed `UiApp::update` transaction.

An editable contribution supplies a stable application document identity, its
current revision, current text and an owner-local public mapper from one neutral
`EditIntent` into the application's `Action`. The exact API spelling may be a
widget callback rather than a stored closure, but recursive action mapping must
map it like every other widget-produced action. Runtime never fabricates an
application action or invokes `UiApp::update` from a widget callback. Within one
live editing session, equal document identity and revision require equal source
text. Editing-policy changes explicitly preserve or reset the runtime session;
they do not masquerade as document edits. Same-revision content drift is a
contract rejection, and revision reuse must never make an old request current.

An exact mounted text-editing owner may retain *ephemeral session state*: caret
anchor/active selection and affinity, preferred inline position for vertical
movement, pointer-selection gesture, current IME preedit, and pending service
request tokens. These states end on exact generation removal or replacement;
compatible reconciliation preserves them only after validating document revision,
text identity and configured session policy. Application-controlled document
replacement must deterministically rebase a session through a supplied validated
change mapping or reset it; it must never guess offsets in unrelated text.
Runtime also issues a non-wrapping editing-session generation. Document identity
change, incompatible revision movement, explicit reset or exact owner replacement
retires that generation, so an application-authored identity/revision pair is
never by itself sufficient to admit a late edit or service completion.

The same application document identity may be presented by multiple mounted
owners. Each owner retains an independent editing-session generation, selection,
preedit, pending edit chain and service tokens. An accepted document change
reconciles every presentation from the new authoritative revision, using a
validated mapping where supplied or resetting that owner's incompatible session;
caret, composition, pending requests and service authority never migrate between
owners merely because their application document identities match.

Undo/redo is a reusable transaction protocol, not a second hidden document.
Applications own the journal and grouping policy needed to reverse committed
application changes; the framework may compute deterministic inverse edits and
session-local grouping metadata, but never record speculative or rejected edits
as committed history. No universal cross-document history, editor product model,
private mutation bridge, or alternate action dispatch is introduced.

One editing ingress produces a runtime-issued, exact-session `EditRequestId` and
one provisional `EditIntent`. The intent names its document identity, base
revision, predecessor request when one exists, checked replacement range,
replacement text, proposed selection and edit kind. The routed input transaction
first validates its target, surface, editing-session generation, document
revision and ranges and preflights the required output capacity. A successful
routed event commits only bounded provisional session state and enqueues the
application action produced by the widget's mapper; **it does not synchronously
call `UiApp::update`**.

The bounded provisional state is a pending edit projection, not a second durable
document or undo journal. It exists because the global FIFO may already contain
multiple committed-text/key events ahead of the application action emitted by
the first event. Each later intent therefore names the exact predecessor and is
derived from the same owner-local pending projection rather than pretending that
the application revision has already advanced. Saturation rejects before the
routed callback/default commits; it never drops the oldest request or silently
falls back to the stale application text.

At an edit action's later queue position, ADR 0006 separately preflights and
invokes `update`. An edit-origin action must return exactly one transaction-local
framework resolution alongside its ordinary update effects; the resolution is
not an effect, queued action or persistent widget contribution. It echoes the
opaque request ID and classifies the proposal as accepted, rejected or
transformed, with the resulting document identity/revision and any required
validated change/selection mapping. The exact API may extend the existing
`IntoEffects` result into a compatible update-output conversion, but `()` must
remain the no-effects result for ordinary non-edit actions and no second update
callback or queue is introduced.

Runtime then builds and reconciles the root and verifies the transaction-local
resolution against the resulting authoritative editable contribution. An exact
proposed text/revision result may use the ordinary accepted shorthand, but
absence of a matching result is not guessed acceptance. Because response
validation occurs after `update` may have mutated application state, a missing,
foreign, duplicate or internally inconsistent resolution is an unexpected
post-mutation integrity failure governed by ADR 0006's terminal `Poisoned`
policy; runtime must not silently reinterpret it as rejection.
Accepted resolution commits the authoritative application document and matching
session update; rejection restores/rebases the pre-request session state;
transformation validates the supplied mapping. Exact acceptance preserves a
causally valid dependent projection. Rejection, or transformation without a
mapping that can validate the whole suffix, removes that suffix from the
displayed projection but does not delete its queued actions or request identities;
new edit ingress rejects before mutation until the invalid suffix drains.
Dependent pending intents remain causally bound to their original predecessor
and immutable application action; runtime does not rewrite an already queued
`Action`. When each dependent action reaches its FIFO position, the application
explicitly accepts it against the then-current revision, transforms it with a
validated mapping, or rejects it as a superseded suffix. Runtime validates that
response against the pending chain and never retargets it to unrelated text.
The application owns the policy decision; runtime owns request identity,
pending-chain integrity and resolution validation. A request can resolve once
only.

The action transaction commits resulting application/interaction state and
effects and marks surface publication dirty. The next successful publication
stages correlated layout, caret, paint, hit and semantics together; it is **not
atomic with** the application action transaction. An unchanged or rejected edit
must not advance authoritative document revision, committed selection or undo
history or start host work. Burst input requires permanent proof for accepted,
rejected and transformed prefixes; hosts are not allowed to make this correct
merely by pumping every input action to completion before accepting the next.

Recoverable rejection before mutation exposes no speculative document/session
change or host request. If an unexpected integrity failure occurs **after**
application-state mutation, the inherited ADR 0006 terminal `Poisoned` policy
applies: do not claim arbitrary application mutation was rolled back, do not
publish a partial new surface or start provisional external work, and reject
further callbacks. Callback panics remain unsupported. A successfully committed
app edit needs subsequent surface publication for presentation but never a
renderer/device success to establish durable application state.

### Exact text coordinates, boundaries and geometry

Canonical durable text positions are validated UTF-8 byte offsets into one
identified *document revision*. A range is a checked, ordered half-open byte
range; a selection retains anchor, active endpoint and visual affinity rather
than reducing direction to an unordered pair. Both endpoints must align to
Unicode scalar boundaries, and ordinary caret/movement/deletion positions must
also respect grapheme and shaping-valid caret stops. A line/word move is a
behavior defined over these validated positions, not a conversion to raw glyph
indices or a platform-native code-unit index. Input-method offsets remain
checked relative to the exact preedit string; native UTF-16 or other units are
translated and validated at the host edge before RunenUI ingress.

`runenui_text` computes a **single correlated caret/selection map** from the
same private Parley layout and artifact used for logical measurement and paint.
The map identifies legal leading/trailing caret stops and affinity around bidi,
ligatures and wrapped line boundaries; it must expose deterministic hit-to-
position, position-to-caret, visual/logical navigation, selection rectangles and
IME candidate geometry through RunenUI-owned neutral values. A source byte
range by itself is insufficient to derive caret geometry from the current
`TextArtifact` public cluster list: clusters are in logical order, runs are in
visual order and glyph/ligature caret boundaries need layout-level data.
`runenui_text` may privately adapt Parley's cursor/selection functions against
its *already retained* layout. It may extend the artifact with immutable
correlated mapping facts, but cannot shape or line-break a second editable copy.

IME preedit is an explicitly labeled transient display projection, never
implicitly committed document text. Its exact document-revision anchor,
replacement range, preedit-relative selection and composition generation are
tracked together; the display-to-document mapping distinguishes synthetic
preedit offsets from durable offsets. Commit inserts the host's committed text
*once* via an ordinary edit intent and retires the matching generation. Cancel,
focus transfer, owner removal, disablement or shutdown removes the transient
projection without a document mutation. Stale/out-of-order commits, foreign
owners, invalid ranges, duplicate commits and mismatched revisions reject with
structured diagnostics; none re-hit-tests or retargets a new owner. Candidate
layout/paint/semantics derive from the same staged projection, not an alternate
buffer or text engine.

### Input and semantic defaults

Physical key identity is never committed characters. Logical keys supply
command/navigation meaning only; text insertion consumes the committed-text
stream. Composition-preedit events manage the existing M4 composition
lifetime and are not simultaneously interpreted as committed characters.
Editing commands (move, extend, select, insert, delete, undo, redo, copy,
cut, paste) are host-neutral intents processed through the canonical M4 route,
default-prevention and sequenced work path, with exact focus/owner and source
checks. Platform shortcut mapping remains in adapters/hosts; a normalized
command has one framework default independent of its originating shortcut.
A prevented cancelable default does not partially edit or enqueue host work;
mandatory focus/capture/composition integrity cleanup remains unpreventable.

Editable role, read-only/disabled state, current public value, selection/range
and supported text navigation/edit actions extend the *existing* neutral
semantic contribution/publication and exact semantic-action admission. Runtime
issues semantic identities; the native adapter projects them to AccessKit and
translates actions back. Semantic offsets are validated against the same
identified source revision and caret map used by visual selection, and bounds
use the same final layout/presentation correlation. Each editable contribution
declares public or secret sensitivity. Secret sensitivity is fail-closed: literal
content is absent from ordinary trace/diagnostic export and semantic value/range
text, copy/cut defaults are unavailable, and an adapter cannot downgrade the
classification. Paste may still produce a secret-classified edit intent whose
literal payload remains absent from trace and diagnostics. Product-specific
controlled disclosure requires an explicit application action outside the
ordinary editing default; no implicit clipboard or semantic opt-out exists. No
second native/AccessKit text-model authority.

### Framework services with host-owned execution

Clipboard read/write, IME enablement/candidate region, cursor shape/visibility,
and drag/drop use a typed, host-neutral *framework service protocol*, distinct
from an application's domain `HostProtocol`. Runtime alone validates requests,
issues non-wrapping opaque exact-owner/session tokens and stages them through
its existing FIFO/effects commit; the host executes OS operations and returns
results via the same checked queue. Hosts retain windows, event loops, user
activation/permission decisions, actual clipboard contents and OS handles.
Adapters translate only neutral facts and platform formats. Renderer and text
shaper acquire no native service authority.

Service requests bind exact mounted generation, editing-session generation,
surface and relevant document/composition revision; late completion after
removal, focus change, session reset, revision replacement or shutdown is
rejected or discarded without leaking payload into another owner. Reusing an
application-authored document identity or revision cannot revive the retired
runtime session token. Clipboard read may fail, be unavailable or require a user
activation; such outcomes are typed and never silently treated as empty text.
Cut removes text only after the configured copy-success policy commits;
paste validates the clipboard result, insertion limits and revision again at
queue-front. Clipboard content is not included in ordinary trace payloads,
error messages or generated diagnostics. A read-only deterministic test host
supplies explicit fake results without ambient clipboard access. Drag/drop
separates hover/admission, accepted typed payload, cancellation and completion;
external filenames/paths and OS security scopes remain host-owned until an
explicitly accepted application request admits them.

### Scroll, pointer, controller and touch

M4 `PointerId`/capture/physical-hit and logical wheel events remain canonical.
M10 adds scroll-container state, viewport-to-content offset, clipping, bounded
extent/overscroll policy and scroll-to-target commands through runtime-owned
layout/publication invalidation. One transform of final logical geometry
correlates scrolled paint, hit, focus and semantics. Wheel default routes to the
closest eligible scroll owner through the ordinary route. That owner consumes
only the clamped delta that changes its logical offset; the exact unconsumed
remainder continues to the next eligible ancestor in route order. A prevented
default suppresses the complete default chain. Visual overscroll does not count
as logical consumption, and axis conversion, epsilon guessing or renderer-
observed position cannot alter the remainder. There is no renderer clipping,
private widget scroll tree, synthetic per-frame action queue or auto-scroll
during a failed transaction.

Pointer text selection obtains exact position from the retained displayed
snapshot and caret map; capture follows the existing exact-owner lifetime and
cancels on owner/stream loss. Hit and selection honor presentation transforms,
clips and stale/retired surface rules rather than using layout rectangles as a
fallback. Cursor requests derive from committed hit/focus state, not platform
callbacks that directly mutate runtime interaction.

Raw gamepad axes/device IDs, dead zones, repeat cadence and native controller
backends stay outside core/runtime. A host normalizes supported controller
input into the existing semantic command families with explicit source/modality
and repeat/cancel semantics. For touch, the initial supported-profile baseline
uses existing independent pointer streams and exact multi-pointer ownership,
with primary tap, move, drag/scroll and cancellation defined and tested; richer
pinch/rotation/gesture families require an explicit profile/consumer contract,
not automatic import of every platform event. Competing touch-scroll and text-
selection gestures remain provisional until deterministic movement thresholds
resolve them. An explicit capture claim wins at its committed route position;
otherwise the nearest eligible default owner wins when its threshold is crossed,
cancels the losing provisional gesture once, and owns that pointer stream until
release/cancel. Threshold values are validated neutral configuration, not ambient
platform guesses, and arbitration never replays one physical stream into a new
owner after commitment.

## Dependency assessment and adoption boundary

External API/metadata reviewed on 2026-09-17. Versions and MSRVs below are
**candidate evidence**, not an instruction to upgrade or add a dependency;
reverify exact resolved graph, supported platforms and licenses before each
implementation slice. RunenUI's declared MSRV is Rust 1.93.0.

| Option | Observed version, MSRV, license | Ownership fit / consequence |
|---|---|---|
| [Parley](https://docs.rs/parley/0.11.1/parley/) | Existing `0.11.1`, Rust `1.88`, MIT OR Apache-2.0 | Keep current private shaper/layout and investigate [`Cursor`/`Selection`](https://docs.rs/parley/0.11.1/parley/editing/struct.Cursor.html) over that exact retained layout. Its [`PlainEditor`](https://docs.rs/parley/0.11.1/parley/editing/struct.PlainEditor.html) retains its own string **and** layout, so wholesale adoption would duplicate app document and current text-layout authority; do not adopt it as runtime editing state. |
| [unicode-segmentation](https://docs.rs/crate/unicode-segmentation/1.13.3) | `1.13.3`, Rust `1.85.0`, MIT OR Apache-2.0 | Small pure boundary helper if Parley's existing Unicode/ICU facilities do not supply required grapheme/word policy. No second shaping or runtime store; additional dependency requires evidence of a missing capability. |
| [Ropey](https://docs.rs/ropey/1.6.1/ropey/) | `1.6.1`, Rust `1.65`, MIT | Large-document rope using scalar-value indices; not a default field/editor store or public position authority. Optional app-owned backing only if a real consumer and byte-index conversion/performance proof justify it. |
| [COSMIC Text](https://docs.rs/crate/cosmic-text/0.19.0) | `0.19.0`, full shaping/layout/render/edit stack, MIT OR Apache-2.0 | Alternative engine would duplicate accepted M8 Parley/shaped-resource/renderer authority. Do not adopt merely to obtain its editor; no MSRV/feature adoption decision without separate primary-source verification. |
| [arboard](https://docs.rs/arboard/3.6.1/arboard/) | `3.6.1`, Rust `1.71.0`, MIT OR Apache-2.0 | Possible *host-private* clipboard provider, not a core/runtime dependency or normative clipboard semantics. Default image support brings extra dependencies; optional Wayland data-control relies on compositor protocols and X11 fallback. Must prove Windows/macOS/X11/Wayland behavior and lifetime/ownership before adoption. |
| [winit](https://docs.rs/crate/winit/0.30.13) | Existing pinned `0.30.13`, Rust `1.70.0`, Apache-2.0 | Retain host/adapter edge for [`IME control and candidate area`](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html), cursor and [`DroppedFile`/`Ime` ingress](https://docs.rs/winit/0.30.13/winit/event/enum.WindowEvent.html); do not assume all OS platform profiles or generic non-file drag/drop are implemented. |
| AccessKit | Existing pinned `accesskit = 0.24.1`, `accesskit_winit = 0.33.2` | Retain native semantic projection/action boundary; exact text-range capability, target-platform behavior, license/MSRV and feature graph require focused verification before implementation. Do not expose AccessKit types in core. |
| Controller library | Not selected | Raw-device library choice belongs to a real host/profile investigation. No new mandatory gamepad crate or runtime polling loop is justified by an unimplemented controller profile. |

The review deliberately rejects introducing a new `runenui_editor`, scroll,
controller or services crate without a demonstrable independent consumer,
optionality, ownership or Cargo-enforced dependency boundary. The adopted
algorithms, if any, are private subordinates behind RunenUI-owned public values.

## Permanent proof and implementation order

The separately delivered M10 conformance matrix must freeze distinct positive,
negative and diagnostic/trace observations for: validated Unicode/bidi caret and
preedit mapping; application-owned edits/undo/reconciliation; deterministic
keyboard/IME input and stale-target rollback; clipboard permission, late result
and confidentiality; semantic range/action correlation; pointer text selection;
scroll/capture/drag/cursor; controller/touch profile contracts; public downstream
widget/host proof; and real native/renderer evidence. It must not recopy M4–M9
observations or claim current production behavior. All implementation rows start
`blocked` until their scoped implementations and proofs are accepted. Editing
proof must distinguish event intent commit, later app action commit, subsequent
publication, recoverable pre-mutation rejection and post-mutation terminal poison;
it must not assert rollback of arbitrary application state. It must also cover
multiple queued edit ingresses ahead of their actions, exact request acknowledgement,
accepted/rejected/transformed prefix resolution, dependent-suffix rebase or
rejection, invalid-suffix ingress refusal and drain, missing/foreign/duplicate
resolution poisoning, document-revision reuse, retired editing-session
generations and late service completion after an apparent application identity/
revision ABA.

Dependency-derived implementation sequence *after* accepted architecture and
matrix registration:

1. M10B — exact text positions, caret/selection geometry and display-preedit
   mapping from one retained Parley layout/artifact, with controlled international
   corpus and no alternate shaper.
2. M10C — application-owned edit intents, selection/undo transaction protocol,
   default keyboard/IME editing and semantic text-range/action integration.
3. M10D — host-neutral clipboard/cursor/IME/drag-drop service protocol with
   explicit failing/successful host-side and deterministic fake-host proof.
4. M10E — production scrolling, pointer selection/capture, controller-command
   normalization and justified touch/multi-pointer baseline.
5. M10F — integrated public-widget/native-platform/real-wgpu acceptance,
   revision/trace/confidentiality, current-truth reconciliation and M10 closure.

This sequence is a dependency hypothesis to be refined by the accepted M10
matrix, not preauthorization to open child implementation issues. In particular,
service failures cannot be hidden by a mock-only M10D closure, and desktop
editing cannot be declared complete from a headless-only M10C proof.

## Consequences and exclusions

A basic editable text control becomes possible through ordinary public widget
contracts, not a special runtime-only text-field path. App authors retain
document policy; framework code retains transient interaction and exact service
admission; `runenui_text` remains the sole text computation authority;
`runenui_runtime` remains the sole queue/layout/semantic/publication authority;
`runenui_winit` and hosts retain native translation/execution; renderer owns
only paint realization. Existing M4–M9 public protocols can be cleanly extended
before 1.0; no compatibility duplication is preserved without a specific owner-
approved contract.

A source-code editor, rich-text document product, global undo manager,
application filesystem/clipboard policy, full M13 multi-window/platform matrix,
M11 control library, universal gesture recognizer, mobile/web profiles and a
second semantic/renderer/text/runtime model are outside this decision.
