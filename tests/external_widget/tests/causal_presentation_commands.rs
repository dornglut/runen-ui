#![allow(refining_impl_trait)]

//! Public temporal discriminator for #474: an Escape-owned presentation must not
//! redirect its delayed `CancelOrBack` to an unrelated later presentation.

use runenui_core::{
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    Menu, MenuButton, MenuItem, NoHostProtocol, PhysicalKey, StyleEnvironment, UiApp, View, column,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, LogicalSize, PumpBudget, SurfaceBuildContext,
    TraceRecordKind, UiInputFinality,
};

const FULL: PumpBudget = PumpBudget::new(128, 128, 128, 128);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

#[derive(Clone, Copy, Debug)]
enum Action {
    ShowQ,
    HideQ,
    HideP,
    ShowP,
    DismissP,
    DismissQ,
    Noop,
}

#[derive(Clone, Debug)]
struct State {
    p_open: bool,
    q_open: bool,
    p_dismissals: usize,
    q_dismissals: usize,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> impl View<Action> {
        let mut p = MenuButton::new("P", state.p_open).id("p-trigger");
        if state.p_open {
            p = p.with_submenu(
                Menu::new(vec![
                    MenuItem::new("P item")
                        .id("p-item")
                        .on_activate(|| Action::Noop)
                        .into_element(),
                ])
                .id("p-menu")
                .on_dismiss(|_| Action::DismissP),
                true,
            );
        }
        let mut children = vec![p.into_element()];
        if state.q_open {
            children.push(
                MenuButton::new("Q", true)
                    .id("q-trigger")
                    .with_submenu(
                        Menu::new(vec![
                            MenuItem::new("Q item")
                                .id("q-item")
                                .on_activate(|| Action::Noop)
                                .into_element(),
                        ])
                        .id("q-menu")
                        .on_dismiss(|_| Action::DismissQ),
                        true,
                    )
                    .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut State, action: Action) {
        match action {
            Action::ShowQ => state.q_open = true,
            Action::HideQ => state.q_open = false,
            Action::HideP => state.p_open = false,
            Action::ShowP => state.p_open = true,
            Action::DismissP => {
                state.p_open = false;
                state.p_dismissals += 1;
            }
            Action::DismissQ => {
                state.q_open = false;
                state.q_dismissals += 1;
            }
            Action::Noop => {}
        }
    }
}

#[test]
fn delayed_escape_from_p_must_not_dismiss_newly_published_q() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(State {
        p_open: true,
        q_open: false,
        p_dismissals: 0,
        q_dismissals: 0,
    });
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("P publication"));
    assert!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("initial focus"))
            .report()
            .is_quiescent()
    );
    let p_item = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "p-item"))
        .unwrap_or_else(|| unreachable!("published P item"))
        .id()
        .clone();
    assert_eq!(runtime.focus().focused_node(), Some(&p_item));
    let p_origin_root = authored_node(&mut runtime, "p-menu");

    let escape = KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Escape,
        LogicalKey::Escape,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    );
    let receipt = runtime
        .submit_keyboard(escape)
        .unwrap_or_else(|_| unreachable!("focused Escape admission"));
    runtime
        .submit_action(Action::ShowQ)
        .unwrap_or_else(|_| unreachable!("Q appearance accepted behind Escape"));

    let key_batch = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one keyboard envelope"));
    assert_eq!(key_batch.report().processed_envelopes(), 1);
    assert!(key_batch.ordered_records().iter().any(|record| {
        matches!(
            record,
            InputArbitrationRecord::InputSettled(settlement)
                if settlement.sequence() == receipt.sequence()
                    && matches!(settlement.finality(), UiInputFinality::Committed(_))
        )
    }));
    assert!(!runtime.state().q_open);

    let action_batch = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one Q application action"));
    assert_eq!(action_batch.report().processed_envelopes(), 1);
    assert!(runtime.state().p_open);
    assert!(runtime.state().q_open);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("Q publication precedes delayed default"));
    assert_eq!(
        authored_node(&mut runtime, "p-menu"),
        p_origin_root,
        "C4 requires an unchanged, still-published P root, not a replaced P"
    );
    let _q_root = authored_node(&mut runtime, "q-menu");

    assert!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("drain delayed semantic command"))
            .report()
            .is_quiescent()
    );
    assert!(
        runtime.state().q_open,
        "an Escape admitted under P must not dismiss unrelated Q published before its delayed CancelOrBack"
    );
    assert_eq!(
        runtime.state().q_dismissals,
        0,
        "Q must not receive a P-origin dismissal"
    );
    assert_eq!(runtime.state().p_dismissals, 0);
    assert_unclaimed_cancel_trace(&runtime);
}

