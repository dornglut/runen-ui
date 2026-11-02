#![allow(refining_impl_trait)]

use core::task::Poll;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use runenui_core::{
    Effects, HostProtocol, IntoEffects, SendSubscriptionSink, SendSubscriptionSource,
    SendSubscriptionStartOutcome, SubscriptionSet, UiApp, View, WorkKey, text,
};
use runenui_runtime::{
    AppRuntime, HostRequestCancelError, HostResponseCompletionError, HostResponseError, PumpBudget,
    RuntimeConfig, RuntimeLimits, RuntimeStatus, RuntimeTerminalReason, SubscriptionDiagnostic,
    SubscriptionOwnerKind,
};

fn key(value: &str) -> WorkKey {
    WorkKey::new(value).unwrap_or_else(|_| unreachable!())
}

struct SubscriptionState {
    revision: u64,
    declare: bool,
    duplicate: bool,
    source_polls: Rc<Cell<usize>>,
    updates: usize,
}

struct SubscriptionApp;

impl UiApp for SubscriptionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(2, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 0, 0, 0))
            // The only nonduplicate stream key retains one subscription record.
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 1, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        // The local source can finish with None. A later update redeclares the
        // same desired stream and must fund its restart, not just retention.
        runenui_core::ProducerBudget::new(3, 0)
            .with_subscriptions(2, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 1, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = SubscriptionState;
    type Action = ();
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("subscriptions")
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.updates += 1;
    }

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        if !state.declare {
            return;
        }
        let polls = Rc::clone(&state.source_polls);
        let _ = subscriptions.try_local(
            key("app.stream"),
            state.revision,
            move |_: &mut core::task::Context<'_>| {
                let count = polls.get();
                polls.set(count + 1);
                Poll::Ready((count == 0).then_some(()))
            },
        );
        if state.duplicate {
            let _ = subscriptions.try_local(
                key("app.stream"),
                state.revision,
                |_: &mut core::task::Context<'_>| Poll::Ready(Some(())),
            );
        }
    }
}

fn subscription_state() -> SubscriptionState {
    SubscriptionState {
        revision: 1,
        declare: true,
        duplicate: false,
        source_polls: Rc::new(Cell::new(0)),
        updates: 0,
    }
}

#[test]
fn application_subscription_starts_initially_and_equal_identity_is_retained() {
    let mut runtime = AppRuntime::<SubscriptionApp>::mount(subscription_state())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state().updates, 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    let polls_after_item = runtime.state().source_polls.get();
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state().updates, 2);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.state().source_polls.get() > polls_after_item);
}

struct DuplicateSubscriptionApp;

impl UiApp for DuplicateSubscriptionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(2, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(2, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = ();
    type Action = ();
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("duplicate")
    }

    fn update((): &mut Self::State, (): Self::Action) {}

    fn subscriptions((): &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        {
            let _ =
                subscriptions.try_local(key("duplicate"), 0, |_: &mut core::task::Context<'_>| {
                    Poll::Ready(Some(()))
                });
        };
        {
            let _ =
                subscriptions.try_local(key("duplicate"), 0, |_: &mut core::task::Context<'_>| {
                    Poll::Ready(Some(()))
                });
        };
    }
}

#[test]
fn duplicate_subscription_key_is_diagnosed_and_starts_no_stream() {
    let mut runtime = AppRuntime::<DuplicateSubscriptionApp>::mount(())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    assert_eq!(
        runtime.subscription_diagnostics(),
        &[SubscriptionDiagnostic::DuplicateKey {
            owner: SubscriptionOwnerKind::Application,
            key: key("duplicate"),
        }]
    );
}

