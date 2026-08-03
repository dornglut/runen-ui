#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, CompositionCancelReason, CompositionEvent, Element, EventContext, EventPhase,
    NoHostProtocol, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetTextInput,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, SubmitCompositionErrorKind, TraceRecordKind, TraceTargetRejection,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Replace,
    Activate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputFact {
    OldCancel(CompositionCancelReason),
    NewComposition,
}

#[derive(Debug)]
struct State {
    replacement: bool,
    activations: usize,
    log: Rc<RefCell<Vec<InputFact>>>,
}

#[derive(Debug)]
struct OldWidget {
    log: Rc<RefCell<Vec<InputFact>>>,
}

impl Widget<Action> for OldWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && let UiEvent::Composition(CompositionEvent::Cancel(cancel)) = event
        {
            self.log
                .borrow_mut()
                .push(InputFact::OldCancel(cancel.reason()));
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        _: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::action(Action::Activate)
    }
}

#[derive(Debug)]
struct NewWidget {
    log: Rc<RefCell<Vec<InputFact>>>,
}

impl Widget<Action> for NewWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target && event.as_composition().is_some() {
            self.log.borrow_mut().push(InputFact::NewComposition);
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        _: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::action(Action::Activate)
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        if state.replacement {
            Element::new(NewWidget {
                log: Rc::clone(&state.log),
            })
            .id("target")
            .key("target")
            .focusable(true)
        } else {
            Element::new(OldWidget {
                log: Rc::clone(&state.log),
            })
            .id("target")
            .key("target")
            .focusable(true)
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Replace => state.replacement = true,
            Action::Activate => state.activations += 1,
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn mounted(log: Rc<RefCell<Vec<InputFact>>>) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(State {
        replacement: false,
        activations: 0,
        log,
    });
    settle(&mut runtime);
    runtime
}

fn target(runtime: &mut AppRuntime<App>) -> runenui_runtime::MountedNodeId {
    let authored =
        runenui_core::ElementId::new("target").unwrap_or_else(|_| unreachable!("valid id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("target remains mounted"))
        .id()
        .clone()
}

fn focus_target(runtime: &mut AppRuntime<App>) {
    let target = target(runtime);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is accepted"));
    settle(runtime);
}

#[test]
fn late_composition_never_retargets_a_same_slot_replacement() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(Rc::clone(&log));
    focus_target(&mut runtime);
    let old_target = target(&mut runtime);
    let old_parts = runtime
        .__mounted_identity_parts_for_test(&old_target)
        .unwrap_or_else(|| unreachable!("old target is local"));
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is accepted"));
    settle(&mut runtime);
    log.borrow_mut().clear();

    runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!("replacement is accepted"));
    settle(&mut runtime);
    let replacement = target(&mut runtime);
    let replacement_parts = runtime
        .__mounted_identity_parts_for_test(&replacement)
        .unwrap_or_else(|| unreachable!("replacement target is local"));

    assert_eq!(old_parts.0, replacement_parts.0, "arena slot is reused");
    assert_ne!(
        old_parts.1, replacement_parts.1,
        "replacement receives a new mounted generation"
    );
    assert_eq!(
        log.borrow().as_slice(),
        [InputFact::OldCancel(CompositionCancelReason::Replacement)]
    );
    log.borrow_mut().clear();

    assert!(matches!(
        runtime.submit_composition_update(
            start.generation().clone(),
            String::from("late"),
            None,
        ),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));
    assert!(matches!(
        runtime.submit_composition_end(start.generation().clone()),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));
    settle(&mut runtime);
    assert!(
        log.borrow().is_empty(),
        "replacement receives no late IME event"
    );
}

#[test]
fn accepted_automation_target_never_retargets_a_same_slot_replacement() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(log);
    let old_target = target(&mut runtime);
    let old_parts = runtime
        .__mounted_identity_parts_for_test(&old_target)
        .unwrap_or_else(|| unreachable!("old target is local"));

    runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!("replacement is queued first"));
    runtime
        .submit_automation_command(
            runenui_core::ElementId::new("target")
                .unwrap_or_else(|_| unreachable!("valid authored ID")),
            SemanticCommand::Activate,
        )
        .unwrap_or_else(|_| unreachable!("automation resolves the old lifetime"));

    let first = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(first.processed_envelopes(), 1);
    let replacement = target(&mut runtime);
    let replacement_parts = runtime
        .__mounted_identity_parts_for_test(&replacement)
        .unwrap_or_else(|| unreachable!("replacement target is local"));
    assert_eq!(old_parts.0, replacement_parts.0, "arena slot is reused");
    assert_ne!(
        old_parts.1, replacement_parts.1,
        "replacement receives a new mounted generation"
    );
    assert_eq!(runtime.state().activations, 0);

    settle(&mut runtime);
    assert_eq!(
        runtime.state().activations,
        0,
        "accepted command remains bound to the stale old generation"
    );
    assert!(
        runtime
            .trace()
            .records()
            .any(|record| matches!(record.kind(), TraceRecordKind::AutomationResolutionUnique))
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::CommandProcessingRejected {
            outcome: TraceTargetRejection::Stale
        }
    )));
}
