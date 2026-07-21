#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View, button, children, column,
    text,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, SubmitCommandErrorKind, TraceRecordKind,
};

#[derive(Debug)]
struct Action;

#[derive(Debug)]
struct State {
    updates: usize,
    factory_calls: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let calls = Rc::clone(&state.factory_calls);
        button("Activate")
            .id("activate")
            .key("activate")
            .on_activate(move || {
                calls.set(calls.get() + 1);
                Action
            })
            .into_element()
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        state.updates += 1;
    }
}

fn state(calls: &Rc<Cell<usize>>) -> State {
    State {
        updates: 0,
        factory_calls: Rc::clone(calls),
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[test]
fn exact_target_activate_routes_then_appends_its_action() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    settle(&mut runtime);
    let target = runtime.index().nodes()[0].id().clone();
    let submission = runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));

    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert_eq!(calls.get(), 1);
    assert_eq!(runtime.state().updates, 0);
    assert!(runtime.trace().records().any(|record| {
        matches!(record.kind(), TraceRecordKind::RoutedEventCommitted)
            && record.work_sequence() == Some(submission.sequence())
    }));

    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert_eq!(runtime.state().updates, 1);
}

#[test]
fn full_and_closed_submission_recover_without_invoking_the_factory() {
    let full_calls = Rc::new(Cell::new(0));
    let mut full = AppRuntime::<App>::mount_with_config(
        state(&full_calls),
        RuntimeConfig::default().with_queue_capacity(1),
    );
    settle(&mut full);
    let target = full.index().nodes()[0].id().clone();
    full.submit_action(Action)
        .unwrap_or_else(|_| unreachable!("the only queue slot is available"));
    let Err(error) = full.submit_command(
        target.clone(),
        SemanticCommand::Activate,
        CommandOrigin::automation(),
    ) else {
        unreachable!("the full queue rejects the command")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::Full);
    assert_eq!(error.into_unaccepted().into_parts().0, target);
    assert_eq!(full_calls.get(), 0);

    let closed_calls = Rc::new(Cell::new(0));
    let mut closed = AppRuntime::<App>::mount(state(&closed_calls));
    let target = closed.index().nodes()[0].id().clone();
    closed.shutdown();
    let Err(error) = closed.submit_command(
        target,
        SemanticCommand::Activate,
        CommandOrigin::controller(),
    ) else {
        unreachable!("the closed runtime rejects the command")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::Closed);
    assert_eq!(closed_calls.get(), 0);
}

#[test]
fn disabled_and_non_actionable_targets_route_without_activation_factory_output() {
    #[derive(Debug)]
    struct NoopApp;
    impl UiApp for NoopApp {
        type State = ();
        type Action = ();
        type HostProtocol = NoHostProtocol;
        fn root((): &Self::State) -> Element<Self::Action> {
            column(children![
                button("disabled")
                    .disabled()
                    .id("disabled")
                    .key("disabled")
                    .on_activate(|| ()),
                text("plain").id("plain").key("plain"),
            ])
            .key("root")
            .into_element()
        }
        fn update((): &mut Self::State, (): Self::Action) {}
    }
    let mut runtime = AppRuntime::<NoopApp>::mount(());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let targets: Vec<_> = runtime
        .index()
        .nodes()
        .iter()
        .skip(1)
        .map(|node| node.id().clone())
        .collect();
    for target in targets {
        runtime
            .submit_command(
                target,
                SemanticCommand::Activate,
                CommandOrigin::accessibility(),
            )
            .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    }
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        0
    );
}