#[test]
#[allow(clippy::assert_is_empty)]
fn subscription_diagnostic_retention_is_explicitly_bounded() {
    let limits = RuntimeLimits::default().with_subscription_diagnostics(1);
    let mut runtime = AppRuntime::<DuplicateSubscriptionApp>::mount_with_config(
        (),
        RuntimeConfig::default().with_limits(limits),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.subscription_diagnostics().len(), 1);

    let disabled_limits = RuntimeLimits::default().with_subscription_diagnostics(0);
    let mut disabled = AppRuntime::<DuplicateSubscriptionApp>::mount_with_config(
        (),
        RuntimeConfig::default().with_limits(disabled_limits),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = disabled
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(disabled.subscription_diagnostics().is_empty());
}

struct SendSubscriptionApp;

struct OneItemSource<Item> {
    sink: Arc<Mutex<Option<SendSubscriptionSink<Item>>>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl<Item: Send + 'static> SendSubscriptionSource<Item> for OneItemSource<Item> {
    fn start(self: Box<Self>, sink: SendSubscriptionSink<Item>) -> SendSubscriptionStartOutcome {
        *lock(&self.sink) = Some(sink);
        SendSubscriptionStartOutcome::Started
    }
}

impl UiApp for SendSubscriptionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(1, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 1, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 1, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(1, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = (
        Rc<Cell<bool>>,
        usize,
        Arc<Mutex<Option<SendSubscriptionSink<u8>>>>,
    );
    type Action = Rc<()>;
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("send subscription")
    }

    fn update(
        state: &mut Self::State,
        _: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.1 += 1;
    }

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        let ui_state = Rc::clone(&state.0);
        let _ = subscriptions.try_send(
            key("send.stream"),
            0,
            OneItemSource {
                sink: Arc::clone(&state.2),
            },
            move |_| {
                ui_state.set(true);
                Rc::new(())
            },
        );
    }
}

#[test]
fn send_subscription_item_uses_ingress_without_requiring_action_send() {
    let mapped = Rc::new(Cell::new(false));
    let sink = Arc::new(Mutex::new(None));
    let mut runtime =
        AppRuntime::<SendSubscriptionApp>::mount((Rc::clone(&mapped), 0, Arc::clone(&sink)))
            .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let sender = lock(&sink)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("started source retained its sink"));
    std::thread::spawn(move || sender.try_send(1_u8))
        .join()
        .unwrap_or_else(|_| unreachable!("producer thread remains deterministic"))
        .unwrap_or_else(|_| unreachable!("post-start item is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(mapped.get());
    assert_eq!(runtime.state().1, 1);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Number(u8),
}

#[derive(Debug)]
enum Response {
    Number(u8),
    Text(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResponseKind {
    Number,
    Text,
}

struct Protocol;

impl HostProtocol for Protocol {
    type Command = Command;
    type Response = Response;
    type ResponseKind = ResponseKind;

    fn expected_response(_: &Self::Command) -> Self::ResponseKind {
        ResponseKind::Number
    }

    fn response_kind(response: &Self::Response) -> Self::ResponseKind {
        match response {
            Response::Number(_) => ResponseKind::Number,
            Response::Text(_) => ResponseKind::Text,
        }
    }
}

enum HostAction {
    Replace,
    Value(u8),
}

struct HostApp;

impl UiApp for HostApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 1, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 1, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let (credits, outputs, host_requests) = match action {
            HostAction::Replace => (3, 1, 1),
            HostAction::Value(_) => (1, 0, 0),
        };
        runenui_core::ProducerBudget::new(credits, outputs)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(
                0,
                0,
                0,
                host_requests,
                0,
            ))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(
                0,
                0,
                0,
                0,
                host_requests,
                0,
            ))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Vec<u8>;
    type Action = HostAction;
    type HostProtocol = Protocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("host")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        request(1)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            HostAction::Replace => request(2),
            HostAction::Value(value) => {
                state.push(value);
                Effects::none()
            }
        }
    }
}

fn request(value: u8) -> Effects<HostAction, Protocol> {
    Effects::host_request(
        Some(key("host.request")),
        Command::Number(value),
        |response| match response {
            Response::Number(value) => HostAction::Value(value),
            Response::Text(_) => unreachable!(),
        },
    )
}


struct HostMappedProfileState {
    exceeds_root: bool,
    mapped: Rc<Cell<usize>>,
    values: Vec<u8>,
}

struct HostMappedProfileApp;

impl UiApp for HostMappedProfileApp {
    type State = HostMappedProfileState;
    type Action = HostAction;
    type HostProtocol = Protocol;

    fn initial_producer_budget(_: &Self::State) -> runenui_core::ProducerBudget {
        HostApp::initial_producer_budget(&Vec::new())
    }

    fn action_producer_budget(
        state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        match action {
            HostAction::Value(_) => {
                runenui_core::ProducerBudget::new(if state.exceeds_root { 4 } else { 1 }, 0)
            }
            HostAction::Replace => runenui_core::ProducerBudget::new(1, 0),
        }
    }

