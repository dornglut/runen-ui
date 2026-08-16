# Changelog

> **Category: Current contract**

All notable changes to RunenUI are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses Semantic Versioning as qualified by the [API stability policy](docs/api-stability.md).

## [Unreleased]

### Changed

- Owner-accepted and guarded-squash-merged M5D public deterministic headless
  testing in [PR #64](https://github.com/dornglut/runen-ui/pull/64). Exact
  reviewed head `471d2acf402a0f7d3f89a1de2a1b908fe23ff619` passed canonical exact-head
  CI #1230 / `31962536977`; accepted squash/main is
  `72d2405211a3fd6d11e0d17680b7769df90b5ffe`, and reviewed head and squash
  share exact complete repository tree `bdbf19f5c2197490d6b922fb792791b205f40370`.
  Accepted-main push CI #1231 / `31967898198` passed at that exact squash. The
  accepted implementation adds public downstream `runenui_testing` with
  `TestHarness<App>`, deterministic public `ManualClock` use, configurable
  nonzero fixed-surface publication, explicit bounded pumping and finite
  `SettleBudget`, exact snapshot-scoped `SemanticQuery`/`SemanticTarget`, ordinary
  public pointer/keyboard/text/composition/automation/action/command/semantic-
  action ingress, and read-only state/focus/reconciliation/frame/layout/hit/
  paint/semantic/trace/replay inspection. It enables no `internal-test-seams`,
  hidden mutation bridge, semantic-to-`MountedNodeId` shortcut, bare-ID surface
  guessing, wall-clock wait, unbounded settle, parallel runtime model, semantic
  LogicalScroll compatibility path, or native accessibility adapter. This
  mandatory post-merge reconciliation promotes exactly the ten M5D-owned
  `TEST-*` rows to `owner-accepted`, producing M5 truth `53 total / 48
  owner-accepted / 5 blocked`, M4 truth `237 total / 237 owner-accepted`, and
  aggregate configured truth `290 total / 285 owner-accepted / 5 blocked`. M5D
  #50 remains open for this reconciliation only; M5E #51 remains blocked until
  the reconciliation itself is exact-head validated, critically reviewed,
  explicitly owner-accepted, guarded-merged, and accepted-main verified.
- Owner-accepted and guarded-squash-merged M5C semantic action ingress and
  accessibility resolution in [PR #62](https://github.com/dornglut/runen-ui/pull/62).
  Exact reviewed head `504899b79059eb94ad4474d67bba1e27eb30b374`
  passed exact-head CI #1170 / `31889342640`; accepted squash/main is
  `846c4e6adfdcd9236586f1b9978f63e71ff4fb86`, and reviewed/squash trees are
  exactly `dfa7cb71166a3f333b560508a7e82fbeb45df000`. Accepted-main push CI #1171 /
  `31903354382` passed at that exact squash. The accepted implementation adds
  public `SemanticActionRequest` construction through
  `SemanticActionRequest::new(surface, target, action)` plus
  `AppRuntime::submit_semantic_action`, exact current semantic
  surface/identity/publication/support/state/readiness/freshness/capacity
  admission, exact owned-request recovery, queue-front and post-callback
  revalidation, semantic-origin callback metadata without a public mounted-owner
  shortcut, and semantic binding/rejection/default outcomes in the existing
  canonical trace/replay schema. Accepted semantic work converges through the
  existing FIFO, `WorkSequence`, routed command, default, action/update, and
  reconciliation authorities; there is no second semantic queue or behavior
  engine, no semantic LogicalScroll, and no native accessibility adapter. This
  mandatory post-merge reconciliation promotes exactly the seven M5C-owned M5
  rows plus inherited M4 `ACCESS-01`/`ACCESS-02` to `owner-accepted`, producing
  M5 truth `53 total / 38 owner-accepted / 15 blocked`, M4 truth `237 total /
  237 owner-accepted`, and aggregate configured truth `290 total / 275
  owner-accepted / 15 blocked`. M5C #49 remains open for this reconciliation
  only; M5D #50 remains blocked until the reconciliation itself is exact-head
  validated, critically reviewed, explicitly owner-accepted, guarded-merged, and
  accepted-main verified.
- Owner-accepted and guarded-squash-merged M5B semantic tree publication and
  incremental updates in [PR #58](https://github.com/dornglut/runen-ui/pull/58).
  Exact reviewed head `3b9db8b37098786cc0d53d38ae5d597c3460c38b` passed exact-head CI #1082 /
  `31847771313`; the accepted squash is
  `43d23aefb81757a516ae569b3e86b9e0f2c71e23`, and reviewed/squash trees are
  exactly `1708d2536c6f1d202ac58dd7cb5f3cc97a438517`. The connector-origin merge
  did not emit the repository's normal push workflow event, so that absence is
  recorded as an infrastructure/event-delivery fact rather than relabeled as
  successful push CI or waived. Exact squash `43d23aef...` was independently
  revalidated without source mutation through the unchanged read-only
  pull-request CI path in temporary PR #60; CI #1084 / `31850376490`, attempt 2,
  passed and PR #60 was closed unmerged. The accepted implementation adds the
  independent exact-`SurfaceId` semantic snapshot/update/diagnostic sibling,
  deterministic transparent-owner composition and publication-local lookup
  indexes, opaque live `SemanticNodeId`s, absolute logical bounds, exact local
  and cross-owner relationship resolution, composed disabled/support state,
  visible-PRIMARY runtime focus projection, checked revisions with deterministic
  deltas/full resynchronization, renderer semantic cutover, and one fallible
  staged `admit -> plan -> candidate-dependent final preflight -> commit`
  surface-publication transaction with recoverable stationary-rehit backpressure
  and exact redraw/hit-test/coordinate/semantic counter exhaustion. This
  mandatory post-merge reconciliation promotes exactly the 19 M5B-owned rows to
  `owner-accepted`, producing M5 truth `53 total / 31 owner-accepted / 22
  blocked` and aggregate configured truth `290 total / 266 owner-accepted / 24
  blocked`. M5C #49 remains blocked until this reconciliation is itself
  exact-head validated, critically reviewed, explicitly owner-accepted,
  guarded-merged, and accepted-main verified. #59 separately owns the
  non-blocking M6-readiness work to remove whole-`SurfaceCache` deep cloning
  without weakening M5B atomic publication.
- Owner-accepted and guarded-squash-merged the post-M5A semantic-readiness gate
  in [PR #56](https://github.com/dornglut/runen-ui/pull/56). The exact reviewed
  head `15c90424a0fbae4312b0cb0c5fb76932b3ce1ee1` passed exact-head CI #902;
  the squash merge commit is `d2f8fabd33860ec1510f82d5792b5bd8f2db8f43`,
  reviewed and squash trees are exactly
  `3be7ed95d5879c5d4dc9639583c5ef8490522267`, and accepted-main CI #903
  passed at that exact squash. The accepted readiness authority freezes PRIMARY
  semantic-focus projection, composed disabled state and
  support-versus-availability semantics, private semantic-to-mounted resolution,
  exact `SurfaceId`-scoped semantic products/actions, stale/post-callback
  revalidation, staged atomic surface publication with recoverable backpressure
  versus exact terminal failure, and the clean renderer/semantic publication
  cutover. Its bounded pre-1.0 source correction removes route-bound
  `SemanticAction::LogicalScroll(LogicalScrollCommand)` while preserving
  `SemanticCommand::LogicalScroll`, `LogicalScrollCommand`, pointer/focus scroll
  derivation, routed callbacks, and accepted M4 scrolling. Accepted M5 authority
  is now `53 total / 12 owner-accepted / 41 blocked`; aggregate configured truth
  is `290 total / 247 owner-accepted / 43 blocked`. The three #55-added rows
  (`SEM-SUPPORT-01`, `SEM-PUB-04`, and `SEM-ACT-07`) remain blocked
  implementation observations, so this acceptance claims no M5B semantic
  publication or M5C action ingress. M5B #48 is the next implementation slice
  after the #55 acceptance/current-contract reconciliation is present on accepted
  `main`.
- Owner-accepted and guarded-squash-merged M5A semantic contribution and
  independent identity in [PR #53](https://github.com/dornglut/runen-ui/pull/53).
  The reviewed feature head `8377ced53c08d7b5be3020368ceddd3ee81294a5`
  passed canonical exact-head CI run `31497457992` / #889 and the final critical
  review; the squash merge commit is
  `e3c304600ec1777cd17a1973946a43c765df1c31`, and all 38 changed-file blob
  identities are byte-identical between the reviewed feature head and accepted
  squash. The accepted implementation replaces M2 semantic proof authority with
  platform-neutral `SemanticContribution`, 0..N owner-local semantic nodes keyed
  by stable `SemanticKey`, strict marker/reference validation, core-owned
  `LogicalSize`/`LogicalRect`, validated owner-local bounds, and a separate
  runtime-owned generational semantic arena/binding store with stale-safe
  retention/revocation/reuse and fail-closed capacity/index integrity. Genuine
  downstream proofs cover action-mapping neutrality and owner-local geometry.
  No AccessKit/native dependency, independent semantic publication product,
  semantic-node action ingress, or public testing harness is introduced. Its
  mandatory post-merge authority/current-contract reconciliation was explicitly
  owner-accepted at exact head `66c2e2a5e2adf3709f93e8d45821a5844986dc0c`,
  passed exact-head CI #897, and was guarded-squash-merged in
  [PR #54](https://github.com/dornglut/runen-ui/pull/54) as
  `d7189d9d145b20edc6ad931ead1589f6277373d2`. Reviewed reconciliation and
  squash trees are identical and accepted-main CI #898 passed at that exact
  squash. The reconciliation records exactly the twelve M5A rows as
  `owner-accepted`, producing accepted-main M5 state `50 total / 12
  owner-accepted / 38 blocked` and aggregate configured-matrix state `287 total /
  247 owner-accepted / 40 blocked`. The subsequent #55 readiness authority is
  accepted through PR #56 as recorded above, making M5B #48 the next
  implementation slice after the required post-#55 reconciliation.
- Owner-accepted and guarded-squash-merged the complete M4D3 replay and M4
  closure slice in [PR #43](https://github.com/dornglut/runen-ui/pull/43). The
  accepted feature head `b5f72ccaa89a9fb54d81ec3f35701cbdfbc9ba5d`
  passed canonical exact-head CI run `31398930987` / #765 and the final critical
  cold review; the squash merge commit is
  `596f0d823b9833d71a038cc4aebe834c7b94e4a6`, and all 16 changed-file blob
  identities match between the reviewed feature head and accepted squash. The
  accepted implementation adds inert offline JSONL v1 replay with replay-only
  identities, exact retained-sequence and causal-parent validation, explicit
  dropped-prefix incompleteness, Counter serialized causal reconstruction,
  deterministic retired/transitional-authority enforcement, and final public
  Counter/downstream M4 closure proofs without creating a second live runtime,
  queue, history, or ordering authority. The final M4 authority reconciliation
  records all eight M4D3-owned rows as `owner-accepted`, producing 237 total /
  235 owner-accepted / 0 proof-complete / 2 blocked rows. The only remaining
  blocked rows are M5-owned `ACCESS-01` and `ACCESS-02`; M4 is complete and M5
  semantics and deterministic public testing becomes the active successor after
  this reconciliation itself is accepted and merged.
- Owner-accepted and guarded-squash-merged the complete M4D2 deterministic
  trace-export, redaction, optional action-label, and subordinate bounded-sink
  slice in [PR #41](https://github.com/dornglut/runen-ui/pull/41). The accepted
  feature head `1bd7dcfdbb46dec52da62faabb739c835e971c80` passed canonical
  exact-head CI run `31321448821` / #712 and the frozen complete-diff review;
  the squash merge commit is `8c67655ffce438c2e35e6478e7299bd704033b8b`,
  and all 23 changed-file blob identities match between the reviewed feature
  head and accepted squash. The accepted implementation adds deterministic
  JSONL v1 projection, default-redacted and explicit-full text/IME capture,
  optional static non-`Debug` action labels, lazy bounded immutable-record sink
  delivery with receiver-side serialization, same-record `Delivered`/`Full`/
  first-`Closed` diagnostics, one-way sink retirement, exact JSON escaping, and
  capacity-zero diagnostic dormancy without adding a second trace/history/order
  authority. All ten `TRACE-EXPORT-*` rows become `owner-accepted` through this
  post-merge authority reconciliation, producing 237 total / 227
  owner-accepted / 0 proof-complete / 10 blocked rows. M4D3 remains blocked until
  this reconciliation is owner-accepted and merged; M4 remains active and
  incomplete.
- Owner-accepted and squash-merged the complete M4D1 trace-schema and causal
  reconstruction slice in [PR #39](https://github.com/dornglut/runen-ui/pull/39).
  The accepted feature head `990c49edb5b68c37dd3b7d37dd3f1196a9557c7a`
  passed canonical exact-head CI run `31269401262` / #657 and the frozen
  complete-diff review; the squash merge commit is
  `2fe269366386d7aee9de2a2573498b64ad486293`. All ten `TRACE-EVENT-*` rows
  become `owner-accepted` through this post-merge authority reconciliation,
  producing 237 total / 217 owner-accepted / 0 proof-complete / 20 blocked rows.
  The accepted implementation normalizes typed input, composition,
  authored-automation, and application-action trace facts; retains redacted
  text/preedit byte/scalar metrics and checked ranges; preserves exact cleanup,
  terminal, cancellation, shutdown, logical-time, work-sequence, and causal
  ancestry; proves non-`Debug` action identity; and reconstructs the real Counter
  and downstream public path through publication. M4D2 remains blocked until
  this reconciliation is owner-accepted and merged; M4 remains active and
  incomplete.
- Owner-accepted and squash-merged the complete M4C5 keyboard,
  committed-text, composition, and deterministic authored-ID automation slice
  in [PR #27](https://github.com/dornglut/runen-ui/pull/27). The accepted
  feature head `d0d2ef1d53a8ab1d940beb4155f5f991229f042e` passed exact-head CI
  run `30843238697` and independent rereview; the squash merge commit is
  `284ecdcfe107e0a7afc88e4bf4fc82eecc52a226`. All fifteen owned rows
  (`KEY-01`–`KEY-04`, `TEXT-01`, `IME-01`–`IME-05`,
  `AUTOMATION-01`–`AUTOMATION-04`, and `MIGRATION-07`) are
  `owner-accepted`, producing 237 total / 207 owner-accepted / 0
  proof-complete / 30 blocked rows. The accepted implementation adds canonical
  raw input ingress, exact focused-lifetime binding, Enter/Space activation,
  opaque composition generations with cancellation before focus loss and
  unmount, deterministic no-fallback automation resolution, redacted causal
  input trace, Counter/downstream proof, and removal of transitional input
  authorities. Rejected composition starts recover a generation-free
  `CompositionStartRequest`; public authored-ID automation work/trace-sequence
  exhaustion is inert and recoverable, while direct commands and accepted work
  retain ordinary terminal exhaustion policy. Undeliverable mandatory
  composition cleanup records causal suppression, retires the exact lifetime,
  terminalizes, and preserves shutdown lineage. Its separate post-merge
  authority reconciliation subsequently completed and M4D1 progressed through
  the accepted slice above; M4 remains active and incomplete.
- Owner-accepted and squash-merged the complete M4C4 focus-scopes and modality
  slice in [PR #22](https://github.com/dornglut/runen-ui/pull/22): one
  exact-generation focus authority, root/nested scope policies, current-order
  linear and retained-publication directional navigation, DF-01–DF-20,
  exact-generation restoration, atomic focus/focus-within transitions, routed
  non-cancelable `FocusOut` before `FocusIn`, retained modality, canonical
  focus/modality trace, downstream public proof, and removal of direct focus
  helpers and transitional result/policy authority. The accepted feature head
  `f3201a83583af0c1d148bec87cd9140ff42795b7` passed exact-head CI run
  `30006170403`; the squash merge commit is
  `f95571634a9c6528e5834e9589b048ad5197bd15`. All 32 M4C4 rows are
  `owner-accepted`, producing 237 total / 192 owner-accepted / 0 proof-complete /
  45 blocked rows at that acceptance checkpoint. M4C5 subsequently completed
  through the accepted slice above; M4 remains active and incomplete.
- Owner-accepted and squash-merged the M4C3 host-neutral pointer lifecycle in
  [PR #15](https://github.com/dornglut/runen-ui/pull/15): checked
  pointer/device identity and payloads, canonical queued ingress, exact
  validation/recovery, per-pointer pressed/capture ownership, ordered boundary/
  capture notifications, stationary re-hit, terminal-context cleanup, release-
  inside activation, route-only logical scrolling, public Counter/downstream
  proofs, and slice-local causal trace. The accepted feature head
  `01b7ae018abeaff8d316764afba5bc8cde074381` passed exact-head CI run
  `29996101708`; the squash merge commit is
  `2fc165b9386f55c061d61232400375b13ad175bf`. All 28 M4C3 rows are
  `owner-accepted`. M4C4 subsequently completed through the accepted slice above;
  M4 remains active and incomplete.
- Added the deterministic, network-free `cargo xtask audit-repository` command
  with stable human/JSON output, fatal matrix/workspace/dependency/authority/
  license/canonical-runtime checks, non-fatal concentration diagnostics, and
  integration of the fatal subset into `cargo validate`.
- Reconstructed the active M4 execution graph in the public repository, moved
  operational branch authority from `master` to `main`, added durable migration
  history for private issue and pull-request records, and made
  [public issue #3](https://github.com/dornglut/runen-ui/issues/3) the M4 pickup
  authority. At that public cutover M4C3 was the next implementation slice and
  could not begin until its public governance, tooling, and readiness
  prerequisites passed.
- Owner-accepted and squash-merged the M4C2 displayed-generation surface-context
  implementation in
  [archive PR #99](docs/history/public-repository-migration.md#accepted-imported-milestone-history).
  The accepted feature head is
  `8127c6143948354f2820f4779c92d2fa9daf79ca`; the squash merge commit is
  `9dbf2b6bc781b4e29e3e9ce10388742eccc90124`. Added shared-namespace opaque
  `SurfaceId`/`SurfaceInputContext`, fresh coordinate and displayed hit-test
  generations for every publication, configurable bounded immutable historical
  snapshots, exact checked logical/resolved ingress with owned rejection recovery,
  canonical command-queue convergence, and causal surface trace. All 12
  `SURFACE-*` rows are `owner-accepted`. Hosted CI run `29802050579` failed
  before step 1 with no job steps; the owner recorded an explicit infrastructure-
  only waiver after the complete exact-head local baseline and final review passed.
  M4C3 became the next implementation slice; M4 remains incomplete.
- Owner-accepted and squash-merged the complete M4C1 routed semantic-command
  kernel in
  [archive PR #77](docs/history/public-repository-migration.md#accepted-imported-milestone-history).
  The accepted feature head is
  `cb3c7c139304231ba3b636cf53951507f348d485`; the squash merge commit is
  `44ceee29c73cea1237fefbd30db4baf2cd97b93d`. All 36 M4C1 conformance rows are
  now `owner-accepted`; at that accepted M4C1 head, M4C2 was queued and M4
  remained active and incomplete.
- Established issue-backed execution tracking through the M4 umbrella and slice
  issues, documented the roadmap/matrix/issues/PR/status authority split, added
  milestone-slice, architecture-debt, and bug issue forms plus a pull-request
  template, and recorded the required contributor pickup sequence. Volatile
  branch, head, blocker, CI, and next-action state now belongs in issues and PRs
  rather than stable architecture documents.
- Implemented the complete M4C1 routed semantic-command kernel: one core-owned
  runtime namespace and mounted/time/work-sequence value authority; narrow
  host-neutral command/event vocabulary; checked open widget event bridge and
  recursive action/work mapping; exact owned command submission recovery;
  foreign/stale/missing distinction; immutable capture/target/bubble routing;
  checked admission; independent propagation/default control; ordered routed,
  default, subscription, and mounted-work commit; semantic `Activate`; and
  route-only cancel/menu/context behavior.
- Corrected routed construction and observation authority before M4C2–M4C5:
  external origins are direct-only, delegated origin construction is confined
  to callback-collected command output, `UiEvent::as_semantic_command` is
  variant-safe, and only the checked erased widget bridge constructs and
  extracts `EventContext` values.
- Made all eight command-submission rejection classes recover exact inputs
  without consuming work or trace sequences, allocating trace records, waking,
  or invoking callbacks. Routed integrity failures now retain accepted work and
  causal facts while distinguishing broken topology, event-bridge mismatch,
  callback-bridge failure, output overflow, semantic-default failure, and commit
  invariant failure.
- Locked routed admission to the actual conservative maximum-safe policy,
  including rejection of a no-output callback when a potentially available
  output family cannot be reserved, and split routing into focused admission,
  transaction, dispatch, default, commit, and failure modules.
- Removed combined input intent and every direct runtime, pointer, and keyboard
  activation authority. Retained pointer/keyboard helpers are focus-only
  negative proofs. Counter now submits exact-target commands, and the genuine
  downstream package proves routed facts, state-only/action-only events,
  non-`Clone` mapped actions, and mapped mounted work through public APIs.
- Extended the sole canonical trace with command acceptance, processing
  rejection, and submission-rejection absence/non-consumption proofs,
  route snapshot and phases, propagation/default controls, widget mutation and
  invalidation, collected outputs, semantic default, admission/poison/commit,
  and causal links into later actions and delegated commands. All 36 M4C1 rows
  are `owner-accepted`; at that accepted M4C1 head, M4C2 was queued,
  M4C3–M4D3 and M5 semantic mapping were blocked, and M4 remained incomplete.
- Owner-accepted the corrected M4C0 documentation and conformance authority,
  made its migration proofs independently closable by their assigned slices,
  reserved `ACCESS-*` for M5 semantic resolution, and at that M4C0 head queued
  M4C1 without adding runtime behavior.
- Accepted the M4B scheduler result from
  [archive PR #75](docs/history/public-repository-migration.md#accepted-imported-milestone-history),
  accepted the M4C delivery charter on 2026-07-18, and normalized M4 conformance ownership in
  documentation-only M4C0.
- Assigned every normative M4 matrix row a permanent unique ID, one primary
  delivery slice, explicit positive/negative/trace proof ownership, an exact
  status, and an M4 gate; accepted M4B rows are `owner-accepted`, while future
  M4C/M4D rows remain `blocked` until their owning slices pass acceptance.
- Split routing, surface, pointer, modality, automation, accessibility, logical
  scrolling, and trace aggregates across M4C1–M4D3 at the documentation-only
  M4C0 head. M4 remains incomplete.
- Removed the C9 wake delivery mutex from the host callback boundary. Wake
  delivery now uses explicit callback-in-flight state: each epoch is claimed
  once under wake-state synchronization, host code runs after every RunenUI
  mutex is released, callbacks remain serialized, and close prevents new claims
  without waiting for an earlier callback. Deterministic transition tests are
  the primary proof; repeated races remain supplementary coverage.
- Closed the final M4B scheduler-integrity correction: terminal host-response
  slots are reclaimed, exact-generation task/subscription/host authority is
  revoked before unmount callbacks, send-subscription startup is
  `Starting -> Running` with exact `NotStarted`, and stale send-task completion
  returns exact ownership.
- Replaced magic trace counts with checked operation-specific admission and made
  enabled-trace action acceptance authoritative; trace capacity zero remains
  behavior-neutral, while unexpected post-mutation commit failures poison.
- Added `WidgetActivationOutput<Action>` and made each wake request epoch
  claimable at most once; M4C1 later removed the transitional direct activation
  result/capacity surface.

- Corrected the M4B mounted activation path so subscription invalidation,
  primary action, and auxiliary exact-owner work commit through one ordered
  transaction plan. The plan preflights queue, work-sequence, generation, and
  family capacity; invalidates exact cancellation targets at commit; and
  installs starts only after acceptance.
- Removed retained application and mounted subscription declaration values.
  Application declarations are evaluated from current post-update state inside
  the transaction, while mounted declarations are evaluated only when their
  queued exact-owner reconciliation envelope reaches the front; stale owners
  suppress the callback and allocate no work.
- Replaced unconditionally polled subscription closures with wake-aware local
  sources and start-once send producers. Send startup now reports structured
  started/unavailable/full/closed/rejected outcomes, and sink submission returns
  the exact item for full, closed, or stale rejection without hidden retry.
- Reclaimed work-registry records immediately on completion, cancellation, or
  refusal; bounded retained subscription diagnostics through `RuntimeLimits`;
  unified terminal/shutdown producer closure; and changed detached host
  responses so only successful bounded-ingress submission reserves a request.
- Added one scheduler callback-acceptance preflight, corrected send-executor
  publication so only accepted jobs become running, made repeating-deadline
  overflow an explicit non-poisoning timer terminal outcome, and attached
  read-only owner/family/generation/key identity plus structured outcomes to
  mandatory scheduler trace facts.
- Removed redundant action-bearing completion envelopes: accepted local/send
  task and subscription results, timer firings, host responses, and typed start
  failures now map directly to one final application-action envelope. Host
  cancellation sequence exhaustion now terminalizes and closes all scheduling
  authority, while queue saturation remains recoverable.
- Attached actual accepted `WorkSequence` values and causal parents to scheduler
  facts from application transaction through request, generation commit, start,
  completion/firing/cancellation, and final action. Full trace-v2 normalization,
  export, sinks, redaction, and replay remain M4D.
- Completed the corrective M4B conformance gates, subsequently owner-accepted
  and squash-merged; M4C runtime and M4D remain blocked in their accepted
  sequence. This does not claim routed events or full M4 completion.
- Implemented the M4B core-owned `UiApp`/host protocol, ordered effects,
  application and mounted complete-set subscriptions, generational keyed work,
  local/send tasks, deterministic timers, typed host requests, configured live
  limits, four-budget readiness pump, atomic wake, revisioned redraw,
  lifecycle/poison shutdown integrity, and scheduler trace facts. M4B is now
  accepted and merged; routed events remain M4C and trace export/replay M4D.
- Corrected M4A capacity handling so configured queue and canonical-trace
  capacities remain logical saturation limits while internal storage grows only
  with accepted envelopes and retained records. Added exact stable/MSRV proofs
  for `usize::MAX` logical capacities, queue-full activation trace isolation and
  work-sequence preservation, activation work-sequence exhaustion, trace
  exhaustion during pumping, and repeated trace-watermark advancement. Aligned
  the accepted M4 authority wording, public API guarantees, workspace ownership,
  and active documentation inventory.
- Established the bounded M4A canonical application-action FIFO, non-wrapping
  work sequencing, explicit processed-envelope pump, and explicit saturation,
  terminal, cancellation, and idempotent shutdown outcomes. Removed direct
  dispatch authority, cut proof activation over to queued actions, replaced
  one-shot `on_press` with repeatable non-`Clone` `on_activate` factories, and
  replaced duplicated unbounded tracing with one bounded canonical sequence and
  exclusive eviction watermark. That foundation is now extended by M4B;
  routed events, sinks, export, replay, and complete M4 support remain pending.
- Accepted the M4 routed event and semantic-command architecture with exact
  core/runtime ownership, safe namespace-based opaque identities, the normative
  event-family policy, retained displayed-generation surface input, observable
  target/current-target/phase facts, immutable routes, non-reentrant propagation,
  exact transition/output ordering, pointer identity/capture, deterministic
  multi-pointer geometry revalidation, integrity-only terminal pointer cleanup
  for unavailable snapshots, focus scopes, exact no-action defaults for
  unconsumed route-only commands, separate keyboard/text/IME streams, semantic
  `on_activate`, and release-inside activation.
- Accepted the M4 deterministic action/effect scheduling architecture with
  `initial_effects`, two-argument no-effects updates, state-derived application
  subscriptions, dedicated complete-set mounted subscription declarations,
  commit-bound owner-local keyed cancellation/replacement, lifecycle-owned
  local/send tasks with one-attempt terminal executor refusal, exact readiness
  checkpoints and four pump budgets, timers, an exact response-kind-validated
  host protocol, configured saturation outcomes, race-free wake/redraw
  acknowledgment, terminal integrity policy, one bounded structured trace v2,
  and bounded/try-based subordinate sink delivery.
- Added a normative M4 conformance matrix covering public downstream routing,
  modality/command convergence, focus/capture/composition ordering, startup and
  application and mounted subscription work, task/timer/host/executor-refusal
  behavior, cancellation, queue/wake races, sink backpressure, saturation, trace
  causality, and shutdown, plus a 20-vector directional-focus corpus that freezes
  public outcomes without exposing the private score.
- Replaced transient preorder runtime authority with a private persistent
  generational mounted tree and separate runtime-local mounted/semantic IDs.
- Added exact sibling-local keyed reconciliation, unkeyed ordinal matching,
  duplicate-key no-reuse diagnostics, cross-parent remounting, deterministic
  reconciliation generations, and lifetime-based reports.
- Replaced the provisional M2 lifecycle seam with state-aware widget
  capabilities, mounted lifecycle/activation contexts, explicit unmount reasons,
  persistent interaction slots, checked erasure fallbacks, and idempotent
  shutdown through `into_state` and `Drop`.
- Moved focus, activation, input targets, measurement requests, trace targets,
  and aligned frame/style/layout publication to `MountedNodeId`; stale and
  foreign targets are now distinct and focus survives compatible rebuilds.
- Added selective `WidgetInvalidation`, integrity-aware capability caches, and
  one topology-aligned whole-surface publication cache. Topology snapshots
  retain structural/alignment facts only; compatible style and layout phases
  read current mounted authored values, including token-reference and gap
  changes. Structural changes rebuild every node-aligned fact, style-token
  context compatibility compares exact token content, measurement compatibility
  uses the provider's explicit identity/revision promise, and private test probes
  count actual phase entry independently from `SurfacePhaseReport` bookkeeping.
- Added generation-capacity preflight before every mutable activation,
  transactional compatible update with immediate mismatch replacement,
  arena-live unmount ordering, immediate state-only focus validation, and
  non-default interaction-slot retention/reset proofs across compatible update,
  dispatch, state-only activation, removal, cross-parent remount, generational
  arena reuse, replacement, and shutdown.
- Replaced reconciliation `Vec<String>` diagnostics with structured duplicate
  sibling-key and state-payload-mismatch values containing deterministic paths.
- Removed un-emitted `NodeMounted`, `NodeUpdated`, and `NodeUnmounted` trace
  variants; trace v2 remains an M4 boundary.
- Split built-in authoring, mounted matching/lifecycle/cache/invalidation/
  interaction/diagnostics, and surface context/cache ownership into focused
  modules.
- Removed `RuntimeNodeId`, `RuntimeNodeRef`, `RuntimeTreeIndex`, `WidgetState`,
  the old lifecycle vocabulary, direct transient capability/action execution,
  and free element publication without compatibility aliases.
- Expanded downstream conformance and Counter proofs for retained state,
  identity, focus, lifecycle/shutdown, invalidation/cache reuse, publication
  alignment, and stale/foreign target safety.
- Completed M2 with an open downstream `Widget<Action>` protocol, safe private
  erasure, process-local widget/state type identity, checked lifecycle/state
  conformance, typed recursive component action mapping, and open proof-level
  event/layout/paint/semantic/diagnostic participation.
- Migrated text, button, container, Counter, traversal, focus, activation,
  layout, surface publication, and debug output off `ElementKind`; removed
  `ElementKind`, built-in element views, `IntoElement`, `IntoElements`, and
  `SurfaceNodeKind` without compatibility aliases.
- Added a non-publishable downstream conformance package that implements and
  interacts with a stateful custom control and child-bearing custom container
  entirely through public APIs on stable and Rust 1.93.0.
- Corrected lifecycle mismatches to distinguish widget type, state type, and
  erased-payload failures with truthful expected/actual accessors.
- Separated public built-in authored views from private behavior-only text,
  button, and linear-container widget implementations; compile-fail proofs
  prevent built-in builders from entering `Element::new` and losing common
  configuration.
- Replaced `ChildBearingWidget`, `Element::with_children`, and
  `WidgetMeasure::Container` with `ChildLayoutWidget`, `ChildLayout`, and the
  canonical `Container<Action>`/`container` authored path shared by row, column,
  and downstream child-layout widgets.
- Replaced hidden `RefCell<Option<Action>>` consumption through `&self` with
  explicit mutable one-shot extraction while retaining non-`Clone` actions and
  immediate successful-dispatch rebuilds.
- Snapshotted every widget measurement capability once per node/publication,
  independently snapshotted child layout once per child-bearing node, combined
  intrinsic/child minimums, added descendant-preserving unsupported/version-skew
  fallbacks, aligned index/frame/style/layout products, and renamed runtime
  `ButtonLabel` to `ControlLabel` without an alias.
- Clarified that stateless widgets explicitly declare/create `()` state and that
  M2 proves state identity/lifecycle compatibility only; M3 owns the breaking
  state-aware mounted behavior contract.
- Completed M1 public API and vocabulary repair with validated logical values,
  IDs/keys/token IDs, deterministic duplicate diagnostics, typed element builders,
  arity-free children, reduced preludes, and read-only generated products.
- Replaced the nested `element!` property grammar with ordinary builder
  expressions and canonical `on_press`; removed prototype compatibility APIs.
- Restricted `Action: Clone` to activation paths and documented public enum/trait
  evolution policy.
- Corrected M1 identity to compare, order, and hash by Unicode-validated text
  independent of static/owned storage; literal and dynamic validation now share
  one grammar, and token families are explicitly non-exhaustive.
- Corrected derived geometry to saturate at finite boundaries and identity
  diagnostics to use true numeric preorder with deterministic same-node ordering.
- Reframed RunenUI truthfully as a pre-1.0 headless architecture proof with
  required headless, desktop, and embedded production profiles.
- Added canonical status, support, architecture, documentation-retention, and
  M0–M12 roadmap authorities.
- Archived the historical Runenwerk UI tree at
  `legacy-runenwerk-ui-archive-2026-07-11` and removed it from active content.
- Consolidated incremental architecture documents into durable styling, layout,
  event/effect, ADR, and history records.
- Reset workspace packages from `1.0.0` to `0.1.0` and disabled publication.
- Established dual MIT/Apache-2.0 licensing, governance, toolchain, stability,
  release, and validation policies.

No stable release has been published.
