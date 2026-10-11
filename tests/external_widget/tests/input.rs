#![allow(refining_impl_trait)]

#[path = "input_arbitration_classification.rs"]
mod arbitration_classification;

#[path = "input_arbitration_finality.rs"]
mod arbitration_finality;

#[path = "input_arbitration_menu.rs"]
mod arbitration_menu;

#[path = "input_arbitration_pointer_route.rs"]
mod arbitration_pointer_route;

#[path = "input_arbitration_controls.rs"]
mod arbitration_controls;

#[path = "input_arbitration_slot_families.rs"]
mod arbitration_slot_families;

#[path = "input_arbitration_text_boundary.rs"]
mod arbitration_text_boundary;

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, EventContext, EventPhase, InputDeviceId,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    NoHostProtocol, PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, button, container,
};
use runenui_external_widget_conformance::{
    ExternalInputAction, ExternalInputAncestor, ExternalInputFact, ExternalInputKind,
    ExternalInputWidget,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, PumpBudget, RuntimeConfig, TraceConfig, TraceRecordKind,
    UiDefaultDisposition, UiInputClaimReason, UiInputConflict, UiInputFamily, UiInputFinality,
};

#[derive(Debug)]
enum Action {
    Input(ExternalInputAction),
}

#[derive(Debug)]
struct State {
    facts: Rc<RefCell<Vec<ExternalInputFact>>>,
    actions: Vec<ExternalInputAction>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        container(
            ExternalInputAncestor::new(Rc::clone(&state.facts)),
            vec![
                Element::new(
                    ExternalInputWidget::new(Rc::clone(&state.facts))
                        .prevent_keyboard(true)
                        .prevent_text(true),
                )
                .id("external.input")
                .key("input")
                .focusable(true)
                .map_action(Action::Input),
            ],
        )
        .id("external.root")
        .key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Input(action) => state.actions.push(action),
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
}

fn target(runtime: &mut AppRuntime<App>) -> runenui_runtime::MountedNodeId {
    let authored =
        runenui_core::ElementId::new("external.input").unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("downstream input target is mounted"))
        .id()
        .clone()
}

fn focus(runtime: &mut AppRuntime<App>) {
    let target = target(runtime);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("public focus command is accepted"));
    settle(runtime);
}

fn keyboard() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("KeyA")),
        LogicalKey::Character(String::from("a")),
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
#[allow(clippy::assert_is_empty)]
fn downstream_widget_uses_only_public_keyboard_text_and_composition_protocols() {
    let facts = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        facts: Rc::clone(&facts),
        actions: Vec::new(),
    });
    settle(&mut runtime);
    focus(&mut runtime);
    facts.borrow_mut().clear();

    runtime
        .submit_keyboard(keyboard())
        .unwrap_or_else(|_| unreachable!("public raw keyboard ingress is accepted"));
    runtime
        .submit_text(
            CommittedTextEvent::new("é", None)
                .unwrap_or_else(|_| unreachable!("nonempty committed text is valid")),
        )
        .unwrap_or_else(|_| unreachable!("public committed text ingress is accepted"));
    let first = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("public composition start is accepted"));
    runtime
        .submit_composition_update(first.generation().clone(), String::from("pré"), None)
        .unwrap_or_else(|_| unreachable!("public pending composition update is accepted"));
    runtime
        .submit_composition_end(first.generation().clone())
        .unwrap_or_else(|_| unreachable!("public pending composition end is accepted"));
    settle(&mut runtime);
    let second = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("retired composition permits a new start"));
    settle(&mut runtime);
    runtime
        .cancel_composition(second.generation().clone())
        .unwrap_or_else(|_| unreachable!("public explicit cancellation is accepted"));
    settle(&mut runtime);

    let facts = facts.borrow();
    for kind in [
        ExternalInputKind::Keyboard(KeyboardPhase::Down),
        ExternalInputKind::CommittedText {
            bytes: 2,
            scalars: 1,
        },
        ExternalInputKind::CompositionStart,
        ExternalInputKind::CompositionUpdate { has_range: false },
        ExternalInputKind::CompositionEnd,
        ExternalInputKind::CompositionCancel(runenui_core::CompositionCancelReason::Explicit),
    ] {
        let routed = facts
            .iter()
            .filter(|fact| fact.kind() == &kind)
            .collect::<Vec<_>>();
        let (deliveries, remainder) = routed.as_slice().as_chunks::<3>();
        assert!(
            remainder.is_empty(),
            "each routed input delivery has exactly three phases"
        );
        assert!(deliveries.iter().all(|delivery| {
            delivery.iter().map(|fact| fact.phase()).eq([
                EventPhase::Capture,
                EventPhase::Target,
                EventPhase::Bubble,
            ])
        }));
    }
    assert!(
        facts
            .iter()
            .filter(|fact| matches!(fact.kind(), ExternalInputKind::CompositionCancel(_)))
            .all(|fact| !fact.default_is_cancelable())
    );
    drop(facts);
    assert!(runtime.state().actions.iter().any(|action| matches!(
        action,
        ExternalInputAction::Observed(ExternalInputKind::Keyboard(KeyboardPhase::Down))
    )));
    assert!(runtime.state().actions.iter().any(|action| matches!(
        action,
        ExternalInputAction::Observed(ExternalInputKind::CommittedText {
            bytes: 2,
            scalars: 1
        })
    )));
    let kinds = runtime.trace().kinds().collect::<Vec<_>>();
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardDefaultPrevented))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::CommittedTextDefaultPrevented))
    );
}