    fn interaction_producer_budget(_: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("mapped host profile")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        let mapped = Rc::clone(&state.mapped);
        Effects::host_request(Some(key("host.mapped")), Command::Number(1), move |response| {
            mapped.set(mapped.get() + 1);
            match response {
                Response::Number(value) => HostAction::Value(value),
                Response::Text(_) => unreachable!("response kind was verified before mapping"),
            }
        })
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            HostAction::Value(value) => state.values.push(value),
            HostAction::Replace => unreachable!("the mapped host never creates a replace action"),
        }
        Effects::none()
    }
}

fn mapped_host_profile_runtime(exceeds_root: bool, trace_capacity: usize) -> (
    AppRuntime<HostMappedProfileApp>,
    Rc<Cell<usize>>,
    runenui_runtime::HostRequestToken,
) {
    let mapped = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<HostMappedProfileApp>::mount_with_config(
        HostMappedProfileState {
            exceeds_root,
            mapped: Rc::clone(&mapped),
            values: Vec::new(),
        },
        RuntimeConfig::default()
            .with_trace_config(runenui_runtime::TraceConfig::new(trace_capacity)),
    )
    .unwrap_or_else(|error| unreachable!("finite initial host producer fits: {error}"));
    runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("initial host work reaches pending request"));
    let requests = runtime.pending_host_requests();
    assert_eq!(requests.len(), 1);
    let token = requests[0].token();
    (runtime, mapped, token)
}

#[test]
fn direct_mapped_host_undercovered_child_terminalizes_without_unwind() {
    for trace_capacity in [0, 256] {
        let (mut runtime, mapper_calls, token) =
            mapped_host_profile_runtime(true, trace_capacity);
        let outcome = runtime.complete_host_request(&token, Response::Number(7));
        assert!(outcome.is_err(), "the mapped action exceeds the originating root");
        assert_eq!(mapper_calls.get(), 1, "mapping consumed the response once");
        assert!(runtime.state().values.is_empty(), "no child application update executed");
        assert!(matches!(runtime.status(), RuntimeStatus::Terminal(_)));
        assert!(runtime.pending_host_requests().is_empty());
        assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
        if trace_capacity != 0 {
            let kinds: Vec<_> = runtime.trace().kinds().collect();
            assert!(kinds.contains(&runenui_runtime::TraceRecordKind::HostResponseAccepted));
            assert!(kinds.contains(&runenui_runtime::TraceRecordKind::WorkCompletionMapped));
            assert!(kinds.iter().any(|kind| matches!(
                kind,
                runenui_runtime::TraceRecordKind::RuntimeTerminal { .. }
            )));
            assert!(!kinds.contains(&runenui_runtime::TraceRecordKind::ActionSubmissionAccepted));
        }
        runtime.shutdown().unwrap_or_else(|_| unreachable!("shutdown cleans terminal work"));
    }
}

#[test]
fn direct_mapped_host_covered_child_commits_and_preserves_trace_chain() {
    for trace_capacity in [0, 256] {
        let (mut runtime, mapper_calls, token) =
            mapped_host_profile_runtime(false, trace_capacity);
        runtime
            .complete_host_request(&token, Response::Number(7))
            .unwrap_or_else(|_| unreachable!("covered mapped child enters FIFO"));
        assert_eq!(mapper_calls.get(), 1);
        assert!(runtime.state().values.is_empty(), "action has not been pumped yet");
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("mapped child is processed"));
        assert_eq!(runtime.state().values, [7]);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert!(runtime.pending_host_requests().is_empty());
        assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
        if trace_capacity != 0 {
            let records: Vec<_> = runtime.trace().records().collect();
            let mapped = records.iter().find(|record| {
                matches!(record.kind(), runenui_runtime::TraceRecordKind::WorkCompletionMapped)
            }).unwrap_or_else(|| unreachable!("host mapping is recorded"));
            assert!(records.iter().any(|record| {
                matches!(record.kind(), runenui_runtime::TraceRecordKind::ActionSubmissionAccepted)
                    && record.causal_parent() == Some(mapped.sequence())
            }));
        }
        runtime.shutdown().unwrap_or_else(|_| unreachable!("normal shutdown"));
    }
}

struct HostFrontierState {
    queue_credits: usize,
    cancel_during_fanout: bool,
    values: Vec<u8>,
}

enum HostFrontierAction {
    Fanout,
    Value(u8),
}

struct HostFrontierApp;

