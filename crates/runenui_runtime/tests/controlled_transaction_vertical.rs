#![allow(refining_impl_trait)]

// #475 experimental controlled preparation through the one real AppRuntime.
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
    ControlledProcessingRefusalReason,
};

enum Action {
    Disable(Box<u64>),
    Follower,
    MountedEffect,
    AncestorEffect,
}

struct State {
    enabled: bool,
    revision: usize,
    token: Box<u64>,
    processed_followers: usize,
    processed_mounted: usize,
    processed: Vec<&'static str>,
    observed: Rc<RefCell<Vec<&'static str>>>,
}

#[derive(Debug)]
struct Ancestor {
    observed: Rc<RefCell<Vec<&'static str>>>,
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
            if context.phase() == EventPhase::Capture || context.phase() == EventPhase::Bubble {
                context.emit(Action::AncestorEffect);
            }
        }
        if matches!(event, UiEvent::Composition(CompositionEvent::Cancel(_))) {
            self.observed.borrow_mut().push("ancestor-ime-cancel");
            if matches!(context.phase(), EventPhase::Capture | EventPhase::Bubble) {
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
                },
                vec![
                    field.into_element(),
                    Element::new(RetainedProbe {
                        revision: state.revision,
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
                observed: Rc::clone(&state.observed),
            })
            .id("other")
            .focusable(true),
        ])
    }

    fn controlled_prepare(state: &Self::State, action: &Self::Action) -> Option<Self::State> {
        let Action::Disable(payload) = action else {
            return None;
        };
        // This opt-in closed authoring contract allocates a DISTINCT state.
        // The committed state and retained non-Clone receiver are untouched.
        Some(Self::State {
            enabled: false,
            revision: state.revision + 1,
            token: Box::new(*state.token + **payload),
            processed_followers: state.processed_followers,
            processed_mounted: state.processed_mounted,
            processed: state.processed.clone(),
            observed: Rc::clone(&state.observed),
        })
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Disable(_) => panic!("controlled disablement MUST NOT execute unrestricted updater"),
            Action::Follower => {
                state.processed_followers += 1;
                state.processed.push("follower");
            }
            Action::MountedEffect => {
                state.processed_mounted += 1;
                state.processed.push("mounted");
            }
            Action::AncestorEffect => state.processed.push("ancestor"),
        }
    }
}

fn state() -> State {
    State {
        enabled: true,
        revision: 0,
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
fn accepted_nonclone_action_refuses_without_mutation_and_retries_in_fifo() {
    for (capacity, followers) in [(16, 9), (4096, 2049)] {
        let mut runtime = AppRuntime::<App>::mount_with_config(
            state(),
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(capacity)
                    .with_transaction_outputs(if capacity == 16 { 4 } else { 1024 }),
            ),
        );
        settle(&mut runtime);
        let editor = target(&mut runtime, "editor");
        runtime.submit_command(
            editor,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        ).unwrap_or_else(|_| unreachable!("focus"));
        settle(&mut runtime);
        runtime.start_composition(None).unwrap_or_else(|_| unreachable!("start active IME"));
        settle(&mut runtime);
        let first = Box::new(11);
        let original_ptr = std::ptr::from_ref(first.as_ref());
        let original_sequence = runtime.submit_action(Action::Disable(first))
            .unwrap_or_else(|_| unreachable!("admitted original"));
        for _ in 0..followers {
            runtime.submit_action(Action::Follower)
                .unwrap_or_else(|_| unreachable!("accepted FIFO predecessor"));
        }
        let result = runtime.pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("pump one")).report().to_owned();
        assert_eq!(result.processed_envelopes(), 1);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert!(runtime.state().enabled);
        assert_eq!(runtime.state().revision, 0);
        assert_eq!(*runtime.state().token, 7);
        assert!(runtime.state().observed.borrow().is_empty(),
            "no application/mounted callback or FocusOut can leak at refusal");
        assert!(runtime.focused_text_input_capability().accepts_composition());
        let mut failures = runtime.take_controlled_processing_refusals();
        assert_eq!(failures.len(), 1);
        let failure = failures.pop().unwrap_or_else(|| unreachable!("one failure"));
        assert_eq!(failure.sequence, original_sequence);
        assert_eq!(failure.reason, ControlledProcessingRefusalReason::OrdinaryQueuePressure);
        let original = match failure.original {
            Action::Disable(value) => value,
            _ => unreachable!("returned original owned action"),
        };
        assert_eq!(std::ptr::from_ref(original.as_ref()), original_ptr,
            "non-Clone original identity is returned, not reconstructed");
        assert_eq!(*original, 11);

        // Every earlier already accepted FIFO follower processes as before;
        // refusal has not prepended, retried or fast-tracked the disablement.
        settle(&mut runtime);
        assert_eq!(runtime.state().processed_followers, followers);
        assert_eq!(runtime.state().processed, vec!["follower"; followers]);
        assert!(runtime.state().observed.borrow().is_empty());
        runtime.submit_action(Action::Disable(original))
            .unwrap_or_else(|_| unreachable!("accepted retry"));
        let result = runtime.pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one retry")).report().to_owned();
        assert_eq!(result.processed_envelopes(), 1);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert!(!runtime.state().enabled);
        assert_eq!(runtime.state().revision, 1);
        assert_eq!(*runtime.state().token, 18);
        let observed = runtime.state().observed.borrow();
        assert!(observed.contains(&"mounted-update"), "real retained update at commit");
        assert_eq!(observed.iter().filter(|event| **event == "ancestor-ime-cancel").count(), 2);
        assert_eq!(observed.iter().filter(|event| **event == "ancestor-focus-out").count(), 2);
        assert!(!observed.contains(&"application-update"), "legacy update was never invoked");
        drop(observed);
        settle(&mut runtime);
        assert_eq!(runtime.state().processed_mounted, 1);
        assert_eq!(runtime.state().processed_followers, followers);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
    }
}