#[test]
fn downstream_text_and_composition_receipts_have_exact_exclusive_ui_finality() {
    let facts = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        facts: Rc::clone(&facts),
        actions: Vec::new(),
    });
    settle(&mut runtime);
    focus(&mut runtime);
    let keyboard = runtime
        .submit_keyboard(keyboard())
        .unwrap_or_else(|_| unreachable!("focused W is admitted"))
        .sequence();
    let text = runtime
        .submit_text(
            CommittedTextEvent::new("é", None)
                .unwrap_or_else(|_| unreachable!("valid Unicode text")),
        )
        .unwrap_or_else(|_| unreachable!("focused text is admitted"))
        .sequence();
    let composition = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is admitted"));
    let pending = runtime
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("pending composition is synchronously visible"));
    assert_eq!(
        pending.keyboard().composition_generation(),
        Some(composition.generation())
    );

    let batch = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("canonical public native input settles"));
    assert!(batch.report().is_quiescent());
    let settled = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settlement) => Some(settlement),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settled.len(), 3);
    for (entry, (sequence, family, reason)) in settled.into_iter().zip([
        (
            keyboard,
            UiInputFamily::Keyboard,
            UiInputClaimReason::TextOwner,
        ),
        (
            text,
            UiInputFamily::CommittedText,
            UiInputClaimReason::TextOwner,
        ),
        (
            composition.sequence(),
            UiInputFamily::Composition,
            UiInputClaimReason::CompositionOwner,
        ),
    ]) {
        assert_eq!(entry.sequence(), sequence);
        assert_eq!(entry.scope(), pending.scope());
        assert_eq!(entry.family(), family);
        let UiInputFinality::Committed(routing) = entry.finality() else {
            unreachable!("accepted focused native family commits");
        };
        assert_eq!(routing.conflict(), UiInputConflict::ExclusiveUi);
        assert!(routing.reasons().contains(&reason));
        assert_eq!(
            entry.ownership_revision(),
            batch.final_ownership().revision()
        );
    }
}

struct SourceQualifiedSpaceApp;

impl UiApp for SourceQualifiedSpaceApp {
    type State = usize;
    type Action = usize;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        button("Space")
            .id("host-space")
            .key("host-space")
            .on_activate(|| 1)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        *state += action;
    }
}

const fn source_space_key(phase: KeyboardPhase, device_id: InputDeviceId) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        PhysicalKey::Space,
        LogicalKey::Space,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        Some(device_id),
    )
}

fn source_space_settlement(
    runtime: &mut AppRuntime<SourceQualifiedSpaceApp>,
    phase: KeyboardPhase,
    device_id: InputDeviceId,
    expected: UiInputConflict,
    owner: Option<InputDeviceId>,
) -> runenui_runtime::UiInputSettlement {
    let receipt = runtime
        .submit_keyboard(source_space_key(phase, device_id))
        .unwrap_or_else(|_| unreachable!("focused native Space is admitted"))
        .sequence();
    let batch = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("one native Space reaches canonical commit"));
    assert_eq!(batch.processed_through(), Some(receipt));
    let settled = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(value) => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settled.len(), 1);
    let value = settled[0];
    assert_eq!(value.sequence(), receipt);
    assert_eq!(value.scope(), batch.final_ownership().scope());
    assert_eq!(value.family(), UiInputFamily::Keyboard);
    assert_eq!(value.device_id(), Some(device_id));
    assert_eq!(
        value.ownership_revision(),
        batch.final_ownership().revision()
    );
    assert_eq!(
        batch
            .final_ownership()
            .keyboard()
            .space_activation_device_id(),
        owner
    );
    assert!(matches!(
        value.finality(),
        UiInputFinality::Committed(facts)
            if facts.conflict() == expected
                && facts.reasons().contains(&UiInputClaimReason::ActivationDefault)
                    == (expected == UiInputConflict::ExclusiveUi)
    ));
    value.clone()
}