impl UiApp for HostFrontierApp {
    fn initial_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(
            state.queue_credits,
            if state.cancel_during_fanout { 4 } else { 3 },
        )
        .with_reconciliation(1, 0, 1)
        .with_subscriptions(0, 0)
        .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 1, 0))
        .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 1, 0))
    }

    fn action_producer_budget(
        state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        match action {
            HostFrontierAction::Fanout if state.cancel_during_fanout => {
                runenui_core::ProducerBudget::new(4, 4)
            }
            HostFrontierAction::Fanout => runenui_core::ProducerBudget::new(3, 3),
            HostFrontierAction::Value(_) => runenui_core::ProducerBudget::new(1, 0),
        }
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = HostFrontierState;
    type Action = HostFrontierAction;
    type HostProtocol = Protocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("host frontier")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::host_request(Some(key("host.frontier")), Command::Number(1), |_| {
            HostFrontierAction::Value(99)
        })
        .then(Effects::action(HostFrontierAction::Fanout))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            HostFrontierAction::Fanout => {
                let output = Effects::action(HostFrontierAction::Value(1))
                    .then(Effects::action(HostFrontierAction::Value(2)))
                    .then(Effects::action(HostFrontierAction::Value(3)));
                if state.cancel_during_fanout {
                    output.then(Effects::cancel(
                        runenui_core::WorkFamily::HostRequest,
                        key("host.frontier"),
                    ))
                } else {
                    output
                }
            }
            HostFrontierAction::Value(value) => {
                state.values.push(value);
                Effects::none()
            }
        }
    }
}

const fn host_frontier_state(queue_credits: usize) -> HostFrontierState {
    HostFrontierState {
        queue_credits,
        cancel_during_fanout: false,
        values: Vec::new(),
    }
}

#[test]
fn host_effect_cancellation_keeps_cancel_credit_at_same_root_p0() {
    let mut state = host_frontier_state(5);
    state.cancel_during_fanout = true;
    let mut runtime = AppRuntime::<HostFrontierApp>::mount(state).unwrap_or_else(|error| {
        unreachable!("source-sized host cancellation frontier is admitted: {error}")
    });

    let report = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("all causal work is pumpable"))
        .report()
        .to_owned();

    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state().values, [1, 2, 3]);
    assert_eq!(report.remaining_queued_envelopes(), 0);
    assert!(runtime.pending_host_requests().is_empty());
}

#[test]
fn host_cancellation_credit_survives_same_root_positive_fanout() {
    let mut runtime = AppRuntime::<HostFrontierApp>::mount(host_frontier_state(5))
        .unwrap_or_else(|error| unreachable!("exact frontier profile is admitted: {error}"));
    let report = runtime
        .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("bounded pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 3);
    let requests = runtime.pending_host_requests();
    assert_eq!(requests.len(), 1);
    let token = requests[0].token();
    drop(requests);

    runtime.cancel_host_request(&token).unwrap_or_else(|error| {
        unreachable!("same-root fanout preserves prepaid cancellation: {error:?}")
    });
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    let report = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("children and cancellation are pumpable"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 4);
    assert_eq!(runtime.state().values, [1, 2, 3]);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn initial_host_start_refuses_before_outputs_when_frontier_cannot_fund_cancel() {
    let Err(error) = AppRuntime::<HostFrontierApp>::mount(host_frontier_state(3)) else {
        unreachable!("three startup envelopes plus cancellation need four credits");
    };
    assert_eq!(
        error.kind(),
        runenui_runtime::InitialAdmissionErrorKind::ProducerContractViolation(
            runenui_core::ProducerContractViolation {
                kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                phase: runenui_core::ProducerPhase::InitialEffects,
                component: runenui_core::BudgetComponent::QueueFrontier,
                declared: 3,
                attempted: Some(4),
            }
        )
    );
    assert_eq!(error.frontier_requirement(), Some((4, 3)));
    let (state, _) = error.into_parts();
    assert_eq!(state.values, Vec::<u8>::new());
}

#[test]
fn host_fanout_profile_overrun_commits_no_child_outputs() {
    let mut runtime =
        AppRuntime::<HostFrontierApp>::mount(host_frontier_state(4)).unwrap_or_else(|error| {
            unreachable!("initial HostRequest and its cancellation fit: {error}")
        });
    let report = runtime
        .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("bounded pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 3);
    assert_eq!(runtime.state().values, Vec::<u8>::new());
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert!(runtime.pending_host_requests().is_empty());
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::ActionSubmissionAccepted
            ))
            .count(),
        1,
        "the initial fanout action was accepted, but none of its child actions were"
    );
}