fn assert_unclaimed_cancel_trace(runtime: &AppRuntime<App>) {
    let decisions = runtime
        .trace()
        .records()
        .filter_map(|record| match record.kind() {
            TraceRecordKind::PresentationCancelOrBackDecision { claimed } => Some(*claimed),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        decisions,
        [false],
        "a stale P-bound default must not be falsely traced as a presentation claim"
    );
}

// All tests below are external downstream consumers of the public runtime.
// The application remains the only durable owner of visibility and dismissals.
const fn initial_state() -> State {
    State {
        p_open: true,
        q_open: false,
        p_dismissals: 0,
        q_dismissals: 0,
    }
}

const fn escape_event(composition: KeyboardCompositionState) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Escape,
        LogicalKey::Escape,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        composition,
        None,
    )
}

fn authored_node(runtime: &mut AppRuntime<App>, authored: &str) -> runenui_core::MountedNodeId {
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored descendant must exist"))
        .id()
        .clone()
}

fn publish(runtime: &mut AppRuntime<App>, context: &SurfaceBuildContext<'_>) {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("surface publication"));
}

fn pump_one(runtime: &mut AppRuntime<App>) {
    let batch = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one envelope"));
    assert_eq!(batch.report().processed_envelopes(), 1);
}

fn pump_full(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("bounded drain"))
            .report()
            .is_quiescent()
    );
}

#[test]
fn c2_stable_p_origin_dismisses_once_only_after_queued_action() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(initial_state());
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    runtime
        .submit_keyboard(escape_event(KeyboardCompositionState::Inactive))
        .unwrap_or_else(|_| unreachable!("P keyboard admission"));
    assert!(runtime.state().p_open);
    pump_one(&mut runtime);
    assert!(runtime.state().p_open, "input only queues a later command");
    pump_one(&mut runtime);
    assert!(
        runtime.state().p_open,
        "command only queues a later dismissal"
    );
    pump_full(&mut runtime);
    assert!(!runtime.state().p_open);
    assert_eq!(runtime.state().p_dismissals, 1);
    assert_eq!(runtime.state().q_dismissals, 0);
}

#[test]
fn c3_retired_p_republished_before_delayed_command_never_dismisses_successor() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(initial_state());
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    runtime
        .submit_keyboard(escape_event(KeyboardCompositionState::Inactive))
        .unwrap_or_else(|_| unreachable!("P keyboard admission"));
    runtime
        .submit_action(Action::HideP)
        .unwrap_or_else(|_| unreachable!("hide P"));
    runtime
        .submit_action(Action::ShowP)
        .unwrap_or_else(|_| unreachable!("show successor P"));
    pump_one(&mut runtime); // input -> command queued behind both actions
    pump_one(&mut runtime); // retire P
    publish(&mut runtime, &context);
    pump_one(&mut runtime); // recreate P with the same authored IDs
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    assert!(runtime.state().p_open);
    assert_eq!(runtime.state().p_dismissals, 0);
}

#[test]
fn c5_transient_q_no_longer_topmost_allows_retained_p_to_dismiss() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(initial_state());
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    runtime
        .submit_keyboard(escape_event(KeyboardCompositionState::Inactive))
        .unwrap_or_else(|_| unreachable!("P keyboard admission"));
    runtime
        .submit_action(Action::ShowQ)
        .unwrap_or_else(|_| unreachable!("show Q"));
    runtime
        .submit_action(Action::HideQ)
        .unwrap_or_else(|_| unreachable!("hide Q"));
    pump_one(&mut runtime); // P's input emits delayed CancelOrBack
    pump_one(&mut runtime); // Q appears
    publish(&mut runtime, &context);
    assert!(runtime.state().q_open);
    pump_one(&mut runtime); // Q disappears before the delayed default
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    assert_eq!(runtime.state().p_dismissals, 1);
    assert!(!runtime.state().p_open);
    assert_eq!(runtime.state().q_dismissals, 0);
}

