#![allow(refining_impl_trait)]

#[path = "../src/app.rs"]
mod app;
#[path = "../src/ui.rs"]
mod ui;

use app::{Counter, CounterApp};
use runenui_core::{
    ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
    LogicalKey, LogicalLength, PhysicalKey, SemanticCommand, StyleEnvironment,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceActionCategory,
    TraceAutomationRecordRole, TraceRecordKind, TraceSequence, WorkSequence,
};

const SURFACE_SIZE: LogicalSize = LogicalSize::new(
    match LogicalLength::new(240.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
    match LogicalLength::new(160.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
);

fn settle(runtime: &mut AppRuntime<CounterApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "counter did not settle: {report:?}");
}

fn authored_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("counter authored id is valid"))
}

const fn enter_down() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Enter,
        LogicalKey::Enter,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

fn descends_from(
    runtime: &AppRuntime<CounterApp>,
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

fn focus_increment_with_automation(runtime: &mut AppRuntime<CounterApp>) {
    let focus = runtime
        .submit_automation_command(
            authored_id("counter.increment"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("automation resolves the increment control"));
    settle(runtime);

    let resolution = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::AutomationResolutionUnique)
                && record.context().automation().is_some_and(|context| {
                    context.role() == TraceAutomationRecordRole::Unique
                        && context.authored_id() == &authored_id("counter.increment")
                        && context.command() == SemanticCommand::RequestFocus
                })
        })
        .unwrap_or_else(|| unreachable!("focus automation resolution is traced"));
    let focus_command = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.work_sequence() == Some(focus.sequence())
        })
        .unwrap_or_else(|| unreachable!("focus automation uses canonical command ingress"));
    assert_eq!(focus_command.causal_parent(), Some(resolution.sequence()));
}

fn activate_increment_with_keyboard(
    runtime: &mut AppRuntime<CounterApp>,
) -> (TraceSequence, WorkSequence) {
    let keyboard = runtime
        .submit_keyboard(enter_down())
        .unwrap_or_else(|_| unreachable!("focused increment accepts raw Enter"));
    settle(runtime);
    assert_eq!(runtime.state().count, 1);

    let keyboard_accepted = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::KeyboardSubmissionAccepted)
                && record.work_sequence() == Some(keyboard.sequence())
        })
        .unwrap_or_else(|| unreachable!("raw keyboard acceptance is traced"));
    let derived = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::KeyboardEnterActivationDerived
            ) && record.work_sequence() == Some(keyboard.sequence())
        })
        .unwrap_or_else(|| unreachable!("Enter derives semantic activation"));
    assert!(descends_from(
        runtime,
        derived.sequence(),
        keyboard_accepted.sequence()
    ));
    assert_eq!(derived.instant(), keyboard_accepted.instant());

    let default_command = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.causal_parent() == Some(derived.sequence())
        })
        .unwrap_or_else(|| unreachable!("Enter default enters canonical command FIFO"));
    let routed_action = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::RoutedActionCollected))
        .unwrap_or_else(|| unreachable!("semantic activation collects one routed action"));
    assert!(descends_from(
        runtime,
        routed_action.sequence(),
        default_command.sequence()
    ));

    let action = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.causal_parent() == Some(routed_action.sequence())
        })
        .unwrap_or_else(|| unreachable!("routed action enters canonical action FIFO"));
    let action_identity = action
        .context()
        .action()
        .unwrap_or_else(|| unreachable!("accepted Counter action owns redacted identity"));
    assert_eq!(
        action_identity.type_name(),
        core::any::type_name::<app::CounterAction>()
    );
    assert_eq!(
        action_identity.category(),
        TraceActionCategory::RoutedCommand
    );
    assert_eq!(action.instant(), routed_action.instant());
    let action_work = action
        .work_sequence()
        .unwrap_or_else(|| unreachable!("accepted action owns one work sequence"));
    (action.sequence(), action_work)
}

fn assert_update_reconciliation_and_publication(
    runtime: &mut AppRuntime<CounterApp>,
    action_sequence: TraceSequence,
    action_work: WorkSequence,
) {
    let transaction = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == Some(action_work)
        })
        .unwrap_or_else(|| unreachable!("accepted action starts application transaction"));
    assert_eq!(transaction.causal_parent(), Some(action_sequence));
    let transaction_instant = transaction.instant();

    let updated = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ApplicationStateUpdated)
                && record.work_sequence() == Some(action_work)
        })
        .unwrap_or_else(|| unreachable!("application update is traced"));
    assert_eq!(updated.causal_parent(), Some(action_sequence));
    assert_eq!(updated.instant(), transaction_instant);
    let reconciled = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::TreeReconciled)
                && record.work_sequence() == Some(action_work)
        })
        .unwrap_or_else(|| unreachable!("updated Counter tree is reconciled"));
    assert_eq!(reconciled.causal_parent(), Some(action_sequence));
    assert_eq!(reconciled.instant(), transaction_instant);

    let redraw = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::RedrawRequested { .. })
                && record.causal_parent() == Some(reconciled.sequence())
        })
        .unwrap_or_else(|| unreachable!("reconciliation requests redraw"));
    let redraw_sequence = redraw.sequence();
    let redraw_instant = redraw.instant();

    let style_environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(&style_environment, SURFACE_SIZE);
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("counter trace publication is admitted"));
    assert!(!publication.frame().nodes().is_empty());
    let published = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::SurfacePublished)
                && record.causal_parent() == Some(redraw_sequence)
        })
        .unwrap_or_else(|| unreachable!("redraw lineage reaches surface publication"));
    assert_eq!(published.instant(), redraw_instant);
}

#[test]
fn counter_public_trace_reconstructs_automation_keyboard_action_update_and_publication() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    settle(&mut runtime);
    focus_increment_with_automation(&mut runtime);
    let (action_sequence, action_work) = activate_increment_with_keyboard(&mut runtime);
    assert_update_reconciliation_and_publication(&mut runtime, action_sequence, action_work);
}