fn assert_host_success_causal_chain(runtime: &AppRuntime<HostApp>) {
    let records: Vec<_> = runtime.trace().records().collect();
    let mapped = records
        .iter()
        .rfind(|record| {
            matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::WorkCompletionMapped
            ) && record.work().is_some_and(|identity| {
                identity.family() == runenui_runtime::TraceWorkFamily::HostRequest
            })
        })
        .unwrap_or_else(|| unreachable!("host response mapping is traced"));
    let parent = |record: &runenui_runtime::TraceRecord| {
        let sequence = record
            .causal_parent()
            .unwrap_or_else(|| unreachable!("host causal fact retains a parent"));
        records
            .iter()
            .find(|candidate| candidate.sequence() == sequence)
            .copied()
            .unwrap_or_else(|| unreachable!("host causal parent is retained"))
    };
    let response_accepted = parent(mapped);
    assert!(matches!(
        response_accepted.kind(),
        runenui_runtime::TraceRecordKind::HostResponseAccepted
    ));
    let mut exposed = parent(response_accepted);
    if matches!(
        exposed.kind(),
        runenui_runtime::TraceRecordKind::HostResponseRejected
    ) {
        exposed = parent(exposed);
    }
    assert!(matches!(
        exposed.kind(),
        runenui_runtime::TraceRecordKind::HostRequestExposed
    ));
    let start_accepted = parent(exposed);
    assert!(matches!(
        start_accepted.kind(),
        runenui_runtime::TraceRecordKind::WorkStartAccepted
    ));
    let attempted = parent(start_accepted);
    assert!(matches!(
        attempted.kind(),
        runenui_runtime::TraceRecordKind::WorkStartAttempted
    ));
    let committed = parent(attempted);
    assert!(matches!(
        committed.kind(),
        runenui_runtime::TraceRecordKind::WorkGenerationCommitted
    ));
    let requested = parent(committed);
    assert!(matches!(
        requested.kind(),
        runenui_runtime::TraceRecordKind::WorkRequested
    ));
    let action = records
        .iter()
        .find(|record| {
            matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::ActionSubmissionAccepted
            ) && record.causal_parent() == Some(mapped.sequence())
        })
        .unwrap_or_else(|| unreachable!("mapped host response accepts one final action"));
    let transaction = records
        .iter()
        .find(|record| {
            matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == action.work_sequence()
        })
        .unwrap_or_else(|| unreachable!("host action reaches application update"));
    assert_eq!(transaction.causal_parent(), Some(action.sequence()));
}

#[test]
#[allow(clippy::assert_is_empty)]
fn host_commands_are_exposed_after_start_and_map_only_valid_live_responses() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    assert!(runtime.pending_host_requests().is_empty());
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let first = runtime.pending_host_requests();
    assert_eq!(first.len(), 1);
    assert_eq!(runtime.__host_response_slot_count_for_test(), 1);
    assert_eq!(first[0].command(), &Command::Number(1));
    let stale_token = first[0].token();
    drop(first);
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    runtime
        .submit_action(HostAction::Replace)
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(matches!(
        runtime.complete_host_request(&stale_token, Response::Number(9)),
        Err(HostResponseError::Stale(Response::Number(9)))
    ));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    let current = runtime.pending_host_requests();
    assert_eq!(current[0].command(), &Command::Number(2));
    assert_eq!(runtime.__host_response_slot_count_for_test(), 1);
    let token = current[0].token();
    drop(current);
    let mismatch = Response::Text("wrong");
    assert!(matches!(
        runtime.complete_host_request(&token, mismatch),
        Err(HostResponseError::MismatchedKind(Response::Text("wrong")))
    ));
    runtime
        .complete_host_request(&token, Response::Number(7))
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &[7]);
    assert_eq!(runtime.__host_response_slot_count_for_test(), 0);

    assert_host_success_causal_chain(&runtime);
}