#[test]
fn c8_composing_keyboard_escape_does_not_derive_a_presentation_dismissal() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(initial_state());
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    let commands_before = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted))
        .count();
    runtime
        .submit_keyboard(escape_event(KeyboardCompositionState::Active))
        .unwrap_or_else(|_| unreachable!("composition-owned Escape"));
    pump_full(&mut runtime);
    assert!(runtime.state().p_open);
    assert_eq!(runtime.state().p_dismissals, 0);
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted))
            .count(),
        commands_before,
        "composition-owned Escape must not submit a delayed semantic command"
    );
}

#[test]
fn c10_pending_focus_entry_precedes_escape_and_restores_original_trigger() {
    use runenui_core::{CommandOrigin, SemanticCommand};
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut state = initial_state();
    state.p_open = false;
    let mut runtime = AppRuntime::<App>::mount(state);
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    let trigger = authored_node(&mut runtime, "p-trigger");
    runtime
        .submit_command(
            trigger.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus original trigger"));
    pump_full(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&trigger));

    runtime
        .submit_action(Action::ShowP)
        .unwrap_or_else(|_| unreachable!("open P through app action"));
    pump_full(&mut runtime);
    publish(&mut runtime, &context); // queue r, a known P-focus entry
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&trigger),
        "published entry request must not force immediate focus"
    );
    let receipt = runtime
        .submit_keyboard(escape_event(KeyboardCompositionState::Inactive))
        .unwrap_or_else(|_| unreachable!("Escape e admitted behind pending entry r"));
    pump_one(&mut runtime); // r commits before e is dispatched
    let p_item = authored_node(&mut runtime, "p-item");
    assert_eq!(runtime.focus().focused_node(), Some(&p_item));
    let keyboard = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("keyboard after pending entry"));
    assert_eq!(keyboard.report().processed_envelopes(), 1);
    assert!(keyboard.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::InputSettled(settlement)
            if settlement.sequence() == receipt.sequence()
                && matches!(settlement.finality(), UiInputFinality::Committed(_))
    )));

    pump_full(&mut runtime);
    assert!(!runtime.state().p_open);
    assert_eq!(runtime.state().p_dismissals, 1);
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&trigger),
        "M11 EnterAndRestore must recover the exact original trigger"
    );
}

#[test]
fn c9_direct_unbound_cancel_uses_current_topmost_and_no_presentation_fallback() {
    use runenui_core::{CommandOrigin, SemanticCommand};
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut state = initial_state();
    state.q_open = true;
    let mut runtime = AppRuntime::<App>::mount(state);
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    let routing_target = authored_node(&mut runtime, "q-item");
    runtime
        .submit_command(
            routing_target,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("unbound direct CancelOrBack"));
    pump_full(&mut runtime);
    assert!(runtime.state().p_open);
    assert!(!runtime.state().q_open);
    assert_eq!(runtime.state().p_dismissals, 0);
    assert_eq!(runtime.state().q_dismissals, 1);

    // With no eligible presentation, the same direct command is not transformed
    // into a dismissal. It still runs the existing routed semantic-default path.
    let mut empty = AppRuntime::<App>::mount(State {
        p_open: false,
        q_open: false,
        p_dismissals: 0,
        q_dismissals: 0,
    });
    publish(&mut empty, &context);
    pump_full(&mut empty);
    let target = authored_node(&mut empty, "p-trigger");
    empty
        .submit_command(
            target,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("unbound command without presentation"));
    pump_full(&mut empty);
    assert_eq!(empty.state().p_dismissals, 0);
    assert_eq!(empty.state().q_dismissals, 0);
}

#[test]
fn c11_causal_finality_survives_trace_disabled_and_zero_budget() {
    use runenui_runtime::{RuntimeConfig, TraceConfig};
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount_with_config(
        initial_state(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    );
    publish(&mut runtime, &context);
    pump_full(&mut runtime);
    let scope = runtime
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("input scope exists"))
        .scope()
        .clone();
    let receipt = runtime
        .submit_keyboard(escape_event(KeyboardCompositionState::Inactive))
        .unwrap_or_else(|_| unreachable!("Escape admitted with trace disabled"));
    runtime
        .submit_action(Action::ShowQ)
        .unwrap_or_else(|_| unreachable!("later Q action"));
    let zero = runtime
        .pump(PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("zero-budget checkpoint"));
    assert_eq!(zero.report().processed_envelopes(), 0);
    assert!(
        !zero
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
    );
    let keyboard = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("exact keyboard envelope"));
    let settlements = keyboard
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(value) => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].scope(), &scope);
    assert_eq!(settlements[0].sequence(), receipt.sequence());
    assert!(matches!(
        settlements[0].finality(),
        UiInputFinality::Committed(_)
    ));
    pump_one(&mut runtime);
    publish(&mut runtime, &context);
    let later = runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("derived command and later work"));
    assert!(later.report().is_quiescent());
    assert!(
        !later
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
    );
    assert_eq!(runtime.trace().len(), 0);
    assert!(runtime.state().q_open);
    assert_eq!(runtime.state().p_dismissals, 0);
    assert_eq!(runtime.state().q_dismissals, 0);
}

