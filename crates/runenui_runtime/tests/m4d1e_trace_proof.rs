#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, CompositionGeneration, Element, NoHostProtocol, SemanticCommand, UiApp, View,
    Widget, WidgetTextInput, column,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, RuntimeTerminalReason,
    TraceDeliveryOutcome, TraceInputRecordRole, TraceRecordKind, TraceRoutedIntegrityFailure,
    TraceSequence,
};

#[derive(Clone, Copy)]
enum Action {
    Remove,
    Noop,
}

struct State {
    target_present: bool,
}

#[derive(Debug)]
struct CompositionTarget;

impl Widget<Action> for CompositionTarget {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
}

struct App;

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // A column and an optional composition target, without initial output.
        runenui_core::ProducerBudget::new(2, 0)
            .with_reconciliation(2, 0, 2)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(2, 2, 2)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // RequestFocus derives one callback-only FocusIn notification.
        // The source and its required terminal continuation coexist.
        runenui_core::ProducerBudget::new(2, 1).with_application_outputs(0)
    }

    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let children = if state.target_present {
            vec![
                Element::new(CompositionTarget)
                    .id("target")
                    .key("target")
                    .focusable(true),
            ]
        } else {
            Vec::new()
        };
        column(children).id("root").key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if matches!(action, Action::Remove) {
            state.target_present = false;
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    let report = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn focus_target(runtime: &mut AppRuntime<App>) {
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "target"))
        .unwrap_or_else(|| unreachable!("composition target is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("composition target accepts focus"));
    settle(runtime);
}

fn descends_from(
    runtime: &AppRuntime<App>,
    mut child: TraceSequence,
    ancestor: TraceSequence,
) -> bool {
    loop {
        if child == ancestor {
            return true;
        }
        let Some(record) = runtime
            .trace()
            .records()
            .find(|record| record.sequence() == child)
        else {
            return false;
        };
        let Some(parent) = record.causal_parent() else {
            return false;
        };
        child = parent;
    }
}

fn trigger_cleanup_integrity_failure(runtime: &mut AppRuntime<App>) {
    runtime
        .submit_action(Action::Remove)
        .unwrap_or_else(|_| unreachable!("removal enters the queue"));
    runtime
        .submit_action(Action::Noop)
        .unwrap_or_else(|_| unreachable!("first filler enters the queue"));
    runtime
        .submit_action(Action::Noop)
        .unwrap_or_else(|_| unreachable!("second filler enters the queue"));

    // Ordinary queue pressure now preserves funded cleanup. Inject a genuine
    // callback bridge integrity failure to retain TRACE-EVENT-10's terminal
    // ancestry proof without treating valid accepted work as an overload error.
    runtime.__fail_routed_callback_bridge_for_test();

    let report = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
}

fn assert_suppressed_cleanup_chain(
    runtime: &AppRuntime<App>,
    generation: &CompositionGeneration,
) -> TraceSequence {
    let rejection = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RoutedIntegrityFailed {
                    failure: TraceRoutedIntegrityFailure::CallbackBridgeFailure
                }
            )
        })
        .last()
        .unwrap_or_else(|| unreachable!("cleanup bridge integrity failure is traced"));
    let rejection_sequence = rejection.sequence();

    let retired = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionRetired)
                && record.context().delivery() == Some(TraceDeliveryOutcome::Suppressed)
        })
        .unwrap_or_else(|| unreachable!("failed cleanup retires exact composition lifetime"));
    let cleanup = retired
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("retirement owns typed cleanup context"));
    assert_eq!(cleanup.role(), TraceInputRecordRole::CompositionCleanup);
    assert_eq!(cleanup.delivery(), Some(TraceDeliveryOutcome::Suppressed));
    assert_eq!(
        cleanup
            .composition()
            .unwrap_or_else(|| unreachable!("cleanup retains composition identity"))
            .generation(),
        generation
    );
    assert_eq!(retired.causal_parent(), Some(rejection_sequence));
    let retired_sequence = retired.sequence();

    let terminal = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RuntimeTerminal {
                    reason: RuntimeTerminalReason::Poisoned
                }
            )
        })
        .unwrap_or_else(|| unreachable!("cleanup failure terminalizes runtime"));
    assert_eq!(terminal.causal_parent(), Some(retired_sequence));
    let terminal_sequence = terminal.sequence();

    let cancelled = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::QueuedWorkCancelled { count: 2 }
            )
        })
        .unwrap_or_else(|| unreachable!("terminal transition cancels queued filler work"));
    assert_eq!(cancelled.causal_parent(), Some(terminal_sequence));
    terminal_sequence
}

fn assert_idempotent_shutdown_from_terminal(
    runtime: &mut AppRuntime<App>,
    terminal_sequence: TraceSequence,
) {
    let first_shutdown = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(!first_shutdown.already_complete());
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
    let shutdown = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::RuntimeShutdown { .. }))
        .last()
        .unwrap_or_else(|| unreachable!("terminal runtime records explicit shutdown"));
    assert!(
        descends_from(runtime, shutdown.sequence(), terminal_sequence),
        "shutdown ancestry remains connected to the terminal transition"
    );
    let shutdown_records = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::RuntimeShutdown { .. }))
        .count();

    let second_shutdown = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(second_shutdown.already_complete());
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(record.kind(), TraceRecordKind::RuntimeShutdown { .. }))
            .count(),
        shutdown_records,
        "idempotent shutdown records no second terminal history"
    );
}

#[test]
fn terminal_composition_failure_reconstructs_through_cancellation_and_idempotent_shutdown() {
    let config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            // Two lifetime credits plus the removal and both unary followers.
            .with_waiting_envelopes(5)
            .with_transaction_outputs(1),
    );
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            target_present: true,
        },
        config,
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    settle(&mut runtime);
    focus_target(&mut runtime);
    let composition = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("focused target accepts composition"));
    settle(&mut runtime);

    trigger_cleanup_integrity_failure(&mut runtime);
    let terminal_sequence = assert_suppressed_cleanup_chain(&runtime, composition.generation());
    assert_idempotent_shutdown_from_terminal(&mut runtime, terminal_sequence);
}
