#![allow(refining_impl_trait)]
// Owner-authorized UI475 final proof-only declarative mounted transaction.
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::{self, Receiver},
};

use runenui_core::{
    CommandOrigin, ControlledDeclarativeWidget, ControlledNoticeKind, ElementId, EventPhase,
    NoHostProtocol, SemanticCommand, SemanticEditableMode, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextField, TextPosition, TextSelection, UiApp,
    View, column, container,
};
use runenui_runtime::{
    AppRuntime, ControlledProcessingRefusalReason, MountedNodeId, PumpBudget, RuntimeConfig,
    RuntimeLimits, RuntimeStatus,
};

enum Action {
    Disable(Box<u64>),
    Follower,
    Mounted,
    Notice(&'static str),
}
struct State {
    enabled: bool,
    revision: u64,
    token: Box<u64>,
    processed: Vec<&'static str>,
    resource_slot: Rc<RefCell<Option<Receiver<u64>>>>,
    created: Rc<Cell<usize>>,
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
                .unwrap_or_else(|_| unreachable!("valid caret")),
        );
        let field = TextField::new(
            snapshot,
            "",
            selection,
            SemanticEditableMode::SingleLine,
            |_| Action::Mounted,
        )
        .unwrap_or_else(|_| unreachable!("valid field"))
        .disabled(!state.enabled)
        .id("editor")
        .key("editor");

        let slot = Rc::clone(&state.resource_slot);
        let created = Rc::clone(&state.created);
        let mut widget = ControlledDeclarativeWidget::new(state.revision, move || {
            created.set(created.get() + 1);
            Box::new(
                slot.borrow_mut()
                    .take()
                    .unwrap_or_else(|| unreachable!("non-Clone receiver created only once")),
            ) as Box<dyn Any>
        });
        if state.revision > 0 {
            widget = widget.on_update(Action::Mounted);
        }
        widget = widget
            .on_notice(
                ControlledNoticeKind::CompositionCancel,
                EventPhase::Capture,
                Action::Notice("ime-c"),
            )
            .on_notice(
                ControlledNoticeKind::CompositionCancel,
                EventPhase::Bubble,
                Action::Notice("ime-b"),
            )
            .on_notice(
                ControlledNoticeKind::FocusOut,
                EventPhase::Capture,
                Action::Notice("out-c"),
            )
            .on_notice(
                ControlledNoticeKind::FocusOut,
                EventPhase::Bubble,
                Action::Notice("out-b"),
            );
        column(vec![
            container(widget, vec![field.into_element()])
                .key("ancestor")
                .id("ancestor")
                .into_element(),
        ])
    }

    fn controlled_prepare(state: &Self::State, action: &Self::Action) -> Option<Self::State> {
        let Action::Disable(delta) = action else {
            return None;
        };
        Some(State {
            enabled: false,
            revision: state.revision + 1,
            token: Box::new(*state.token + **delta),
            processed: state.processed.clone(),
            resource_slot: Rc::clone(&state.resource_slot),
            created: Rc::clone(&state.created),
        })
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Disable(_) => unreachable!("controlled-only action"),
            Action::Follower => state.processed.push("follower"),
            Action::Mounted => state.processed.push("mounted"),
            Action::Notice(name) => state.processed.push(name),
        }
    }
}
fn state() -> (State, mpsc::Sender<u64>, Rc<Cell<usize>>) {
    let (tx, rx) = mpsc::channel();
    let created = Rc::new(Cell::new(0));
    (
        State {
            enabled: true,
            revision: 0,
            token: Box::new(7),
            processed: Vec::new(),
            resource_slot: Rc::new(RefCell::new(Some(rx))),
            created: Rc::clone(&created),
        },
        tx,
        created,
    )
}
fn target(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = ElementId::new(name).unwrap_or_else(|_| unreachable!("test id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("live node"))
        .id()
        .clone()
}
fn settle(runtime: &mut AppRuntime<App>) {
    let report = runtime
        .pump(PumpBudget::new(usize::MAX, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump"))
        .report()
        .to_owned();
    assert!(report.is_quiescent());
}
fn focused(
    config: RuntimeConfig,
) -> (
    AppRuntime<App>,
    MountedNodeId,
    mpsc::Sender<u64>,
    Rc<Cell<usize>>,
) {
    let (state, tx, created) = state();
    let mut runtime = AppRuntime::<App>::mount_with_config(state, config);
    settle(&mut runtime);
    let ancestor = target(&mut runtime, "ancestor");
    let editor = target(&mut runtime, "editor");
    runtime
        .submit_command(
            editor,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus"));
    settle(&mut runtime);
    runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("IME"));
    settle(&mut runtime);
    (runtime, ancestor, tx, created)
}
fn resource<'a>(runtime: &'a AppRuntime<App>, ancestor: &MountedNodeId) -> &'a Receiver<u64> {
    runtime
        .experiment_retained_resource(ancestor)
        .and_then(|resource| resource.downcast_ref::<Receiver<u64>>())
        .unwrap_or_else(|| unreachable!("exact retained non-Clone receiver"))
}

#[test]
#[allow(clippy::too_many_lines)]
fn staged_widget_and_routed_outputs_late_refusal_then_commit() {
    for (capacity, r, predecessors) in [(16, 4, 9), (4096, 1024, 2049)] {
        let config = RuntimeConfig::default().with_limits(
            RuntimeLimits::default()
                .with_waiting_envelopes(capacity)
                .with_transaction_outputs(r),
        );
        let (mut runtime, ancestor, tx, created) = focused(config);
        assert_eq!(created.get(), 1);
        assert_eq!(runtime.experiment_retained_revision(&ancestor), Some(0));
        tx.send(41)
            .unwrap_or_else(|_| unreachable!("live resource"));
        let first = Box::new(11);
        let ptr = std::ptr::from_ref(first.as_ref());
        let seq = runtime
            .submit_action(Action::Disable(first))
            .unwrap_or_else(|_| unreachable!("accepted"));
        for _ in 0..predecessors {
            runtime
                .submit_action(Action::Follower)
                .unwrap_or_else(|_| unreachable!("accepted follower"));
        }
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("pump"));
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert!(runtime.state().enabled);
        assert_eq!(*runtime.state().token, 7);
        assert_eq!(runtime.state().revision, 0);
        assert_eq!(runtime.experiment_retained_revision(&ancestor), Some(0));
        assert_eq!(resource(&runtime, &ancestor).try_recv().ok(), Some(41));
        assert_eq!(created.get(), 1);
        let mut refusals = runtime.take_controlled_processing_refusals();
        assert_eq!(refusals.len(), 1);
        let refusal = refusals.pop().unwrap_or_else(|| unreachable!("exact one"));
        assert_eq!(refusal.sequence, seq);
        assert_eq!(
            refusal.reason,
            ControlledProcessingRefusalReason::OrdinaryQueuePressure
        );
        let Action::Disable(original) = refusal.original else {
            unreachable!("owned action")
        };
        assert_eq!(std::ptr::from_ref(original.as_ref()), ptr);
        settle(&mut runtime);
        assert_eq!(runtime.state().processed, vec!["follower"; predecessors]);
        runtime
            .submit_action(Action::Disable(original))
            .unwrap_or_else(|_| unreachable!("retry accepted"));
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("retry pump"));
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert!(!runtime.state().enabled);
        assert_eq!(runtime.state().revision, 1);
        assert_eq!(*runtime.state().token, 18);
        assert_eq!(runtime.experiment_retained_revision(&ancestor), Some(1));
        assert_eq!(created.get(), 1);
        tx.send(42)
            .unwrap_or_else(|_| unreachable!("same resource"));
        assert_eq!(resource(&runtime, &ancestor).try_recv().ok(), Some(42));
        settle(&mut runtime);
        let done = &runtime.state().processed;
        assert_eq!(&done[..predecessors], &vec!["follower"; predecessors]);
        let order = ["ime-c", "ime-b", "out-c", "out-b"];
        let positions: Vec<usize> = order
            .iter()
            .map(|name| {
                done.iter()
                    .position(|item| item == name)
                    .unwrap_or_else(|| unreachable!("required authored notice"))
            })
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(done.iter().filter(|name| **name == "mounted").count(), 1);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
    }
}