#[test]
fn host_cancellation_is_exact_and_suppresses_later_completion() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let requests = runtime.pending_host_requests();
    let token = requests[0].token();
    drop(requests);
    runtime
        .cancel_host_request(&token)
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
    assert!(matches!(
        runtime.complete_host_request(&token, Response::Number(1)),
        Err(HostResponseError::Stale(Response::Number(1)))
    ));
    assert!(matches!(
        runtime.cancel_host_request(&token),
        Err(HostRequestCancelError::Stale)
    ));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn repeated_host_cancellation_and_replacement_retain_only_live_authority() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    for _ in 0..10_000 {
        let token = runtime.pending_host_requests()[0].token();
        runtime
            .cancel_host_request(&token)
            .unwrap_or_else(|_| unreachable!("current request cancels exactly once"));
        let _ = runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
        assert_eq!(runtime.__live_work_record_count_for_test(), 0);

        runtime
            .submit_action(HostAction::Replace)
            .unwrap_or_else(|_| unreachable!("replacement action remains bounded"));
        let _ = runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        assert_eq!(runtime.__host_response_slot_count_for_test(), 1);
        assert_eq!(runtime.__live_work_record_count_for_test(), 1);
    }
    let token = runtime.pending_host_requests()[0].token();
    runtime
        .cancel_host_request(&token)
        .unwrap_or_else(|_| unreachable!("final request cancels"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
    assert_eq!(runtime.__live_work_record_count_for_test(), 0);
    assert_eq!(runtime.__completion_payload_count_for_test(), 0);
}

struct TraceBoundaryHostState {
    mapper_calls: Rc<Cell<usize>>,
    updates: usize,
}

struct TraceBoundaryHostApp;

impl UiApp for TraceBoundaryHostApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 1, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 1, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = TraceBoundaryHostState;
    type Action = u8;
    type HostProtocol = Protocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("trace-boundary-host")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        let mapper_calls = Rc::clone(&state.mapper_calls);
        Effects::host_request(None, Command::Number(1), move |response| {
            mapper_calls.set(mapper_calls.get() + 1);
            match response {
                Response::Number(value) => value,
                Response::Text(_) => unreachable!(),
            }
        })
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        state.updates += 1;
    }
}

#[cfg(feature = "internal-test-seams")]
fn trace_boundary_host_runtime() -> (
    AppRuntime<TraceBoundaryHostApp>,
    Rc<Cell<usize>>,
    runenui_runtime::HostRequestToken,
) {
    let mapper_calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<TraceBoundaryHostApp>::mount(TraceBoundaryHostState {
        mapper_calls: Rc::clone(&mapper_calls),
        updates: 0,
    })
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    (runtime, mapper_calls, token)
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn detached_host_completion_admits_its_exact_four_record_plan_beside_publication_authority() {
    let (mut runtime, mapper_calls, token) = trace_boundary_host_runtime();
    let completion = runtime
        .host_response_completion(&token, Response::Number(7))
        .unwrap_or_else(|_| unreachable!("live request creates a completion"));
    completion
        .submit()
        .unwrap_or_else(|_| unreachable!("live completion enters ingress"));
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 4);
    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(mapper_calls.get(), 1, "status: {:?}", runtime.status());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
    let kinds: Vec<_> = runtime.trace().kinds().collect();
    let tail = &kinds[kinds.len() - 4..];
    assert!(matches!(
        tail[0],
        runenui_runtime::TraceRecordKind::WorkCompletionImported
    ));
    assert!(matches!(
        tail[1],
        runenui_runtime::TraceRecordKind::HostResponseAccepted
    ));
    assert!(matches!(
        tail[2],
        runenui_runtime::TraceRecordKind::WorkCompletionMapped
    ));
    assert!(matches!(
        tail[3],
        runenui_runtime::TraceRecordKind::ActionSubmissionAccepted
    ));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn detached_host_completion_with_only_three_unreserved_records_never_runs_mapper() {
    let (mut runtime, mapper_calls, token) = trace_boundary_host_runtime();
    let completion = runtime
        .host_response_completion(&token, Response::Number(7))
        .unwrap_or_else(|_| unreachable!("live request creates a completion"));
    completion
        .submit()
        .unwrap_or_else(|_| unreachable!("live completion enters ingress"));
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 3);
    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(mapper_calls.get(), 0);
    assert_eq!(runtime.state().updates, 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
    assert_eq!(runtime.__completion_payload_count_for_test(), 0);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn host_cancellation_preserves_another_roots_reserved_sequence_at_max() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("initial host start must be processed"));
    let token = runtime.pending_host_requests()[0].token();

    // The independent action retains two unsequenced credits in another root.
    // A host cancellation cannot spend MAX and steal the other root's promise.
    runtime
        .submit_action(HostAction::Replace)
        .unwrap_or_else(|_| unreachable!("independent producer admission fits"));
    runtime.__seed_next_work_sequence_for_test(u64::MAX);

    assert!(matches!(
        runtime.cancel_host_request(&token),
        Err(HostRequestCancelError::Terminal(
            RuntimeTerminalReason::WorkSequenceExhausted
        ))
    ));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert!(runtime.pending_host_requests().is_empty());
    assert!(runtime.state().is_empty());
}