#[test]
fn public_space_lifetime_ignores_foreign_device_down_and_up_without_ghost_claim() {
    let mut runtime = AppRuntime::<SourceQualifiedSpaceApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    );
    let budget = PumpBudget::new(32, 32, 32, 32);
    let _ = runtime
        .pump(budget)
        .unwrap_or_else(|_| unreachable!("button fixture mounts"));
    let authored =
        runenui_core::ElementId::new("host-space").unwrap_or_else(|_| unreachable!("static id"));
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("space button exists"))
        .id()
        .clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("host-space focus admitted"));
    let _ = runtime
        .pump(budget)
        .unwrap_or_else(|_| unreachable!("host-space focus commits"));
    let a = InputDeviceId::new(21).unwrap_or_else(|| unreachable!("valid source A"));
    let b = InputDeviceId::new(22).unwrap_or_else(|| unreachable!("valid source B"));
    let down_a = source_space_settlement(
        &mut runtime,
        KeyboardPhase::Down,
        a,
        UiInputConflict::ExclusiveUi,
        Some(a),
    );
    assert!(matches!(
        down_a.finality(),
        UiInputFinality::Committed(facts)
            if facts.default_disposition() == UiDefaultDisposition::Applied
    ));
    let down_b = source_space_settlement(
        &mut runtime,
        KeyboardPhase::Down,
        b,
        UiInputConflict::ObservedNonexclusive,
        Some(a),
    );
    let up_b = source_space_settlement(
        &mut runtime,
        KeyboardPhase::Up,
        b,
        UiInputConflict::ObservedNonexclusive,
        Some(a),
    );
    let up_a = source_space_settlement(
        &mut runtime,
        KeyboardPhase::Up,
        a,
        UiInputConflict::ExclusiveUi,
        None,
    );
    assert!(down_a.sequence() < down_b.sequence());
    assert!(down_b.sequence() < up_b.sequence());
    assert!(up_b.sequence() < up_a.sequence());
    assert_eq!(
        runtime.state(),
        &0,
        "queued activation is not yet committed"
    );
    let finality = up_a.finality();
    assert!(matches!(
        finality,
        UiInputFinality::Committed(facts)
            if facts.default_disposition() == UiDefaultDisposition::Queued
    ));
    assert!(
        runtime
            .pump(budget)
            .unwrap_or_else(|_| unreachable!("later activation work commits"))
            .ordered_records()
            .iter()
            .all(|record| !matches!(record, InputArbitrationRecord::InputSettled(_)))
    );
    assert_eq!(
        runtime.state(),
        &1,
        "exact owning Space release activates once"
    );
}

#[derive(Debug)]
struct NavigationKeyProbe {
    observed: Rc<RefCell<Vec<(PhysicalKey, LogicalKey)>>>,
}

impl Widget<()> for NavigationKeyProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && let UiEvent::Keyboard(event) = event
        {
            self.observed
                .borrow_mut()
                .push((event.physical_key().clone(), event.logical_key().clone()));
        }
        WidgetEventOutput::none()
    }
}

#[derive(Debug)]
struct NavigationState {
    observed: Rc<RefCell<Vec<(PhysicalKey, LogicalKey)>>>,
}

struct NavigationApp;

impl UiApp for NavigationApp {
    type State = NavigationState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(NavigationKeyProbe {
            observed: Rc::clone(&state.observed),
        })
        .id("navigation.key.probe")
        .key("navigation-key-probe")
        .focusable(true)
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn downstream_widget_observes_structured_navigation_key_identities() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<NavigationApp>::mount(NavigationState {
        observed: Rc::clone(&observed),
    });
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    let authored = runenui_core::ElementId::new("navigation.key.probe")
        .unwrap_or_else(|_| unreachable!("fixture authored id is valid"));
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("navigation probe is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("navigation probe focus is accepted"));
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );

    for (physical, logical) in [
        (PhysicalKey::Home, LogicalKey::Home),
        (PhysicalKey::End, LogicalKey::End),
        (PhysicalKey::PageUp, LogicalKey::PageUp),
        (PhysicalKey::PageDown, LogicalKey::PageDown),
    ] {
        runtime
            .submit_keyboard(KeyboardEvent::new(
                KeyboardPhase::Down,
                physical,
                logical,
                KeyModifiers::NONE,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            ))
            .unwrap_or_else(|_| unreachable!("structured navigation key is accepted"));
    }
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );

    assert_eq!(
        observed.borrow().as_slice(),
        &[
            (PhysicalKey::Home, LogicalKey::Home),
            (PhysicalKey::End, LogicalKey::End),
            (PhysicalKey::PageUp, LogicalKey::PageUp),
            (PhysicalKey::PageDown, LogicalKey::PageDown),
        ]
    );
}