#[test]
fn injected_failure_after_mounted_script_preparation_has_no_effects() {
    let config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            .with_waiting_envelopes(16)
            .with_transaction_outputs(4),
    );
    let (mut runtime, ancestor, tx, created) = focused(config);
    tx.send(77)
        .unwrap_or_else(|_| unreachable!("live resource"));
    runtime.experiment_reject_after_mounted_stage();
    let original = Box::new(13);
    let pointer = std::ptr::from_ref(original.as_ref());
    let seq = runtime
        .submit_action(Action::Disable(original))
        .unwrap_or_else(|_| unreachable!("accepted"));
    runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pumped"));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.state().enabled);
    assert_eq!(runtime.state().revision, 0);
    assert_eq!(*runtime.state().token, 7);
    assert_eq!(runtime.state().processed, Vec::<&'static str>::new());
    assert_eq!(runtime.experiment_retained_revision(&ancestor), Some(0));
    assert_eq!(created.get(), 1);
    assert_eq!(resource(&runtime, &ancestor).try_recv().ok(), Some(77));
    let pending = runtime
        .submit_action(Action::Follower)
        .unwrap_or_else(|_| unreachable!("accepted follower"));
    runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("backpressured"));
    assert!(
        runtime.state().processed.is_empty(),
        "one-slot refusal backpressure"
    );
    let mut refused = runtime.take_controlled_processing_refusals();
    assert_eq!(refused.len(), 1);
    let refusal = refused.pop().unwrap_or_else(|| unreachable!("one refusal"));
    assert_eq!(refusal.sequence, seq);
    assert_eq!(
        refusal.reason,
        ControlledProcessingRefusalReason::AfterWidgetStaging
    );
    let Action::Disable(token) = refusal.original else {
        unreachable!("owned original")
    };
    assert_eq!(std::ptr::from_ref(token.as_ref()), pointer);
    settle(&mut runtime);
    assert_eq!(runtime.state().processed, vec!["follower"]);
    assert_ne!(pending, seq);
    runtime
        .submit_action(Action::Disable(token))
        .unwrap_or_else(|_| unreachable!("retry"));
    settle(&mut runtime);
    assert_eq!(runtime.experiment_retained_revision(&ancestor), Some(1));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}