#[test]
#[allow(clippy::assert_is_empty)]
fn host_cancellation_sequence_exhaustion_terminalizes_and_closes_authority() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    let completion = runtime
        .host_response_completion(&token, Response::Number(5))
        .unwrap_or_else(|_| unreachable!());
    runtime.__seed_next_work_sequence_for_test(0);

    assert!(matches!(
        runtime.cancel_host_request(&token),
        Err(HostRequestCancelError::Terminal(
            RuntimeTerminalReason::WorkSequenceExhausted
        ))
    ));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert_eq!(runtime.__host_response_slot_count_for_test(), 0);
    assert!(runtime.pending_host_requests().is_empty());
    assert!(matches!(
        completion.submit(),
        Err(HostResponseCompletionError::Closed(_))
    ));
    assert!(matches!(
        runtime.cancel_host_request(&token),
        Err(HostRequestCancelError::Terminal(
            RuntimeTerminalReason::WorkSequenceExhausted
        ))
    ));
    let Err(error) = runtime.submit_action(HostAction::Replace) else {
        unreachable!("terminal runtime rejects the exact action");
    };
    assert!(matches!(error.into_action(), HostAction::Replace));
    let _ = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    let _ = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
}

#[test]
fn live_host_cancellation_uses_its_reserved_credit_under_unrelated_queue_pressure() {
    let config = RuntimeConfig::default().with_queue_capacity(5);
    let mut runtime = AppRuntime::<HostApp>::mount_with_config(Vec::new(), config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    runtime
        .submit_action(HostAction::Value(1))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(HostAction::Value(2))
        .unwrap_or_else(|_| unreachable!());

    let cancellation = runtime
        .cancel_host_request(&token)
        .unwrap_or_else(|error| unreachable!("live host request owns cleanup credit: {error:?}"));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.pending_host_requests().is_empty());
    assert!(cancellation.get() > 0);
    let drained = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| {
            unreachable!("reserved cancellation and unrelated actions are pumpable")
        })
        .report()
        .to_owned();
    assert_eq!(drained.processed_envelopes(), 3);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state(), &[1, 2]);
}

#[cfg(feature = "internal-test-seams")]
#[test]
#[allow(clippy::assert_is_empty)]
fn one_remaining_sequence_is_the_final_host_mapper_action() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    runtime.__seed_next_work_sequence_for_test(u64::MAX);

    let sequence = runtime
        .complete_host_request(&token, Response::Number(7))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(sequence.get(), u64::MAX);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.state().is_empty());
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::WorkCompletionMapped
            ))
            .count(),
        1
    );

    let Err(error) = runtime.submit_action(HostAction::Replace) else {
        unreachable!("the next sequence request terminalizes before another callback");
    };
    assert!(matches!(error.into_action(), HostAction::Replace));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
}

#[test]
#[allow(clippy::assert_is_empty)]
fn cancellation_invalidates_accepted_detached_response_before_ui_mapping() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    let completion = runtime
        .host_response_completion(&token, Response::Number(9))
        .unwrap_or_else(|_| unreachable!());
    let late = runtime
        .host_response_completion(&token, Response::Number(10))
        .unwrap_or_else(|_| unreachable!());
    completion.submit().unwrap_or_else(|_| unreachable!());
    runtime
        .cancel_host_request(&token)
        .unwrap_or_else(|_| unreachable!());
    assert!(matches!(
        late.submit(),
        Err(HostResponseCompletionError::Stale(_))
    ));

    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(runtime.state().is_empty());
    assert!(matches!(
        runtime.complete_host_request(&token, Response::Number(11)),
        Err(HostResponseError::Stale(Response::Number(11)))
    ));
}

#[test]
fn concrete_send_host_response_crosses_ingress_before_ui_mapping() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let requests = runtime.pending_host_requests();
    let token = requests[0].token();
    drop(requests);
    let completion = runtime
        .host_response_completion(&token, Response::Number(11))
        .unwrap_or_else(|_| unreachable!());
    let submitted = std::thread::spawn(move || completion.submit().is_ok())
        .join()
        .unwrap_or_else(|_| unreachable!());
    assert!(submitted);
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &[11]);
}

