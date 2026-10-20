#![allow(refining_impl_trait)]

// UI475-HYBRID-EXP: bounded proof-only experiment, never a production contract.
// Exercises REAL RunenUI TextField/IME/ancestor/retained execution with the
// unmerged #476 live-FocusOut correction ported onto accepted #474.
// An output-free authoring mode is a HYPOTHETICAL contract counterfactual,
// NOT an authorized change to currently guaranteed routed outputs.
use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CompositionEvent, Element, ElementId, EventContext,
    EventPhase, FocusEventKind, NoHostProtocol, SemanticCommand, SemanticEditableMode,
    TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextField,
    TextPosition, TextSelection, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    WidgetUpdateContext, column, container,
};
use runenui_runtime::{
    AppRuntime, MountedNodeId, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    RuntimeTerminalReason,
};

enum Action {
    Disable(Box<u64>),
    Follower,
    MountedEffect,
    AncestorEffect,
    Cascade(u8),
}

struct State {
    enabled: bool,
    revision: usize,
    emit_user_work: bool,
    cascade_next: Option<u8>,
    recursive_processed: usize,
    token: Box<u64>,
    processed_followers: usize,
    processed_mounted: usize,
    processed: Vec<&'static str>,
    observed: Rc<RefCell<Vec<&'static str>>>,
}

#[derive(Debug)]
struct Ancestor {
    observed: Rc<RefCell<Vec<&'static str>>>,
    emit_user_work: bool,
}

impl Widget<Action> for Ancestor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if event
            .as_focus()
            .is_some_and(|focus| focus.kind() == FocusEventKind::Out)
        {
            self.observed.borrow_mut().push("ancestor-focus-out");
            if self.emit_user_work
                && (context.phase() == EventPhase::Capture || context.phase() == EventPhase::Bubble)
            {
                context.emit(Action::AncestorEffect);
            }
        }
        if matches!(event, UiEvent::Composition(CompositionEvent::Cancel(_))) {
            self.observed.borrow_mut().push("ancestor-ime-cancel");
            if self.emit_user_work
                && matches!(context.phase(), EventPhase::Capture | EventPhase::Bubble)
            {
                context.emit(Action::AncestorEffect);
            }
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for Ancestor {}

#[derive(Debug)]
struct RetainedProbe {
    revision: usize,
    cascade_next: Option<u8>,
    observed: Rc<RefCell<Vec<&'static str>>>,
}

// A retained widget state contains an owned, non-Clone receiver. Staging may
// not duplicate, replay or restore this state through a generic Clone bound.
struct NonCloneMountedState {
    revision: Box<usize>,
    _receiver: std::sync::mpsc::Receiver<()>,
}

impl Widget<Action> for RetainedProbe {
    type State = NonCloneMountedState;

    fn create_state(&self) -> Self::State {
        let (_sender, receiver) = std::sync::mpsc::channel();
        NonCloneMountedState {
            revision: Box::new(self.revision),
            _receiver: receiver,
        }
    }

    fn update(&self, local: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *local.revision != self.revision {
            *local.revision = self.revision;
            self.observed.borrow_mut().push("mounted-update");
            context.emit(Action::MountedEffect);
            if let Some(next) = self.cascade_next {
                context.emit(Action::Cascade(next));
            }
        }
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(1), TextDocumentRevision::new(1));
        let selection = TextSelection::collapsed(
            TextPosition::new(snapshot, "", 0, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("valid empty document caret")),
        );
        let field = TextField::new(
            snapshot,
            "",
            selection,
            SemanticEditableMode::SingleLine,
            |_| Action::MountedEffect,
        )
        .unwrap_or_else(|_| unreachable!("valid TextField"))
        .disabled(!state.enabled)
        .id("editor")
        .key("editor");
        column(vec![
            container(
                Ancestor {
                    observed: Rc::clone(&state.observed),
                    emit_user_work: state.emit_user_work,
                },
                vec![
                    field.into_element(),
                    Element::new(RetainedProbe {
                        revision: state.revision,
                        cascade_next: state.cascade_next,
                        observed: Rc::clone(&state.observed),
                    })
                    .key("retained")
                    .into_element(),
                ],
            )
            .key("ancestor")
            .into_element(),
            Element::new(RetainedProbe {
                revision: 0,
                cascade_next: None,
                observed: Rc::clone(&state.observed),
            })
            .id("other")
            .focusable(true),
        ])
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Disable(payload) => {
                state.enabled = false;
                state.revision += 1;
                *state.token += *payload;
                state.observed.borrow_mut().push("application-update");
            }
            Action::Follower => {
                state.processed_followers += 1;
                state.processed.push("follower");
            }
            Action::MountedEffect => {
                state.processed_mounted += 1;
                state.processed.push("mounted");
            }
            Action::AncestorEffect => state.processed.push("ancestor"),
            Action::Cascade(depth) => {
                state.revision += 1;
                state.cascade_next = depth.checked_sub(1);
                state.recursive_processed += 1;
                state.observed.borrow_mut().push("cascade");
            }
        }
    }
}

fn state(emit_user_work: bool) -> State {
    State {
        enabled: true,
        revision: 0,
        emit_user_work,
        cascade_next: None,
        recursive_processed: 0,
        token: Box::new(7),
        processed_followers: 0,
        processed_mounted: 0,
        processed: Vec::new(),
        observed: Rc::new(RefCell::new(Vec::new())),
    }
}

