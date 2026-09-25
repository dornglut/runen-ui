# M11 Conformance Matrix

> **Category:** Target architecture
>
> **Status:** M11A owner-accepted
>
> **Milestone:** M11
>
> This matrix adds only standard-control observations. M4 remains authoritative
> for routed input, focus, pointer capture, keyboard defaults, controller command
> ingress, and application action ordering. M5 remains authoritative for semantic
> identity/publication/actions and public deterministic testing. M8 remains
> authoritative for text measurement/layout. M9 remains authoritative for
> interaction style facts, transitions, and paint/hit/focus/semantic correlation.
> M10 remains authoritative for editable text and interaction services.

```text
5 total unique rows
5 owner-accepted
0 implementation-complete
0 proof-complete
0 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M11CTRL-01 | Public `Text` is the standard static text/label built-in: it uses the production text measurement/publication path, publishes ordinary text semantics, accepts common-node authoring, and remains non-actionable without a hidden control runtime. | `crates/runenui_testing/tests/m11a_controls.rs::text_is_static_semantic_content_without_activation`; accepted M8 text/layout proofs | actionable-text query must remain empty; no second text/control authority audit | Existing M8 text artifact/publication diagnostics and M5 semantic publication | M11A | owner-accepted | Required |
| M11CTRL-02 | Public `Button<Action>` activation converges through the same canonical semantic default and application-action path for semantic/accessibility, pointer, Enter keyboard, automation, programmatic, and normalized controller origins. | `crates/runenui_testing/tests/m5e_counter_origins.rs::counter_converges_semantic_pointer_keyboard_automation_programmatic_and_controller_activation`; `examples/reference_winit/tests/controller_input.rs` | Accepted M4 stale/disabled/non-actionable/prevented activation corpus | Existing M4 command/default/action trace records | M11A | owner-accepted | Required |
| M11CTRL-03 | Enabled/disabled Button state is coherent across activation capability and semantic publication: disabled controls remain representable and queryable but reject activation without committing application state. Repeat enabled activation creates fresh application actions while durable state remains application-owned. | `crates/runenui_testing/tests/m11a_controls.rs` | Disabled semantic action rejection plus accepted M4/M5 disabled command/focus proofs | Existing semantic action rejection and routed default trace authority | M11A | owner-accepted | Required |
| M11CTRL-04 | Button hover/focus/active/disabled facts use the ordinary M9 interaction-style and transition authorities; standard controls introduce no private style-state machine, animation clock, hit path, or presentation geometry. | Accepted M9C interaction/style integration and existing built-in Button use in focus/pointer tests | Source audit excluding control-private interaction/style/timeline state | Existing M9 interaction/motion trace and publication correlation | M11A | owner-accepted | Required |
| M11CTRL-05 | Text and Button remain ordinary public `View`/`Widget` implementations. Their semantics, layout, hit, focus, accessibility, rendering and application actions remain achievable by downstream widgets through the same public contracts; no built-in-only runtime capability is introduced. | `tests/external_widget` public conformance; Counter deterministic/native/visual reference coverage | Public API/architecture audit excluding built-in type checks and private control registry/runtime | Existing M4–M10 conformance diagnostics and repository architecture guards | M11A | owner-accepted | Required |