#[test]
fn detached_host_completion_reserves_only_after_successful_ingress_acceptance() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();

    let unsent = runtime
        .host_response_completion(&token, Response::Number(20))
        .unwrap_or_else(|_| unreachable!());
    drop(unsent);
    let replacement = runtime
        .host_response_completion(&token, Response::Number(21))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .complete_host_request(&token, Response::Number(22))
        .unwrap_or_else(|_| unreachable!());
    assert!(matches!(
        replacement.submit(),
        Err(HostResponseCompletionError::Stale(_))
    ));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &[22]);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn cancellation_claims_before_detached_submission_and_retry_after_full_is_stale() {
    let limits = RuntimeLimits::default().with_completion_ingress(0);
    let mut runtime = AppRuntime::<HostApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default().with_limits(limits),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    let completion = runtime
        .host_response_completion(&token, Response::Number(32))
        .unwrap_or_else(|_| unreachable!());
    let Err(HostResponseCompletionError::Full(completion)) = completion.submit() else {
        unreachable!("full ingress leaves the response slot open")
    };
    runtime
        .cancel_host_request(&token)
        .unwrap_or_else(|_| unreachable!());
    assert!(matches!(
        completion.submit(),
        Err(HostResponseCompletionError::Stale(_))
    ));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(runtime.state().is_empty());
}

#[test]
fn replacement_and_shutdown_invalidate_detached_completion_ownership() {
    let mut replacement = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = replacement
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = replacement.pending_host_requests()[0].token();
    let completion = replacement
        .host_response_completion(&token, Response::Number(50))
        .unwrap_or_else(|_| unreachable!());
    replacement
        .submit_action(HostAction::Replace)
        .unwrap_or_else(|_| unreachable!());
    let _ = replacement
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(matches!(
        completion.submit(),
        Err(HostResponseCompletionError::Stale(_))
    ));

    let mut shutdown = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = shutdown
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = shutdown.pending_host_requests()[0].token();
    let completion = shutdown
        .host_response_completion(&token, Response::Number(51))
        .unwrap_or_else(|_| unreachable!());
    let _ = shutdown
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(matches!(
        completion.submit(),
        Err(HostResponseCompletionError::Closed(_))
    ));
}

#[test]
fn direct_completion_and_public_cancellation_each_arm_one_coalesced_wake() {
    let direct_wakes = Arc::new(AtomicUsize::new(0));
    let mut direct = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = direct
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let probe = Arc::clone(&direct_wakes);
    direct.set_wake_transport(move || {
        probe.fetch_add(1, Ordering::SeqCst);
    });
    let token = direct.pending_host_requests()[0].token();
    direct
        .complete_host_request(&token, Response::Number(60))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(direct_wakes.load(Ordering::SeqCst), 1);

    let cancellation_wakes = Arc::new(AtomicUsize::new(0));
    let mut cancellation = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = cancellation
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let probe = Arc::clone(&cancellation_wakes);
    cancellation.set_wake_transport(move || {
        probe.fetch_add(1, Ordering::SeqCst);
    });
    let token = cancellation.pending_host_requests()[0].token();
    cancellation
        .cancel_host_request(&token)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(cancellation_wakes.load(Ordering::SeqCst), 1);
}

#[test]
fn full_detached_host_submission_returns_ownership_without_reserving_request() {
    let limits = RuntimeLimits::default().with_completion_ingress(0);
    let mut runtime = AppRuntime::<HostApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default().with_limits(limits),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    let completion = runtime
        .host_response_completion(&token, Response::Number(30))
        .unwrap_or_else(|_| unreachable!());
    let Err(HostResponseCompletionError::Full(completion)) = completion.submit() else {
        unreachable!("zero-capacity ingress returns the exact completion");
    };
    drop(completion);

    runtime
        .complete_host_request(&token, Response::Number(31))
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &[31]);
}

#[test]
fn first_detached_host_submission_wins_and_later_submission_is_stale() {
    let mut runtime = AppRuntime::<HostApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let token = runtime.pending_host_requests()[0].token();
    let first = runtime
        .host_response_completion(&token, Response::Number(40))
        .unwrap_or_else(|_| unreachable!());
    let second = runtime
        .host_response_completion(&token, Response::Number(41))
        .unwrap_or_else(|_| unreachable!());
    first.submit().unwrap_or_else(|_| unreachable!());
    let Err(HostResponseCompletionError::Stale(second)) = second.submit() else {
        unreachable!("accepted generation rejects later detached submission");
    };
    drop(second);
    assert!(matches!(
        runtime.host_response_completion(&token, Response::Number(42)),
        Err(HostResponseError::Stale(Response::Number(42)))
    ));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &[40]);
}
