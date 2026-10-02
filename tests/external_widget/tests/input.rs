#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, EventContext, EventPhase, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, container,
};
use runenui_external_widget_conformance::{
    ExternalInputAction, ExternalInputAncestor, ExternalInputFact, ExternalInputKind,
    ExternalInputWidget,
};
use runenui_runtime::{AppRuntime, PumpBudget, TraceRecordKind};

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