#[test]
fn c12_keyboard_escape_with_no_eligible_origin_does_not_dismiss_later_q() {
    use runenui_core::{CommandOrigin, SemanticCommand};

    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(State {
        p_open: false,
        q_open: false,
        p_dismissals: 0,
        q_dismissals: 0,
    });
    publish(&mut runtime, &context);
    pump_full(&mut runtime);

    // A real focused keyboard recipient exists, but no presentation is eligible
    // to own the forthcoming Escape at its actual default-derivation boundary.
    let trigger = authored_node(&mut runtime, "p-trigger");
    runtime
        .submit_command(
            trigger.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus ordinary trigger"));
    pump_full(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&trigger));

    let scope = runtime
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("input scope"))
        .scope()
        .clone();
    let receipt = runtime
        .submit_keyboard(escape_event(KeyboardCompositionState::Inactive))
        .unwrap_or_else(|_| unreachable!("focused Escape admission"));
    runtime
        .submit_action(Action::ShowQ)
        .unwrap_or_else(|_| unreachable!("Q action admitted after Escape"));

    let keyboard = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one keyboard envelope"));
    assert_eq!(keyboard.report().processed_envelopes(), 1);
    let settled = keyboard
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settlement) => Some(settlement),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settled.len(), 1);
    assert_eq!(settled[0].scope(), &scope);
    assert_eq!(settled[0].sequence(), receipt.sequence());
    assert!(matches!(
        settled[0].finality(),
        UiInputFinality::Committed(_)
    ));
    assert!(!runtime.state().q_open);

    let action = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one later Q action"));
    assert_eq!(action.report().processed_envelopes(), 1);
    assert!(runtime.state().q_open);
    assert!(!runtime.state().p_open);
    publish(&mut runtime, &context);
    let q_root = authored_node(&mut runtime, "q-menu");
    assert_eq!(runtime.state().q_dismissals, 0);

    let command = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one delayed keyboard-derived command"));
    assert_eq!(command.report().processed_envelopes(), 1);
    assert!(
        !command
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
    );
    let rest = runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("drain noncausal work"));
    assert!(rest.report().is_quiescent());
    assert!(
        !rest
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
    );
    assert!(runtime.state().q_open);
    assert!(!runtime.state().p_open);
    assert_eq!(runtime.state().q_dismissals, 0);
    assert_eq!(runtime.state().p_dismissals, 0);
    assert_unclaimed_cancel_trace(&runtime);
    assert!(
        runtime.trace().records().any(|record| matches!(
            record.kind(),
            TraceRecordKind::SemanticDefaultApplied { command }
                if *command == SemanticCommand::CancelOrBack
        )),
        "unclaimed keyboard cancel must retain the ordinary nonpresentation default"
    );

    // The same Q is still eligible for a genuinely unbound *direct* global
    // command. This must continue to use execution-time topmost selection.
    let direct_target = authored_node(&mut runtime, "q-item");
    assert_eq!(authored_node(&mut runtime, "q-menu"), q_root);
    runtime
        .submit_command(
            direct_target,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("ordinary direct global command"));
    pump_one(&mut runtime);
    assert_eq!(runtime.state().q_dismissals, 0);
    pump_full(&mut runtime);
    assert!(!runtime.state().q_open);
    assert_eq!(runtime.state().q_dismissals, 1);
    assert_eq!(runtime.state().p_dismissals, 0);
    let decisions = runtime
        .trace()
        .records()
        .filter_map(|record| match record.kind() {
            TraceRecordKind::PresentationCancelOrBackDecision { claimed } => Some(*claimed),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(decisions, [false, true]);
}
