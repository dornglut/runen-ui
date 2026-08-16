use core::num::NonZeroUsize;
use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, ElementId, FocusReason, FocusScope, IntoEffects,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    NoHostProtocol, PhysicalKey, SemanticCommand, UiApp, View, column, container,
};
use runenui_external_widget_conformance::{
    ExternalFocusFact, ExternalFocusWidget, ExternalInputAction, ExternalInputAncestor,
    ExternalInputFact, ExternalInputKind, ExternalInputWidget, external_focus_panel,
};
use runenui_runtime::{MountedNodeId, PumpBudget, SurfaceFrame};
use runenui_testing::{SettleBudget, SettleOutcome, TestHarness};

fn settle_budget() -> SettleBudget {
    SettleBudget::new(NonZeroUsize::MIN, PumpBudget::new(64, 64, 64, 64))
}

fn authored_node(frame: &SurfaceFrame, authored: &str) -> Option<MountedNodeId> {
    let Ok(authored) = ElementId::new(authored) else {
        return None;
    };
    frame
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .map(|node| node.id().clone())
}

struct FocusState {
    log: Rc<RefCell<Vec<ExternalFocusFact>>>,
}

struct FocusApp;

impl UiApp for FocusApp {
    type State = FocusState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        column(vec![
            external_focus_panel(Rc::clone(&state.log)).focus_scope(FocusScope::new()),
            Element::new(ExternalFocusWidget::new(
                "outside",
                Rc::clone(&state.log),
                true,
            ))
            .id("focus.outside")
            .key("outside")
            .focusable(true),
        ])
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn public_harness_drives_downstream_controller_traversal_and_remembered_restoration() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut harness = TestHarness::<FocusApp>::mount(FocusState {
        log: Rc::clone(&log),
    });

    let Some((root, a, b, outside)) = (|| {
        let publication = harness.publish().ok()?;
        Some((
            authored_node(publication.frame(), "focus.root")?,
            authored_node(publication.frame(), "focus.a")?,
            authored_node(publication.frame(), "focus.b")?,
            authored_node(publication.frame(), "focus.outside")?,
        ))
    })() else {
        return;
    };

    assert!(
        harness
            .submit_command(
                a.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.focus().focused_node(), Some(&a));

    assert!(
        harness
            .submit_command(a, SemanticCommand::FocusNext, CommandOrigin::controller())
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.focus().focused_node(), Some(&b));

    assert!(
        harness
            .submit_command(
                outside.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.focus().focused_node(), Some(&outside));

    log.borrow_mut().clear();
    assert!(
        harness
            .submit_command(root, SemanticCommand::RestoreFocus, CommandOrigin::controller())
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.focus().focused_node(), Some(&b));
    assert_eq!(
        harness.focus().reason(),
        Some(FocusReason::RememberedRestoration)
    );
    assert!(!log.borrow().is_empty());
    assert!(harness.trace_replay().is_ok());
}

#[derive(Debug)]
enum InputAction {
    Input(ExternalInputAction),
}

#[derive(Debug)]
struct InputState {
    facts: Rc<RefCell<Vec<ExternalInputFact>>>,
    actions: Vec<ExternalInputAction>,
}

struct InputApp;

impl UiApp for InputApp {
    type State = InputState;
    type Action = InputAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        container(
            ExternalInputAncestor::new(Rc::clone(&state.facts)),
            vec![
                Element::new(ExternalInputWidget::new(Rc::clone(&state.facts)))
                    .id("external.input")
                    .key("input")
                    .focusable(true)
                    .map_action(InputAction::Input),
            ],
        )
        .id("external.input.root")
        .key("input-root")
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            InputAction::Input(action) => state.actions.push(action),
        }
    }
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
fn public_harness_delivers_downstream_keyboard_text_and_composition_ingress() {
    let facts = Rc::new(RefCell::new(Vec::new()));
    let mut harness = TestHarness::<InputApp>::mount(InputState {
        facts: Rc::clone(&facts),
        actions: Vec::new(),
    });

    let Some(target) = (|| {
        let publication = harness.publish().ok()?;
        authored_node(publication.frame(), "external.input")
    })() else {
        return;
    };
    assert!(
        harness
            .submit_command(
                target,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    facts.borrow_mut().clear();

    assert!(harness.submit_keyboard(keyboard()).is_ok());
    let Ok(text) = CommittedTextEvent::new("é", None) else {
        return;
    };
    assert!(harness.submit_text(text).is_ok());

    let Ok(first) = harness.start_composition(None) else {
        return;
    };
    assert!(
        harness
            .submit_composition_update(first.generation().clone(), String::from("pré"), None)
            .is_ok()
    );
    assert!(
        harness
            .submit_composition_end(first.generation().clone())
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );

    let Ok(second) = harness.start_composition(None) else {
        return;
    };
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert!(
        harness
            .cancel_composition(second.generation().clone())
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );

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
        assert!(facts.iter().any(|fact| fact.kind() == &kind));
    }
    drop(facts);

    assert!(harness.state().actions.iter().any(|action| matches!(
        action,
        ExternalInputAction::Observed(ExternalInputKind::Keyboard(KeyboardPhase::Down))
    )));
    assert!(harness.state().actions.iter().any(|action| matches!(
        action,
        ExternalInputAction::Observed(ExternalInputKind::CommittedText {
            bytes: 2,
            scalars: 1
        })
    )));
    assert!(harness.trace_replay().is_ok());
}
