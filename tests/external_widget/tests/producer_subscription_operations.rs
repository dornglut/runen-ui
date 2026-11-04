#![allow(clippy::unwrap_used, clippy::panic)]
use runenui_core::*;
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, PumpBudget, RuntimeConfig, RuntimeStatus,
    RuntimeTerminalReason, TraceConfig,
};
use std::{
    cell::Cell,
    pin::Pin,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
#[derive(Clone)]
struct State {
    active: bool,
    send: bool,
    drops: Rc<Cell<usize>>,
    polls: Rc<Cell<usize>>,
    send_drops: Arc<AtomicUsize>,
    starts: Arc<AtomicUsize>,
    returned: Rc<Cell<bool>>,
}
struct Local {
    drops: Rc<Cell<usize>>,
    polls: Rc<Cell<usize>>,
}
impl Drop for Local {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
impl LocalSubscriptionSource<()> for Local {
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<()>> {
        self.polls.set(self.polls.get() + 1);
        Poll::Pending
    }
}
struct SendSource {
    drops: Arc<AtomicUsize>,
    starts: Arc<AtomicUsize>,
}
impl Drop for SendSource {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl SendSubscriptionSource<String> for SendSource {
    fn start(self: Box<Self>, _: SendSubscriptionSink<String>) -> SendSubscriptionStartOutcome {
        self.starts.fetch_add(1, Ordering::SeqCst);
        SendSubscriptionStartOutcome::Started
    }
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(_: &State) -> impl View<()> {
        text("subscription probe")
    }
    fn update(s: &mut State, (): ()) -> impl IntoUpdateOutput<(), NoHostProtocol> {
        s.active = true;
    }
    fn subscriptions(s: &State, cx: &mut SubscriptionSet<()>) {
        if !s.active {
            return;
        }
        let source = || Local {
            drops: s.drops.clone(),
            polls: s.polls.clone(),
        };
        cx.try_local(WorkKey::new("first").unwrap(), 1, source())
            .unwrap();
        if s.send {
            let original = SendSource {
                drops: s.send_drops.clone(),
                starts: s.starts.clone(),
            };
            let returned = s.returned.clone();
            let mapper = move |_: String| {
                returned.set(true);
            };
            let ((key, revision, original, mapper), cause) = cx
                .try_send(WorkKey::new("second").unwrap(), 7, original, mapper)
                .unwrap_err()
                .into_parts();
            assert_eq!(key, WorkKey::new("second").unwrap());
            assert_eq!(revision, 7);
            assert!(Arc::ptr_eq(&original.starts, &s.starts));
            assert_eq!(cx.producer_violation(), Some(cause));
            assert!(!s.returned.get());
            drop(original);
            drop(mapper);
        } else {
            let ((_, _, original), cause) = cx
                .try_local(WorkKey::new("second").unwrap(), 7, source())
                .unwrap_err()
                .into_parts();
            assert!(Rc::ptr_eq(&original.drops, &s.drops));
            assert_eq!(cx.producer_violation(), Some(cause));
            drop(original);
        }
        // Ignoring a later source cannot clear the first failed complete-set transaction.
        let _ = cx.try_local(WorkKey::new("third").unwrap(), 8, source());
    }
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &State, (): &()) -> ProducerBudget {
        ProducerBudget::new(8, 2)
            .with_reconciliation(1, 1, 0)
            .with_subscriptions(1, 0)
            .with_work_residency(WorkResidencyBudget::new(0, 0, 0, 1, 0, 0))
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 1, 0)
    }
}
#[test]
fn complete_subscription_set_overrun_returns_sources_and_starts_nothing() {
    let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    for send in [false, true] {
        for trace in [0, 512] {
            let state = State {
                active: false,
                send,
                drops: Rc::new(Cell::new(0)),
                polls: Rc::new(Cell::new(0)),
                send_drops: Arc::new(AtomicUsize::new(0)),
                starts: Arc::new(AtomicUsize::new(0)),
                returned: Rc::new(Cell::new(false)),
            };
            let mut app = AppRuntime::<App>::mount_with_config(
                state.clone(),
                RuntimeConfig::default().with_trace_config(TraceConfig::new(trace)),
            )
            .unwrap();
            app.pump(full).unwrap();
            app.submit_action(()).unwrap();
            let batch = app.pump(full).unwrap();
            assert!(matches!(
                app.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(_))
            ));
            assert!(app.state().active);
            assert_eq!(state.polls.get(), 0);
            assert_eq!(state.starts.load(Ordering::SeqCst), 0);
            assert!(!state.returned.get());
            assert_eq!(state.drops.get(), if send { 2 } else { 3 });
            assert_eq!(state.send_drops.load(Ordering::SeqCst), usize::from(send));
            assert!(
                !batch
                    .ordered_records()
                    .iter()
                    .any(|r| matches!(r, InputArbitrationRecord::InputSettled(_)))
            );
        }
    }
}