fn target(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = ElementId::new(name).unwrap_or_else(|_| unreachable!("known test id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted test target"))
        .id()
        .clone()
}

fn settle(runtime: &mut AppRuntime<App>) {
    let report = runtime
        .pump(PumpBudget::new(usize::MAX, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("public pump"))
        .report()
        .to_owned();
    assert!(report.is_quiescent());
}


#[test]
fn live_compatible_focusout_retirement_and_earlier_fifo_are_observable() {
    // This test specifically exercises the old-live-route callback absent
    // from accepted main, using experimental #476 integration after #474.
    for emit_user_work in [true, false] {
        let mut runtime = AppRuntime::<App>::mount_with_config(
            state(emit_user_work),
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(16)
                    .with_transaction_outputs(4),
            ),
        );
        settle(&mut runtime);
        let editor = target(&mut runtime, "editor");
        runtime
            .submit_command(
                editor,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("editor focused"));
        settle(&mut runtime);
        runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("active IME"));

        // Complete accepted IME start before the disablement action.
        settle(&mut runtime);
        runtime
            .submit_action(Action::Disable(Box::new(11)))
            .unwrap_or_else(|_| unreachable!("original accepted"));
        for _ in 0..2 {
            runtime
                .submit_action(Action::Follower)
                .unwrap_or_else(|_| unreachable!("accepted predecessor"));
        }
        let report = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one app action"))
            .report()
            .to_owned();
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert!(!runtime.state().enabled);
        assert_eq!(*runtime.state().token, 18);
        let observed = runtime.state().observed.borrow();
        assert_eq!(
            observed.iter().filter(|event| **event == "ancestor-ime-cancel").count(),
            2,
            "live IME cancellation routed through C/T/B ancestor"
        );
        assert_eq!(
            observed.iter().filter(|event| **event == "ancestor-focus-out").count(),
            2,
            "live FocusOut reached original capture/bubble ancestor"
        );
        let cancel_at = observed
            .iter()
            .position(|event| *event == &"ancestor-ime-cancel")
            .unwrap_or_else(|| unreachable!("IME routed"));
        let out_at = observed
            .iter()
            .position(|event| *event == &"ancestor-focus-out")
            .unwrap_or_else(|| unreachable!("FocusOut routed"));
        assert!(cancel_at < out_at);
        drop(observed);

        settle(&mut runtime);
        assert_eq!(runtime.state().processed_followers, 2);
        assert_eq!(runtime.state().processed_mounted, 1);
        assert_eq!(&runtime.state().processed[..2], &["follower", "follower"]);
        let ancestors = runtime
            .state()
            .processed
            .iter()
            .filter(|action| **action == "ancestor")
            .count();
        assert_eq!(ancestors, if emit_user_work { 4 } else { 0 });
        // In the output-free counterfactual all framework callbacks still run.
        // With real guaranteed emissions all outputs follow pre-existing FIFO.
    }
}

#[test]
fn zero_user_output_does_not_fix_routed_reservation_or_atomicity() {
    for emit_user_work in [true, false] {
        let mut runtime = AppRuntime::<App>::mount_with_config(
            state(emit_user_work),
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(16)
                    .with_transaction_outputs(4),
            ),
        );
        settle(&mut runtime);
        let editor = target(&mut runtime, "editor");
        runtime
            .submit_command(
                editor,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("editor focus"));
        settle(&mut runtime);
        runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("active IME"));
        settle(&mut runtime);
        runtime
            .submit_action(Action::Disable(Box::new(11)))
            .unwrap_or_else(|_| unreachable!("original accepted"));
        // 9 earlier accepted FIFO followers plus 2*R=8 conservative
        // routed slots exceed C=16 after the initiating action pops.
        for _ in 0..9 {
            runtime
                .submit_action(Action::Follower)
                .unwrap_or_else(|_| unreachable!("accepted predecessor"));
        }
        let report = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one accepted action"))
            .report()
            .to_owned();
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(
            runtime.status(),
            RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned),
            "even callbacks that emit no app work face the unchanged full-R route admission"
        );
        assert!(!runtime.state().enabled, "app state changed before admission");
        assert_eq!(*runtime.state().token, 18);
        let observed = runtime.state().observed.borrow();
        assert!(observed.contains(&"application-update"));
        assert!(
            observed.contains(&"mounted-update"),
            "non-Clone mounted resource mutated before route admission"
        );
        assert!(
            !observed.contains(&"ancestor-focus-out"),
            "required callback was not reached before terminal failure"
        );
        assert_eq!(runtime.state().processed_followers, 0);
    }
}

#[test]
fn recursively_generated_mounted_work_has_no_whole_turn_finality() {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state(true),
        RuntimeConfig::default().with_limits(
            RuntimeLimits::default()
                .with_waiting_envelopes(16)
                .with_transaction_outputs(4),
        ),
    );
    settle(&mut runtime);
    runtime
        .submit_action(Action::Cascade(3))
        .unwrap_or_else(|_| unreachable!("accepted recursion origin"));
    settle(&mut runtime);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state().recursive_processed, 4);
    assert_eq!(runtime.state().processed_mounted, 4);
    assert_eq!(runtime.state().revision, 4);
    // This bounded, real mounted-output lineage terminates when lightly loaded.
    // It does not imply an enforceable bound for arbitrary recursive widgets.
}
